//! Regex patterns ported from `jionlp/rule/rule_pattern.py`.
//!
//! Patterns that use lookaround compile against [`fancy_regex::Regex`];
//! lookaround-free patterns use the faster `regex::Regex`. In either case the
//! compiled pattern lives inside a `Lazy` so the regex is built once and
//! shared.
//!
//! Capture group #1 is the *content* you want — mirroring the Python
//! patterns that wrap the target in `( ... )` inside boundary lookarounds.

use fancy_regex::Regex as FancyRegex;
use once_cell::sync::Lazy;
use regex::Regex;

// ─────────────────────────── Character-class patterns ────────────────────────

/// `[一-龥]` — GB2312-ish CJK range. Matches a single Chinese char.
pub const CHINESE_CHAR_PATTERN: &str = r"[\u{4E00}-\u{9FA5}]";

/// One or more Chinese chars (handy for `extract_chinese`).
pub const CHINESE_CHARS_PATTERN: &str = r"[\u{4E00}-\u{9FA5}]+";

/// Extended CJK (gb13000.1 + extensions).
pub const ANCIENT_CHINESE_CHAR_PATTERN: &str = r"[\u{4E00}-\u{9FA5}\u{3400}-\u{4DB5}]";

// ──────────────────────────────── Email ──────────────────────────────────────

const EMAIL_BOUNDARY: &str = r"[^0-9a-zA-Z!#\$%&'\*\+\-/=\?\^_`\{\|\}~]";

pub static EMAIL_PATTERN: Lazy<FancyRegex> = Lazy::new(|| {
    FancyRegex::new(&format!(
        r"(?<={b})([a-zA-Z0-9_.\-]+@[a-zA-Z0-9_.\-]+(?:\.[a-zA-Z0-9]+)*\.[a-zA-Z0-9]{{2,6}})(?={b})",
        b = EMAIL_BOUNDARY
    ))
    .expect("EMAIL_PATTERN")
});

// ─────────────────────────── Phone numbers ───────────────────────────────────

/// Cell-phone number (prefix 13-19, 11 digits total, optional +86, dashes).
pub static CELL_PHONE_PATTERN: Lazy<FancyRegex> = Lazy::new(|| {
    FancyRegex::new(
        r"(?<=[^\d])(((\+86)?([\- ])?)?((1[3-9][0-9]))([\- ])?\d{4}([\- ])?\d{4})(?=[^\d])",
    )
    .expect("CELL_PHONE_PATTERN")
});

/// Cell-phone without boundary checks (for validation).
pub static CELL_PHONE_CHECK_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^((1[3-9][0-9]))([\- ])?\d{4}([\- ])?\d{4}$").expect("CELL_PHONE_CHECK")
});

/// Mainland Chinese landline.
pub static LANDLINE_PHONE_PATTERN: Lazy<FancyRegex> = Lazy::new(|| {
    FancyRegex::new(
        r"(?<=[^\d])(([\(（])?0\d{2,3}[\)） \u{2014}\-]{1,2}\d{7,8}|\d{3,4}[ \-]\d{3,4}[ \-]\d{4})(?=[^\d])",
    )
    .expect("LANDLINE_PHONE_PATTERN")
});

/// Captures the area-code prefix of a landline number — `(0\d{2,3})` followed
/// by a separator. Group 1 is the area code.
pub static LANDLINE_PHONE_AREA_CODE_PATTERN: Lazy<FancyRegex> =
    Lazy::new(|| FancyRegex::new(r"(0\d{2,3})[\)） \u{2014}\-]").expect("LANDLINE_AREA"));

// ──────────────────────────────── ID card ────────────────────────────────────

/// Full 18-char mainland ID number extractor.
///
/// First 6 digits: provincial admin code (1x, 2x, 3x, 4x, 5x, 6x, 71/81/82/91).
/// Next 8: birthdate (YYYYMMDD).
/// Next 3: sequence.
/// Last: checksum (0-9 or X).
pub static ID_CARD_PATTERN: Lazy<FancyRegex> = Lazy::new(|| {
    FancyRegex::new(concat!(
        r"(?<=[^0-9a-zA-Z])",
        r"((1[1-5]|2[1-3]|3[1-7]|4[1-6]|5[0-4]|6[1-5]|71|81|82|91)",
        r"(0[0-9]|1[0-9]|2[0-9]|3[0-9]|4[0-3]|5[1-3]|90)",
        r"(0[0-9]|1[0-9]|2[0-9]|3[0-9]|4[0-3]|5[1-7]|6[1-4]|7[1-4]|8[1-7])",
        r"(18|19|20)\d{2}",
        r"(0[1-9]|1[0-2])",
        r"(0[1-9]|[12][0-9]|3[01])",
        r"\d{3}[0-9Xx])",
        r"(?=[^0-9a-zA-Z])"
    ))
    .expect("ID_CARD_PATTERN")
});

