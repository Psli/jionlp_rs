//! SimHash — locality-sensitive hash for near-duplicate detection.
//!
//! Port of the spirit of `jionlp/algorithm/simhash/`, slimmed to a
//! self-contained 64-bit implementation that operates on character
//! n-grams. External tokenizers (jieba / jiojio) are intentionally *not*
//! a dependency: the Rust port is meant to be a cheap building block that
//! callers can feed with their own tokenization if they want weighted
//! features.
//!
//! ## Algorithm (standard SimHash, Charikar 2002)
//!
//! 1. Extract n-gram features from the input (default n=2).
//! 2. For each feature, take a 64-bit hash (FxHash, matching the rest of
//!    the crate).
//! 3. Maintain a signed accumulator `v[64]`: for each bit position of each
//!    feature hash, `+weight` if 1 else `-weight`. Weight is feature
//!    frequency (how many times the n-gram appeared).
//! 4. The output bit is 1 iff the corresponding accumulator is positive.
//!
//! ## Usage
//!
//! ```ignore
//! use jionlp_core::algorithm::simhash;
//!
//! let a = simhash("今天天气真好");
//! let b = simhash("今天天气非常好");
//! let dist = jionlp_core::algorithm::hamming_distance(a, b);
//! assert!(dist <= 10);   // near-duplicates: Hamming distance is small.
//! ```

use rustc_hash::{FxHashMap, FxHasher};
use std::hash::Hasher;

/// Default n-gram size for [`simhash`]. 2 is a good trade-off for Chinese:
/// captures bigram context without blowing up the feature set.
pub const DEFAULT_NGRAM: usize = 2;

/// Compute a 64-bit SimHash of `text` using character bigrams.
pub fn simhash(text: &str) -> u64 {
    simhash_ngram(text, DEFAULT_NGRAM)
}

/// Same as [`simhash`] but with a custom n-gram size.
///
/// `n=1` = character unigrams (fast, coarse).
/// `n=2` = bigrams (default, good for CJK).
/// `n=3` = trigrams (finer but more memory).
pub fn simhash_ngram(text: &str, n: usize) -> u64 {
    let n = n.max(1);

    // Count n-gram frequencies to weight common features heavier.
    let mut freq: FxHashMap<String, u32> = FxHashMap::default();
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return 0;
    }
    if chars.len() < n {
        *freq.entry(text.to_string()).or_insert(0) += 1;
    } else {
        for window in chars.windows(n) {
            let key: String = window.iter().collect();
            *freq.entry(key).or_insert(0) += 1;
        }
    }

    // Accumulate signed bit votes.
    let mut v: [i64; 64] = [0; 64];
    for (feature, weight) in &freq {
        let h = feature_hash(feature);
        let w = *weight as i64;
        for bit in 0..64 {
            let set = (h >> bit) & 1 == 1;
            v[bit] += if set { w } else { -w };
        }
    }

    // Output bits.
    let mut out: u64 = 0;
    for (bit, &count) in v.iter().enumerate() {
        if count > 0 {
            out |= 1u64 << bit;
        }
    }
    out
}

fn feature_hash(s: &str) -> u64 {
    let mut h = FxHasher::default();
    h.write(s.as_bytes());
    h.finish()
}

/// Hamming distance between two 64-bit SimHash values. Lower = more similar.
///
/// A distance of 0 means identical hashes (text may still differ in rare
/// positions of no-change features). Common thresholds:
///   * `<= 3` : strong near-duplicate
///   * `<= 6` : likely related
///   * `> 10`: probably unrelated
#[inline]
pub fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Jaccard-like similarity in [0, 1]: `1 - hamming / 64`.
#[inline]
pub fn simhash_similarity(a: u64, b: u64) -> f64 {
    1.0 - (hamming_distance(a, b) as f64) / 64.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_text_identical_hash() {
        let h1 = simhash("今天天气真好");
        let h2 = simhash("今天天气真好");
        assert_eq!(h1, h2);
        assert_eq!(hamming_distance(h1, h2), 0);
    }

    #[test]
    fn similar_text_small_distance() {
        // "真好" vs "非常好" — share most bigrams.
        let h1 = simhash("今天天气真好");
        let h2 = simhash("今天天气非常好");
        let d = hamming_distance(h1, h2);
        assert!(d < 20, "expected near-duplicate, got distance {d}");
    }

    #[test]
    fn different_text_larger_distance() {
        let h1 = simhash("今天天气真好,我想去公园散步");
        let h2 = simhash("机器学习是人工智能的分支,研究算法");
        let d = hamming_distance(h1, h2);
        assert!(d > 15, "expected dissimilar, got distance {d}");
    }

    #[test]
    fn similarity_in_range() {
        let s = simhash_similarity(0, 0);
        assert!((s - 1.0).abs() < 1e-9);
        let s = simhash_similarity(0, u64::MAX);
        assert!(s.abs() < 1e-9);
    }

    #[test]
    fn empty_text_yields_zero() {
        assert_eq!(simhash(""), 0);
    }

    #[test]
    fn short_text_under_ngram_is_handled() {
        // 1 char text with default n=2 — should not panic and should be
        // deterministic.
        let h = simhash("好");
        assert_ne!(h, 0, "single char should still hash to non-zero");
    }

    #[test]
    fn ngram_size_changes_hash() {
        let a = simhash_ngram("今天天气真好", 1);
        let b = simhash_ngram("今天天气真好", 2);
        assert_ne!(a, b, "different n-gram sizes should yield different hashes");
    }
}
