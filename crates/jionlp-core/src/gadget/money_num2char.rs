//! Number ↔ Chinese numeral — port of `jionlp/gadget/money_num2char.py`
//! (num2char direction) plus a complementary `char2num` for parsing Chinese
//! numeric expressions back to `f64`.
//!
//! `num2char` is used for generating formal invoice-style amounts:
//!   120402810.03  →  壹亿贰仟零肆拾萬贰仟捌佰壹拾點零叁  (tra)
//!   38009         →  三万八千零九                        (sim)
//!
//! `char2num` is the inverse direction useful as a building block for a
//! future `parse_money` implementation:
//!   "三千五百万"      →  35_000_000
//!   "一点五亿"       →  150_000_000
//!   "二十三"          →  23
//!
//! Only the "bank-style" (non-colloquial) system is covered — "一万" works,
//! "万八" and other colloquialisms do not.

use once_cell::sync::Lazy;
use regex::Regex;
use rustc_hash::FxHashMap;

// ───────────────────────── num2char direction ───────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumStyle {
    /// 一二三四五六七八九零, 千百十
    Simplified,
    /// 壹贰叁肆伍陆柒捌玖零, 仟佰拾
    Traditional,
}

fn digit_char(d: char, style: NumStyle) -> char {
    match (d, style) {
        ('0', _) => '零',
        ('1', NumStyle::Simplified) => '一',
        ('2', NumStyle::Simplified) => '二',
        ('3', NumStyle::Simplified) => '三',
        ('4', NumStyle::Simplified) => '四',
        ('5', NumStyle::Simplified) => '五',
        ('6', NumStyle::Simplified) => '六',
        ('7', NumStyle::Simplified) => '七',
        ('8', NumStyle::Simplified) => '八',
        ('9', NumStyle::Simplified) => '九',
        ('1', NumStyle::Traditional) => '壹',
        ('2', NumStyle::Traditional) => '贰',
        ('3', NumStyle::Traditional) => '叁',
        ('4', NumStyle::Traditional) => '肆',
        ('5', NumStyle::Traditional) => '伍',
        ('6', NumStyle::Traditional) => '陆',
        ('7', NumStyle::Traditional) => '柒',
        ('8', NumStyle::Traditional) => '捌',
        ('9', NumStyle::Traditional) => '玖',
        _ => '零',
    }
}

/// 4-char-block inner suffix: index 0 is units, 1 is 十, 2 is 百, 3 is 千.
fn inner_suffix(idx_from_ones: usize, style: NumStyle) -> &'static str {
    match (idx_from_ones, style) {
        (0, _) => "",
        (1, NumStyle::Simplified) => "十",
        (2, NumStyle::Simplified) => "百",
        (3, NumStyle::Simplified) => "千",
        (1, NumStyle::Traditional) => "拾",
        (2, NumStyle::Traditional) => "佰",
        (3, NumStyle::Traditional) => "仟",
        _ => "",
    }
}

/// Block suffix: 0=nothing, 1=万, 2=亿, 3=兆.
fn outer_suffix(idx: usize, style: NumStyle) -> &'static str {
    match (idx, style) {
        (0, _) => "",
        (1, NumStyle::Simplified) => "万",
        (2, _) => "亿",
        (3, _) => "兆",
        (1, NumStyle::Traditional) => "萬",
        _ => "",
    }
}

fn parse_block(block: &str, style: NumStyle) -> String {
    // block is up to 4 chars of digits, representing qian-bai-shi-ge.
    let len = block.len();
    let mut out = String::new();
    for (i, d) in block.chars().enumerate() {
        let pos = len - 1 - i; // position from ones
        if d == '0' {
            out.push('零');
        } else {
            out.push(digit_char(d, style));
            out.push_str(inner_suffix(pos, style));
        }
    }
    // Collapse consecutive 零, strip trailing 零.
    static ZEROS_MID: Lazy<Regex> = Lazy::new(|| Regex::new(r"零+").unwrap());
    let collapsed = ZEROS_MID.replace_all(&out, "零");
    collapsed.trim_end_matches('零').to_string()
}

fn split_integer_into_4char_blocks(s: &str) -> Vec<&str> {
    // Produce blocks from LEAST significant → MOST significant, each <=4 chars.
    let bytes = s.as_bytes();
    let mut blocks = Vec::new();
    let mut end = bytes.len();
    while end > 0 {
        let start = end.saturating_sub(4);
        blocks.push(&s[start..end]);
        end = start;
    }
    blocks
}

