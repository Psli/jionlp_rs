//! Homophone substitution — port of
//! `jionlp/textaug/homophone_substitution.py`.
//!
//! For each Chinese char in the input, with probability `sub_ratio`
//! replace it with another char that shares at least one pinyin reading.
//! The reverse index `pinyin → [chars]` is built once from
//! `dict::char_dictionary()` and cached for the process lifetime.
//!
//! Only the primary reading of each char is indexed — this is a pragmatic
//! choice: the Python original considers every reading but that blows up
//! the candidate pool and rarely produces better augmentations.

use crate::dict;
use crate::textaug::prng::SplitMix64;
use crate::Result;
use once_cell::sync::OnceCell;
use rustc_hash::FxHashMap;

static REVERSE_INDEX: OnceCell<FxHashMap<String, Vec<char>>> = OnceCell::new();

fn reverse_index() -> Result<&'static FxHashMap<String, Vec<char>>> {
    REVERSE_INDEX.get_or_try_init(|| {
        let char_dict = dict::char_dictionary()?;
        let mut map: FxHashMap<String, Vec<char>> = FxHashMap::default();
        for (&c, info) in char_dict.iter() {
            if let Some(py) = info.pinyin.first() {
                map.entry(py.clone()).or_default().push(c);
            }
        }
        Ok(map)
    })
}

pub fn homophone_substitution(
    text: &str,
    n: usize,
    sub_ratio: f64,
    seed: u64,
) -> Result<Vec<String>> {
    if n == 0 || text.is_empty() {
        return Ok(Vec::new());
    }
    let sub = sub_ratio.clamp(0.0, 1.0);

    let char_dict = dict::char_dictionary()?;
    let rev = reverse_index()?;
    let mut rng = SplitMix64::from_opt(seed);

    let mut out: Vec<String> = Vec::with_capacity(n);
    let char_count = text.chars().count();
    let cap = (n as f64 / sub.max(1e-6))
        .min((char_count * 2 + 4) as f64) as usize
        + n;
    let mut attempts = 0usize;

    while out.len() < n && attempts < cap {
        attempts += 1;
        let cand = augment_once(text, sub, char_dict, rev, &mut rng);
        if cand == text {
            continue;
        }
        if !out.contains(&cand) {
            out.push(cand);
        }
    }
    Ok(out)
}

fn augment_once(
    text: &str,
    sub: f64,
    char_dict: &FxHashMap<char, dict::CharInfo>,
    rev: &FxHashMap<String, Vec<char>>,
    rng: &mut SplitMix64,
) -> String {
    let mut buf = String::with_capacity(text.len());
    for c in text.chars() {
        let replaced = rng.uniform01() < sub;
        if replaced {
            if let Some(info) = char_dict.get(&c) {
                if let Some(py) = info.pinyin.first() {
                    if let Some(candidates) = rev.get(py) {
                        if !candidates.is_empty() {
                            let idx = rng.uniform_int(candidates.len());
                            buf.push(candidates[idx]);
                            continue;
                        }
                    }
                }
            }
        }
        buf.push(c);
    }
    buf
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
    fn generates_variants() {
        ensure_init();
        let r = homophone_substitution("今天天气真好", 3, 0.5, 42).unwrap();
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn deterministic_with_seed() {
        ensure_init();
        let a = homophone_substitution("中华人民共和国", 3, 0.5, 7).unwrap();
        let b = homophone_substitution("中华人民共和国", 3, 0.5, 7).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn variants_preserve_length() {
        ensure_init();
        let src = "今天天气真好";
        let src_len = src.chars().count();
        let r = homophone_substitution(src, 3, 0.5, 1).unwrap();
        for v in r {
            assert_eq!(v.chars().count(), src_len);
        }
    }

    #[test]
    fn empty_returns_empty() {
        ensure_init();
        assert!(homophone_substitution("", 3, 0.5, 1).unwrap().is_empty());
    }
}
