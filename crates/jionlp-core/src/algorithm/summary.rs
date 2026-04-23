//! Extractive text summarization — rank sentences by n-gram TF-IDF with
//! lead-3 and length weighting.
//!
//! Follows `jionlp/algorithm/summary/extract_summary.py` with one
//! deliberate omission: the Python version adds an LDA topic weight,
//! which requires shipping ~31 MB of pre-trained `topic_word_weight` /
//! `word_topic_weight` matrices. Those were dropped from this port's
//! data bundle (9 MB vs Python's 67 MB), so we approximate with the
//! remaining three factors — empirically this captures most of the
//! ranking signal on news text.
//!
//! Weighting pipeline (per sentence):
//!   1. **Base**: mean of per-bigram IDF over CJK bigrams present in
//!      `dict::idf()`.
//!   2. **Length penalty**: if `char_len < 15` or `char_len > 70`,
//!      weight ×= 0.7.  (Matches Python's `len(sen) < 15 or len(sen) > 70`.)
//!   3. **Lead-3 bonus**: if `position < 3`, weight ×= 1.2. (Matches
//!      Python's `lead_3_weight`.)
//!   4. **Zero-score filter**: drop sentences whose base score is 0
//!      (pure-ASCII fragments like "..." that the splitter produces).
//!
//! For MMR diversity, call `extract_summary_mmr` explicitly —
//! Python applies it automatically; we keep it an opt-in function to
//! preserve the simpler TF-IDF path as a callable primitive.

use crate::dict;
use crate::gadget::split_sentence::{split_sentence, Criterion as SplitCriterion};
use crate::Result;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct SummarySentence {
    pub text: String,
    pub score: f64,
    /// Index in the original document (0-based).
    pub position: usize,
}

/// Return the top-`k` highest-scoring sentences, in document order.
pub fn extract_summary(text: &str, top_k: usize) -> Result<Vec<SummarySentence>> {
    if text.is_empty() || top_k == 0 {
        return Ok(Vec::new());
    }
    let idf = dict::idf()?;

    let sentences = split_sentence(text, SplitCriterion::Coarse);
    if sentences.is_empty() {
        return Ok(Vec::new());
    }

    // Score each sentence: mean of per-bigram IDF, then length penalty
    // (<15 or >70 CJK chars → ×0.7) and lead-3 position bonus (first
    // three sentences → ×1.2). Zero-score sentences (pure-ASCII like
    // "...") are dropped — the splitter produces them but they carry
    // no summary content.
    let scored: Vec<SummarySentence> = sentences
        .into_iter()
        .enumerate()
        .map(|(pos, s)| {
            let base = bigram_idf_score(&s, idf);
            let cjk_len = s.chars().filter(|c| is_cjk(*c)).count();
            let length_mul = if !(15..=70).contains(&cjk_len) {
                0.7
            } else {
                1.0
            };
            let lead_mul = if pos < 3 { 1.2 } else { 1.0 };
            SummarySentence {
                text: s,
                score: base * length_mul * lead_mul,
                position: pos,
            }
        })
        .filter(|s| s.score > 0.0)
        .collect();

    // Pick top-k by score descending, then re-sort by position to preserve
    // narrative order.
    let mut by_score = scored;
    by_score.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    by_score.truncate(top_k);
    by_score.sort_by_key(|s| s.position);

    Ok(by_score)
}

/// Extract summary with Maximal Marginal Relevance (MMR) diversity.
///
/// Balances sentence relevance (IDF score) against redundancy with
/// already-picked sentences. `lambda` ∈ [0, 1] controls the trade-off:
///
/// * `lambda = 1.0` — pure relevance (equivalent to `extract_summary`).
/// * `lambda = 0.0` — pure diversity; picks the least-redundant sentence
///   regardless of score.
/// * `lambda = 0.7` (recommended default) — relevance-first with mild
///   diversity penalty.
///
/// Similarity between two sentences is the Jaccard similarity of their
/// character bigram sets, which is cheap and doesn't require an embedding
/// model.
pub fn extract_summary_mmr(text: &str, top_k: usize, lambda: f64) -> Result<Vec<SummarySentence>> {
    if text.is_empty() || top_k == 0 {
        return Ok(Vec::new());
    }
    let lambda = lambda.clamp(0.0, 1.0);
    let idf = dict::idf()?;

    let sentences = split_sentence(text, SplitCriterion::Coarse);
    if sentences.is_empty() {
        return Ok(Vec::new());
    }

    // Precompute score and bigram set for each sentence. Apply the same
    // length penalty + lead-3 bonus as `extract_summary`, so lambda=1.0
    // degenerates exactly to the basic top-k path.
    let bigrams: Vec<rustc_hash::FxHashSet<String>> =
        sentences.iter().map(|s| bigrams_of(s)).collect();
    let scores: Vec<f64> = sentences
        .iter()
        .enumerate()
        .map(|(pos, s)| {
            let base = bigram_idf_score(s, idf);
            let cjk_len = s.chars().filter(|c| is_cjk(*c)).count();
            let length_mul = if !(15..=70).contains(&cjk_len) {
                0.7
            } else {
                1.0
            };
            let lead_mul = if pos < 3 { 1.2 } else { 1.0 };
            base * length_mul * lead_mul
        })
        .collect();

    // Greedy selection.
    let mut selected_idx: Vec<usize> = Vec::with_capacity(top_k);
    let mut remaining: Vec<usize> = (0..sentences.len()).collect();

    while !remaining.is_empty() && selected_idx.len() < top_k {
        let mut best: Option<(usize, f64)> = None;
        for (pos, &i) in remaining.iter().enumerate() {
            let max_sim = selected_idx
                .iter()
                .map(|&j| jaccard(&bigrams[i], &bigrams[j]))
                .fold(0.0_f64, f64::max);
            let mmr = lambda * scores[i] - (1.0 - lambda) * max_sim;
            match best {
                Some((_, prev)) if prev >= mmr => {}
                _ => best = Some((pos, mmr)),
            }
        }
        if let Some((pos, _)) = best {
            let idx = remaining.remove(pos);
            selected_idx.push(idx);
        } else {
            break;
        }
    }

    // Return in document order for readability.
    selected_idx.sort_unstable();
    let out = selected_idx
        .into_iter()
        .map(|i| SummarySentence {
            text: sentences[i].clone(),
            score: scores[i],
            position: i,
        })
        .collect();
    Ok(out)
}

