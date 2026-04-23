//! Phone-number location lookup — port of `jionlp/gadget/phone_location.py`.
//!
//! Resolves a cell or landline phone-number string into its (province, city)
//! home location and, for cell phones, the telecom operator.
//!
//! ## Lookup strategy
//!
//! * Cell phone: use the first 7 digits (3-digit carrier prefix + 4-digit
//!   range) as the key into `dict::phone_location().cell_prefix`. If that
//!   misses, fall back to the carrier prefix alone (first 3 digits) to still
//!   return an operator — but province/city stay `None`.
//! * Landline: extract the leading area-code via
//!   `LANDLINE_PHONE_AREA_CODE_PATTERN` and look it up in
//!   `dict::phone_location().area_code`.

use crate::dict;
use crate::rule::pattern::LANDLINE_PHONE_AREA_CODE_PATTERN;
use crate::Result;
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhoneInfo {
    /// The original input text.
    pub number: String,
    pub province: Option<String>,
    pub city: Option<String>,
    /// "cell_phone", "landline_phone", or "unknown".
    pub phone_type: &'static str,
    /// Only set for cell phones. e.g. "中国移动" / "中国联通" / "中国电信".
    pub operator: Option<String>,
}

/// Classify a phone string and look up its location.
pub fn phone_location(text: &str) -> Result<PhoneInfo> {
    // 1) Try cell phone: look for 1[3-9]x 11-digit sequence.
    if let Some(cap) = find_cell(text) {
        return cell_phone_location_impl(text, &cap);
    }
    // 2) Try landline: match area code.
    if let Some(code) = extract_area_code(text) {
        return landline_lookup(text, &code);
    }
    Ok(PhoneInfo {
        number: text.to_string(),
        province: None,
        city: None,
        phone_type: "unknown",
        operator: None,
    })
}

fn find_cell(text: &str) -> Option<String> {
    // CELL_PHONE_CHECK_PATTERN is anchored `^...$`; we need .find_iter-style
    // search instead. Use a lookaround-free copy of the cell body.
    static CELL_SEARCH: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"1[3-9]\d[\- ]?\d{4}[\- ]?\d{4}").unwrap());
    CELL_SEARCH
        .find(text)
        .map(|m| m.as_str().chars().filter(|c| c.is_ascii_digit()).collect())
}

fn extract_area_code(text: &str) -> Option<String> {
    // fancy_regex::Regex::captures returns Result<Option<Captures>>.
    let caps = LANDLINE_PHONE_AREA_CODE_PATTERN
        .captures(text)
        .ok()
        .flatten()?;
    caps.get(1).map(|m| m.as_str().to_string())
}

/// Inner logic for a pre-extracted cell-phone digit string. `digits` must be
/// exactly 11 ASCII digits. Exposed for callers that already validated.
pub fn cell_phone_location(text: &str, digits: &str) -> Result<PhoneInfo> {
    cell_phone_location_impl(text, digits)
}

fn cell_phone_location_impl(text: &str, digits: &str) -> Result<PhoneInfo> {
    let pl = dict::phone_location()?;
    let ops = dict::telecom_operator()?;

    let (province, city) = if digits.len() >= 7 {
        let key = &digits[..7];
        match pl.cell_prefix.get(key) {
            Some(loc) => split_loc(loc),
            None => (None, None),
        }
    } else {
        (None, None)
    };

    let op_key_len = digits.len().min(3);
    let operator = ops
        .get(&digits[..op_key_len])
        .cloned()
        .or_else(|| {
            // Fall back: try 4-digit carrier key if 3 fails (rare).
            if digits.len() >= 4 {
                ops.get(&digits[..4]).cloned()
            } else {
                None
            }
        });

    Ok(PhoneInfo {
        number: text.to_string(),
        province,
        city,
        phone_type: "cell_phone",
        operator,
    })
}

/// Look up a landline location given the raw text and a pre-extracted area
/// code. Also used internally by [`phone_location`].
pub fn landline_phone_location(text: &str) -> Result<PhoneInfo> {
    match extract_area_code(text) {
        Some(code) => landline_lookup(text, &code),
        None => Ok(PhoneInfo {
            number: text.to_string(),
            province: None,
            city: None,
            phone_type: "landline_phone",
            operator: None,
        }),
    }
}

fn landline_lookup(text: &str, area_code: &str) -> Result<PhoneInfo> {
    let pl = dict::phone_location()?;
    let (province, city) = match pl.area_code.get(area_code) {
        Some(loc) => split_loc(loc),
        None => (None, None),
    };
    Ok(PhoneInfo {
        number: text.to_string(),
        province,
        city,
        phone_type: "landline_phone",
        operator: None,
    })
}

/// Split "province city" strings. Python stores them space-separated in
/// phone_location.txt and slash-separated in landline_phone_area_code.txt
/// (e.g. "010 北京/北京"), so accept both.
fn split_loc(s: &str) -> (Option<String>, Option<String>) {
    let parts: Vec<&str> = s.split(|c: char| c == ' ' || c == '/').collect();
    match parts.len() {
        0 => (None, None),
        1 => (Some(parts[0].to_string()), None),
        _ => (
            non_empty(parts[0]),
            non_empty(parts[parts.len() - 1]),
        ),
    }
}

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
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
    fn cell_phone_basic() {
        ensure_init();
        // 138 is a 中国移动 number.
        let info = phone_location("13812345678").unwrap();
        assert_eq!(info.phone_type, "cell_phone");
        assert_eq!(info.operator.as_deref(), Some("中国移动"));
    }

    #[test]
    fn landline_beijing() {
        ensure_init();
        // Typical mainland format: "(010)12345678" or "010-12345678".
        let info = phone_location("010-12345678").unwrap();
        assert_eq!(info.phone_type, "landline_phone");
        assert!(info.province.as_deref().unwrap_or("").contains("北京"));
    }

    #[test]
    fn unknown_input() {
        ensure_init();
        let info = phone_location("not a phone").unwrap();
        assert_eq!(info.phone_type, "unknown");
        assert!(info.province.is_none());
    }

    #[test]
    fn operator_unicom() {
        ensure_init();
        let info = phone_location("13012345678").unwrap();
        assert_eq!(info.operator.as_deref(), Some("中国联通"));
    }

    #[test]
    fn split_loc_space() {
        assert_eq!(
            split_loc("山东 济南"),
            (Some("山东".to_string()), Some("济南".to_string()))
        );
    }

    #[test]
    fn split_loc_slash() {
        assert_eq!(
            split_loc("北京/北京"),
            (Some("北京".to_string()), Some("北京".to_string()))
        );
    }
}
