//! Per-character radical / structure / coding lookup — port of
//! `jionlp/gadget/char_radical.py`.
//!
//! For each char in the input, returns:
//! - radical (部首),
//! - structure (字形结构 — 一体 / 左右 / 上下 / …),
//! - corner coding (四角编码),
//! - stroke order (笔画顺序 — digits),
//! - wubi coding (五笔 86/98 编码).
//!
//! Unknown chars (ASCII, digits, rare hanzi not in the dict) yield a
//! placeholder entry so the output list stays aligned with the input.

use crate::dict::{self, CharInfo};
use crate::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadicalInfo {
    pub char: char,
    pub radical: String,
    pub structure: String,
    pub corner_coding: String,
    pub stroke_order: String,
    pub wubi_coding: String,
}

const CR_UNK: &str = "<cr_unk>";
const CORNER_UNK: &str = "00000";
const WUBI_UNK: &str = "XXXX";
const STROKE_UNK: &str = "<so_unk>";

fn from_dict(c: char, info: &CharInfo) -> RadicalInfo {
    RadicalInfo {
        char: c,
        radical: info.radical.clone(),
        structure: info.structure.to_string(),
        corner_coding: info.corner_coding.clone(),
        stroke_order: info.stroke_order.clone(),
        wubi_coding: info.wubi_coding.clone(),
    }
}

fn unknown(c: char) -> RadicalInfo {
    RadicalInfo {
        char: c,
        radical: CR_UNK.to_string(),
        structure: "一体结构".to_string(),
        corner_coding: CORNER_UNK.to_string(),
        stroke_order: STROKE_UNK.to_string(),
        wubi_coding: WUBI_UNK.to_string(),
    }
}

/// Look up radical/structure info for every char in `text`. Returns a list
/// of the same length as `text.chars().count()`.
pub fn char_radical(text: &str) -> Result<Vec<RadicalInfo>> {
    let dict = dict::char_dictionary()?;
    let out: Vec<RadicalInfo> = text
        .chars()
        .map(|c| match dict.get(&c) {
            Some(info) => from_dict(c, info),
            None => unknown(c),
        })
        .collect();
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
    fn length_matches_input() {
        ensure_init();
        let text = "今天L.A.洛杉矶";
        let r = char_radical(text).unwrap();
        assert_eq!(r.len(), text.chars().count());
    }

    #[test]
    fn hanzi_lookup() {
        ensure_init();
        let r = char_radical("天").unwrap();
        assert_eq!(r.len(), 1);
        assert_ne!(r[0].radical, CR_UNK, "天 should be in the dictionary");
        assert!(!r[0].structure.is_empty());
    }

    #[test]
    fn ascii_fallback() {
        ensure_init();
        let r = char_radical("A").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].radical, CR_UNK);
        assert_eq!(r[0].corner_coding, CORNER_UNK);
    }

    #[test]
    fn mixed_input() {
        ensure_init();
        let r = char_radical("hi中A").unwrap();
        assert_eq!(r.len(), 4);
        // 'h' unknown
        assert_eq!(r[0].radical, CR_UNK);
        // '中' should be known
        assert_ne!(r[2].radical, CR_UNK);
    }
}
