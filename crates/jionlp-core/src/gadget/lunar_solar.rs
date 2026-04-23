//! Chinese lunar ↔ Gregorian converter — port of
//! `jionlp/gadget/lunar_solar_date.py`.
//!
//! Covers 1900-01-01 through 2100-12-29 (201 years). Encoding reproduces
//! the Python `CHINESE_YEAR_CODE` + `CHINESE_NEW_YEAR` tables byte-for-
//! byte, so `lunar_to_solar` outputs match Python exactly.
//!
//! ## Year code layout
//!
//! 20 bits per year:
//! * bits 0..4   = leap month index (0 = no leap this year)
//! * bits 4..16  = 12 flags, LSB = 12th month, ... MSB = 1st month;
//!                 `1` = 30-day month, `0` = 29-day month
//! * bits 16..20 = leap-month day count (0 → 29d, 1 → 30d).
//!                 Meaningful only when bits 0..4 ≠ 0.

use chrono::NaiveDate;

/// Maps year (1900..=2100) to a 20-bit encoded `u32`. See file-level doc.
#[rustfmt::skip]
const YEAR_CODES: [u32; 201] = [
    19416, 19168, 42352, 21717, 53856, 55632, 91476, 22176,
    39632, 21970, 19168, 42422, 42192, 53840, 119381, 46400,
    54944, 44450, 38320, 84343, 18800, 42160, 46261, 27216,
    27968, 109396, 11104, 38256, 21234, 18800, 25958, 54432,
    59984, 92821, 23248, 11104, 100067, 37600, 116951, 51536,
    54432, 120998, 46416, 22176, 107956, 9680, 37584, 53938,
    43344, 46423, 27808, 46416, 86869, 19872, 42416, 83315,
    21168, 43432, 59728, 27296, 44710, 43856, 19296, 43748,
    42352, 21088, 62051, 55632, 23383, 22176, 38608, 19925,
    19152, 42192, 54484, 53840, 54616, 46400, 46752, 103846,
    38320, 18864, 43380, 42160, 45690, 27216, 27968, 44870,
    43872, 38256, 19189, 18800, 25776, 29859, 59984, 27480,
    23232, 43872, 38613, 37600, 51552, 55636, 54432, 55888,
    30034, 22176, 43959, 9680, 37584, 51893, 43344, 46240,
    47780, 44368, 21977, 19360, 42416, 86390, 21168, 43312,
    31060, 27296, 44368, 23378, 19296, 42726, 42208, 53856,
    60005, 54576, 23200, 30371, 38608, 19195, 19152, 42192,
    118966, 53840, 54560, 56645, 46496, 22224, 21938, 18864,
    42359, 42160, 43600, 111189, 27936, 44448, 84835, 37744,
    18936, 18800, 25776, 92326, 59984, 27296, 108228, 43744,
    37600, 53987, 51552, 54615, 54432, 55888, 23893, 22176,
    42704, 21972, 21200, 43448, 43344, 46240, 46758, 44368,
    21920, 43940, 42416, 21168, 45683, 26928, 29495, 27296,
    44368, 84821, 19296, 42352, 21732, 53600, 59752, 54560,
    55968, 92838, 22224, 19168, 43476, 41680, 53584, 62034,
    54560,
];

/// Gregorian (month, day) of Chinese New Year for each year 1900..=2100.
/// Index is `year - 1900`.
#[rustfmt::skip]
const NEW_YEAR_GREGORIAN: [(u32, u32); 201] = [
    (1, 31), (2, 19), (2, 8), (1, 29), (2, 16), (2, 4),
    (1, 25), (2, 13), (2, 2), (1, 22), (2, 10), (1, 30),
    (2, 18), (2, 6), (1, 26), (2, 14), (2, 3), (1, 23),
    (2, 11), (2, 1), (2, 20), (2, 8), (1, 28), (2, 16),
    (2, 5), (1, 24), (2, 13), (2, 2), (1, 23), (2, 10),
    (1, 30), (2, 17), (2, 6), (1, 26), (2, 14), (2, 4),
    (1, 24), (2, 11), (1, 31), (2, 19), (2, 8), (1, 27),
    (2, 15), (2, 5), (1, 25), (2, 13), (2, 2), (1, 22),
    (2, 10), (1, 29), (2, 17), (2, 6), (1, 27), (2, 14),
    (2, 3), (1, 24), (2, 12), (1, 31), (2, 18), (2, 8),
    (1, 28), (2, 15), (2, 5), (1, 25), (2, 13), (2, 2),
    (1, 21), (2, 9), (1, 30), (2, 17), (2, 6), (1, 27),
    (2, 15), (2, 3), (1, 23), (2, 11), (1, 31), (2, 18),
    (2, 7), (1, 28), (2, 16), (2, 5), (1, 25), (2, 13),
    (2, 2), (2, 20), (2, 9), (1, 29), (2, 17), (2, 6),
    (1, 27), (2, 15), (2, 4), (1, 23), (2, 10), (1, 31),
    (2, 19), (2, 7), (1, 28), (2, 16), (2, 5), (1, 24),
    (2, 12), (2, 1), (1, 22), (2, 9), (1, 29), (2, 18),
    (2, 7), (1, 26), (2, 14), (2, 3), (1, 23), (2, 10),
    (1, 31), (2, 19), (2, 8), (1, 28), (2, 16), (2, 5),
    (1, 25), (2, 12), (2, 1), (1, 22), (2, 10), (1, 29),
    (2, 17), (2, 6), (1, 26), (2, 13), (2, 3), (1, 23),
    (2, 11), (1, 31), (2, 19), (2, 8), (1, 28), (2, 15),
    (2, 4), (1, 24), (2, 12), (2, 1), (1, 22), (2, 10),
    (1, 30), (2, 17), (2, 6), (1, 26), (2, 14), (2, 2),
    (1, 23), (2, 11), (2, 1), (2, 19), (2, 8), (1, 28),
    (2, 15), (2, 4), (1, 24), (2, 12), (2, 2), (1, 21),
    (2, 9), (1, 29), (2, 17), (2, 5), (1, 26), (2, 14),
    (2, 3), (1, 23), (2, 11), (1, 31), (2, 19), (2, 7),
    (1, 27), (2, 15), (2, 5), (1, 24), (2, 12), (2, 2),
    (1, 22), (2, 9), (1, 29), (2, 17), (2, 6), (1, 26),
    (2, 14), (2, 3), (1, 24), (2, 10), (1, 30), (2, 18),
    (2, 7), (1, 27), (2, 15), (2, 5), (1, 25), (2, 12),
    (2, 1), (1, 21), (2, 9),
];

