//! Stop-word filter — port of `jionlp/gadget/remove_stopwords.py`.
//!
//! Initial scope: basic stop-word filtering with optional negative-word
//! preservation. The Python original additionally supports
//! `remove_time` / `remove_location` / `remove_number` / `remove_non_chinese`
//! via regex, which depend on Phase 3 (`rule_pattern`). Those flags are
//! accepted but currently no-op — see PLAN.md.

use crate::{dict, Result};

#[derive(Debug, Clone, Copy, Default)]
pub struct RemoveOpts {
    /// Keep negative words (如"未"、"没有"、"不") even if they appear in the
    /// stop-word list.
    pub save_negative_words: bool,
}

/// Filter `words` against the stop-word dictionary.
pub fn remove_stopwords(words: &[String], opts: RemoveOpts) -> Result<Vec<String>> {
    let stop = dict::stopwords()?;
    let neg = if opts.save_negative_words {
        Some(dict::negative_words()?)
    } else {
        None
    };

    let mut out: Vec<String> = Vec::with_capacity(words.len());
    for w in words {
        if w.is_empty() {
            continue;
        }
        if stop.contains(w) {
            match neg {
                Some(neg_set) if neg_set.contains(w) => out.push(w.clone()),
                _ => continue,
            }
        } else {
            out.push(w.clone());
        }
    }
    Ok(out)
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

    fn vs(words: &[&str]) -> Vec<String> {
        words.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn filters_known_stopwords() {
        ensure_init();
        // "的" is almost certainly in the Chinese stopword list.
        let r = remove_stopwords(&vs(&["他", "的", "苹果"]), RemoveOpts::default()).unwrap();
        assert!(!r.contains(&"的".to_string()));
        assert!(r.contains(&"苹果".to_string()));
    }

    #[test]
    fn keeps_non_stopwords() {
        ensure_init();
        let input = vs(&["机器学习", "深度学习", "神经网络"]);
        let r = remove_stopwords(&input, RemoveOpts::default()).unwrap();
        assert_eq!(r, input);
    }

    #[test]
    fn drops_empty_strings() {
        ensure_init();
        let r = remove_stopwords(&vs(&["", "苹果", ""]), RemoveOpts::default()).unwrap();
        assert_eq!(r, vec!["苹果".to_string()]);
    }

    #[test]
    fn save_negative_words_flag_preserves_negations() {
        ensure_init();
        // 不 is both a stopword and a negative word — with flag it should stay.
        let input = vs(&["不", "是", "好"]);
        let with = remove_stopwords(
            &input,
            RemoveOpts {
                save_negative_words: true,
            },
        )
        .unwrap();
        let without = remove_stopwords(&input, RemoveOpts::default()).unwrap();
        assert!(with.contains(&"不".to_string()));
        assert!(!without.contains(&"不".to_string()));
    }
}