/// Convert an unsigned integer string to a Chinese numeral.
/// Internal helper — prefer [`num2char`].
fn integer_to_chinese(integer: &str, style: NumStyle) -> String {
    if integer.is_empty() {
        return String::new();
    }
    let trimmed = integer.trim_start_matches('0');
    if trimmed.is_empty() {
        return digit_char('0', style).to_string();
    }

    let blocks = split_integer_into_4char_blocks(trimmed);
    // Walk from MOST-significant block to LEAST, appending block + outer suffix.
    let mut out = String::new();
    for (i, block) in blocks.iter().enumerate().rev() {
        let chunk = parse_block(block, style);
        if chunk.is_empty() {
            // All-zero block: emit a zero to preserve sep, but only if not leading.
            if !out.is_empty() && !out.ends_with('零') {
                out.push('零');
            }
        } else {
            out.push_str(&chunk);
            out.push_str(outer_suffix(i, style));
        }
    }

    // Collapse runs of 零, strip trailing.
    static ZEROS_MID: Lazy<Regex> = Lazy::new(|| Regex::new(r"零+").unwrap());
    let collapsed = ZEROS_MID.replace_all(&out, "零");
    let mut s = collapsed.trim_end_matches('零').to_string();

    // "一十X" is colloquially written as "十X" in simplified; Python does the
    // same replacement at the very start of the output.
    if style == NumStyle::Simplified {
        if let Some(stripped) = s.strip_prefix("一十") {
            s = format!("十{}", stripped);
        }
    }
    s
}

fn decimal_to_chinese(frac: &str, style: NumStyle) -> String {
    let mut out = String::new();
    for d in frac.chars() {
        out.push(digit_char(d, style));
    }
    out
}

/// Convert a numeric string (or `int`/`float` formatted with `{}`) to its
/// Chinese numeral form. Commas are stripped. Non-numeric input returns Err.
pub fn num2char(num: &str, style: NumStyle) -> Result<String, String> {
    let cleaned = num.replace(',', "");
    let (integer_part, frac_part) = match cleaned.split_once('.') {
        Some((a, b)) => (a, Some(b)),
        None => (cleaned.as_str(), None),
    };
    if integer_part.is_empty() || !integer_part.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("invalid integer part in '{}'", num));
    }
    if let Some(f) = frac_part {
        if !f.chars().all(|c| c.is_ascii_digit()) {
            return Err(format!("invalid fractional part in '{}'", num));
        }
    }

    let int_ch = integer_to_chinese(integer_part, style);
    match frac_part {
        None => Ok(int_ch),
        Some(f) if f.chars().all(|c| c == '0') => Ok(int_ch),
        Some(f) => {
            let dot = match style {
                NumStyle::Simplified => "点",
                NumStyle::Traditional => "點",
            };
            Ok(format!("{}{}{}", int_ch, dot, decimal_to_chinese(f, style)))
        }
    }
}

// ───────────────────────── char2num direction ───────────────────────────────

static CHAR_TO_DIGIT: Lazy<FxHashMap<char, u64>> = Lazy::new(|| {
    let mut m = FxHashMap::default();
    for (ch, v) in [
        ('零', 0u64),
        ('〇', 0),
        ('一', 1),
        ('壹', 1),
        ('二', 2),
        ('贰', 2),
        ('两', 2),
        ('俩', 2),
        ('三', 3),
        ('叁', 3),
        ('仨', 3),
        ('四', 4),
        ('肆', 4),
        ('五', 5),
        ('伍', 5),
        ('六', 6),
        ('陆', 6),
        ('七', 7),
        ('柒', 7),
        ('八', 8),
        ('捌', 8),
        ('九', 9),
        ('玖', 9),
    ] {
        m.insert(ch, v);
    }
    m
});

/// Scaling unit characters: 十/百/千 are in-block; 万/亿/兆 are block suffixes.
fn unit_value(c: char) -> Option<u64> {
    Some(match c {
        '十' | '拾' => 10,
        '百' | '佰' => 100,
        '千' | '仟' => 1_000,
        '万' | '萬' => 10_000,
        '亿' => 100_000_000,
        '兆' => 1_000_000_000_000,
        _ => return None,
    })
}

/// Parse a Chinese numeric expression into `f64`.
///
/// Supports integers with 十百千万亿兆 as well as a simple decimal form using
/// `点` / `點` (the part after the dot is parsed digit-by-digit).
///
/// Returns `Err` if the input contains characters outside the known numeric
/// set.
pub fn char2num(s: &str) -> Result<f64, String> {
    let (int_part, dec_part) = split_at_dot(s);

    let int_val = parse_chinese_integer(int_part)?;

    match dec_part {
        None => Ok(int_val as f64),
        Some(d) => {
            // Parse digit-by-digit.
            let mut frac_s = String::new();
            for ch in d.chars() {
                let digit = CHAR_TO_DIGIT
                    .get(&ch)
                    .ok_or_else(|| format!("bad decimal digit: {}", ch))?;
                frac_s.push(char::from_digit(*digit as u32, 10).unwrap());
            }
            let frac: f64 = format!("0.{}", frac_s)
                .parse()
                .map_err(|e: std::num::ParseFloatError| e.to_string())?;
            Ok(int_val as f64 + frac)
        }
    }
}

