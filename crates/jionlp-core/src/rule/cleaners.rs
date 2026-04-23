//! Port of Python Extractor's remove_* / replace_* family plus `clean_text`
//! and `convert_full2half`. Each function mirrors Python's signature:
//!
//! * `remove_X(text) -> String` — delete matched spans.
//! * `replace_X(text, placeholder) -> String` — substitute matched spans
//!   with `placeholder`.
//!
//! Implementation: reuse the lookaround-style fancy_regex patterns from
//! `rule/pattern.rs`. For replace, we run the pattern iteratively and
//! stitch the unchanged chunks with the placeholder.

use super::pattern::*;
use fancy_regex::Regex as FancyRegex;

/// Core helper: walk a fancy_regex with lookbehind padding and substitute
/// capture group #1 with `replacement`. If `replacement` is empty, spans
/// are removed.
fn replace_via(pattern: &FancyRegex, text: &str, replacement: &str) -> String {
    let padded = format!(" {} ", text);
    let mut result = String::with_capacity(padded.len());
    let mut idx = 0usize;
    let bytes = padded.as_bytes();
    while idx < bytes.len() {
        match pattern.captures_from_pos(&padded, idx) {
            Ok(Some(caps)) => {
                if let Some(m) = caps.get(1) {
                    if m.start() > idx {
                        result.push_str(&padded[idx..m.start()]);
                    }
                    result.push_str(replacement);
                    idx = m.end();
                    if m.start() == m.end() {
                        // Guard against zero-width match.
                        idx += 1;
                    }
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
    if idx < bytes.len() {
        result.push_str(&padded[idx..]);
    }
    // Strip the leading/trailing sentinel spaces.
    let trimmed = result
        .strip_prefix(' ')
        .unwrap_or(&result)
        .strip_suffix(' ')
        .unwrap_or("");
    trimmed.to_string()
}

// ───────────────────────── email ───────────────────────────────────────────

pub fn remove_email(text: &str) -> String {
    replace_via(&EMAIL_PATTERN, text, "")
}
/// Remove email addresses AND their prefix tokens (`E-mail:` / `邮箱:` etc).
/// Python's `remove_email(text, delete_prefix=True)` equivalent.
pub fn remove_email_with_prefix(text: &str) -> String {
    static PREFIX: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
        regex::Regex::new(
            r"(?i)(e(\-|—| |_)?mail|(电子)?邮[箱件])(地址)?[:：\t \u{3000}]*",
        )
        .unwrap()
    });
    let cleaned = replace_via(&EMAIL_PATTERN, text, "");
    PREFIX.replace_all(&cleaned, "").to_string()
}
pub fn replace_email(text: &str, placeholder: &str) -> String {
    replace_via(&EMAIL_PATTERN, text, placeholder)
}

// ───────────────────────── URL ─────────────────────────────────────────────

pub fn remove_url(text: &str) -> String {
    replace_via(&URL_PATTERN, text, "")
}
/// Remove URLs + common prefix tokens (`网址:` / `url:` etc).
pub fn remove_url_with_prefix(text: &str) -> String {
    static PREFIX: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
        regex::Regex::new(r"(?i)(网址|地址|链接|url|link)[:：\t \u{3000}]*").unwrap()
    });
    let cleaned = remove_url(text);
    PREFIX.replace_all(&cleaned, "").to_string()
}
pub fn replace_url(text: &str, placeholder: &str) -> String {
    replace_via(&URL_PATTERN, text, placeholder)
}

// ───────────────────────── phone ───────────────────────────────────────────

pub fn remove_phone_number(text: &str) -> String {
    let t = replace_via(&CELL_PHONE_PATTERN, text, "");
    replace_via(&LANDLINE_PHONE_PATTERN, &t, "")
}
/// Remove phone numbers + common prefix tokens (`电话:` / `tel:` / `手机号:` etc).
pub fn remove_phone_number_with_prefix(text: &str) -> String {
    static PREFIX: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
        regex::Regex::new(
            r"(?i)(手机(号码?)?|(固定)?电话|tel(ephone)?|phone)[:：\t \u{3000}]*",
        )
        .unwrap()
    });
    let cleaned = remove_phone_number(text);
    PREFIX.replace_all(&cleaned, "").to_string()
}
pub fn replace_phone_number(text: &str, placeholder: &str) -> String {
    let t = replace_via(&CELL_PHONE_PATTERN, text, placeholder);
    replace_via(&LANDLINE_PHONE_PATTERN, &t, placeholder)
}

