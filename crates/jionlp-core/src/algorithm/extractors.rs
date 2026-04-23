//! Free-text entity extractors — port of
//! `jionlp/algorithm/ner/time_extractor.py` and
//! `jionlp/algorithm/ner/money_extractor.py`.
//!
//! Both extractors follow the same two-phase algorithm:
//!   1. A broad-brush regex (TIME_CHAR_STRING / MONEY_CHAR_STRING) finds
//!      "candidate regions" of the input — runs of characters that *look
//!      like* a time/money expression.
//!   2. Inside each candidate, grid-search substrings from longest-first;
//!      for each substring, call the corresponding parser and, if it
//!      succeeds, emit an entity with character-level byte offset.
//!
//! This mirrors Python's approach. The Rust port omits the optional
//! jiojio-segmenter boundary check (Python `use_jiojio`) since jiojio has
//! no Rust counterpart yet — it's a boundary-sanity heuristic and can be
//! added later without shape changes.

use crate::gadget::money_parser::{parse_money, MoneyInfo};
use crate::gadget::time_parser::{parse_time_with_ref, TimeInfo};
use chrono::NaiveDateTime;
use once_cell::sync::Lazy;
use regex::Regex;

/// A span of extracted text: `text`, byte-offsets and optional parse.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeEntity {
    pub text: String,
    pub offset: (usize, usize),
    pub time_type: &'static str,
    pub detail: Option<TimeInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoneyEntity {
    pub text: String,
    pub offset: (usize, usize),
    pub detail: Option<MoneyInfo>,
}

// ───────────────────────── TIME ──────────────────────────────────────────

/// Broad-brush regex — runs of "time-ish" characters. Copied (with minor
/// escaping adjustments) from Python TIME_CHAR_STRING.
static TIME_CHAR_STRING: Lazy<Regex> = Lazy::new(|| {
    // Rust `regex` crate disallows some escape sequences that Python
    // tolerates (e.g. `\~` is `syntax error`). Replace `\~`/`\—` with the
    // bare char, and `\-` with `-` at non-class-special positions.
    Regex::new(concat!(
        r"(现在|开始|黎明|过去|未来|愚人|感恩|圣诞|情人|儿童|劳动|父亲|母亲|礼拜|霜降|立春|立冬|小寒|大寒|",
        r"立夏|立秋|冬至|",
        r"[102年月日3589647时午至天上个分今下:\-点晚前一小后周起内以底三晨钟来半两凌当十份季Qq去早多第五中初廿.度二从六期旬到间四节号：",
        r"代~—～春明昨星末秋之同·世纪本七九秒每次八夏/夜零正冬腊余工作元国清傍交易首 ()（）、万宵全暑头端庆旦－际消费者权益大里农阴历双财",
        r"近运深, ”夕〇几汛假壹无数白百刻许左右的这本])+",
    ))
    .unwrap()
});

/// Start/end chars that can't bound a time expression (Python
/// FAKE_POSITIVE_START_STRING / END_STRING).
static FAKE_START_CHARS: &[char] = &['起', '到', '至', '以', '开', '－', '—', '-', '~', '～'];
static FAKE_END_CHARS: &[char] = &['到', '至', '－', '—', '-', '~', '～', ','];

/// Strings that look time-ish but rarely mean time in practice. Python's
/// `non_time_string_list` + common additions.
const NON_TIME_STRINGS: &[&str] = &["一点", "0时", "一日", "黎明", "十分", "百分", "万分"];

/// Character sets used by the boundary checks.
#[allow(dead_code)]
static NUM_PAT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[０-９0-9一二三四五六七八九十百千万]").unwrap());
static FOUR_NUM_YEAR_PAT: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d{4}$").unwrap());
static UNIT_PAT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(多)?[万亿元]").unwrap());

const SINGLE_CHAR_TIMES: &[&str] = &["春", "夏", "秋", "冬"];