/// Convert a lunar date to Gregorian. Returns `None` when out of range
/// (1900-01-01 .. 2100-12-29) or when the lunar date doesn't exist
/// (e.g. month 30 day 31, or leap flag in a year with no leap month).
pub fn lunar_to_solar(
    lunar_year: i32,
    lunar_month: u32,
    lunar_day: u32,
    leap_month: bool,
) -> Option<NaiveDate> {
    if !(1900..=2100).contains(&lunar_year) {
        return None;
    }
    if !(1..=12).contains(&lunar_month) || !(1..=30).contains(&lunar_day) {
        return None;
    }

    let idx = (lunar_year - 1900) as usize;
    let code = YEAR_CODES[idx];
    let (leap_idx, month_days, leap_days) = decode_year(code);

    // Validate leap-month request against the year's actual leap month.
    if leap_month && Some(lunar_month) != leap_idx {
        return None;
    }

    // Validate day exists in the chosen month.
    let days_in_target = if leap_month {
        leap_days
    } else {
        month_days[(lunar_month - 1) as usize]
    };
    if lunar_day > days_in_target {
        return None;
    }

    // Sum days from lunar-new-year start to the target (exclusive of target).
    let mut days_passed: i64 = 0;
    for m in 1..lunar_month {
        days_passed += month_days[(m - 1) as usize] as i64;
        // If the year has a leap month before the target, count it too.
        if leap_idx == Some(m) {
            days_passed += leap_days as i64;
        }
    }
    // If the target itself is the leap month, we must have already counted
    // the non-leap version of that month.
    if leap_month {
        days_passed += month_days[(lunar_month - 1) as usize] as i64;
    }
    days_passed += (lunar_day - 1) as i64;

    // Start date = Chinese New Year of `lunar_year`.
    let (ny_month, ny_day) = NEW_YEAR_GREGORIAN[idx];
    let ny = NaiveDate::from_ymd_opt(lunar_year, ny_month, ny_day)?;
    ny.checked_add_signed(chrono::Duration::days(days_passed))
}

/// Return the length of lunar month `month` in `year` (non-leap). Used by
/// festival lookups that need to know whether a month has 29 or 30 days.
pub fn lunar_month_length(year: i32, month: u32) -> Option<u32> {
    if !(1900..=2100).contains(&year) || !(1..=12).contains(&month) {
        return None;
    }
    let idx = (year - 1900) as usize;
    let code = YEAR_CODES[idx];
    let (_, month_days, _) = decode_year(code);
    Some(month_days[(month - 1) as usize])
}