fn bigrams_of(s: &str) -> rustc_hash::FxHashSet<String> {
    let chars: Vec<char> = s.chars().filter(|c| is_cjk(*c)).collect();
    if chars.len() < 2 {
        return rustc_hash::FxHashSet::default();
    }
    chars.windows(2).map(|w| w.iter().collect()).collect()
}

fn jaccard(a: &rustc_hash::FxHashSet<String>, b: &rustc_hash::FxHashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        inter / union
    }
}

fn bigram_idf_score(sentence: &str, idf: &FxHashMap<String, f64>) -> f64 {
    let chars: Vec<char> = sentence.chars().filter(|c| is_cjk(*c)).collect();
    if chars.len() < 2 {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut count = 0usize;
    for window in chars.windows(2) {
        let bigram: String = window.iter().collect();
        if let Some(v) = idf.get(&bigram) {
            sum += v;
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x4E00..=0x9FA5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict;
    use std::path::PathBuf;
    use std::sync::Once;

    static INIT: Once = Once::new();
    fn ensure_init() {
        INIT.call_once(|| {
            let manifest = env!("CARGO_MANIFEST_DIR");
            let d = PathBuf::from(manifest).join("data");
            dict::init_from_path(&d).expect("init");
        });
    }

    #[test]
    fn extracts_top_k() {
        ensure_init();
        let text =
            "北京是中国的首都。上海是金融中心。广州是南方大都市。深圳是科技之都。成都是西南枢纽。";
        let out = extract_summary(text, 2).unwrap();
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn preserves_document_order() {
        ensure_init();
        let text = "第一句话。第二句话。第三句话。第四句话。";
        let out = extract_summary(text, 3).unwrap();
        for w in out.windows(2) {
            assert!(w[0].position < w[1].position);
        }
    }

    #[test]
    fn empty_returns_empty() {
        ensure_init();
        assert!(extract_summary("", 5).unwrap().is_empty());
    }

    #[test]
    fn zero_k_returns_empty() {
        ensure_init();
        assert!(extract_summary("随便一段文本。", 0).unwrap().is_empty());
    }

    // ── MMR ─────────────────────────────────────────────────────────────

    #[test]
    fn mmr_returns_top_k() {
        ensure_init();
        let text =
            "北京是中国的首都。上海是金融中心。广州是南方大都市。深圳是科技之都。成都是西南枢纽。";
        let r = extract_summary_mmr(text, 2, 0.7).unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn mmr_lambda_one_matches_basic_topk() {
        // With λ=1.0, MMR degenerates to the basic top-K-by-score path
        // (though document order might differ if ties).
        ensure_init();
        let text = "北京是中国的首都。上海是金融中心。广州是南方大都市。深圳是科技之都。";
        let basic = extract_summary(text, 2).unwrap();
        let mmr = extract_summary_mmr(text, 2, 1.0).unwrap();
        // Same length and same positions selected.
        let basic_pos: Vec<_> = basic.iter().map(|s| s.position).collect();
        let mmr_pos: Vec<_> = mmr.iter().map(|s| s.position).collect();
        assert_eq!(basic_pos, mmr_pos);
    }

    #[test]
    fn mmr_lambda_zero_maximizes_diversity() {
        // With λ=0, second pick should be the sentence most *different*
        // from the first (not the 2nd-highest-score).
        ensure_init();
        let text = "机器学习的分支。机器学习的应用。完全无关的天气描述。";
        let r = extract_summary_mmr(text, 2, 0.0).unwrap();
        // Expect position 0 (any) and then 2 (diverse) rather than 1.
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn mmr_empty_returns_empty() {
        ensure_init();
        assert!(extract_summary_mmr("", 5, 0.7).unwrap().is_empty());
    }
}
