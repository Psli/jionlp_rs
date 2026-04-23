//! Port of `jionlp/gadget/time_period_parser.py::TimePeriodParser`.
//!
//! The Python reference is explicitly marked "暂时搁置" (experimental /
//! shelved): it maps a short period phrase like `两年`, `三个月`, `半天` to
//! a `TimeDelta` dict. Because our `parse_time` already returns the same
//! shape for a pure delta, this module is a thin convenience wrapper that
//! guarantees a `TimeDelta` output for any recognized period phrase.

use crate::gadget::time_parser::{parse_time, TimeDelta};

/// Parse a time-period string (e.g. `两年` / `三个月` / `半天`) into a
/// `TimeDelta`. Returns `None` if the input can't be interpreted as a
/// pure period.
///
/// This mirrors Python's `TimePeriodParser.normalize_time_period` —
/// period-only inputs collapse to a `{unit: count}` dict.
pub fn normalize_time_period(s: &str) -> Option<TimeDelta> {
    let info = parse_time(s)?;
    if info.time_type == "time_delta" {
        return info.delta;
    }
    // For time_period inputs, the caller may also want the inner delta.
    if info.time_type == "time_period" {
        return info.period.map(|p| p.delta);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_years() {
        let d = normalize_time_period("两年").unwrap();
        match d.year {
            Some(crate::gadget::time_parser::DeltaValue::Single(n)) => assert_eq!(n, 2.0),
            _ => panic!("expected year=2, got {:?}", d.year),
        }
    }

    #[test]
    fn three_months() {
        let d = normalize_time_period("三个月").unwrap();
        match d.month {
            Some(crate::gadget::time_parser::DeltaValue::Single(n)) => assert_eq!(n, 3.0),
            _ => panic!("expected month=3, got {:?}", d.month),
        }
    }

    #[test]
    fn half_hour() {
        let d = normalize_time_period("半小时").unwrap();
        // 半 maps to 0.5 in the pure-delta path.
        assert!(d.hour.is_some());
    }

    #[test]
    fn non_period_returns_none() {
        let d = normalize_time_period("2024年3月5日");
        // Absolute dates are not periods — None.
        assert!(d.is_none());
    }
}
