//! Traditional ↔ Simplified Chinese conversion — port of
//! `jionlp/gadget/ts_conversion.py`.
//!
//! Two modes:
//! - [`TsMode::Char`] — character-by-character mapping (fast, cheap).
//! - [`TsMode::Word`] — longest-prefix match over combined char+word table
//!                     (captures regional idioms like 太空梭 ↔ 航天飞机).

use crate::trie::LabeledTrie;
use crate::{dict, Error, Result};
use once_cell::sync::OnceCell;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TsMode {
    Char,
    Word,
}

/// Direction label stored in the shared trie (so we don't build two tries).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dir {
    Tra,
    Sim,
}

struct WordTables {
    tra2sim: FxHashMap<String, String>,
    sim2tra: FxHashMap<String, String>,
    trie: LabeledTrie<Dir>,
}

static WORD_TABLES: OnceCell<WordTables> = OnceCell::new();

fn word_tables() -> Result<&'static WordTables> {
    WORD_TABLES.get_or_try_init(|| {
        // Combine char + word tables, as Python's tra2sim_token / sim2tra_token.
        let mut tra2sim = dict::tra2sim_char()?.clone();
        for (k, v) in dict::tra2sim_word()? {
            tra2sim.insert(k.clone(), v.clone());
        }

        let mut sim2tra = dict::sim2tra_char()?.clone();
        for (k, v) in dict::sim2tra_word()? {
            sim2tra.insert(k.clone(), v.clone());
        }

        let mut trie: LabeledTrie<Dir> = LabeledTrie::new();
        for k in tra2sim.keys() {
            trie.insert(k, Dir::Tra);
        }
        for k in sim2tra.keys() {
            trie.insert(k, Dir::Sim);
        }

        Ok(WordTables {
            tra2sim,
            sim2tra,
            trie,
        })
    })
}

/// Traditional → Simplified.
pub fn tra2sim(text: &str, mode: TsMode) -> Result<String> {
    match mode {
        TsMode::Char => tra2sim_char_mode(text),
        TsMode::Word => tra2sim_word_mode(text),
    }
}

/// Simplified → Traditional.
pub fn sim2tra(text: &str, mode: TsMode) -> Result<String> {
    match mode {
        TsMode::Char => sim2tra_char_mode(text),
        TsMode::Word => sim2tra_word_mode(text),
    }
}

fn tra2sim_char_mode(text: &str) -> Result<String> {
    let m = dict::tra2sim_char()?;
    Ok(map_each_char(text, m))
}

fn sim2tra_char_mode(text: &str) -> Result<String> {
    let m = dict::sim2tra_char()?;
    Ok(map_each_char(text, m))
}

fn map_each_char(text: &str, m: &FxHashMap<String, String>) -> String {
    let mut out = String::with_capacity(text.len());
    // A tiny reusable buffer to avoid allocating per char.
    let mut buf = [0u8; 4];
    for c in text.chars() {
        let s: &str = c.encode_utf8(&mut buf);
        match m.get(s) {
            Some(mapped) => out.push_str(mapped),
            None => out.push(c),
        }
    }
    out
}

fn tra2sim_word_mode(text: &str) -> Result<String> {
    convert_with_trie(text, Dir::Tra)
}

fn sim2tra_word_mode(text: &str) -> Result<String> {
    convert_with_trie(text, Dir::Sim)
}

fn convert_with_trie(text: &str, want: Dir) -> Result<String> {
    let wt = word_tables()?;

    // Index text by char positions for the Python-style `text[i:i+depth]`
    // substring behavior.
    let chars: Vec<char> = text.chars().collect();
    let mut byte_idx: Vec<usize> = Vec::with_capacity(chars.len() + 1);
    {
        let mut off = 0usize;
        byte_idx.push(0);
        for c in &chars {
            off += c.len_utf8();
            byte_idx.push(off);
        }
    }

    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    let end = chars.len();

    while i < end {
        let upper = (i + wt.trie.depth()).min(end);
        let window = &text[byte_idx[i]..byte_idx[upper]];
        let (step, label) = wt.trie.longest_prefix(window);
        let matched_str = &text[byte_idx[i]..byte_idx[i + step]];

        match label {
            Some(dir) if *dir == want => {
                let table = match want {
                    Dir::Tra => &wt.tra2sim,
                    Dir::Sim => &wt.sim2tra,
                };
                match table.get(matched_str) {
                    Some(v) => out.push_str(v),
                    None => {
                        // Label claimed it's convertible but lookup missed — guard.
                        return Err(Error::InvalidArg(format!(
                            "trie/table mismatch on '{}'",
                            matched_str
                        )));
                    }
                }
            }
            // Either opposite direction or no match — copy input as-is.
            _ => out.push_str(matched_str),
        }
        i += step;
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

    #[test]
    fn tra2sim_char_basic() {
        ensure_init();
        let r = tra2sim("今天天氣好晴朗", TsMode::Char).unwrap();
        assert_eq!(r, "今天天气好晴朗");
    }

    #[test]
    fn sim2tra_char_basic() {
        ensure_init();
        let r = sim2tra("今天天气好晴朗", TsMode::Char).unwrap();
        assert_eq!(r, "今天天氣好晴朗");
    }

    #[test]
    fn char_mode_leaves_unknown_unchanged() {
        ensure_init();
        let r = tra2sim("ABC123", TsMode::Char).unwrap();
        assert_eq!(r, "ABC123");
    }

    #[test]
    fn word_mode_basic_round_trip() {
        ensure_init();
        // Word-mode round-trip is not guaranteed to be identity because some
        // 简 chars map to >1 繁 variants; but length should be preserved and
        // running word-mode on its output should at least produce *valid*
        // text (no errors).
        let src = "今天天气好晴朗,想吃方便面。";
        let t = sim2tra(src, TsMode::Word).unwrap();
        let _ = tra2sim(&t, TsMode::Word).unwrap();
    }

    #[test]
    fn word_mode_converts_idiom() {
        ensure_init();
        // "速食麵" should be recognized by tra2sim word-mode and mapped to
        // "方便面" per Python example.
        let r = tra2sim("想喫速食麵。", TsMode::Word).unwrap();
        assert!(
            r.contains("方便面"),
            "expected '方便面' in output, got: {}",
            r
        );
    }

    #[test]
    fn empty_input() {
        ensure_init();
        assert_eq!(tra2sim("", TsMode::Char).unwrap(), "");
        assert_eq!(tra2sim("", TsMode::Word).unwrap(), "");
    }
}
