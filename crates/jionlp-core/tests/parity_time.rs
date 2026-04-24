//! Python-Rust parity regression suite — time parser domain.
//!
//! The expected outputs below are curated from JioNLP's Python reference —
//! specifically from docstring examples, README examples, and tests. These
//! golden cases document behaviors Rust must preserve. Add a case here
//! whenever you port a module; when behavior intentionally diverges, update
//! the expectation and note the divergence in PLAN.md's risk log.
//!
//! Rationale for keeping these as Rust tests (not a Python-generated file):
//! the Python jionlp pulls jiojio and several system deps that aren't
//! trivially installable in CI. Golden values are cheap; keep them in
//! source.

use jionlp_core as jio;
use std::path::PathBuf;
use std::sync::Once;

static INIT: Once = Once::new();
fn ensure_init() {
    INIT.call_once(|| {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let dict = PathBuf::from(manifest).join("data");
        jio::dict::init_from_path(&dict).expect("init");
    });
}

// ── absolute date ──

#[test]
fn parity_parse_time_absolute() {
    // Python parse_time docstring:
    //   '2024年3月5日' → ['2024-03-05 00:00:00', '2024-03-05 23:59:59']
    use chrono::{Datelike, Timelike};
    let t = jio::parse_time("2024年3月5日").unwrap();
    assert_eq!(t.start.year(), 2024);
    assert_eq!(t.start.month(), 3);
    assert_eq!(t.start.day(), 5);
    assert_eq!(t.start.hour(), 0);
    assert_eq!(t.end.hour(), 23);
    assert_eq!(t.end.minute(), 59);
}

#[test]
fn parity_parse_time_abs_slash_format() {
    // Python parses "2024/3/5" equivalently to "2024年3月5日".
    let t = jio::parse_time("2024/3/5").unwrap();
    use chrono::Datelike;
    assert_eq!(t.start.year(), 2024);
    assert_eq!(t.start.month(), 3);
    assert_eq!(t.start.day(), 5);
}

#[test]
fn parity_parse_time_two_digit_year_future() {
    // Python: "24年3月5日" → 2024.
    let t = jio::parse_time("24年3月5日").unwrap();
    use chrono::Datelike;
    assert_eq!(t.start.year(), 2024);
}

#[test]
fn parity_parse_time_two_digit_year_past() {
    // Python: "98年3月5日" → 1998.
    let t = jio::parse_time("98年3月5日").unwrap();
    use chrono::Datelike;
    assert_eq!(t.start.year(), 1998);
}

// ── relative / delta ──

#[test]
fn parity_parse_time_relative_tomorrow() {
    // Relative days compute against `now`. Verify offset-1 logic with a
    // fixed reference.
    let now = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();
    let t = jio::parse_time_with_ref("明天", now).unwrap();
    assert_eq!(
        t.start.date(),
        chrono::NaiveDate::from_ymd_opt(2024, 3, 16).unwrap()
    );
}

#[test]
fn parity_parse_time_timespan() {
    let now = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();
    let t = jio::parse_time_with_ref("下午3点到5点", now).unwrap();
    use chrono::Timelike;
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.hour(), 15);
    assert_eq!(t.end.hour(), 17);
}

#[test]
fn parity_parse_time_delta_days() {
    let now = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();
    let t = jio::parse_time_with_ref("三天后", now).unwrap();
    assert_eq!(
        t.start.date(),
        chrono::NaiveDate::from_ymd_opt(2024, 3, 18).unwrap()
    );
}

#[test]
fn parity_parse_time_delta_half_hour() {
    let now = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();
    let t = jio::parse_time_with_ref("半小时后", now).unwrap();
    use chrono::Timelike;
    assert_eq!(t.start.hour(), 11);
    assert_eq!(t.start.minute(), 0);
}

#[test]
fn parity_parse_time_delta_weeks_ago() {
    let now = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();
    let t = jio::parse_time_with_ref("两周前", now).unwrap();
    assert_eq!(
        t.start.date(),
        chrono::NaiveDate::from_ymd_opt(2024, 3, 1).unwrap()
    );
}

#[test]
fn parity_parse_time_named_this_week() {
    let now = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();
    let t = jio::parse_time_with_ref("本周", now).unwrap();
    // Python returns time_point for named weeks.
    assert_eq!(t.time_type, "time_point");
    assert_eq!(
        t.start.date(),
        chrono::NaiveDate::from_ymd_opt(2024, 3, 11).unwrap()
    );
}

#[test]
fn parity_parse_time_named_last_month() {
    let now = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();
    let t = jio::parse_time_with_ref("上个月", now).unwrap();
    assert_eq!(t.start.date().month(), 2);
    use chrono::Datelike;
    assert_eq!(t.end.date().day(), 29); // 2024 leap year
}

