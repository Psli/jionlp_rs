//! Port of `jionlp/gadget/idiom_solitaire.py` — Chinese idiom chain
//! generator. Given a current idiom `cur`, return the next idiom whose
//! first character matches `cur`'s last character (or last pinyin when
//! `same_pinyin=true`).

use crate::{dict, textaug::prng::SplitMix64, Result};
use once_cell::sync::OnceCell;

/// Small state object that caches the idiom list and tracks used idioms
/// across a chain.
pub struct IdiomSolitaireGame {
    rng_seed: u64,
    used: std::cell::RefCell<rustc_hash::FxHashSet<String>>,
}

impl IdiomSolitaireGame {
    /// Start a new game with the given RNG seed. Deterministic output is
    /// valuable for tests; callers can seed with any u64.
    pub fn new(rng_seed: u64) -> Self {
        Self {
            rng_seed,
            used: std::cell::RefCell::new(rustc_hash::FxHashSet::default()),
        }
    }

    /// Reset the used-idioms set (equivalent to Python's `restart=True`).
    pub fn restart(&self) {
        self.used.borrow_mut().clear();
    }

    /// Find the next idiom in the chain. Uses character-based match
    /// (`same_pinyin=false` in the Python API). `with_prob=true` samples
    /// proportionally to the freq field; `with_prob=false` picks uniformly.
    ///
    /// Returns:
    ///   * `Ok(Some(idiom))` on success.
    ///   * `Ok(None)` when no next idiom can be found.
    pub fn next_by_char(
        &self,
        cur_idiom: &str,
        with_prob: bool,
    ) -> Result<Option<String>> {
        let dict = all_idioms()?;
        {
            let mut used = self.used.borrow_mut();
            if dict.contains_key(cur_idiom) {
                used.insert(cur_idiom.to_string());
            }
        }
        let last_char = cur_idiom.chars().last();
        let last_char = match last_char {
            Some(c) => c,
            None => return Ok(None),
        };
        let used = self.used.borrow();
        let mut candidates: Vec<(&String, u32)> = dict
            .iter()
            .filter(|(idiom, _)| {
                idiom.chars().next() == Some(last_char) && !used.contains(*idiom)
            })
            .map(|(idiom, freq)| (idiom, *freq))
            .collect();
        drop(used);
        if candidates.is_empty() {
            return Ok(None);
        }
        candidates.sort_by(|a, b| a.0.cmp(b.0));

        // SplitMix64 keyed with the current idiom + used-count for variety
        // across chain steps while remaining deterministic.
        let seed = self
            .rng_seed
            .wrapping_add(hash_str(cur_idiom))
            .wrapping_add(self.used.borrow().len() as u64);
        let mut rng = SplitMix64::new(seed);
        let pick: &String = if with_prob {
            let total: u64 = candidates.iter().map(|(_, f)| *f as u64).sum();
            let target = rng.next_u64() % total.max(1);
            let mut acc = 0u64;
            let mut chosen = candidates[0].0;
            for (idiom, freq) in &candidates {
                acc += *freq as u64;
                if target < acc {
                    chosen = *idiom;
                    break;
                }
            }
            chosen
        } else {
            let idx = (rng.next_u64() % candidates.len() as u64) as usize;
            candidates[idx].0
        };
        self.used.borrow_mut().insert(pick.clone());
        Ok(Some(pick.clone()))
    }
}

fn hash_str(s: &str) -> u64 {
    // Simple FNV-1a; we don't need cryptographic properties.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

static IDIOMS: OnceCell<&'static rustc_hash::FxHashMap<String, u32>> = OnceCell::new();

fn all_idioms() -> Result<&'static rustc_hash::FxHashMap<String, u32>> {
    IDIOMS.get_or_try_init(|| dict::chinese_idioms().map(|d| &*d))
        .copied()
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
    fn next_by_char_finds_something() {
        ensure_init();
        let game = IdiomSolitaireGame::new(42);
        // 千方百计 ends in 计 — any idiom starting with 计 is valid.
        let next = game.next_by_char("千方百计", true).unwrap();
        let next = next.expect("should find a successor");
        assert!(next.starts_with('计'), "got {next}");
    }

    #[test]
    fn unknown_idiom_still_works_as_seed() {
        ensure_init();
        let game = IdiomSolitaireGame::new(1);
        // Non-existent idiom ending in 山 — still tries to find successor.
        let next = game.next_by_char("月亮山", false).unwrap();
        if let Some(n) = next {
            assert!(n.starts_with('山'));
        }
    }

    #[test]
    fn used_idioms_are_not_reused() {
        ensure_init();
        let game = IdiomSolitaireGame::new(99);
        let a = game.next_by_char("一马当先", false).unwrap().unwrap();
        let b = game.next_by_char(&a, false).unwrap();
        if let Some(b) = b {
            assert_ne!(a, b);
        }
    }
}
