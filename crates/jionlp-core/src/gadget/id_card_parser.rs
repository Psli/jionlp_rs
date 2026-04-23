//! ID card parser — port of `jionlp/gadget/id_card_parser.py`.
//!
//! Given a mainland 18-character ID number, decode the province / city /
//! county (using the admin code in the first 6 digits), the birthdate, the
//! binary gender (determined by the 17th digit's parity), and the checksum
//! character.
//!
//! The admin-code lookup cascades: exact 6-digit → 4-digit + "00" → 2-digit
//! + "0000". This mirrors Python behavior and handles cases where the exact
//! county code is unknown but the province/city can still be determined.
//!
//! Note: this parser does NOT verify the ISO 7064 MOD 11-2 checksum. Use
//! [`crate::check_id_card`] for regex-level validity, or add a dedicated
//! checksum function if full verification is required.

use crate::rule::pattern::ID_CARD_CHECK_PATTERN;
use crate::{dict, Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdCardInfo {
    pub province: String,
    pub city: Option<String>,
    pub county: Option<String>,
    pub birth_year: String,
    pub birth_month: String,
    pub birth_day: String,
    /// "男" or "女".
    pub gender: &'static str,
    /// 0-9 or "x" (lowercased — "X" in input is normalized).
    pub check_code: String,
}

/// Parse an 18-char mainland ID number. Returns `Ok(None)` if the input
/// doesn't match the format, or `Err` if dictionaries aren't initialized.
pub fn parse_id_card(id_card: &str) -> Result<Option<IdCardInfo>> {
    if !ID_CARD_CHECK_PATTERN.is_match(id_card) {
        return Ok(None);
    }
    let chars: Vec<char> = id_card.chars().collect();
    // Regex already guarantees length == 18 and pure ASCII.
    if chars.len() != 18 {
        return Ok(None);
    }

    let cl = dict::china_location()?;

    let code6 = &id_card[..6];
    let code4 = format!("{}00", &id_card[..4]);
    let code2 = format!("{}0000", &id_card[..2]);

    let loc = cl
        .codes
        .get(code6)
        .or_else(|| cl.codes.get(&code4))
        .or_else(|| cl.codes.get(&code2));

    let (province, city, county) = match loc {
        Some(t) => t.clone(),
        None => return Ok(None),
    };

    // 17th char (index 16) encodes gender: odd → male, even → female.
    let gender_digit = chars[16]
        .to_digit(10)
        .ok_or_else(|| Error::InvalidArg("non-digit in gender position of id_card".into()))?;
    let gender: &'static str = if gender_digit % 2 == 1 { "男" } else { "女" };

    let check_code = match chars[17] {
        'X' => "x".to_string(),
        c => c.to_string(),
    };

    Ok(Some(IdCardInfo {
        province,
        city,
        county,
        birth_year: id_card[6..10].to_string(),
        birth_month: id_card[10..12].to_string(),
        birth_day: id_card[12..14].to_string(),
        gender,
        check_code,
    }))
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
    fn guangzhou_citizen() {
        ensure_init();
        // 440105 = 广州市 海珠区
        let info = parse_id_card("440105199001012345").unwrap().unwrap();
        assert_eq!(info.province, "广东省");
        assert_eq!(info.city.as_deref(), Some("广州市"));
        // County may or may not be in the dictionary; we only require it
        // is *some* name consistent with the code.
        assert!(info.county.is_some());
        assert_eq!(info.birth_year, "1990");
        assert_eq!(info.birth_month, "01");
        assert_eq!(info.birth_day, "01");
        // Idx 16 (17th char, 0-indexed) = '4' → even → 女.
        assert_eq!(info.gender, "女");
    }

    #[test]
    fn gender_male() {
        ensure_init();
        // 17th digit = 5 → odd → 男
        let info = parse_id_card("110105199001015671").unwrap().unwrap();
        assert_eq!(info.gender, "男");
    }

    #[test]
    fn gender_female() {
        ensure_init();
        // 17th digit = 2 → even → 女
        let info = parse_id_card("110105199001012021").unwrap().unwrap();
        assert_eq!(info.gender, "女");
    }

    #[test]
    fn uppercase_x_normalized() {
        ensure_init();
        let info = parse_id_card("11010519900101567X").unwrap().unwrap();
        assert_eq!(info.check_code, "x");
    }

    #[test]
    fn invalid_format_returns_none() {
        ensure_init();
        assert!(parse_id_card("not an id").unwrap().is_none());
        // Admin code range invalid (99 not in the allowed prov prefixes).
        assert!(parse_id_card("990000199001012345").unwrap().is_none());
    }

    #[test]
    fn city_level_fallback() {
        ensure_init();
        // Build a code whose county doesn't exist but city (440100) does.
        // We can't easily guarantee a specific non-existent county, so just
        // verify 440100 (广州市) itself works as a prov+city level match.
        let info = parse_id_card("440100199001012345").unwrap().unwrap();
        assert_eq!(info.province, "广东省");
        assert_eq!(info.city.as_deref(), Some("广州市"));
    }
}
