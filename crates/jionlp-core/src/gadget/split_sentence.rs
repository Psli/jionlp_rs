//! Sentence splitter — port of `jionlp/gadget/split_sentence.py`.
//!
//! Splits Chinese text on punctuation, at two granularities:
//! - [`Criterion::Coarse`] — only period/exclamation/question/quote/newline
//! - [`Criterion::Fine`]   — additionally commas, colons, semicolons, ellipsis
//!
//! Front/back quote characters (「」‘’“”) carry special merge rules that
//! mirror the Python original: an opening quote attaches to the following
//! sentence; a closing quote attaches to the preceding sentence unless a
//! terminal punctuation appears immediately before it.

use once_cell::sync::Lazy;
use rustc_hash::FxHashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Criterion {
    Coarse,
    Fine,
}

static PUNCS_COARSE: Lazy<FxHashSet<&'static str>> = Lazy::new(|| {
    ["。", "！", "？", "\n", "“", "”", "‘", "’"]
        .into_iter()
        .collect()
});

static PUNCS_FINE: Lazy<FxHashSet<&'static str>> = Lazy::new(|| {
    [
        "……", "\r\n", "，", "。", ";", "；", "…", "！", "!", "?", "？", "\r", "\n", "“", "”", "‘",
        "’", "：",
    ]
    .into_iter()
    .collect()
});

static FRONT_QUOTES: Lazy<FxHashSet<&'static str>> = Lazy::new(|| ["“", "‘"].into_iter().collect());

static BACK_QUOTES: Lazy<FxHashSet<&'static str>> = Lazy::new(|| ["”", "’"].into_iter().collect());

/// Multi-char fine punctuation must be tried before single-char ones so that
/// "……" doesn't get split as two "…". "\r\n" likewise.
const FINE_MULTICHAR: &[&str] = &["……", "\r\n"];

/// Split `text` at punctuation, returning sentences with their trailing
/// punctuation attached (matching Python behavior).
pub fn split_sentence(text: &str, criterion: Criterion) -> Vec<String> {
    let tokens = tokenize(text, criterion);
    assemble(tokens, criterion)
}

/// Tokenize the input into an alternating stream of text chunks and
/// punctuation chunks. Equivalent to Python's `re.split(...)` with a
/// capturing group.
fn tokenize(text: &str, criterion: Criterion) -> Vec<String> {
    let puncs: &FxHashSet<&'static str> = match criterion {
        Criterion::Coarse => &PUNCS_COARSE,
        Criterion::Fine => &PUNCS_FINE,
    };

    let mut out: Vec<String> = Vec::new();
    let mut buf = String::new();
    let bytes = text.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        // Try multi-char fine punctuations first.
        if matches!(criterion, Criterion::Fine) {
            let mut matched = None;
            for &m in FINE_MULTICHAR {
                if text[i..].starts_with(m) {
                    matched = Some(m);
                    break;
                }
            }
            if let Some(m) = matched {
                if !buf.is_empty() {
                    out.push(std::mem::take(&mut buf));
                }
                out.push(m.to_string());
                i += m.len();
                continue;
            }
        }

        // Single char (advance by its UTF-8 length).
        let ch_end = next_char_boundary(text, i);
        let ch = &text[i..ch_end];

        if puncs.contains(ch) {
            if !buf.is_empty() {
                out.push(std::mem::take(&mut buf));
            }
            out.push(ch.to_string());
        } else {
            buf.push_str(ch);
        }
        i = ch_end;
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

