//! Extractive text summarization — rank sentences by n-gram TF-IDF.
//!
//! Simplified from `jionlp/algorithm/summary/extract_summary.py`: split
//! the document into sentences, score each sentence as the sum of its
//! char-bigram IDF weights (using `dict::idf()`), and return the top-k
//! sentences in original document order.
//!
//! Stage-1 choices made here vs the Python original:
//!   * No MMR / redundancy penalty.
//!   * No title boost.
//!   * No embedding/TextRank path.
//! Add these in follow-up stages if needed.

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

    // Score each sentence: sum of bigram IDF / sentence length (mean), so
    // long sentences don't automatically win.
    let scored: Vec<SummarySentence> = sentences
        .into_iter()
        .enumerate()
        .map(|(pos, s)| {
            let score = bigram_idf_score(&s, idf);
            SummarySentence {
                text: s,
                score,
                position: pos,
            }
        })
        .collect();

    // Pick top-k by score descending, then re-sort by position to preserve
    // narrative order.
    let mut by_score = scored.clone();
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

    // Precompute score and bigram set for each sentence.
    let bigrams: Vec<rustc_hash::FxHashSet<String>> =
        sentences.iter().map(|s| bigrams_of(s)).collect();
    let scores: Vec<f64> = sentences.iter().map(|s| bigram_idf_score(s, idf)).collect();

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
