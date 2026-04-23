//! Extractors — port of `jionlp/rule/extractor.py`.
//!
//! All extract_* functions return a `Vec<Extracted>`. When you only need the
//! text, call `.into_iter().map(|e| e.text).collect()`.
//!
//! ## Boundary handling
//!
//! Python's extractors wrap the real target in a capture group inside
//! lookbehind/lookahead boundary assertions like `(?<=[^\d])(...)(?=[^\d])`.
//! To avoid missing matches at the start/end of the string, we wrap the input
//! in a single space on both sides before scanning and subtract 1 from the
//! reported offsets. This mirrors the Python implementation's
//! `item.span()[0] - 1`.

use super::pattern::*;
use fancy_regex::Regex as FancyRegex;

/// An extracted span of text together with its byte offset in the original
/// input (inclusive start, exclusive end).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extracted {
    pub text: String,
    pub offset: (usize, usize),
}

/// Internal helper: run a fancy_regex that expects its target in capture #1,
/// after wrapping the text with sentinel spaces for boundary assertions.
fn extract_via(pattern: &FancyRegex, text: &str) -> Vec<Extracted> {
    // Wrap with spaces so leading/trailing boundary lookarounds succeed.
    let padded = format!(" {} ", text);
    let mut out = Vec::new();
    let mut idx = 0;
    while let Ok(Some(m)) = pattern.captures_from_pos(&padded, idx) {
        let grp = match m.get(1) {
            Some(g) => g,
            None => break,
        };
        let start = grp.start();
        let end = grp.end();
        // Subtract 1 for the leading sentinel.
        let real_start = start.saturating_sub(1);
        let real_end = end.saturating_sub(1);
        out.push(Extracted {
            text: grp.as_str().to_string(),
            offset: (real_start, real_end),
        });
        // Advance past this match to find the next one.
        idx = end;
        if idx >= padded.len() {
            break;
        }
    }
    out
}

/// Extract email addresses.
pub fn extract_email(text: &str) -> Vec<Extracted> {
    extract_via(&EMAIL_PATTERN, text)
}

/// Extract Chinese mobile phone numbers (1[3-9]x-prefixed, 11 digits).
pub fn extract_cell_phone(text: &str) -> Vec<Extracted> {
    extract_via(&CELL_PHONE_PATTERN, text)
}

/// Extract mainland landline numbers.
pub fn extract_landline_phone(text: &str) -> Vec<Extracted> {
    extract_via(&LANDLINE_PHONE_PATTERN, text)
}

/// Extract both cell and landline numbers (cell takes precedence).
pub fn extract_phone_number(text: &str) -> Vec<Extracted> {
    let mut all = extract_cell_phone(text);
    all.extend(extract_landline_phone(text));
    all.sort_by_key(|e| e.offset.0);
    all
}

/// Extract IPv4 addresses.
pub fn extract_ip_address(text: &str) -> Vec<Extracted> {
    extract_via(&IP_ADDRESS_PATTERN, text)
}

/// Extract 18-digit mainland Chinese ID card numbers.
pub fn extract_id_card(text: &str) -> Vec<Extracted> {
    extract_via(&ID_CARD_PATTERN, text)
}

/// Extract URLs (http/https/ftp/file and bare www.).
pub fn extract_url(text: &str) -> Vec<Extracted> {
    extract_via(&URL_PATTERN, text)
}

/// Extract QQ numbers (6-11 digits not starting with 0).
pub fn extract_qq(text: &str) -> Vec<Extracted> {
    extract_via(&QQ_PATTERN, text)
}

/// Extract license plates (mainland China, 92-format + new-energy).
pub fn extract_motor_vehicle_licence_plate(text: &str) -> Vec<Extracted> {
    extract_via(&MOTOR_VEHICLE_LICENCE_PLATE_PATTERN, text)
}

/// Extract runs of Chinese characters.
pub fn extract_chinese(text: &str) -> Vec<String> {
    // Lookaround-free; plain regex is faster.
    static P: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
        regex::Regex::new(r"[\u{4E00}-\u{9FA5}]+").unwrap()
    });
    P.find_iter(text).map(|m| m.as_str().to_string()).collect()
}

// ─────────────────────── Parentheses (paired, nested) ────────────────────────