#[inline]
fn next_char_boundary(s: &str, i: usize) -> usize {
    let bytes = s.as_bytes();
    let b = bytes[i];
    let len = if b < 0x80 {
        1
    } else if b < 0xC0 {
        1 // invalid leading byte — be defensive, advance 1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    };
    (i + len).min(bytes.len())
}

/// Re-assemble tokens into sentences according to JioNLP's quote-handling
/// merge rules.
fn assemble(tokens: Vec<String>, criterion: Criterion) -> Vec<String> {
    let puncs: &FxHashSet<&'static str> = match criterion {
        Criterion::Coarse => &PUNCS_COARSE,
        Criterion::Fine => &PUNCS_FINE,
    };

    let mut out: Vec<String> = Vec::new();
    let mut quote_flag = false;

    for sen in tokens {
        if sen.is_empty() {
            continue;
        }

        // Punctuation token?
        if puncs.contains(sen.as_str()) {
            if out.is_empty() {
                if FRONT_QUOTES.contains(sen.as_str()) {
                    quote_flag = true;
                }
                out.push(sen);
                continue;
            }

            if FRONT_QUOTES.contains(sen.as_str()) {
                // Front quote: attach based on whether previous sentence
                // already ended with terminal punctuation.
                let last_char = last_char_str(out.last().unwrap());
                if puncs.contains(last_char) {
                    out.push(sen);
                } else {
                    out.last_mut().unwrap().push_str(&sen);
                }
                quote_flag = true;
            } else {
                // Plain punctuation: glue to previous sentence.
                out.last_mut().unwrap().push_str(&sen);
            }
            continue;
        }

        // Non-punctuation token.
        if out.is_empty() {
            out.push(sen);
            continue;
        }

        if quote_flag {
            // Previous token was a front quote — merge with it.
            out.last_mut().unwrap().push_str(&sen);
            quote_flag = false;
            continue;
        }

        let last = out.last().unwrap();
        let last_char = last_char_str(last);
        if BACK_QUOTES.contains(last_char) {
            let last_len_chars = last.chars().count();
            if last_len_chars <= 1 {
                out.last_mut().unwrap().push_str(&sen);
            } else {
                // Look at the char before the closing quote.
                let second_last = nth_char_from_end(last, 2);
                if puncs.contains(second_last) {
                    out.push(sen);
                } else {
                    out.last_mut().unwrap().push_str(&sen);
                }
            }
        } else {
            out.push(sen);
        }
    }
    out
}

#[inline]
fn last_char_str(s: &str) -> &str {
    let mut it = s.char_indices();
    match it.next_back() {
        Some((idx, _)) => &s[idx..],
        None => "",
    }
}

/// Get the `n`-th char from the end as a `&str` (1-indexed: `n=1` is last).
/// Returns `""` if out of range.
fn nth_char_from_end(s: &str, n: usize) -> &str {
    if n == 0 {
        return "";
    }
    let mut it = s.char_indices().rev();
    for _ in 0..(n - 1) {
        if it.next().is_none() {
            return "";
        }
    }
    match it.next() {
        Some((idx, c)) => &s[idx..idx + c.len_utf8()],
        None => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fine_basic() {
        let text = "中华古汉语，泱泱大国，历史传承的瑰宝。";
        let r = split_sentence(text, Criterion::Fine);
        assert_eq!(r, vec!["中华古汉语，", "泱泱大国，", "历史传承的瑰宝。"]);
    }

    #[test]
    fn coarse_basic() {
        let text = "今天天气真好。我要去公园！你呢？";
        let r = split_sentence(text, Criterion::Coarse);
        assert_eq!(r, vec!["今天天气真好。", "我要去公园！", "你呢？"]);
    }

    #[test]
    fn coarse_does_not_split_comma() {
        let text = "中华古汉语，泱泱大国，历史传承的瑰宝。";
        let r = split_sentence(text, Criterion::Coarse);
        assert_eq!(r, vec!["中华古汉语，泱泱大国，历史传承的瑰宝。"]);
    }

    #[test]
    fn handles_ellipsis() {
        let text = "这是第一句……这是第二句。";
        let r = split_sentence(text, Criterion::Fine);
        assert_eq!(r, vec!["这是第一句……", "这是第二句。"]);
    }

    #[test]
    fn handles_mixed_newlines() {
        let text = "第一行\n第二行\r\n第三行";
        let r = split_sentence(text, Criterion::Fine);
        assert_eq!(r, vec!["第一行\n", "第二行\r\n", "第三行"]);
    }

    #[test]
    fn empty_string_returns_empty() {
        assert!(split_sentence("", Criterion::Fine).is_empty());
        assert!(split_sentence("", Criterion::Coarse).is_empty());
    }

    #[test]
    fn no_punctuation() {
        let r = split_sentence("没有标点符号的一句话", Criterion::Coarse);
        assert_eq!(r, vec!["没有标点符号的一句话"]);
    }
}