/// Extract time entities from `text`, each parsed against `now`. If
/// `with_parsing` is `false`, the returned entities omit the `detail`
/// field (saves a bit of work).
pub fn extract_time(
    text: &str,
    now: NaiveDateTime,
    with_parsing: bool,
    ret_all: bool,
) -> Vec<TimeEntity> {
    let candidates = extract_time_candidates(text);
    let mut out = Vec::new();

    for cand in candidates {
        let mut bias = 0usize;
        let candidate_text = &text[cand.0..cand.1];
        let cand_bytes = candidate_text.as_bytes();
        while bias < cand_bytes.len() {
            let sub = &candidate_text[bias..];
            match grid_search_time(sub, now) {
                Some((matched_text, result, offset_in_sub)) => {
                    let abs_start = cand.0 + bias + offset_in_sub.0;
                    let abs_end = cand.0 + bias + offset_in_sub.1;

                    // Rule 1: filter low-probability time phrases.
                    if !ret_all && NON_TIME_STRINGS.contains(&matched_text.as_str()) {
                        bias += offset_in_sub.1;
                        continue;
                    }
                    // Rule 2: four-digit year followed by 万/亿/元 → not a year.
                    if FOUR_NUM_YEAR_PAT.is_match(&matched_text) {
                        let after = &text[abs_end..];
                        if let Some(m) = UNIT_PAT.find(after) {
                            if m.start() == 0 || m.start() < 6 {
                                bias += offset_in_sub.1;
                                continue;
                            }
                        }
                    }
                    let time_type = result.time_type;
                    out.push(TimeEntity {
                        text: matched_text,
                        offset: (abs_start, abs_end),
                        time_type,
                        detail: if with_parsing { Some(result) } else { None },
                    });
                    bias += offset_in_sub.1;
                }
                None => break,
            }
        }
    }
    out
}

/// Scan `text` for regions that look time-ish; each returned `(start,
/// end)` is a byte range of the original text.
fn extract_time_candidates(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for m in TIME_CHAR_STRING.find_iter(text) {
        let s = m.as_str();
        if s.chars().count() >= 2 || SINGLE_CHAR_TIMES.contains(&s) {
            out.push((m.start(), m.end()));
        }
    }
    out
}

/// Grid search: from longest window down to shortest, and from leftmost
/// to rightmost offset. First successful parse wins. Returns
/// `(matched_text, parse_result, offset_in_candidate)`.
fn grid_search_time(
    candidate: &str,
    now: NaiveDateTime,
) -> Option<(String, TimeInfo, (usize, usize))> {
    let chars: Vec<(usize, char)> = candidate.char_indices().collect();
    let total = chars.len();
    let length = total.min(35);

    for i in 0..length {
        for j in 0..=i {
            // Window char indices [j, length - i + j + 1).
            let end_char = length - i + j + 1;
            if end_char > total {
                continue;
            }
            let start_byte = chars[j].0;
            let end_byte = if end_char == total {
                candidate.len()
            } else {
                chars[end_char].0
            };
            let sub = &candidate[start_byte..end_byte];

            if !time_filter(sub) {
                continue;
            }
            // Strip auxiliary chars that confuse the parser.
            let cleaned = sub.replace(['的', ' '], "");
            if cleaned.is_empty() {
                continue;
            }
            if let Some(result) = parse_time_with_ref(&cleaned, now) {
                return Some((sub.to_string(), result, (start_byte, end_byte)));
            }
        }
    }
    None
}

/// Character-level boundary filter — reject substrings starting or ending
/// with punctuation that would make the entity incoherent.
fn time_filter(sub: &str) -> bool {
    if sub.is_empty() {
        return false;
    }
    let chars: Vec<char> = sub.chars().collect();
    let first = chars[0];
    let last = chars[chars.len() - 1];

    if FAKE_START_CHARS.contains(&first) {
        return false;
    }
    if FAKE_END_CHARS.contains(&last) {
        if chars.len() < 2 {
            return false;
        }
        let last2: String = chars[chars.len() - 2..].iter().collect();
        if last2 != "夏至" && last2 != "冬至" {
            return false;
        }
    }
    // `的` must not be at start or end.
    if first == '的' || last == '的' {
        return false;
    }
    // Bracket parity.
    if matches!(first, ')' | '）') || matches!(last, '(' | '（') {
        return false;
    }
    if sub.len() != sub.trim().len() {
        return false;
    }
    true
}

// ───────────────────────── MONEY ─────────────────────────────────────────

/// Broad-brush regex for money candidate regions. Simplified from Python
/// `MONEY_CHAR_STRING` — accepts a run of digits / punctuation / CN digits
/// / currency/unit chars.
static MONEY_CHAR_STRING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(concat!(
        r"([0-9０-９\.,，~～\-—到至",
        r"零一二三四五六七八九十百千万亿兆",
        r"壹贰叁肆伍陆柒捌玖拾佰仟萬",
        r"个多几十数元角分块毛钱币港日台美欧英韩泰铢卢布澳加新镑RMB",
        r"USD EUR JPY GBP",
        r"k K w W圆整大约近至少超过不到不足逾(含)（含）以上以下左右上下",
        r"]+)"
    ))
    .unwrap()
});

