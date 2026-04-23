//! HTML cleansing — subset of `jionlp/rule/html_cleansing.py`.
//!
//! This initial port handles the 80%-case useful to most callers:
//!   - `remove_html_tag`  — strip `<...>`, preserving text content.
//!   - `clean_html`       — strip script/style/comments + tags + decode
//!                          a handful of common HTML entities.
//!   - `remove_redundant_char` — drop noise chars ` \t\n啊哈呀~·…` etc.
//!
//! The Python module additionally extracts meta info and prunes navigational
//! `<div>` blocks by CSS-class heuristic; those are deferred.

use once_cell::sync::Lazy;
use regex::Regex;

// `<[^<中文标点]+?>` — reluctant match; Python's pattern forbids a handful of
// CJK chars inside a tag so that "<" followed by Chinese text doesn't get
// mis-matched as a tag.
static HTML_TAG_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<[^<\u{4E00}-\u{9FA5}，。；！？、“”‘’（）—《》…●]+?>")
        .expect("HTML_TAG_PATTERN")
});

static SCRIPT_TAG_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)<script[\s\S]*?</script>").expect("SCRIPT_TAG_PATTERN")
});
static STYLE_TAG_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)<style[\s\S]*?</style>").expect("STYLE_TAG_PATTERN")
});
static COMMENT_TAG_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<!--[\s\S]*?-->").expect("COMMENT_TAG_PATTERN"));

static REDUNDANT_CHARS: &[char] = &[
    ' ', '-', '\t', '\n', '啊', '哈', '呀', '~', '\u{3000}', '\u{00A0}', '•', '·', '・',
];

// A small but pragmatic HTML entity table. Python falls back to the `html`
// module's full table; we cover the most common named entities plus a few
// numeric forms.
fn decode_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'&' {
            // Copy one UTF-8 char.
            let ch_end = next_char_boundary(input, i);
            out.push_str(&input[i..ch_end]);
            i = ch_end;
            continue;
        }
        if let Some(semi) = input[i..].find(';') {
            let ent = &input[i + 1..i + semi];
            if let Some(ch) = resolve_entity(ent) {
                out.push(ch);
                i = i + semi + 1;
                continue;
            }
        }
        out.push('&');
        i += 1;
    }
    out
}

#[inline]
fn next_char_boundary(s: &str, i: usize) -> usize {
    let b = s.as_bytes()[i];
    let len = if b < 0x80 {
        1
    } else if b < 0xC0 {
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    };
    (i + len).min(s.len())
}

fn resolve_entity(name: &str) -> Option<char> {
    if let Some(rest) = name.strip_prefix('#') {
        // Numeric: &#123; or &#xAB;
        let (radix, digits) = if let Some(hex) = rest.strip_prefix(['x', 'X']) {
            (16u32, hex)
        } else {
            (10u32, rest)
        };
        return u32::from_str_radix(digits, radix).ok().and_then(char::from_u32);
    }
    Some(match name {
        "lt" => '<',
        "gt" => '>',
        "amp" => '&',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{00A0}',
        "copy" => '©',
        "reg" => '®',
        "trade" => '™',
        "hellip" => '…',
        "mdash" => '—',
        "ndash" => '–',
        "ldquo" => '“',
        "rdquo" => '”',
        "lsquo" => '‘',
        "rsquo" => '’',
        "middot" => '·',
        _ => return None,
    })
}

/// Strip `<…>` tags, preserving their content. Does NOT touch script/style
/// body text — use [`clean_html`] for that.
pub fn remove_html_tag(text: &str) -> String {
    HTML_TAG_PATTERN.replace_all(text, "").into_owned()
}

/// Best-effort HTML → plain-text: drop script/style/comments, strip tags,
/// decode common entities.
pub fn clean_html(text: &str) -> String {
    let t = SCRIPT_TAG_PATTERN.replace_all(text, "");
    let t = STYLE_TAG_PATTERN.replace_all(&t, "");
    let t = COMMENT_TAG_PATTERN.replace_all(&t, "");
    let t = HTML_TAG_PATTERN.replace_all(&t, "");
    decode_entities(&t)
}

