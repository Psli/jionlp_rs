//! Location recognizer (简化版) — lightweight port of
//! `jionlp/gadget/location_recognizer.py`.
//!
//! Given a free-text input, find all Chinese administrative-division names
//! (provinces / prefecture-level cities / counties) that appear in the
//! text, along with their level and byte offset. Uses Aho-Corasick with
//! left-most-longest match to avoid O(n·m) naive scanning.
//!
//! Limitations vs the full Python implementation:
//!   * No alias table (e.g. "北京" matches, but "京" alone does not).
//!   * No conflict resolution when a short name is a substring of a longer
//!     one — AC's longest-match handles most cases but some edge cases
//!     (e.g. "湖南省湖北" → finds 湖南省 then 湖北) depend on the admin dict.
//!   * No co-occurrence scoring to pick the "most likely" prov/city/county
//!     triple — the Python version has heuristics that rank combinations.

use crate::{dict, Result};
use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use once_cell::sync::OnceCell;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocationLevel {
    Province,
    City,
    County,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationMatch {
    pub name: String,
    pub level: LocationLevel,
    pub offset: (usize, usize),
}

struct RecognizerIndex {
    ac: AhoCorasick,
    /// Parallel to ac's pattern indices.
    entries: Vec<(String, LocationLevel)>,
}

static INDEX: OnceCell<RecognizerIndex> = OnceCell::new();

fn index() -> Result<&'static RecognizerIndex> {
    INDEX.get_or_try_init(build_index)
}

/// Mainland province/autonomous-region/municipality single-char abbreviations.
///
/// These are the standard 简称 used on license plates and in colloquial
/// speech (京 for 北京, 沪 for 上海, 粤 for 广东 …). Each short maps to the
/// canonical full-name string used in the `china_location` dictionary.
///
/// Source: GB/T 2260 + `jionlp/rule/rule_pattern.py::CHINA_PROVINCE_ALIAS`.
const PROVINCE_ALIASES: &[(&str, &str)] = &[
    ("京", "北京市"),
    ("津", "天津市"),
    ("沪", "上海市"),
    ("渝", "重庆市"),
    ("黑", "黑龙江省"),
    ("吉", "吉林省"),
    ("辽", "辽宁省"),
    ("新", "新疆维吾尔自治区"),
    ("藏", "西藏自治区"),
    ("青", "青海省"),
    ("蒙", "内蒙古自治区"),
    ("晋", "山西省"),
    ("冀", "河北省"),
    ("豫", "河南省"),
    ("甘", "甘肃省"),
    ("陇", "甘肃省"),
    ("陕", "陕西省"),
    ("秦", "陕西省"),
    ("川", "四川省"),
    ("蜀", "四川省"),
    ("贵", "贵州省"),
    ("黔", "贵州省"),
    ("云", "云南省"),
    ("滇", "云南省"),
    ("宁", "宁夏回族自治区"),
    ("苏", "江苏省"),
    ("浙", "浙江省"),
    ("皖", "安徽省"),
    ("鲁", "山东省"),
    ("赣", "江西省"),
    ("鄂", "湖北省"),
    ("湘", "湖南省"),
    ("粤", "广东省"),
    ("闽", "福建省"),
    ("桂", "广西壮族自治区"),
    ("琼", "海南省"),
    ("港", "香港特别行政区"),
    ("澳", "澳门特别行政区"),
    ("台", "台湾省"),
];

fn build_index() -> Result<RecognizerIndex> {
    let cl = dict::china_location()?;
    // Invert the admin-code map: walk every code and remember level.
    //
    // The code's 3rd-4th digits are the city slot and 5th-6th are county.
    // We rely on the shape of the code to decide the level:
    //   - "XX0000" → province (if city_slot == 00 and county_slot == 00)
    //   - "XXYY00" → city     (county_slot == 00, city_slot != 00)
    //   - "XXYYZZ" → county   (otherwise)
    // Note: 直辖市 (like 110000 北京市) register as province per this logic.

    let mut patterns: Vec<String> = Vec::with_capacity(cl.codes.len() + PROVINCE_ALIASES.len());
    let mut entries: Vec<(String, LocationLevel)> =
        Vec::with_capacity(cl.codes.len() + PROVINCE_ALIASES.len());

    // Use a dedupe set since the same name string (e.g. "东区") may appear
    // as a county under multiple cities. We'd still like one AC pattern.
    let mut seen: rustc_hash::FxHashSet<(String, u8)> = rustc_hash::FxHashSet::default();

    for (code, (prov, city, county)) in cl.codes.iter() {
        let level = classify_level(code);
        let name = match level {
            LocationLevel::Province => prov.clone(),
            LocationLevel::City => match city {
                Some(c) => c.clone(),
                None => prov.clone(),
            },
            LocationLevel::County => match county {
                Some(c) => c.clone(),
                None => continue,
            },
        };
        if name.is_empty() {
            continue;
        }
        let level_key = match level {
            LocationLevel::Province => 0u8,
            LocationLevel::City => 1,
            LocationLevel::County => 2,
        };
        if seen.insert((name.clone(), level_key)) {
            patterns.push(name.clone());
            entries.push((name, level));
        }
    }

    // Add single-char province aliases. They always map to a canonical
    // province entry, preserving the full name in the match result so
    // downstream code can treat "京" and "北京市" identically.
    for (alias, canonical) in PROVINCE_ALIASES {
        if seen.insert((alias.to_string(), 0)) {
            patterns.push(alias.to_string());
            entries.push((canonical.to_string(), LocationLevel::Province));
        }
    }

    let ac = AhoCorasickBuilder::new()
        .match_kind(MatchKind::LeftmostLongest)
        .ascii_case_insensitive(false)
        .build(&patterns)
        .map_err(|e| crate::Error::InvalidArg(format!("AC build: {e}")))?;

    Ok(RecognizerIndex { ac, entries })
}