/// Convert a Gregorian date to its lunar equivalent. Returns
/// `Some((lunar_year, lunar_month, lunar_day, is_leap_month))` or `None`
/// when the date is outside the 1900-2100 converter range.
///
/// Algorithm: find the lunar year whose 春节 (lunar 1/1) ≤ the given date,
/// then walk forward month-by-month counting days until reaching the target.
pub fn solar_to_lunar(date: NaiveDate) -> Option<(i32, u32, u32, bool)> {
    use chrono::Datelike;
    let year = date.year();
    if !(1900..=2100).contains(&year) {
        return None;
    }

    // Determine which lunar year we're in: if the solar date is before this
    // year's Spring Festival, the lunar year is the previous Gregorian year.
    let this_idx = (year - 1900) as usize;
    let (ny_m, ny_d) = NEW_YEAR_GREGORIAN[this_idx];
    let this_ny = NaiveDate::from_ymd_opt(year, ny_m, ny_d)?;
    let (lunar_year, start_date) = if date < this_ny {
        if year == 1900 {
            return None;
        }
        let prev_idx = (year - 1 - 1900) as usize;
        let (pm, pd) = NEW_YEAR_GREGORIAN[prev_idx];
        (year - 1, NaiveDate::from_ymd_opt(year - 1, pm, pd)?)
    } else {
        (year, this_ny)
    };

    let idx = (lunar_year - 1900) as usize;
    let code = YEAR_CODES[idx];
    let (leap_idx, month_days, leap_days) = decode_year(code);

    let mut days_remaining = (date - start_date).num_days();
    if days_remaining < 0 {
        return None;
    }

    let mut m: u32 = 1;
    let mut leap_encountered = false;
    while m <= 12 {
        let this_days = month_days[(m - 1) as usize] as i64;
        if days_remaining < this_days {
            return Some((lunar_year, m, (days_remaining + 1) as u32, false));
        }
        days_remaining -= this_days;
        // If the leap month occurs right after this non-leap month, include it.
        if leap_idx == Some(m) && !leap_encountered {
            if days_remaining < leap_days as i64 {
                return Some((lunar_year, m, (days_remaining + 1) as u32, true));
            }
            days_remaining -= leap_days as i64;
            leap_encountered = true;
        }
        m += 1;
    }
    None
}

/// Returns (leap_month_index, [month_days; 12], leap_month_days).
fn decode_year(code: u32) -> (Option<u32>, [u32; 12], u32) {
    let leap_idx = code & 0xf;
    let leap = if leap_idx == 0 { None } else { Some(leap_idx) };
    let leap_days: u32 = if (code >> 16) & 0xf == 0 { 29 } else { 30 };

    let mut month_days = [29u32; 12];
    // The 12 middle bits: leftmost (bit 15) = month 1, rightmost (bit 4) = month 12.
    for m in 0..12 {
        let shift = 15 - m;
        let bit = (code >> shift) & 1;
        month_days[m as usize] = if bit == 1 { 30 } else { 29 };
    }
    (leap, month_days, leap_days)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_festival_samples_match_table() {
        // Each Spring Festival is lunar 1/1, should hit the NEW_YEAR_GREGORIAN
        // entry exactly.
        let d = lunar_to_solar(2024, 1, 1, false).unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2024, 2, 10).unwrap());

        let d = lunar_to_solar(2003, 1, 1, false).unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2003, 2, 1).unwrap());

        let d = lunar_to_solar(2050, 1, 1, false).unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2050, 1, 23).unwrap());
    }

    #[test]
    fn lantern_festival_2003() {
        // 元宵节 = 正月十五 — user-reported regression case.
        let d = lunar_to_solar(2003, 1, 15, false).unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2003, 2, 15).unwrap());
    }

    #[test]
    fn mid_autumn_1989() {
        // 八月十五. From Python docstring: 1989-9-23 non-leap → 1989-10-22.
        // Wait: docstring says 9月23日 → 10月22日, that's 9/23 lunar = 10/22 solar.
        // Mid-Autumn is 8/15 — verify independently.
        let d = lunar_to_solar(1989, 8, 15, false).unwrap();
        // 1989 Mid-Autumn was September 14.
        assert_eq!(d, NaiveDate::from_ymd_opt(1989, 9, 14).unwrap());
    }

    #[test]
    fn dragon_boat_2023() {
        // 端午 = 五月初五 → 2023-06-22.
        let d = lunar_to_solar(2023, 5, 5, false).unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2023, 6, 22).unwrap());
    }

    #[test]
    fn new_year_eve_fallback() {
        // 除夕 is the day before the next year's New Year. For 2024's NYE
        // (= last day of lunar 2023), use the convention: 2023年腊月三十
        // or 2023年腊月二十九 if that year's 12th month is short.
        // Python's hardcoded NYE for "year" is the Gregorian day just before
        // year's New Year. Our converter returns the *lunar* end-of-year;
        // we compute 腊月(最后一日) of lunar_year=2023.
        let code = YEAR_CODES[(2023 - 1900) as usize];
        let (_leap_idx, month_days, _leap_days) = decode_year(code);
        let last_day = month_days[11]; // 12th month
        let d = lunar_to_solar(2023, 12, last_day, false).unwrap();
        // 2024 New Year = 2024-02-10, so NYE = 2024-02-09.
        assert_eq!(d, NaiveDate::from_ymd_opt(2024, 2, 9).unwrap());
    }

    #[test]
    fn out_of_range_returns_none() {
        assert!(lunar_to_solar(1899, 1, 1, false).is_none());
        assert!(lunar_to_solar(2101, 1, 1, false).is_none());
    }

    #[test]
    fn invalid_day_returns_none() {
        // 2024-lunar month 1 has < 30 days in some years. Request day 31 → None.
        assert!(lunar_to_solar(2024, 1, 31, false).is_none());
    }
}