#[test]
fn parity_parse_time_named_next_quarter() {
    let now = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();
    let t = jio::parse_time_with_ref("下季度", now).unwrap();
    assert_eq!(
        t.start.date(),
        chrono::NaiveDate::from_ymd_opt(2024, 4, 1).unwrap()
    );
    assert_eq!(
        t.end.date(),
        chrono::NaiveDate::from_ymd_opt(2024, 6, 30).unwrap()
    );
}

#[test]
fn parity_parse_time_named_last_year() {
    let now = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();
    let t = jio::parse_time_with_ref("去年", now).unwrap();
    use chrono::Datelike;
    assert_eq!(t.start.date().year(), 2023);
    assert_eq!(t.end.date().year(), 2023);
}

// ── span / range ──

#[test]
fn parity_parse_time_range() {
    // Python: '2024年3月5日到8日' → time_span 2024-03-05..2024-03-08
    let t = jio::parse_time("2024年3月5日到8日").unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(
        t.start.date(),
        chrono::NaiveDate::from_ymd_opt(2024, 3, 5).unwrap()
    );
    assert_eq!(
        t.end.date(),
        chrono::NaiveDate::from_ymd_opt(2024, 3, 8).unwrap()
    );
}

// ── recurring ──

#[test]
fn parity_parse_time_recurring_weekday() {
    let now = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();
    let t = jio::parse_time_with_ref("每周一", now).unwrap();
    assert_eq!(
        t.start.date(),
        chrono::NaiveDate::from_ymd_opt(2024, 3, 18).unwrap()
    );
}

// ── holidays / named dates ──

#[test]
fn parity_parse_time_holiday_guoqing() {
    // Python: '国庆节' with ref 2024-03-15 → 2024-10-01 00:00:00
    use chrono::{Datelike, NaiveDate};
    let now = NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();
    let t = jio::parse_time_with_ref("国庆节", now).unwrap();
    assert_eq!(t.start.year(), 2024);
    assert_eq!(t.start.month(), 10);
    assert_eq!(t.start.day(), 1);
}

// ── fuzzy ──

#[test]
fn parity_parse_time_fuzzy_gangcai() {
    let now = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();
    let t = jio::parse_time_with_ref("刚才", now).unwrap();
    assert_eq!(t.definition, "blur");
    assert!(t.end <= now);
}

#[test]
fn parity_parse_time_fuzzy_mashangjiu() {
    let now = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();
    let t = jio::parse_time_with_ref("马上", now).unwrap();
    assert_eq!(t.definition, "blur");
    assert!(t.end > now);
}

#[test]
fn parity_parse_time_fuzzy_longest_match() {
    let now = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();
    // "不久之前" (4 chars) should win over "不久" or "不久前" (2-3 chars).
    let t = jio::parse_time_with_ref("不久之前", now).unwrap();
    assert_eq!(t.definition, "blur");
}

#[test]
fn parity_parse_time_fuzzy_entire_table() {
    let now = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();
    // Every fuzzy keyword the parser recognizes should return Some(blur).
    for kw in [
        "刚才",
        "刚刚",
        "不久前",
        "最近",
        "近期",
        "晚些时候",
        "马上",
        "立刻",
        "稍后",
        "即将",
        "过一会儿",
        "一会儿",
        "等会儿",
    ] {
        let t = jio::parse_time_with_ref(kw, now)
            .unwrap_or_else(|| panic!("parser returned None for '{kw}'"));
        assert_eq!(t.definition, "blur", "kw={kw}");
    }
}

// ── lunar ──

#[test]
fn parity_lunar_spring_festival_2024() {
    let t = jio::parse_time("2024年春节").unwrap();
    assert_eq!(
        t.start.date(),
        chrono::NaiveDate::from_ymd_opt(2024, 2, 10).unwrap()
    );
}

#[test]
fn parity_lunar_mid_autumn_alias() {
    // "中秋" (without 节) should still resolve.
    let now = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let t = jio::parse_time_with_ref("中秋", now).unwrap();
    assert_eq!(
        t.start.date(),
        chrono::NaiveDate::from_ymd_opt(2024, 9, 17).unwrap()
    );
}

#[test]
fn parity_lunar_new_year_eve() {
    // Python treats `2025年除夕` as the lunar new year's eve of lunar
    // year 2025 (solar 2026-02-16).
    let t = jio::parse_time("2025年除夕").unwrap();
    assert_eq!(
        t.start.date(),
        chrono::NaiveDate::from_ymd_opt(2026, 2, 16).unwrap()
    );
}

#[test]
fn parity_lunar_dragon_boat_2023() {
    let t = jio::parse_time("2023年端午节").unwrap();
    assert_eq!(
        t.start.date(),
        chrono::NaiveDate::from_ymd_opt(2023, 6, 22).unwrap()
    );
}

#[test]
fn parity_lunar_chongyang_with_clock() {
    let now = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let t = jio::parse_time_with_ref("重阳节上午10点", now).unwrap();
    use chrono::Timelike;
    assert_eq!(t.start.hour(), 10);
}

