//! Pinyin annotation — simplified port of `jionlp/gadget/pinyin.py`.
//!
//! Current scope: per-character lookup only. Each char's primary pinyin is
//! pulled from `chinese_char_dictionary.zip`'s gloss field (parsed by
//! `dict::char_dictionary()`).
//!
//! Not yet ported: phrase-level lookups via `pinyin_phrase.zip` (greedy
//! longest-prefix match). That adds a second trie and handles cases like
//! "任性" vs "任家萱" where context changes a char's reading. Tracked in
//! PLAN.md.
//!
//! ## Formats
//!
//! * `Standard`  — accented form: "zhōng", "huá", "rén"
//! * `Simple`    — ascii + trailing tone digit: "zhong1", "hua2"
//! * `Detail`    — split into consonant / vowel / tone
//!
//! Non-Chinese chars fall back to `<py_unk>`.

use crate::dict;
use crate::trie::LabeledTrie;
use crate::Result;
use once_cell::sync::OnceCell;

pub const PY_UNK: &str = "<py_unk>";

/// Cached trie over every phrase in `dict::pinyin_phrase()`. Label is the
/// phrase's string key; the caller looks up readings from the dict map.
static PHRASE_TRIE: OnceCell<LabeledTrie<String>> = OnceCell::new();

fn phrase_trie() -> Result<&'static LabeledTrie<String>> {
    PHRASE_TRIE.get_or_try_init(|| {
        let phrases = dict::pinyin_phrase()?;
        let mut trie: LabeledTrie<String> = LabeledTrie::new();
        for key in phrases.keys() {
            trie.insert(key, key.clone());
        }
        Ok(trie)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinyinFormat {
    Standard,
    Simple,
    Detail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinyinEntry {
    Standard(String),
    Simple(String),
    Detail(PinyinDetail),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PinyinDetail {
    pub consonant: String,
    pub vowel: String,
    pub tone: String,
}

/// Annotate each char in `text` with its primary pinyin reading.
///
/// As of round 8, the lookup strategy is:
///   1. Try greedy-longest phrase match at the current position
///      (`pinyin_phrase.zip`, ~80k phrases).
///   2. Fall back to single-char reading from `char_dictionary()`.
///
/// This resolves heteronym (多音字) ambiguities — e.g. `任家萱` gets `rén`
/// (the surname reading) instead of the primary `rèn`.
pub fn pinyin(text: &str, format: PinyinFormat) -> Result<Vec<PinyinEntry>> {
    let char_map = dict::char_dictionary()?;
    let phrases = dict::pinyin_phrase()?;
    let trie = phrase_trie()?;

    // Index text by char positions so we can slice substrings by char count.
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

    let mut out: Vec<PinyinEntry> = Vec::with_capacity(chars.len());
    let mut i = 0usize;
    let end = chars.len();

    while i < end {
        // Try phrase match first.
        let upper = (i + trie.depth()).min(end);
        let window = &text[byte_idx[i]..byte_idx[upper]];
        let (step, label) = trie.longest_prefix(window);

        if step > 1 {
            if let Some(key) = label {
                if let Some(readings) = phrases.get(key) {
                    for std in readings.iter() {
                        out.push(format_entry(std, format));
                    }
                    i += step;
                    continue;
                }
            }
        }

        // Single-char fallback.
        let c = chars[i];
        let primary = char_map
            .get(&c)
            .and_then(|info| info.pinyin.first().map(String::as_str));
        out.push(match primary {
            Some(std) => format_entry(std, format),
            None => unk_entry(format),
        });
        i += 1;
    }

    Ok(out)
}

fn format_entry(standard: &str, format: PinyinFormat) -> PinyinEntry {
    match format {
        PinyinFormat::Standard => PinyinEntry::Standard(standard.to_string()),
        PinyinFormat::Simple => PinyinEntry::Simple(standard_to_simple(standard)),
        PinyinFormat::Detail => {
            PinyinEntry::Detail(detail_from_simple(&standard_to_simple(standard)))
        }
    }
}

fn unk_entry(format: PinyinFormat) -> PinyinEntry {
    match format {
        PinyinFormat::Standard => PinyinEntry::Standard(PY_UNK.to_string()),
        PinyinFormat::Simple => PinyinEntry::Simple(PY_UNK.to_string()),
        PinyinFormat::Detail => PinyinEntry::Detail(PinyinDetail::default()),
    }
}

/// Convert the accented standard form (e.g. "zhōng") to the ASCII-plus-tone
/// form (e.g. "zhong1"). Tone 5 (neutral) is used when no accent is found.
pub fn standard_to_simple(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 1);
    let mut tone = '5';
    for c in s.chars() {
        let (letter, t) = normalize_vowel(c);
        out.push(letter);
        if let Some(nt) = t {
            tone = nt;
        }
    }
    out.push(tone);
    out
}

fn normalize_vowel(c: char) -> (char, Option<char>) {
    // Returns (ascii letter, Some(tone digit) if the char carries a tone mark).
    match c {
        'à' => ('a', Some('4')),
        'á' => ('a', Some('2')),
        'ā' => ('a', Some('1')),
        'ǎ' => ('a', Some('3')),
        'ò' => ('o', Some('4')),
        'ó' => ('o', Some('2')),
        'ō' => ('o', Some('1')),
        'ǒ' => ('o', Some('3')),
        'è' => ('e', Some('4')),
        'é' => ('e', Some('2')),
        'ē' => ('e', Some('1')),
        'ě' => ('e', Some('3')),
        'ì' => ('i', Some('4')),
        'í' => ('i', Some('2')),
        'ī' => ('i', Some('1')),
        'ǐ' => ('i', Some('3')),
        'ù' => ('u', Some('4')),
        'ú' => ('u', Some('2')),
        'ū' => ('u', Some('1')),
        'ǔ' => ('u', Some('3')),
        'ǜ' => ('v', Some('4')),
        'ǘ' => ('v', Some('2')),
        'ǖ' => ('v', Some('1')),
        'ǚ' => ('v', Some('3')),
        'ü' => ('v', None),
        'ǹ' => ('n', Some('4')),
        'ń' => ('n', Some('2')),
        'ň' => ('n', Some('3')),
        'ḿ' => ('m', Some('2')),
        other => (other, None),
    }
}

/// Split a simple-form pinyin (e.g. "zhong1") into consonant/vowel/tone.
pub fn detail_from_simple(simple: &str) -> PinyinDetail {
    // Multi-char consonants must be tried before single-char.
    const MULTI: &[&str] = &["zh", "ch", "sh", "ng", "hm", "hng"];
    const SINGLE: &[char] = &[
        'b', 'c', 'd', 'f', 'g', 'h', 'j', 'k', 'l', 'm', 'n', 'p', 'q', 'r', 's', 't', 'w', 'x',
        'y', 'z',
    ];

    let mut consonant = String::new();
    let rest: &str = {
        let mut matched = None;
        for m in MULTI {
            if simple.starts_with(m) {
                consonant.push_str(m);
                matched = Some(&simple[m.len()..]);
                break;
            }
        }
        match matched {
            Some(r) => r,
            None => {
                let first = simple.chars().next().unwrap_or(' ');
                if SINGLE.contains(&first) {
                    consonant.push(first);
                    &simple[first.len_utf8()..]
                } else {
                    simple
                }
            }
        }
    };

    let (vowel, tone) = match rest.chars().last() {
        Some(c) if c.is_ascii_digit() => {
            (rest[..rest.len() - c.len_utf8()].to_string(), c.to_string())
        }
        _ => (rest.to_string(), String::new()),
    };

    PinyinDetail {
        consonant,
        vowel,
        tone,
    }
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
    fn standard_basic() {
        ensure_init();
        let r = pinyin("中国", PinyinFormat::Standard).unwrap();
        assert_eq!(r.len(), 2);
        match &r[0] {
            PinyinEntry::Standard(s) => assert_eq!(s, "zhōng"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn simple_format() {
        ensure_init();
        let r = pinyin("中国", PinyinFormat::Simple).unwrap();
        match &r[0] {
            PinyinEntry::Simple(s) => assert_eq!(s, "zhong1"),
            _ => panic!(),
        }
        match &r[1] {
            PinyinEntry::Simple(s) => assert_eq!(s, "guo2"),
            _ => panic!(),
        }
    }

    #[test]
    fn detail_format() {
        ensure_init();
        let r = pinyin("中", PinyinFormat::Detail).unwrap();
        match &r[0] {
            PinyinEntry::Detail(d) => {
                assert_eq!(d.consonant, "zh");
                assert_eq!(d.vowel, "ong");
                assert_eq!(d.tone, "1");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn ascii_fallback() {
        ensure_init();
        let r = pinyin("A", PinyinFormat::Standard).unwrap();
        match &r[0] {
            PinyinEntry::Standard(s) => assert_eq!(s, PY_UNK),
            _ => panic!(),
        }
    }

    #[test]
    fn length_matches_input() {
        ensure_init();
        let text = "中文hello测试";
        let r = pinyin(text, PinyinFormat::Standard).unwrap();
        assert_eq!(r.len(), text.chars().count());
    }

    #[test]
    fn standard_to_simple_samples() {
        assert_eq!(standard_to_simple("zhōng"), "zhong1");
        assert_eq!(standard_to_simple("guó"), "guo2");
        assert_eq!(standard_to_simple("lǜ"), "lv4");
        // No tone mark → tone 5 (neutral).
        assert_eq!(standard_to_simple("de"), "de5");
    }

    #[test]
    fn phrase_trie_matches_idiom() {
        ensure_init();
        // "一丘之貉" is in pinyin_phrase.txt → ["yī", "qiū", "zhī", "hé"].
        // Note: single-char fallback would give "mò" for 貉 (more common
        // reading); phrase match should override.
        let r = pinyin("一丘之貉", PinyinFormat::Standard).unwrap();
        let plain: Vec<String> = r
            .iter()
            .map(|e| match e {
                PinyinEntry::Standard(s) => s.clone(),
                _ => panic!(),
            })
            .collect();
        assert_eq!(plain, vec!["yī", "qiū", "zhī", "hé"]);
    }
}