fn split_at_dot(s: &str) -> (&str, Option<&str>) {
    for (i, c) in s.char_indices() {
        if c == '点' || c == '點' {
            return (&s[..i], Some(&s[i + c.len_utf8()..]));
        }
    }
    (s, None)
}

/// Parse a "small-unit" block (`一千二百三十` style; ≤ 9999 value).
///
/// This also accepts a bare digit character ('一', '五') as 1/5 respectively,
/// and accepts an empty block as 0 (useful when higher units are given
/// without a smaller count, e.g. "万" standalone won't appear but "一万"
/// leaves a zero block after consuming 万).
fn parse_block_chinese(s: &str) -> Result<u64, String> {
    if s.is_empty() {
        return Ok(0);
    }
    let mut total: u64 = 0;
    let mut last_digit: Option<u64> = None;

    for ch in s.chars() {
        if let Some(&d) = CHAR_TO_DIGIT.get(&ch) {
            last_digit = Some(d);
        } else if let Some(unit) = unit_value(ch) {
            if unit >= 10_000 {
                return Err(format!("bigger unit {} inside small block", ch));
            }
            let count = last_digit.take().unwrap_or(1);
            total += count * unit;
        } else {
            return Err(format!("bad char in Chinese numeral: {}", ch));
        }
    }

    if let Some(d) = last_digit {
        total += d;
    }
    Ok(total)
}

fn parse_chinese_integer(s: &str) -> Result<u64, String> {
    if s.is_empty() {
        return Ok(0);
    }
    // Split on big-unit characters, right-to-left, building up the value.
    // Approach: find the largest big-unit present; parse left part recursively
    // as multiplier; parse right part recursively as remainder.
    for big in ['兆', '亿', '万', '萬'] {
        if let Some(pos) = find_char(s, big) {
            let left = &s[..pos];
            let right = &s[pos + big.len_utf8()..];
            let mult = if left.is_empty() {
                1
            } else {
                parse_chinese_integer(left)?
            };
            let unit = unit_value(big).unwrap();
            let rem = parse_chinese_integer(right)?;
            return Ok(mult * unit + rem);
        }
    }
    // No big unit — must be a small block.
    parse_block_chinese(s)
}

fn find_char(s: &str, ch: char) -> Option<usize> {
    s.char_indices()
        .find_map(|(i, c)| if c == ch { Some(i) } else { None })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── num2char ────────────────────────────────────────────────────────

    #[test]
    fn num2char_simple_sim() {
        assert_eq!(
            num2char("38009", NumStyle::Simplified).unwrap(),
            "三万八千零九"
        );
    }

    #[test]
    fn num2char_simple_with_decimal() {
        assert_eq!(
            num2char("1234.56", NumStyle::Simplified).unwrap(),
            "一千二百三十四点五六"
        );
    }

    #[test]
    fn num2char_sim_collapses_yishi() {
        // 10 -> 十, 15 -> 十五 (not 一十 / 一十五)
        assert_eq!(num2char("10", NumStyle::Simplified).unwrap(), "十");
        assert_eq!(num2char("15", NumStyle::Simplified).unwrap(), "十五");
    }

    #[test]
    fn num2char_tra_invoice() {
        // Classic invoice-style: 壹贰叁肆… and 仟/佰/拾
        let r = num2char("1234", NumStyle::Traditional).unwrap();
        assert_eq!(r, "壹仟贰佰叁拾肆");
    }

    #[test]
    fn num2char_rejects_non_digit() {
        assert!(num2char("abc", NumStyle::Simplified).is_err());
    }

    #[test]
    fn num2char_strips_commas() {
        assert_eq!(
            num2char("38,009", NumStyle::Simplified).unwrap(),
            "三万八千零九"
        );
    }

    // ── char2num ────────────────────────────────────────────────────────

    #[test]
    fn char2num_small_numbers() {
        assert_eq!(char2num("二十三").unwrap(), 23.0);
        assert_eq!(char2num("一百零五").unwrap(), 105.0);
        assert_eq!(char2num("一千二百三十四").unwrap(), 1234.0);
    }

    #[test]
    fn char2num_wan() {
        assert_eq!(char2num("三千五百万").unwrap(), 35_000_000.0);
        assert_eq!(char2num("一万").unwrap(), 10_000.0);
    }

    #[test]
    fn char2num_yi() {
        assert_eq!(char2num("一亿").unwrap(), 100_000_000.0);
        assert_eq!(char2num("三亿四千万").unwrap(), 340_000_000.0);
        // NOTE: "一点五亿" (1.5 * 亿) is not supported — the fractional form
        // followed by a big unit needs composite parsing that parse_money
        // will provide.
    }

    #[test]
    fn char2num_decimal_point() {
        let r = char2num("三点一四").unwrap();
        assert!((r - 3.14).abs() < 1e-9);
    }

    #[test]
    fn char2num_traditional_invoice() {
        assert_eq!(char2num("壹仟贰佰叁拾肆").unwrap(), 1234.0);
    }
}