#[test]
fn parity_lunar_out_of_range() {
    // Round 15: we now support 1900-2100. Only years outside that span
    // return None.
    assert!(jio::parse_time("1899年春节").is_none());
    assert!(jio::parse_time("2101年春节").is_none());
}

#[test]
fn parity_lunar_full_coverage_1900_2100() {
    // Full-range smoke: 2019, 2050, 1950 all resolve.
    use chrono::Datelike;
    for (y, expect_m) in [(2019, 2u32), (2050, 1u32), (1950, 2u32)] {
        let t = jio::parse_time(&format!("{y}年春节")).unwrap();
        assert_eq!(t.start.year(), y);
        assert_eq!(t.start.month(), expect_m);
    }
}

#[test]
fn parity_chinese_numeral_year_lingsan() {
    // 零三 → 2003 per 2-digit expansion rule.
    use chrono::Datelike;
    let t = jio::parse_time("零三年元宵节").unwrap();
    assert_eq!(t.start.year(), 2003);
    assert_eq!(t.start.month(), 2);
    assert_eq!(t.start.day(), 15);
}

#[test]
fn parity_clock_banxiao() {
    // "8点半" = 08:30
    let now = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    use chrono::Timelike;
    let t = jio::parse_time_with_ref("8点半", now).unwrap();
    assert_eq!(t.start.hour(), 8);
    assert_eq!(t.start.minute(), 30);
}

#[test]
fn parity_user_regression_lingsan_yuanxiao() {
    // User-reported example: 零三年元宵节晚上8点半 → 2003-02-15 20:30:00
    use chrono::Timelike;
    let t = jio::parse_time("零三年元宵节晚上8点半").unwrap();
    assert_eq!(
        t.start.date(),
        chrono::NaiveDate::from_ymd_opt(2003, 2, 15).unwrap()
    );
    assert_eq!(t.start.hour(), 20);
    assert_eq!(t.start.minute(), 30);
}

// ── parse_time — round 16 cases ──

fn r16_ref() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2021, 6, 15)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap()
}

#[test]
fn parity_parse_time_lunar_chinese_year_clock() {
    // Python: parse_time('零三年元宵节晚上8点半') → 2003-02-15 20:30:00.
    let t = jio::parse_time_with_ref("零三年元宵节晚上8点半", r16_ref()).unwrap();
    assert_eq!(t.time_type, "time_point");
    assert_eq!(t.start.to_string(), "2003-02-15 20:30:00");
}

#[test]
fn parity_parse_time_year_span_quarters() {
    // Python: parse_time('2021年前两个季度') → time_span 2021 Q1-Q2.
    let t = jio::parse_time_with_ref("2021年前两个季度", r16_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "2021-01-01 00:00:00");
    assert_eq!(t.end.to_string(), "2021-06-30 23:59:59");
}

#[test]
fn parity_parse_time_cong_chinese_day_range() {
    // Python: parse_time('从2018年12月九号到十五号') → 2018-12-09..2018-12-15.
    let t = jio::parse_time_with_ref("从2018年12月九号到十五号", r16_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "2018-12-09 00:00:00");
    assert_eq!(t.end.to_string(), "2018-12-15 23:59:59");
}

#[test]
fn parity_parse_time_nth_weekday_festival() {
    // Python: parse_time('2019年感恩节') → 2019-11-28 (4th Thursday of Nov).
    let t = jio::parse_time_with_ref("2019年感恩节", r16_ref()).unwrap();
    assert_eq!(t.time_type, "time_point");
    assert_eq!(t.start.to_string(), "2019-11-28 00:00:00");
}

#[test]
fn parity_parse_time_recurring_weekly_with_clock_range() {
    // Python: parse_time('每周六上午9点到11点') →
    //   time_period, delta={day:7}, point covers 09:00..11:00.
    let t = jio::parse_time_with_ref("每周六上午9点到11点", r16_ref()).unwrap();
    assert_eq!(t.time_type, "time_period");
    let d = t.delta.as_ref().expect("delta populated for time_period");
    match d.day {
        Some(jio::DeltaValue::Single(n)) => assert_eq!(n, 7.0),
        _ => panic!("expected weekly cadence {{day: 7}}, got {:?}", d.day),
    }
    let p = t.period.as_ref().expect("period populated");
    assert_eq!(p.point_time.len(), 1);
    assert_eq!(p.point_time[0].0.format("%H:%M:%S").to_string(), "09:00:00");
    assert_eq!(p.point_time[0].1.format("%H:%M:%S").to_string(), "11:00:00");
}

