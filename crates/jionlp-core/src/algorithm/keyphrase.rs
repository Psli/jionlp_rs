//! Keyphrase extraction — TF-IDF scoring over character n-grams.
//!
//! A slim port of `jionlp/algorithm/keyphrase`: instead of depending on a
//! Chinese word segmenter (jieba / jiojio), we score contiguous Chinese
//! char windows (defaulting to n=2..4) and rank them with TF × IDF using
//! `dict::idf()`.
//!
//! Heuristic rules used by the Python original that we preserve here:
//!   * Candidates are *only* runs of Chinese characters (ASCII / digits
//!     break sequences).
//!   * Candidates must contain at least one non-stopword char.
//!   * Candidates crossing punctuation boundaries are discarded.
//!   * Ranking weight = sum of per-char IDF × term-frequency in the doc.
//!
//! This is a Stage-1 implementation — good enough for "given a short
//! paragraph, give me 3-5 likely keyphrases". For production-grade
//! TextRank / embedding-based rankers, see `PLAN.md`.

use crate::dict;
use crate::Result;
use rustc_hash::FxHashMap;

/// Punctuation characters that terminate a candidate phrase.
const PUNCTUATION: &[char] = &[
    '，', '。', '！', '？', '、', '；', '：', '“', '”', '‘', '’', '（', '）', '《', '》',
    '—', '·', ',', '.', '!', '?', ';', ':', '"', '\'', '(', ')', '<', '>', '[', ']',
    '{', '}', '/', '\\', '|', '\t', '\n', '\r', ' ',
];

#[derive(Debug, Clone, PartialEq)]
pub struct KeyPhrase {
    pub phrase: String,
    pub weight: f64,
}

/// Extract up to `top_k` keyphrases from `text`. n-grams from `min_n` to
/// `max_n` characters are considered (inclusive). Scores are sum of
/// per-char IDF × occurrence count; phrases composed of stopwords
/// (exactly one-char run that looks like a stopword) are dropped.
///
/// Returns phrases in descending weight order. When `text` has no valid
/// candidates, returns an empty vec.
pub fn extract_keyphrase(
    text: &str,
    top_k: usize,
    min_n: usize,
    max_n: usize,
) -> Result<Vec<KeyPhrase>> {
    if text.is_empty() || top_k == 0 {
        return Ok(Vec::new());
    }
    let min_n = min_n.max(1);
    let max_n = max_n.max(min_n);

    let idf = dict::idf()?;
    let stopwords = dict::stopwords()?;

    // Split text into "Chinese-only runs" separated by punctuation and
    // non-Hanzi. Each run is a place where candidates can live.
    let runs: Vec<String> = split_into_runs(text);

    // Count bigram/trigram/… occurrences in each run.
    let mut freq: FxHashMap<String, u32> = FxHashMap::default();
    for run in &runs {
        let chars: Vec<char> = run.chars().collect();
        for n in min_n..=max_n {
            if chars.len() < n {
                continue;
            }
            for window in chars.windows(n) {
                let phrase: String = window.iter().collect();
                *freq.entry(phrase).or_insert(0) += 1;
            }
        }
    }

    // Score: for each candidate phrase, compute IDF(phrase) if known,
    // else sum per-char IDF (fallback 5.0 for rare / unknown chars).
    let mut scored: Vec<KeyPhrase> = freq
        .into_iter()
        .filter_map(|(phrase, tf)| {
            if is_all_stopwords(&phrase, stopwords) {
                return None;
            }
            let base = match idf.get(&phrase) {
                Some(v) => *v,
                None => phrase.chars().map(|c| char_idf(&c.to_string(), idf)).sum(),
            };
            Some(KeyPhrase {
                phrase,
                weight: base * (tf as f64),
            })
        })
        .collect();

    scored.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(top_k);
    Ok(scored)
}

// ───────────────────────── TextRank variant ────────────────────────────────