fn classify_level(code: &str) -> LocationLevel {
    classify_level_pub(code)
}

/// Public helper for `location_parser` to reuse the same level-coding rules.
pub fn classify_level_pub(code: &str) -> LocationLevel {
    // Codes are always 6 digits.
    if code.len() < 6 {
        return LocationLevel::Province;
    }
    let city = &code[2..4];
    let county = &code[4..6];
    if county != "00" {
        LocationLevel::County
    } else if city != "00" {
        LocationLevel::City
    } else {
        LocationLevel::Province
    }
}

/// Find all administrative-division names in `text`. Returns matches in
/// left-most-longest order.
pub fn recognize_location(text: &str) -> Result<Vec<LocationMatch>> {
    let idx = index()?;
    let mut out = Vec::new();
    for m in idx.ac.find_iter(text) {
        let (name, level) = &idx.entries[m.pattern().as_usize()];
        out.push(LocationMatch {
            name: name.clone(),
            level: level.clone(),
            offset: (m.start(), m.end()),
        });
    }
    Ok(out)
}

/// Convenience: return only province / city / county names separately.
pub fn parse_location(text: &str) -> Result<ParsedLocation> {
    let matches = recognize_location(text)?;
    let mut parsed = ParsedLocation {
        province: None,
        city: None,
        county: None,
    };
    for m in matches {
        match m.level {
            LocationLevel::Province if parsed.province.is_none() => parsed.province = Some(m.name),
            LocationLevel::City if parsed.city.is_none() => parsed.city = Some(m.name),
            LocationLevel::County if parsed.county.is_none() => parsed.county = Some(m.name),
            _ => {}
        }
    }
    Ok(parsed)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLocation {
    pub province: Option<String>,
    pub city: Option<String>,
    pub county: Option<String>,
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
    fn finds_province_and_city() {
        ensure_init();
        let r = recognize_location("我出生在广东省广州市海珠区。").unwrap();
        let names: Vec<&str> = r.iter().map(|m| m.name.as_str()).collect();
        assert!(names.iter().any(|n| n == &"广东省"));
        assert!(names.iter().any(|n| n == &"广州市"));
    }

    #[test]
    fn parse_location_extracts_triple() {
        ensure_init();
        let p = parse_location("家住北京市朝阳区建国门").unwrap();
        // "北京市" matches both province-level (110000) and city-level (110100).
        // LeftmostLongest picks one; the result may be either level depending
        // on dictionary iteration order. Just assert it was picked up.
        assert!(p.province.is_some() || p.city.is_some());
    }

    #[test]
    fn no_match_returns_empty() {
        ensure_init();
        let r = recognize_location("hello world").unwrap();
        assert!(r.is_empty());
    }

    #[test]
    fn classify_level_codes() {
        assert_eq!(classify_level("440000"), LocationLevel::Province);
        assert_eq!(classify_level("440100"), LocationLevel::City);
        assert_eq!(classify_level("440105"), LocationLevel::County);
    }

    #[test]
    fn offsets_are_byte_positions() {
        ensure_init();
        let text = "hello 广东省 world";
        let r = recognize_location(text).unwrap();
        let m = r.iter().find(|m| m.name == "广东省").unwrap();
        assert_eq!(&text[m.offset.0..m.offset.1], "广东省");
    }

    #[test]
    fn alias_recognized_as_province() {
        ensure_init();
        // "京" alone should resolve to 北京市. Use a minimal phrase to avoid
        // the AC picking up another unrelated match first.
        let r = recognize_location("北京出生后去了京").unwrap();
        // Either the single-char "京" or the full "北京市" must be present.
        let names: Vec<&str> = r.iter().map(|m| m.name.as_str()).collect();
        assert!(
            names.iter().any(|n| n == &"北京市"),
            "expected 北京市 in {:?}",
            names
        );
    }

    #[test]
    fn alias_standalone() {
        ensure_init();
        let r = recognize_location("粤，沪，川").unwrap();
        let names: Vec<&str> = r.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"广东省"));
        assert!(names.contains(&"上海市"));
        assert!(names.contains(&"四川省"));
    }
}