#[test]
fn parity_parse_time_delta_day_range() {
    // Python: parse_time('30~90日') →
    //   time_delta, {day: [30, 90]}.
    let t = jio::parse_time_with_ref("30~90日", r16_ref()).unwrap();
    assert_eq!(t.time_type, "time_delta");
    let d = t.delta.as_ref().expect("delta populated");
    match d.day {
        Some(jio::DeltaValue::Range(lo, hi)) => {
            assert_eq!(lo, 30.0);
            assert_eq!(hi, 90.0);
        }
        _ => panic!("expected day Range(30, 90), got {:?}", d.day),
    }
}

// ── parse_time — round 17 cases ──

fn r17_ref() -> chrono::NaiveDateTime {
    // Friday 2024-03-15.
    chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap()
}

#[test]
fn parity_parse_time_eight_digit_ymd() {
    let t = jio::parse_time_with_ref("20210901", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_point");
    assert_eq!(t.start.to_string(), "2021-09-01 00:00:00");
    assert_eq!(t.end.to_string(), "2021-09-01 23:59:59");
}

#[test]
fn parity_parse_time_year_solar_season() {
    let t = jio::parse_time_with_ref("2021年第1季度", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "2021-01-01 00:00:00");
    assert_eq!(t.end.to_string(), "2021-03-31 23:59:59");
}

#[test]
fn parity_parse_time_year_blur_boundary() {
    let t = jio::parse_time_with_ref("2024年末", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.definition, "blur");
    assert_eq!(t.start.to_string(), "2024-11-01 00:00:00");
    assert_eq!(t.end.to_string(), "2024-12-31 23:59:59");
}

#[test]
fn parity_parse_time_limit_month_day() {
    let t = jio::parse_time_with_ref("下个月9号", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_point");
    // 2024-03 + 1 month = 2024-04.
    assert_eq!(t.start.to_string(), "2024-04-09 00:00:00");
}

#[test]
fn parity_parse_time_century_full() {
    // Python treats 20世纪 = 1900..1999 (convention starts at year 1900,
    // not 1901).
    let t = jio::parse_time_with_ref("20世纪", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "1900-01-01 00:00:00");
    assert_eq!(t.end.to_string(), "1999-12-31 23:59:59");
}

#[test]
fn parity_parse_time_century_decade() {
    let t = jio::parse_time_with_ref("20世纪二十年代", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "1920-01-01 00:00:00");
    assert_eq!(t.end.to_string(), "1929-12-31 23:59:59");
}

#[test]
fn parity_parse_time_standalone_weekday() {
    // now = Friday 2024-03-15; 周一 → next Monday 2024-03-18.
    let t = jio::parse_time_with_ref("周一", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2024-03-18 00:00:00");
}

#[test]
fn parity_parse_time_named_weekday() {
    // 下周五 = 2024-03-22.
    let t = jio::parse_time_with_ref("下周五", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2024-03-22 00:00:00");
}

#[test]
fn parity_parse_time_year_week() {
    // 2024 week 5: Monday = 2024-01-29.
    let t = jio::parse_time_with_ref("2024年第5周", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2024-01-29 00:00:00");
    assert_eq!(t.end.to_string(), "2024-02-04 23:59:59");
}

#[test]
fn parity_parse_time_nth_weekday_in_month() {
    // 2024-10 first Friday = 2024-10-04.
    let t = jio::parse_time_with_ref("2024年10月的第一个周五", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2024-10-04 00:00:00");
}

#[test]
fn parity_parse_time_year_day_ordinal() {
    let t = jio::parse_time_with_ref("2024年第100天", r17_ref()).unwrap();
    // 2024-01-01 + 99 days = 2024-04-09.
    assert_eq!(t.start.to_string(), "2024-04-09 00:00:00");
}

#[test]
fn parity_parse_time_special_now() {
    let t = jio::parse_time_with_ref("现在", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_point");
    assert_eq!(t.start.to_string(), "2024-03-15 10:30:00");
    assert_eq!(t.end, t.start);
}

#[test]
fn parity_parse_time_special_all_day() {
    let t = jio::parse_time_with_ref("全天", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2024-03-15 00:00:00");
    assert_eq!(t.end.to_string(), "2024-03-15 23:59:59");
}

#[test]
fn parity_parse_time_jinming_two_days() {
    let t = jio::parse_time_with_ref("今明两天", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2024-03-15 00:00:00");
    assert_eq!(t.end.to_string(), "2024-03-16 23:59:59");
}

#[test]
fn parity_parse_time_super_blur_past_two_days() {
    // Python `前两天` = [now-7d, now-2d] whole days.
    let t = jio::parse_time_with_ref("前两天", r17_ref()).unwrap();
    assert_eq!(t.definition, "blur");
    assert_eq!(t.start.to_string(), "2024-03-08 00:00:00");
    assert_eq!(t.end.to_string(), "2024-03-13 23:59:59");
}

#[test]
fn parity_parse_time_super_blur_future_three_days() {
    // Python `未来三天` = [now, now+3d] preserving now's time.
    let t = jio::parse_time_with_ref("未来三天", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2024-03-15 10:30:00");
    assert_eq!(t.end.to_string(), "2024-03-18 10:30:00");
}

// ── lunar (round 18) ──

#[test]
fn parity_parse_time_lunar_mid_year() {
    // Python: parse_time('2012年农历正月十九') → 2012-02-10.
    let t = jio::parse_time_with_ref("2012年农历正月十九", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2012-02-10 00:00:00");
}

#[test]
fn parity_parse_time_lunar_standalone() {
    let t = jio::parse_time_with_ref("腊月二十八", r17_ref()).unwrap();
    // 2024 now → 2024 lunar 12/28 maps to 2025-01-27.
    assert_eq!(t.start.to_string(), "2025-01-27 00:00:00");
}

// ── solar terms / seasons ──

#[test]
fn parity_parse_time_solar_term_with_year() {
    let t = jio::parse_time_with_ref("2021年清明", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2021-04-05 00:00:00");
}

#[test]
fn parity_parse_time_solar_term_standalone() {
    // now.year() = 2024; 立春 2024 = Feb 4.
    let t = jio::parse_time_with_ref("立春", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2024-02-04 00:00:00");
}

#[test]
fn parity_parse_time_season_with_year() {
    // Python: `2021年春天` uses solar-term boundaries (立春..立夏-1).
    let t = jio::parse_time_with_ref("2021年春天", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "2021-02-03 00:00:00");
    assert_eq!(t.end.to_string(), "2021-05-04 23:59:59");
}

#[test]
fn parity_parse_time_season_limit_year() {
    // Python solar-term boundaries: 立夏 2023 = May 6.
    let t = jio::parse_time_with_ref("去年夏季", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2023-05-06 00:00:00");
    assert_eq!(t.end.to_string(), "2023-08-07 23:59:59");
}

#[test]
fn parity_parse_time_limit_year_festival_fixed() {
    // 今年儿童节 with now=2024 → 2024-06-01.
    let t = jio::parse_time_with_ref("今年儿童节", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2024-06-01 00:00:00");
}

#[test]
fn parity_parse_time_limit_year_festival_lunar() {
    // 明年端午 with now=2024 → lunar 2025/5/5 = Gregorian 2025-05-31.
    let t = jio::parse_time_with_ref("明年端午", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2025-05-31 00:00:00");
}

#[test]
fn parity_parse_time_limit_year_festival_nth_weekday() {
    // 今年母亲节 = 2nd Sunday of May 2024 = 2024-05-12.
    let t = jio::parse_time_with_ref("今年母亲节", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2024-05-12 00:00:00");
}

// ── pure delta ──

#[test]
fn parity_parse_time_pure_delta_year() {
    let t = jio::parse_time_with_ref("3年", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_delta");
    match t.delta.as_ref().unwrap().year {
        Some(jio::DeltaValue::Single(n)) => assert_eq!(n, 3.0),
        _ => panic!("expected year Single(3)"),
    }
}

#[test]
fn parity_parse_time_pure_delta_month_chinese() {
    let t = jio::parse_time_with_ref("两个月", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_delta");
    match t.delta.as_ref().unwrap().month {
        Some(jio::DeltaValue::Single(n)) => assert_eq!(n, 2.0),
        _ => panic!("expected month Single(2)"),
    }
}

#[test]
fn parity_parse_time_pure_delta_blur_tens_of_days() {
    // Python `几十` = [20, 100].
    let t = jio::parse_time_with_ref("几十天", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_delta");
    assert_eq!(t.definition, "blur");
    match t.delta.as_ref().unwrap().day {
        Some(jio::DeltaValue::Range(lo, hi)) => {
            assert_eq!(lo, 20.0);
            assert_eq!(hi, 100.0);
        }
        _ => panic!("expected day Range(20, 100)"),
    }
}

#[test]
fn parity_parse_time_delta_to_span_future() {
    // Python `再过五天` → open-ended [now+5d, inf].
    let t = jio::parse_time_with_ref("再过五天", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "2024-03-20 10:30:00");
}

#[test]
fn parity_parse_time_delta_inner_span() {
    // Python `5分钟内` → span [now, now+5min].
    let t = jio::parse_time_with_ref("5分钟内", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "2024-03-15 10:30:00");
    assert_eq!(t.end.to_string(), "2024-03-15 10:35:00");
}

#[test]
fn parity_parse_time_delta_open_ended_subday() {
    // Round 32: `之前` now emits `time_span` (Python parity). The span
    // runs from 48h ago to now.
    let t = jio::parse_time_with_ref("48小时之前", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "2024-03-13 10:30:00");
    assert_eq!(t.end.to_string(), "2024-03-15 10:30:00");
}

#[test]
fn parity_parse_time_workday_delta() {
    // now = Fri 2024-03-15; 1 workday before = Thu 2024-03-14.
    let t = jio::parse_time_with_ref("1个工作日前", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2024-03-14 00:00:00");
}

#[test]
fn parity_parse_time_pure_delta_workday() {
    let t = jio::parse_time_with_ref("3个工作日", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_delta");
    match t.delta.as_ref().unwrap().workday {
        Some(jio::DeltaValue::Single(n)) => assert_eq!(n, 3.0),
        _ => panic!("expected workday Single(3)"),
    }
}

#[test]
fn parity_parse_time_pure_delta_range_cn() {
    // 两三天 → {day: [2, 3]}.
    let t = jio::parse_time_with_ref("两三天", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_delta");
    assert_eq!(t.definition, "blur");
    match t.delta.as_ref().unwrap().day {
        Some(jio::DeltaValue::Range(lo, hi)) => {
            assert_eq!(lo, 2.0);
            assert_eq!(hi, 3.0);
        }
        _ => panic!("expected day Range(2, 3)"),
    }
}

// ── blur hour ──

#[test]
fn parity_parse_time_blur_hour_morning() {
    // Python classifies blur-hour phrases as time_point (a named part of
    // the day) with blur definition.
    let t = jio::parse_time_with_ref("早上", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_point");
    assert_eq!(t.start.to_string(), "2024-03-15 06:00:00");
    assert_eq!(t.end.to_string(), "2024-03-15 09:59:59");
}

#[test]
fn parity_parse_time_blur_hour_with_day_prefix() {
    let t = jio::parse_time_with_ref("明天晚上", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2024-03-16 18:00:00");
    assert_eq!(t.end.to_string(), "2024-03-16 23:59:59");
}

#[test]
fn parity_parse_time_approx_clock() {
    let t = jio::parse_time_with_ref("约9点", r17_ref()).unwrap();
    assert_eq!(t.definition, "blur");
    assert_eq!(t.start.format("%H:%M").to_string(), "09:00");
}

#[test]
fn parity_parse_time_super_blur_hms() {
    // Python super_blur_two_hms: `前两个小时` → [now-6h, now-2h] hour-precision.
    let t = jio::parse_time_with_ref("前两个小时", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "2024-03-15 04:00:00");
    assert_eq!(t.end.to_string(), "2024-03-15 08:59:59");
}

// ── recurring ──

#[test]
fn parity_parse_time_recurring_hourly() {
    let t = jio::parse_time_with_ref("每小时", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_period");
    match t.delta.as_ref().unwrap().hour {
        Some(jio::DeltaValue::Single(n)) => assert_eq!(n, 1.0),
        _ => panic!("expected hour Single(1)"),
    }
}

#[test]
fn parity_parse_time_recurring_every_n_min() {
    let t = jio::parse_time_with_ref("每30分钟", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_period");
    match t.delta.as_ref().unwrap().minute {
        Some(jio::DeltaValue::Single(n)) => assert_eq!(n, 30.0),
        _ => panic!("expected minute Single(30)"),
    }
}

#[test]
fn parity_parse_time_recurring_gap() {
    // 每隔一天 = cadence 2 days.
    let t = jio::parse_time_with_ref("每隔一天", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_period");
    match t.delta.as_ref().unwrap().day {
        Some(jio::DeltaValue::Single(n)) => assert_eq!(n, 2.0),
        _ => panic!("expected day Single(2)"),
    }
}

#[test]
fn parity_parse_time_recurring_yearly_festival() {
    let t = jio::parse_time_with_ref("每年春节", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_period");
    // 2024 春节 = 2024-02-10.
    assert_eq!(t.start.to_string(), "2024-02-10 00:00:00");
    match t.delta.as_ref().unwrap().year {
        Some(jio::DeltaValue::Single(n)) => assert_eq!(n, 1.0),
        _ => panic!("expected year Single(1)"),
    }
}

// ── open-ended spans ──

#[test]
fn parity_parse_time_open_ended_after() {
    let t = jio::parse_time_with_ref("2024年3月之后", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.definition, "blur");
    // End-of-March 2024.
    assert_eq!(t.start.format("%Y-%m").to_string(), "2024-03");
    // Sentinel far-future.
    assert_eq!(t.end.to_string(), "9999-12-31 23:59:59");
}

#[test]
fn parity_parse_time_open_ended_before() {
    // Python: X之前 with past X → end = X's full-day end.
    let t = jio::parse_time_with_ref("2024年春节之前", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "0001-01-01 00:00:00");
    assert_eq!(t.end.to_string(), "2024-02-10 23:59:59");
}

// ── round 29 — parse_time tail ──

#[test]
fn parity_enum_days() {
    let t = jio::parse_time_with_ref("8月14日、15日、16日", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "2024-08-14 00:00:00");
    assert_eq!(t.end.to_string(), "2024-08-16 23:59:59");
}

#[test]
fn parity_limit_year_span_month() {
    let t = jio::parse_time_with_ref("今年前两个季度", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2024-01-01 00:00:00");
    assert_eq!(t.end.to_string(), "2024-06-30 23:59:59");
}

#[test]
fn parity_year_solar_season_boundary() {
    let t = jio::parse_time_with_ref("2021年第1季度初", r17_ref()).unwrap();
    assert_eq!(t.definition, "blur");
    assert_eq!(t.start.to_string(), "2021-01-01 00:00:00");
    assert_eq!(t.end.to_string(), "2021-01-31 23:59:59");
}

#[test]
fn parity_school_break() {
    let t = jio::parse_time_with_ref("2024年暑假", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2024-07-01 00:00:00");
    assert_eq!(t.end.to_string(), "2024-08-31 23:59:59");
}

#[test]
fn parity_limit_year_week() {
    let t = jio::parse_time_with_ref("明年第10周", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2025-03-10 00:00:00");
}

#[test]
fn parity_year_ordinal() {
    let t = jio::parse_time_with_ref("第一年", r17_ref()).unwrap();
    assert_eq!(t.start.to_string(), "2024-01-01 00:00:00");
    assert_eq!(t.end.to_string(), "2024-12-31 23:59:59");
}

#[test]
fn parity_yinian_siji() {
    let t = jio::parse_time_with_ref("一年四季", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_delta");
    match t.delta.as_ref().unwrap().year {
        Some(jio::DeltaValue::Single(n)) => assert_eq!(n, 1.0),
        _ => panic!("expected year Single(1)"),
    }
}

#[test]
fn parity_recurring_weekday_filtered() {
    let t = jio::parse_time_with_ref("每周工作日", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_period");
    // now = Fri 2024-03-15; next weekday = Mon 2024-03-18.
    assert_eq!(t.start.to_string(), "2024-03-18 00:00:00");
    match t.delta.as_ref().unwrap().day {
        Some(jio::DeltaValue::Single(n)) => assert_eq!(n, 1.0),
        _ => panic!("expected day Single(1)"),
    }
}

// ── round 32 — partial upgrades ──

#[test]
fn parity_delta_month_zhihou_upgraded_to_span() {
    // Python: `3个月之后` → open-ended time_span from (now+3mo).
    let t = jio::parse_time_with_ref("3个月之后", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    // Start lands in June 2024 (~3*30.417 days after now).
    assert_eq!(t.start.format("%Y-%m").to_string(), "2024-06");
}

#[test]
fn parity_delta_hour_yiqian_upgraded_to_span() {
    // Python: `48小时之前` → time_span [now-48h, now].
    let t = jio::parse_time_with_ref("48小时之前", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "2024-03-13 10:30:00");
    assert_eq!(t.end.to_string(), "2024-03-15 10:30:00");
}

#[test]
fn parity_delta_quarter_future_point() {
    // Python `两个季度后` = month-span of cur (now + 2 quarters = Sep 2024).
    let t = jio::parse_time_with_ref("两个季度后", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "2024-09-01 00:00:00");
    assert_eq!(t.end.to_string(), "2024-09-30 23:59:59");
}

#[test]
fn parity_clock_er_ke() {
    // 二刻 / 两刻 = 30 min.
    let t = jio::parse_time_with_ref("9点二刻", r17_ref()).unwrap();
    assert_eq!(t.start.format("%H:%M").to_string(), "09:30");
    let t = jio::parse_time_with_ref("9点两刻", r17_ref()).unwrap();
    assert_eq!(t.start.format("%H:%M").to_string(), "09:30");
}

#[test]
fn parity_approx_clock_range() {
    // 大约晚上8到10点 — approximation prefix + clock range → blur span.
    let t = jio::parse_time_with_ref("大约晚上8到10点", r17_ref()).unwrap();
    assert_eq!(t.definition, "blur");
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.format("%H:%M").to_string(), "20:00");
    assert_eq!(t.end.format("%H:%M").to_string(), "22:00");
}

#[test]
fn parity_blur_year_before() {
    // Python: `32年前` → time_span covering the target year (1992).
    let t = jio::parse_time_with_ref("32年前", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "1992-01-01 00:00:00");
    assert_eq!(t.end.to_string(), "1992-12-31 23:59:59");
}

#[test]
fn parity_date_range_tilde() {
    // Date range separated by ASCII tilde.
    let t = jio::parse_time_with_ref("2024年3月~2024年4月", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "2024-03-01 00:00:00");
    assert_eq!(t.end.to_string(), "2024-04-30 23:59:59");
}

#[test]
fn parity_nested_delta_quarter_minute() {
    // `一个季度的十五分后` = now + 3 months + 15 min.
    let t = jio::parse_time_with_ref("一个季度的十五分后", r17_ref()).unwrap();
    assert_eq!(t.time_type, "time_point");
    assert_eq!(t.start.to_string(), "2024-06-15 10:45:00");
}

// ── all Python test_time_parser.py cases ──

fn ts1() -> chrono::NaiveDateTime {
    chrono::NaiveDateTime::parse_from_str("2021-06-14T01:06:40", "%Y-%m-%dT%H:%M:%S").unwrap()
}

#[test]
fn parity_time_py_eight_digit() {
    let t = jio::parse_time_with_ref("20240307", ts1()).unwrap();
    assert_eq!(t.time_type, "time_point");
    assert_eq!(t.start.to_string(), "2024-03-07 00:00:00");
    assert_eq!(t.end.to_string(), "2024-03-07 23:59:59");
}

#[test]
fn parity_time_py_slash_format() {
    let t = jio::parse_time_with_ref("2019/04/19", ts1()).unwrap();
    assert_eq!(t.start.to_string(), "2019-04-19 00:00:00");
}

#[test]
fn parity_time_py_minute_precision() {
    // Minute-precision end goes to :59 second.
    let t = jio::parse_time_with_ref("2018-11-29 18:59", ts1()).unwrap();
    assert_eq!(t.start.to_string(), "2018-11-29 18:59:00");
    assert_eq!(t.end.to_string(), "2018-11-29 18:59:59");
}

#[test]
fn parity_time_py_dot_separated() {
    let t = jio::parse_time_with_ref("2019.9.6", ts1()).unwrap();
    assert_eq!(t.start.to_string(), "2019-09-06 00:00:00");
}

#[test]
fn parity_time_py_bare_year() {
    let t = jio::parse_time_with_ref("2018", ts1()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "2018-01-01 00:00:00");
    assert_eq!(t.end.to_string(), "2018-12-31 23:59:59");
}

#[test]
fn parity_time_py_two_digit_year() {
    let t = jio::parse_time_with_ref("03年2月28日", ts1()).unwrap();
    assert_eq!(t.start.to_string(), "2003-02-28 00:00:00");
}

#[test]
fn parity_time_py_month_only() {
    let t = jio::parse_time_with_ref("98年4月", ts1()).unwrap();
    assert_eq!(t.start.to_string(), "1998-04-01 00:00:00");
    assert_eq!(t.end.to_string(), "1998-04-30 23:59:59");
}

#[test]
fn parity_time_py_double_em_dash() {
    // Pattern #105 — `——` (double em-dash) as range separator.
    let t = jio::parse_time_with_ref("2024年3月——2024年4月", ts1()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "2024-03-01 00:00:00");
    assert_eq!(t.end.to_string(), "2024-04-30 23:59:59");
}

#[test]
fn parity_time_py_double_dash_ascii() {
    let t = jio::parse_time_with_ref("2024年3月--2024年4月", ts1()).unwrap();
    assert_eq!(t.start.to_string(), "2024-03-01 00:00:00");
}

#[test]
fn parity_time_py_ym_range_mixed() {
    // Python: `1999.08-2002.02` → time_span 1999-08-01..2002-02-28.
    let t = jio::parse_time_with_ref("1999.08-2002.02", ts1()).unwrap();
    assert_eq!(t.time_type, "time_span");
    assert_eq!(t.start.to_string(), "1999-08-01 00:00:00");
    assert_eq!(t.end.to_string(), "2002-02-28 23:59:59");
}

#[test]
fn parity_time_py_ym_to_year() {
    // Python: `2008.03-2009` → time_span 2008-03-01..2009-12-31.
    let t = jio::parse_time_with_ref("2008.03-2009", ts1()).unwrap();
    assert_eq!(t.start.to_string(), "2008-03-01 00:00:00");
    assert_eq!(t.end.to_string(), "2009-12-31 23:59:59");
}

#[test]
fn parity_time_py_fullwidth_colon() {
    let t = jio::parse_time_with_ref("2021-09-12-11：23", ts1()).unwrap();
    assert_eq!(t.start.to_string(), "2021-09-12 11:23:00");
    assert_eq!(t.end.to_string(), "2021-09-12 11:23:59");
}

// ── normalize_time_period ──

#[test]
fn parity_time_period_parser() {
    // normalize_time_period: `两年` → {year: 2}.
    let d = jio::normalize_time_period("两年").unwrap();
    match d.year {
        Some(jio::DeltaValue::Single(n)) => assert_eq!(n, 2.0),
        _ => panic!("expected year=2"),
    }
}

// ── extract_time ──

#[test]
fn parity_extract_time_from_text() {
    ensure_init();
    let now =
        chrono::NaiveDateTime::parse_from_str("2021-09-01T15:15:32", "%Y-%m-%dT%H:%M:%S").unwrap();
    let text = "中秋、国庆两个假期已在眼前。2021年中秋节是9月21日，星期二。";
    let r = jio::extract_time(text, now, false, false);
    let texts: Vec<&str> = r.iter().map(|e| e.text.as_str()).collect();
    assert!(
        texts
            .iter()
            .any(|t| t.contains("2021") || t.contains("中秋") || t.contains("9月")),
        "expected time entity, got {:?}",
        texts
    );
}