/// Drop redundant / noise chars. If `custom` is `Some`, those chars replace
/// the default set; otherwise the default noise set is used.
pub fn remove_redundant_char(text: &str, custom: Option<&str>) -> String {
    match custom {
        Some(chars) => {
            let set: rustc_hash::FxHashSet<char> = chars.chars().collect();
            text.chars().filter(|c| !set.contains(c)).collect()
        }
        None => text
            .chars()
            .filter(|c| !REDUNDANT_CHARS.contains(c))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_simple_tags() {
        assert_eq!(
            remove_html_tag("<p>hello <b>world</b></p>"),
            "hello world"
        );
    }

    #[test]
    fn preserves_chinese_lt() {
        // "<中文" should not be consumed as a start of a tag.
        assert_eq!(remove_html_tag("<p>中文<b>text</b>结束</p>"), "中文text结束");
    }

    #[test]
    fn clean_html_removes_script() {
        let html = "<html><script>var x = 1;</script><body>hi</body></html>";
        let r = clean_html(html);
        assert!(!r.contains("var x"));
        assert!(r.contains("hi"));
    }

    #[test]
    fn clean_html_removes_style() {
        let html = "<style>body{color:red}</style>hello";
        assert_eq!(clean_html(html), "hello");
    }

    #[test]
    fn clean_html_removes_comments() {
        assert_eq!(clean_html("a<!-- c -->b"), "ab");
    }

    #[test]
    fn decodes_common_entities() {
        assert_eq!(clean_html("a &amp; b &lt; c &gt; d"), "a & b < c > d");
        assert_eq!(clean_html("&#65; &#x4E2D;"), "A 中");
    }

    #[test]
    fn remove_redundant_default() {
        assert_eq!(remove_redundant_char("你 好\t哈！", None), "你好！");
    }

    #[test]
    fn remove_redundant_custom() {
        assert_eq!(
            remove_redundant_char("a!b?c.", Some("!?.")),
            "abc"
        );
    }

    #[test]
    fn extract_meta_description() {
        let html = r#"<html><head><meta name="description" content="my page"/>
            <meta name="keywords" content="a,b,c"/></head></html>"#;
        let m = extract_meta_info(html);
        assert_eq!(m.get("description").map(String::as_str), Some("my page"));
        assert_eq!(m.get("keywords").map(String::as_str), Some("a,b,c"));
    }

    #[test]
    fn remove_menu_divs() {
        let html = r#"<body><div id="menu">nav</div><div>body</div></body>"#;
        let out = remove_menu_div_tag(html);
        assert!(!out.contains("menu"));
        assert!(out.contains("body"));
    }
}

// ───────────────────────── meta + menu helpers (Round 31) ────────────────

static META_TAG_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?is)<meta[^>]*?>"#).expect("META_TAG_PATTERN")
});
static META_NAME_ATTR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)name\s*=\s*['"]([^'"]+)['"]"#).expect("META_NAME_ATTR")
});
static META_CONTENT_ATTR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?is)content\s*=\s*['"]([^'"]*)['"]"#).expect("META_CONTENT_ATTR")
});

/// Extract meta-tag attributes from an HTML document into a map.
/// Recognized keys (matching Python): description, keywords,
/// classification, language. Other keys are ignored.
pub fn extract_meta_info(html: &str) -> std::collections::HashMap<String, String> {
    let mut out: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for tag in META_TAG_PATTERN.find_iter(html) {
        let s = tag.as_str();
        let name = META_NAME_ATTR
            .captures(s)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_ascii_lowercase());
        let content = META_CONTENT_ATTR
            .captures(s)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());
        if let (Some(n), Some(c)) = (name, content) {
            if ["description", "keywords", "classification", "language"].contains(&n.as_str()) {
                out.insert(n, c);
            }
        }
    }
    out
}

/// Keywords in div id/class that mark navigational / menu blocks — mirrors
/// Python's `div_attr_remove_list` (approx).
const MENU_KEYWORDS: &[&str] = &[
    "menu", "nav", "sidebar", "header", "footer", "navbar",
    "copyright", "ad", "advert", "banner", "tab", "breadcrumb",
];

static DIV_OPEN_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?is)<div\b[^>]*>"#).expect("DIV_OPEN")
});
static ATTR_ID_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\bid\s*=\s*['"]([^'"]+)['"]"#).expect("ATTR_ID")
});
static ATTR_CLASS_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\bclass\s*=\s*['"]([^'"]+)['"]"#).expect("ATTR_CLASS")
});

/// Remove `<div>` blocks whose id/class contains any of the menu keywords.
/// Performs a forward scan with depth tracking to skip the entire subtree.
pub fn remove_menu_div_tag(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0usize;
    while cursor < html.len() {
        // Find next <div ...>.
        let slice = &html[cursor..];
        let Some(m) = DIV_OPEN_PATTERN.find(slice) else {
            out.push_str(slice);
            break;
        };
        let abs_start = cursor + m.start();
        out.push_str(&html[cursor..abs_start]);
        let tag_text = &html[abs_start..cursor + m.end()];

        let id_match = ATTR_ID_PATTERN
            .captures(tag_text)
            .and_then(|c| c.get(1))
            .map(|x| x.as_str().to_ascii_lowercase());
        let class_match = ATTR_CLASS_PATTERN
            .captures(tag_text)
            .and_then(|c| c.get(1))
            .map(|x| x.as_str().to_ascii_lowercase());
        let is_menu = MENU_KEYWORDS.iter().any(|kw| {
            id_match.as_deref().map(|s| s.contains(kw)).unwrap_or(false)
                || class_match.as_deref().map(|s| s.contains(kw)).unwrap_or(false)
        });

        if is_menu {
            // Skip matching close tag at balanced depth.
            let mut depth = 1i32;
            let mut scan = cursor + m.end();
            while depth > 0 && scan < html.len() {
                let rest = &html[scan..];
                let open_match = DIV_OPEN_PATTERN.find(rest);
                let close_match = rest.find("</div>");
                match (open_match, close_match) {
                    (Some(om), Some(ci)) if om.start() < ci => {
                        depth += 1;
                        scan += om.end();
                    }
                    (_, Some(ci)) => {
                        depth -= 1;
                        scan += ci + "</div>".len();
                    }
                    _ => break,
                }
            }
            cursor = scan;
        } else {
            out.push_str(tag_text);
            cursor += m.end();
        }
    }
    out
}