// ───────────────────────── IP ──────────────────────────────────────────────

pub fn remove_ip_address(text: &str) -> String {
    replace_via(&IP_ADDRESS_PATTERN, text, "")
}
pub fn replace_ip_address(text: &str, placeholder: &str) -> String {
    replace_via(&IP_ADDRESS_PATTERN, text, placeholder)
}

// ───────────────────────── ID card ─────────────────────────────────────────

pub fn remove_id_card(text: &str) -> String {
    replace_via(&ID_CARD_PATTERN, text, "")
}
pub fn replace_id_card(text: &str, placeholder: &str) -> String {
    replace_via(&ID_CARD_PATTERN, text, placeholder)
}

// ───────────────────────── QQ ──────────────────────────────────────────────

pub fn remove_qq(text: &str) -> String {
    replace_via(&QQ_PATTERN, text, "")
}
pub fn replace_qq(text: &str, placeholder: &str) -> String {
    replace_via(&QQ_PATTERN, text, placeholder)
}

// ───────────────────────── Chinese ─────────────────────────────────────────

/// Replace every run of Chinese characters with `placeholder`.
pub fn replace_chinese(text: &str, placeholder: &str) -> String {
    static P: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
        regex::Regex::new(r"[\u{4E00}-\u{9FA5}]+").unwrap()
    });
    P.replace_all(text, placeholder).to_string()
}

// ───────────────────────── parentheses ────────────────────────────────────

/// Remove parentheses and their inner text (all paired forms by default).
pub fn remove_parentheses(text: &str, table: Option<&str>) -> String {
    let table = table.unwrap_or(PARENTHESES_TABLE);
    let chars: Vec<char> = table.chars().collect();
    let open_to_close: std::collections::HashMap<char, char> = chars
        .chunks(2)
        .filter_map(|p| if p.len() == 2 { Some((p[0], p[1])) } else { None })
        .collect();
    let opens: std::collections::HashSet<char> = open_to_close.keys().copied().collect();
    let closes: std::collections::HashSet<char> = open_to_close.values().copied().collect();

    // Use a stack-based single pass: when we open, push; when we close and
    // stack non-empty and it matches, pop; skip any char inside stack depth > 0.
    let mut out = String::with_capacity(text.len());
    let mut stack: Vec<char> = Vec::new();
    for c in text.chars() {
        if opens.contains(&c) {
            stack.push(c);
            continue;
        }
        if closes.contains(&c) {
            if let Some(&open) = stack.last() {
                if open_to_close.get(&open) == Some(&c) {
                    stack.pop();
                    continue;
                }
            }
            continue;
        }
        if stack.is_empty() {
            out.push(c);
        }
    }
    out
}

