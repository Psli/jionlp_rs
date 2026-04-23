//! Random add/delete augmentation — port of
//! `jionlp/textaug/random_add_delete.py`.
//!
//! For each char in the input, with probability `add_ratio` insert a
//! randomly-chosen **non-CJK noise char** before it, and with probability
//! `delete_ratio` drop it. The Python original samples the insertion
//! alphabet from `char_distribution.zip` (a big frequency table); we use a
//! hand-curated set of common separators/whitespace/ASCII that are
//! semantically neutral for most downstream tasks. Avoids loading a third
//! zip dictionary.
//!
//! Generates up to `n` distinct variants. Empty text returns an empty vec.

use crate::textaug::prng::SplitMix64;

/// Insertion alphabet — chars that rarely change meaning when added. This
/// mirrors the spirit of Python's smoothed non-CJK distribution: mostly
/// whitespace and punctuation that a reader would skim over.
const NOISE_CHARS: &[char] = &[
    ' ', ' ', ' ', // ASCII space weighted higher
    '\u{3000}',    // fullwidth space
    ',', '.', '/', '-', '_', '·',
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
    'a', 'e', 'i', 'o', 'u',
];

pub fn random_add_delete(
    text: &str,
    n: usize,
    add_ratio: f64,
    delete_ratio: f64,
    seed: u64,
) -> Vec<String> {
    if n == 0 || text.is_empty() {
        return Vec::new();
    }
    let add = add_ratio.clamp(0.0, 1.0);
    let del = delete_ratio.clamp(0.0, 1.0);

    let mut rng = SplitMix64::from_opt(seed);
    let mut out: Vec<String> = Vec::with_capacity(n);
    let char_count = text.chars().count();

    // Cap attempts so pathologically-low ratios don't loop forever.
    let cap = (n as f64 / (add + del).max(1e-6))
        .min((char_count * 2 + 4) as f64) as usize
        + n;

    let mut attempts = 0usize;
    while out.len() < n && attempts < cap {
        attempts += 1;
        let cand = augment_once(text, add, del, &mut rng);
        if cand == text {
            continue;
        }
        if !out.contains(&cand) {
            out.push(cand);
        }
    }
    out
}

fn augment_once(text: &str, add: f64, del: f64, rng: &mut SplitMix64) -> String {
    let mut buf = String::with_capacity(text.len() + 4);
    for c in text.chars() {
        if rng.uniform01() < add {
            let idx = rng.uniform_int(NOISE_CHARS.len());
            buf.push(NOISE_CHARS[idx]);
        }
        if rng.uniform01() < del {
            continue;
        }
        buf.push(c);
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_n_variants() {
        let r = random_add_delete("今天天气真好我要去公园散步", 3, 0.3, 0.1, 42);
        assert_eq!(r.len(), 3);
        for v in &r {
            assert!(!v.is_empty());
        }
    }

    #[test]
    fn deterministic_with_seed() {
        let a = random_add_delete("你好世界人民", 3, 0.3, 0.1, 42);
        let b = random_add_delete("你好世界人民", 3, 0.3, 0.1, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn all_variants_differ_from_source() {
        let src = "今天天气真好我要去公园散步";
        let r = random_add_delete(src, 3, 0.3, 0.1, 1);
        for v in r {
            assert_ne!(v, src);
        }
    }

    #[test]
    fn empty_returns_empty() {
        assert!(random_add_delete("", 3, 0.3, 0.1, 1).is_empty());
    }

    #[test]
    fn zero_n_returns_empty() {
        assert!(random_add_delete("some text 一些文本", 0, 0.3, 0.1, 1).is_empty());
    }
}