/// Extract keyphrases using TextRank — a PageRank-style graph ranking
/// over character n-gram co-occurrences.
///
/// ## Algorithm
///
/// 1. Split text into Chinese-char runs (punctuation breaks them).
/// 2. Generate n-gram candidates (`min_n..=max_n`) per run, filtering
///    stopwords.
/// 3. Build an undirected graph: each candidate is a node; two candidates
///    co-occurring in the same run add +1 to their edge weight.
/// 4. Run PageRank for a small fixed number of iterations (default 20).
/// 5. Return top-k by final score.
///
/// Compared to TF-IDF (`extract_keyphrase`), TextRank picks up phrases
/// that occur with other important phrases — more "thematic" vs just
/// "rare". For short docs the difference is small; it matters more on
/// full articles.
pub fn extract_keyphrase_textrank(
    text: &str,
    top_k: usize,
    min_n: usize,
    max_n: usize,
) -> Result<Vec<KeyPhrase>> {
    if text.is_empty() || top_k == 0 {
        return Ok(Vec::new());
    }
    let min_n = min_n.max(1);
    let max_n = max_n.max(min_n);
    let stopwords = dict::stopwords()?;

    let runs = split_into_runs(text);

    // Collect unique candidates per run, preserving within-run order for
    // cooccurrence edges.
    let mut all_candidates: FxHashMap<String, usize> = FxHashMap::default();
    let mut run_candidates: Vec<Vec<usize>> = Vec::new();

    for run in &runs {
        let chars: Vec<char> = run.chars().collect();
        let mut cands_this_run: Vec<usize> = Vec::new();
        for n in min_n..=max_n {
            if chars.len() < n {
                continue;
            }
            for window in chars.windows(n) {
                let phrase: String = window.iter().collect();
                if is_all_stopwords(&phrase, stopwords) {
                    continue;
                }
                let next_id = all_candidates.len();
                let id = *all_candidates.entry(phrase).or_insert(next_id);
                cands_this_run.push(id);
            }
        }
        run_candidates.push(cands_this_run);
    }

    let n_nodes = all_candidates.len();
    if n_nodes == 0 {
        return Ok(Vec::new());
    }

    // Build sparse co-occurrence graph.
    // `edges[i]` = Vec<(neighbor_id, weight)>; weights double-counted for
    // undirected form (ok for normalization).
    let mut edges: Vec<FxHashMap<usize, f64>> = vec![FxHashMap::default(); n_nodes];
    for run_ids in &run_candidates {
        // All pairs within a run co-occur.
        for i in 0..run_ids.len() {
            for j in (i + 1)..run_ids.len() {
                let (a, b) = (run_ids[i], run_ids[j]);
                if a == b {
                    continue;
                }
                *edges[a].entry(b).or_insert(0.0) += 1.0;
                *edges[b].entry(a).or_insert(0.0) += 1.0;
            }
        }
    }

    // PageRank iteration. Damping factor d=0.85, 20 iters is sufficient
    // for convergence to 3 decimals on short docs.
    let d = 0.85;
    let iters = 20;
    let mut score = vec![1.0_f64; n_nodes];
    let mut next = vec![0.0_f64; n_nodes];

    // Precompute out-degree weights so we don't divide by zero.
    let out_sum: Vec<f64> = edges
        .iter()
        .map(|m| m.values().sum::<f64>().max(1e-12))
        .collect();

    for _ in 0..iters {
        for v in next.iter_mut() {
            *v = (1.0 - d) / n_nodes as f64;
        }
        for (i, neighbors) in edges.iter().enumerate() {
            if neighbors.is_empty() {
                continue;
            }
            let contrib = d * score[i] / out_sum[i];
            for (&j, &w) in neighbors.iter() {
                next[j] += contrib * w;
            }
        }
        score.copy_from_slice(&next);
    }

    // Collect results sorted descending.
    let mut result: Vec<KeyPhrase> = all_candidates
        .into_iter()
        .map(|(phrase, id)| KeyPhrase {
            phrase,
            weight: score[id],
        })
        .collect();
    result.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
    result.truncate(top_k);
    Ok(result)
}

fn split_into_runs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for c in text.chars() {
        if PUNCTUATION.contains(&c) || !is_cjk(c) {
            if !buf.is_empty() {
                out.push(std::mem::take(&mut buf));
            }
        } else {
            buf.push(c);
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x4E00..=0x9FA5)
}

fn is_all_stopwords(phrase: &str, stopwords: &rustc_hash::FxHashSet<String>) -> bool {
    // A phrase is "stopwordy" when it equals a single-term stopword or
    // every char is in the stopword set individually.
    if stopwords.contains(phrase) {
        return true;
    }
    phrase
        .chars()
        .all(|c| stopwords.contains(&c.to_string()))
}

fn char_idf(term: &str, idf: &FxHashMap<String, f64>) -> f64 {
    // Fallback weight for unknown terms — pick a middle-ground value so
    // rare terms still get a reasonable score.
    *idf.get(term).unwrap_or(&5.0)
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
    fn returns_top_k() {
        ensure_init();
        let text =
            "机器学习是人工智能的一个分支,研究如何从数据中自动学习规律和模式。\
             机器学习广泛应用于自然语言处理、计算机视觉和推荐系统。";
        let r = extract_keyphrase(text, 5, 2, 4).unwrap();
        assert!(r.len() <= 5);
        assert!(!r.is_empty());
        // "机器学习" is repeated and is a rare phrase — expect it near the top.
        assert!(
            r.iter().any(|k| k.phrase.contains("机器")),
            "expected a 机器* phrase in top 5, got: {:?}",
            r.iter().map(|k| k.phrase.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn empty_text() {
        ensure_init();
        let r = extract_keyphrase("", 5, 2, 4).unwrap();
        assert!(r.is_empty());
    }

    #[test]
    fn punctuation_breaks_candidates() {
        ensure_init();
        // No candidate should span the comma.
        let text = "苹果,香蕉";
        let r = extract_keyphrase(text, 5, 2, 2).unwrap();
        for k in &r {
            assert!(!k.phrase.contains("果香"), "crossed punctuation: {}", k.phrase);
        }
    }

    #[test]
    fn top_k_zero_returns_empty() {
        ensure_init();
        let r = extract_keyphrase("一些文本内容", 0, 2, 4).unwrap();
        assert!(r.is_empty());
    }

    #[test]
    fn sorted_descending() {
        ensure_init();
        let r = extract_keyphrase(
            "北京是中国的首都。北京有很多名胜古迹。北京人口众多。",
            10, 2, 4,
        )
        .unwrap();
        for w in r.windows(2) {
            assert!(w[0].weight >= w[1].weight);
        }
    }

    // ── TextRank ─────────────────────────────────────────────────────────

    #[test]
    fn textrank_returns_top_k() {
        ensure_init();
        let text = "机器学习是人工智能的一个分支。机器学习研究如何从数据中学习。机器学习应用广泛。";
        let r = extract_keyphrase_textrank(text, 5, 2, 4).unwrap();
        assert!(!r.is_empty());
        assert!(r.len() <= 5);
    }

    #[test]
    fn textrank_sorted_descending() {
        ensure_init();
        let r = extract_keyphrase_textrank(
            "北京是中国首都。北京有名胜古迹。北京人口多。",
            10, 2, 4,
        )
        .unwrap();
        for w in r.windows(2) {
            assert!(w[0].weight >= w[1].weight);
        }
    }

    #[test]
    fn textrank_empty_text() {
        ensure_init();
        assert!(extract_keyphrase_textrank("", 5, 2, 4).unwrap().is_empty());
    }
}