/// Full-string validator form of the ID card pattern.
pub static ID_CARD_CHECK_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(concat!(
        r"^(1[1-5]|2[1-3]|3[1-7]|4[1-6]|5[0-4]|6[1-5]|71|81|82|91)",
        r"(0[0-9]|1[0-9]|2[0-9]|3[0-4]|4[0-3]|5[1-3]|90)",
        r"(0[0-9]|1[0-9]|2[0-9]|3[0-9]|4[0-3]|5[1-7]|6[1-4]|7[1-4]|8[1-7])",
        r"(18|19|20)\d{2}",
        r"(0[1-9]|1[0-2])",
        r"(0[1-9]|[12][0-9]|3[01])",
        r"\d{3}[0-9Xx]$"
    ))
    .expect("ID_CARD_CHECK_PATTERN")
});

// ────────────────────────────────── IP ───────────────────────────────────────

/// One IPv4 octet (0-255).
const IP_OCTET: &str = r"(25[0-5]|2[0-4]\d|[01]?\d\d?)";

pub static IP_ADDRESS_PATTERN: Lazy<FancyRegex> = Lazy::new(|| {
    FancyRegex::new(&format!(
        r"(?<=[^0-9])({o}\.{o}\.{o}\.{o})(?=[^0-9])",
        o = IP_OCTET
    ))
    .expect("IP_ADDRESS_PATTERN")
});

// ──────────────────────────────── URL ────────────────────────────────────────

pub static URL_PATTERN: Lazy<FancyRegex> = Lazy::new(|| {
    FancyRegex::new(concat!(
        r"(?<=[^.])(",
        r"(?:(?:https?|ftp|file)://|(?<![a-zA-Z\-.])www\.)",
        r"[\-A-Za-z0-9\+&@\(\)#/%\?=\~_\|!:,\.;]+",
        r"[\-A-Za-z0-9\+&@#/%=\~_\|]",
        r")",
        r#"(?=[\.<\u{4E00}-\u{9FA5}￥"，。；！？、“”‘’>（）—《》…● \t\n])"#
    ))
    .expect("URL_PATTERN")
});

// ──────────────────────────────── QQ ─────────────────────────────────────────

pub static QQ_PATTERN: Lazy<FancyRegex> =
    Lazy::new(|| FancyRegex::new(r"(?<=[^0-9])([1-9][0-9]{5,10})(?=[^0-9])").expect("QQ"));

pub const STRICT_QQ_PATTERN: &str = r"(qq|QQ|\+q|\+Q|加q|加Q|q号|Q号)";

// ───────────────────────── Motor vehicle plate ───────────────────────────────

/// Provincial-abbrev characters used as the first char of mainland plates.
/// Chars 港澳台 are excluded via `[3..]` in Python — only the mainland set.
pub const CHINA_PROVINCE_MAINLAND: &str =
    "京津沪渝黑吉辽新藏青蒙晋冀豫甘陕川贵云宁苏浙皖鲁赣鄂湘粤闽桂琼";

pub static MOTOR_VEHICLE_LICENCE_PLATE_PATTERN: Lazy<FancyRegex> = Lazy::new(|| {
    FancyRegex::new(&format!(
        r"([{p}][A-HJ-NP-Za-hj-np-z][·. 　]?[A-HJ-NP-Za-hj-np-z0-9]{{5,6}})(?![\da-zA-Z])",
        p = CHINA_PROVINCE_MAINLAND
    ))
    .expect("MVLP")
});

/// Standalone validator form (no lookaround) — matches the *entire* input.
pub static MOTOR_VEHICLE_LICENCE_PLATE_CHECK_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"^[{p}][A-HJ-NP-Za-hj-np-z][·. \u{{3000}}]?[A-HJ-NP-Za-hj-np-z0-9]{{5,6}}$",
        p = CHINA_PROVINCE_MAINLAND
    ))
    .expect("MVLP_CHECK")
});

pub static NEV_SMALL_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([ABCDEFGHJK][A-HJ-NP-Za-hj-np-z]\d{4}|[ABCDEFGHJK]\d{5})$").expect("NEV_SMALL")
});

pub static NEV_BIG_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(\d{5}[ABCDEFGHJK])$").expect("NEV_BIG"));

// ─────────────────────── Paired parentheses table ────────────────────────────

/// "左括号1右括号1左括号2右括号2…" — same ordering as Python PARENTHESES_PATTERN.
pub const PARENTHESES_TABLE: &str = "{}「」[]【】()（）<>《》〈〉『』〔〕｛｝＜＞〖〗";

/// Convenience check patterns — whole-string validators.
pub static WHOLE_CHINESE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[\u{4E00}-\u{9FA5}]+$").expect("WHOLE_CHINESE"));

pub static ANY_CHINESE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[\u{4E00}-\u{9FA5}]").expect("ANY_CHINESE"));

pub static WHOLE_ARABIC_NUM_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[0-9]+$").expect("WHOLE_ARABIC"));

pub static ANY_ARABIC_NUM_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[0-9]").expect("ANY_ARABIC"));