/// Replace every paired parenthesis span with `placeholder`.
pub fn replace_parentheses(text: &str, placeholder: &str, table: Option<&str>) -> String {
    let spans = crate::rule::extractor::extract_parentheses(text, table.unwrap_or(PARENTHESES_TABLE));
    if spans.is_empty() {
        return text.to_string();
    }
    // Deduplicate overlapping outer/inner spans — keep outermost only.
    let mut sorted: Vec<&crate::rule::extractor::Extracted> = spans.iter().collect();
    sorted.sort_by_key(|e| e.offset.0);
    let mut kept: Vec<&crate::rule::extractor::Extracted> = Vec::new();
    for s in sorted {
        if kept.last().map(|last| s.offset.0 < last.offset.1).unwrap_or(false) {
            continue; // nested within the current outer span.
        }
        kept.push(s);
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    for s in kept {
        if s.offset.0 > cursor {
            out.push_str(&text[cursor..s.offset.0]);
        }
        out.push_str(placeholder);
        cursor = s.offset.1;
    }
    if cursor < text.len() {
        out.push_str(&text[cursor..]);
    }
    out
}

// ───────────────────────── exception & full2half ──────────────────────────

/// Strip "exception" characters (corrupted/rare CJK chars). Mirrors
/// Python's `remove_exception_char` — keeps common CJK, ASCII, CJK
/// punctuation, and numeric; drops everything else.
pub fn remove_exception_char(text: &str) -> String {
    text.chars()
        .filter(|c| {
            c.is_ascii()
                || *c == ' '
                || ('\u{4E00}'..='\u{9FA5}').contains(c)
                || ('\u{3000}'..='\u{303F}').contains(c)   // CJK punctuation
                || ('\u{FF00}'..='\u{FFEF}').contains(c)   // fullwidth forms
                || ('\u{2E80}'..='\u{2EFF}').contains(c)   // CJK radicals sup
                || ('\u{2F00}'..='\u{2FDF}').contains(c)   // Kangxi radicals
        })
        .collect()
}

/// Convert full-width characters (０-９A-Ｚａ-ｚ + 、。，etc.) to half-width.
pub fn convert_full2half(text: &str) -> String {
    text.chars()
        .map(|c| {
            let code = c as u32;
            if code == 0x3000 {
                ' '
            } else if (0xFF01..=0xFF5E).contains(&code) {
                // ！..～ → !..~ (ASCII 0x21..0x7E)
                let mapped = (code - 0xFEE0) as u32;
                char::from_u32(mapped).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

// ───────────────────────── clean_text aggregate ────────────────────────────

/// Aggregate text cleaner mirroring Python's `clean_text(text, ...)`.
///
/// Default pipeline (when all flags are `true`):
///   1. remove HTML tags
///   2. remove exception chars
///   3. convert full-width to half-width
///   4. remove redundant whitespace
///   5. remove parentheses-wrapped content
///   6. remove URLs
///   7. remove emails
///   8. remove phone numbers
pub fn clean_text(
    text: &str,
    remove_html: bool,
    remove_exception: bool,
    full2half: bool,
    dedupe_whitespace: bool,
    strip_parens: bool,
    strip_url: bool,
    strip_email: bool,
    strip_phone: bool,
) -> String {
    let mut t = text.to_string();
    if remove_html {
        t = crate::rule::html_cleansing::remove_html_tag(&t);
    }
    if remove_exception {
        t = remove_exception_char(&t);
    }
    if full2half {
        t = convert_full2half(&t);
    }
    if dedupe_whitespace {
        t = crate::rule::html_cleansing::remove_redundant_char(&t, None);
    }
    if strip_parens {
        t = remove_parentheses(&t, None);
    }
    if strip_url {
        t = remove_url(&t);
    }
    if strip_email {
        t = remove_email(&t);
    }
    if strip_phone {
        t = remove_phone_number(&t);
    }
    t
}

// ───────────────────────── extract_wechat_id ──────────────────────────────

/// Very loose WeChat ID extractor — 6-20 chars, starts with a letter,
/// allowed charset `a-zA-Z0-9_-`. Emits unique matches in order.
pub fn extract_wechat_id(text: &str) -> Vec<String> {
    static P: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
        regex::Regex::new(r"(?i)\b[a-z][a-z0-9_-]{5,19}\b").unwrap()
    });
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut out = Vec::new();
    for m in P.find_iter(text) {
        let s = m.as_str().to_string();
        if seen.insert(s.clone()) {
            out.push(s);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_email_strips() {
        assert_eq!(remove_email("a b alice@example.com c"), "a b  c");
    }

    #[test]
    fn replace_email_substitutes() {
        assert_eq!(
            replace_email("contact alice@example.com today", "<EMAIL>"),
            "contact <EMAIL> today"
        );
    }

    #[test]
    fn remove_parens_simple() {
        assert_eq!(remove_parentheses("a (b c) d", None), "a  d");
        assert_eq!(remove_parentheses("前文（附注：xx）后文", None), "前文后文");
    }

    #[test]
    fn convert_full2half_basic() {
        assert_eq!(convert_full2half("Ａ"), "A");
        assert_eq!(convert_full2half("１２３"), "123");
        assert_eq!(convert_full2half("　"), " ");
    }

    #[test]
    fn replace_chinese_with_token() {
        assert_eq!(
            replace_chinese("hello 世界 and 测试", "<ZH>"),
            "hello <ZH> and <ZH>"
        );
    }

    #[test]
    fn clean_text_pipeline_all_on() {
        let dirty = "<p>你好 (内部) https://x.com a@b.com</p>";
        let r = clean_text(dirty, true, true, true, true, true, true, true, true);
        assert!(!r.contains("<p>"));
        assert!(!r.contains("(内部)"));
        assert!(!r.contains("https"));
        assert!(!r.contains("a@b.com"));
    }
}
