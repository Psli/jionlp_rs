//! Adjacent-char swap augmentation — port of
//! `jionlp/textaug/swap_char_position.py`.
//!
//! Produces variants of `text` by randomly swapping neighboring Chinese
//! characters. Swaps only happen within a run of consecutive CJK chars —
//! an ASCII letter or punctuation breaks the window. The swap distance
//! is drawn from a discrete Gaussian (scale 1.0 by default), which in the
//! Python original gives ~76% adjacent / ~22% distance-2 / ~2% distance-3.
//!
//! ## Determinism
//!
//! This Rust port uses a simple xorshift PRNG keyed by `seed` so the same
//! input + seed always produces the same outputs. Pass `seed = 0` to get
//! non-deterministic behavior seeded from the system clock.

use crate::rule::checker::check_any_chinese_char;
use crate::textaug::prng::SplitMix64;

/// Produce up to `n` distinct augmented variants of `text` by swapping
/// neighboring Chinese chars.
///
/// * `swap_ratio`: per-char probability of being swap-eligible (default 0.02).
/// * `seed`: PRNG seed (0 = non-deterministic, seeded from the clock).
/// * `scale`: standard deviation of the swap-distance Gaussian (default 1.0).
pub fn swap_char_position(
    text: &str,
    n: usize,
    swap_ratio: f64,
    seed: u64,
    scale: f64,
) -> Vec<String> {
    if n == 0 || text.is_empty() {
        return Vec::new();
    }
    let mut rng = SplitMix64::from_opt(seed);
    let scale = scale.max(0.1);
    let swap_ratio = swap_ratio.clamp(0.0, 1.0);

    let mut out: Vec<String> = Vec::with_capacity(n);
    let mut attempts = 0usize;
    let cap = (n as f64 / swap_ratio.max(1e-6))
        .min((text.chars().count() * 2) as f64) as usize
        + n;

    while out.len() < n && attempts < cap {
        attempts += 1;
        let candidate = augment_once(text, swap_ratio, scale, &mut rng);
        if candidate == text {
            continue;
        }
        if !out.contains(&candidate) {
            out.push(candidate);
        }
    }
    out
}

fn augment_once(text: &str, swap_ratio: f64, scale: f64, rng: &mut SplitMix64) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    for i in 0..len {
        if rng.uniform01() >= swap_ratio {
            continue;
        }
        let ci = chars[i];
        if !check_any_chinese_char(&ci.to_string()) {
            continue;
        }
        let j = pick_swap_partner(&chars, i, scale, rng);
        if j != i {
            chars.swap(i, j);
        }
    }
    chars.into_iter().collect()
}

fn pick_swap_partner(chars: &[char], orig: usize, scale: f64, rng: &mut SplitMix64) -> usize {
    // Find the longest run of CJK chars surrounding `orig`.
    let mut start_off: isize = 0;
    let mut end_off: isize = 0;

    while orig as isize + start_off > 0
        && check_any_chinese_char(&chars[(orig as isize + start_off - 1) as usize].to_string())
    {
        start_off -= 1;
    }
    while (orig as isize + end_off) < (chars.len() as isize - 1)
        && check_any_chinese_char(&chars[(orig as isize + end_off + 1) as usize].to_string())
    {
        end_off += 1;
    }

    if start_off == end_off {
        return orig; // isolated CJK char — no swap possible.
    }

    // Sample a non-zero integer from a Gaussian, rejecting out-of-range.
    for _ in 0..32 {
        let z = rng.normal() * scale;
        let delta = z.round() as isize;
        if delta == 0 {
            continue;
        }
        if delta >= start_off && delta <= end_off {
            return (orig as isize + delta) as usize;
        }
    }
    // Fallback: swap with the immediate right neighbor if possible, else left.
    if end_off >= 1 {
        orig + 1
    } else {
        orig - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_n_variants_on_long_text() {
        // Use a higher ratio to guarantee enough candidate swaps.
        let r = swap_char_position(
            "民盟发言人昂山素季目前情况良好正在康复中",
            3,
            0.2,
            1,
            1.0,
        );
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn deterministic_with_seed() {
        let a = swap_char_position("中华人民共和国是美好的家园", 3, 0.3, 42, 1.0);
        let b = swap_char_position("中华人民共和国是美好的家园", 3, 0.3, 42, 1.0);
        assert_eq!(a, b, "same seed → same results");
    }

    #[test]
    fn variants_all_have_same_length_and_same_chars() {
        // Swaps must preserve the multiset of characters.
        let src = "中华人民共和国是美好的家园";
        let src_sorted: Vec<char> = {
            let mut v: Vec<char> = src.chars().collect();
            v.sort();
            v
        };
        let r = swap_char_position(src, 5, 0.3, 1, 1.0);
        for variant in r {
            let mut v: Vec<char> = variant.chars().collect();
            v.sort();
            assert_eq!(v, src_sorted);
        }
    }

    #[test]
    fn empty_text_returns_empty() {
        assert!(swap_char_position("", 3, 0.1, 1, 1.0).is_empty());
    }

    #[test]
    fn variants_differ_from_source() {
        let src = "中华人民共和国是美好的家园";
        let r = swap_char_position(src, 3, 0.3, 1, 1.0);
        assert!(!r.is_empty());
        for v in r {
            assert_ne!(v, src);
        }
    }
}