const NON_MONEY_STRINGS: &[&str] = &["多元", "十分", "百分", "万分"];

/// Characters that tend to break money boundary cleanness.
fn money_filter(sub: &str) -> bool {
    if sub.is_empty() {
        return false;
    }
    let chars: Vec<char> = sub.chars().collect();
    let first = chars[0];
    let last = chars[chars.len() - 1];
    // Leading punctuation / separator is never a valid money start.
    if matches!(
        first,
        ',' | '，' | '-' | '—' | '~' | '～' | '.' | '(' | '（' | ')' | '）'
    ) {
        return false;
    }
    // Trailing separator is never valid either.
    if matches!(last, ',' | '，' | '-' | '—' | '~' | '～' | '(' | '（') {
        return false;
    }
    true
}

/// Extract money entities from free text, each parsed via `parse_money`.
pub fn extract_money(text: &str, with_parsing: bool, ret_all: bool) -> Vec<MoneyEntity> {
    let mut out = Vec::new();
    for m in MONEY_CHAR_STRING.find_iter(text) {
        let candidate = m.as_str();
        let cand_start = m.start();
        let mut bias = 0usize;
        let cand_len = candidate.len();
        while bias < cand_len {
            let sub = &candidate[bias..];
            match grid_search_money(sub) {
                Some((matched_text, result, offset_in_sub)) => {
                    if !ret_all && NON_MONEY_STRINGS.contains(&matched_text.as_str()) {
                        bias += offset_in_sub.1;
                        continue;
                    }
                    let abs_start = cand_start + bias + offset_in_sub.0;
                    let abs_end = cand_start + bias + offset_in_sub.1;
                    out.push(MoneyEntity {
                        text: matched_text,
                        offset: (abs_start, abs_end),
                        detail: if with_parsing { Some(result) } else { None },
                    });
                    bias += offset_in_sub.1;
                }
                None => break,
            }
        }
    }
    out
}

fn grid_search_money(candidate: &str) -> Option<(String, MoneyInfo, (usize, usize))> {
    let chars: Vec<(usize, char)> = candidate.char_indices().collect();
    let total = chars.len();
    let length = total.min(40);

    for i in 0..length {
        for j in 0..=i {
            let end_char = length - i + j + 1;
            if end_char > total {
                continue;
            }
            let start_byte = chars[j].0;
            let end_byte = if end_char == total {
                candidate.len()
            } else {
                chars[end_char].0
            };
            let sub = &candidate[start_byte..end_byte];
            if !money_filter(sub) {
                continue;
            }
            if let Some(result) = parse_money(sub) {
                return Some((sub.to_string(), result, (start_byte, end_byte)));
            }
        }
    }
    None
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

    fn now() -> NaiveDateTime {
        NaiveDateTime::parse_from_str("2021-09-01T15:15:32", "%Y-%m-%dT%H:%M:%S").unwrap()
    }

    #[test]
    fn extract_time_multi_entities() {
        ensure_init();
        let text = "2021年中秋节是9月21日，星期二。";
        let r = extract_time(text, now(), false, false);
        let names: Vec<&str> = r.iter().map(|e| e.text.as_str()).collect();
        // At least one entity in the expected vicinity.
        assert!(!r.is_empty());
        assert!(
            names
                .iter()
                .any(|n| n.contains("2021") || n.contains("9月") || n.contains("星期")),
            "got {:?}",
            names
        );
    }

    #[test]
    fn extract_time_handles_empty() {
        ensure_init();
        let r = extract_time("hello world no dates here", now(), false, false);
        assert!(r.is_empty());
    }

    #[test]
    fn extract_money_basic() {
        let text = "海航亏损7000万港元出售香港公寓。以2.6亿港元的价格出售";
        let r = extract_money(text, false, false);
        let texts: Vec<&str> = r.iter().map(|e| e.text.as_str()).collect();
        assert!(texts
            .iter()
            .any(|t| t.contains("7000万") || t.contains("港元")));
        assert!(texts
            .iter()
            .any(|t| t.contains("2.6亿") || t.contains("港元")));
    }

    #[test]
    fn extract_money_excludes_non_money() {
        // "百分" alone shouldn't be flagged as a money entity.
        let r = extract_money("他有百分之一的可能性", false, false);
        assert!(r.iter().all(|e| e.text != "百分"));
    }
}