/// Extract the contents of each matched parenthesis pair. Handles ASCII and
/// Chinese brackets, supports arbitrary nesting (each pair produces one entry
/// containing the inner text *including* inner pairs).
///
/// `table` is a string of concatenated open/close pairs, e.g. `()[]{}【】`.
pub fn extract_parentheses(text: &str, table: &str) -> Vec<Extracted> {
    let (open_to_close, all_opens, all_closes) = build_pair_tables(table);

    let mut stack: Vec<(char, usize)> = Vec::new();
    let mut out: Vec<Extracted> = Vec::new();

    for (byte_idx, ch) in text.char_indices() {
        if all_opens.contains(&ch) {
            stack.push((ch, byte_idx));
        } else if all_closes.contains(&ch) {
            if let Some(&(open_ch, open_idx)) = stack.last() {
                if open_to_close.get(&open_ch) == Some(&ch) {
                    stack.pop();
                    let end = byte_idx + ch.len_utf8();
                    out.push(Extracted {
                        text: text[open_idx..end].to_string(),
                        offset: (open_idx, end),
                    });
                }
                // If mismatched, silently skip (mirror Python's lenient behavior).
            }
        }
    }
    // Sort by starting offset for stable output.
    out.sort_by_key(|e| e.offset.0);
    out
}

fn build_pair_tables(
    table: &str,
) -> (
    rustc_hash::FxHashMap<char, char>,
    rustc_hash::FxHashSet<char>,
    rustc_hash::FxHashSet<char>,
) {
    let chars: Vec<char> = table.chars().collect();
    let mut map = rustc_hash::FxHashMap::default();
    let mut opens = rustc_hash::FxHashSet::default();
    let mut closes = rustc_hash::FxHashSet::default();
    let mut i = 0;
    while i + 1 < chars.len() {
        let o = chars[i];
        let c = chars[i + 1];
        map.insert(o, c);
        opens.insert(o);
        closes.insert(c);
        i += 2;
    }
    (map, opens, closes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_basic() {
        let r = extract_email("contact: user@example.com, see more.");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].text, "user@example.com");
    }

    #[test]
    fn email_multiple() {
        let r = extract_email("a@b.co or c@d.net please");
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn cell_phone_basic() {
        let r = extract_cell_phone("my number 13912345678 call me");
        assert_eq!(r.len(), 1);
        assert!(r[0].text.contains("13912345678"));
    }

    #[test]
    fn id_card_basic() {
        // A structurally valid (made-up) ID card number.
        let r = extract_id_card("我的身份证是 11010519900307123X，请保密");
        assert_eq!(r.len(), 1);
        assert!(r[0].text.ends_with("X") || r[0].text.ends_with("x") || r[0].text.chars().last().unwrap().is_ascii_digit());
    }

    #[test]
    fn ip_basic() {
        let r = extract_ip_address("server at 192.168.1.1 and also 10.0.0.1 online");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].text, "192.168.1.1");
    }

    #[test]
    fn url_basic() {
        let r = extract_url("see https://example.com/path?q=1. ok");
        assert_eq!(r.len(), 1);
        assert!(r[0].text.starts_with("https://"));
    }

    #[test]
    fn qq_basic() {
        let r = extract_qq("QQ号码 123456789 ，联系");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].text, "123456789");
    }

    #[test]
    fn chinese_runs() {
        let r = extract_chinese("hello 中文 world 测试 ok");
        assert_eq!(r, vec!["中文".to_string(), "测试".to_string()]);
    }

    #[test]
    fn parentheses_simple() {
        let r = extract_parentheses("hello (world)!", "()");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].text, "(world)");
    }

    #[test]
    fn parentheses_nested() {
        let r = extract_parentheses("a (b (c) d) e", "()");
        let texts: Vec<&str> = r.iter().map(|e| e.text.as_str()).collect();
        assert!(texts.contains(&"(c)"));
        assert!(texts.contains(&"(b (c) d)"));
    }

    #[test]
    fn parentheses_mixed() {
        let r = extract_parentheses("【标题】内容 (说明)", "()（）【】");
        let texts: Vec<&str> = r.iter().map(|e| e.text.as_str()).collect();
        assert!(texts.contains(&"【标题】"));
        assert!(texts.contains(&"(说明)"));
    }

    #[test]
    fn plate_basic() {
        let r = extract_motor_vehicle_licence_plate("车牌 川A·23047B 停车");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].text, "川A·23047B");
    }
}
