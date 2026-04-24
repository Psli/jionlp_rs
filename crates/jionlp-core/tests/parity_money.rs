//! Python-Rust parity regression suite — money parser domain.
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

#[allow(dead_code)]
static INIT: Once = Once::new();
#[allow(dead_code)]
fn ensure_init() {
    INIT.call_once(|| {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let dict = PathBuf::from(manifest).join("data");
        jio::dict::init_from_path(&dict).expect("init");
    });
}

// ── basic parse_money cases ──

#[test]
fn parity_parse_money_docstring_simple() {
    // Python docstring:
    //   "六十四万零一百四十三元一角七分" → 640143.17元
    // Our basic parser doesn't yet handle 角/分 tails; verify the simpler
    // "一百元" case we do support and keep the complex one as a TODO.
    let m = jio::parse_money("一百元").unwrap();
    assert_eq!(m.num, 100.0);
    assert_eq!(m.case, "元");
}

#[test]
fn parity_parse_money_yuan_jiao_fen() {
    // Python docstring: 六十四万零一百四十三元一角七分 → 640143.17元.
    // Our Stage 1 parser doesn't yet consume the Chinese big-number form
    // for the integer part of compound money; verify the simpler
    // "100元5角3分" case we do support.
    let m = jio::parse_money("100元5角3分").unwrap();
    assert!((m.num - 100.53).abs() < 1e-9);
    assert_eq!(m.case, "元");
}

#[test]
fn parity_parse_money_range() {
    let m = jio::parse_money("100-200元").unwrap();
    assert_eq!(m.num, 100.0);
    assert_eq!(m.end_num, Some(200.0));
}

#[test]
fn parity_parse_money_blur() {
    let m = jio::parse_money("约100元").unwrap();
    assert_eq!(m.definition, "blur");
    assert_eq!(m.num, 100.0);
}

#[test]
fn parity_parse_money_currency_symbols() {
    let y = jio::parse_money("￥100").unwrap();
    assert_eq!(y.case, "元");
    let d = jio::parse_money("$100.5").unwrap();
    assert_eq!(d.case, "美元");
    let e = jio::parse_money("€50").unwrap();
    assert_eq!(e.case, "欧元");
}

#[test]
fn parity_parse_money_comma_and_decimal() {
    let m = jio::parse_money("1,234.56元").unwrap();
    assert!((m.num - 1234.56).abs() < 1e-9);
    assert_eq!(m.case, "元");
}

#[test]
fn parity_parse_money_bare_default_yuan() {
    // Python default_unit='元' when no currency is given.
    let m = jio::parse_money_with_default("1000", "元").unwrap();
    assert_eq!(m.num, 1000.0);
    assert_eq!(m.case, "元");
}

// ── round 33 — parse_money from test_money_parser.py ──

fn money_case(text: &str, num: f64, case: &str, def: &str) {
    let m = jio::parse_money(text).unwrap_or_else(|| panic!("parse_money({}) returned None", text));
    assert!(
        (m.num - num).abs() < 1e-3,
        "num: got {} vs expected {} ({})",
        m.num,
        num,
        text
    );
    assert_eq!(m.case, case, "case ({})", text);
    assert_eq!(m.definition, def, "definition ({})", text);
}

#[test]
fn parity_money_fullwidth_comma() {
    money_case("82，225.00元", 82225.00, "元", "accurate");
}

#[test]
fn parity_money_hkd() {
    money_case("25481港元", 25481.00, "港元", "accurate");
}

#[test]
fn parity_money_usd_decimal() {
    money_case("45564.44美元", 45564.44, "美元", "accurate");
}

#[test]
fn parity_money_cn_hybrid() {
    money_case("1.2万元", 12000.0, "元", "accurate");
}

#[test]
fn parity_money_quadrillion_jpy() {
    // 3千万亿 = 3 × 10^3 × 10^12 = 3e15.
    money_case("3千万亿日元", 3e15, "日元", "accurate");
}

#[test]
fn parity_money_blur_approx_usd() {
    money_case("约4.287亿美元", 428_700_000.0, "美元", "blur");
}

#[test]
fn parity_money_blur_jin() {
    money_case("近700万元", 7_000_000.0, "元", "blur-");
}

#[test]
fn parity_money_range_wan() {
    let m = jio::parse_money("1万-5万元").unwrap();
    assert!((m.num - 10_000.0).abs() < 1e-3);
    assert_eq!(m.end_num, Some(50_000.0));
    assert_eq!(m.definition, "blur");
}

#[test]
fn parity_money_duo_suffix() {
    money_case("3000多欧元", 3000.0, "欧元", "blur");
}

#[test]
fn parity_money_quantifier_shiduo() {
    let m = jio::parse_money("十几块钱").unwrap();
    assert!((m.num - 10.0).abs() < 1e-3);
    assert_eq!(m.end_num, Some(20.0));
    assert_eq!(m.case, "元");
    assert_eq!(m.definition, "blur");
}

#[test]
fn parity_money_quantifier_jishiwan() {
    // Round 36: Python convention for 几十 = 10..100 (not 10..90).
    let m = jio::parse_money("几十万块").unwrap();
    assert!((m.num - 100_000.0).abs() < 1e-3);
    assert_eq!(m.end_num, Some(1_000_000.0));
    assert_eq!(m.case, "元");
    assert_eq!(m.definition, "blur");
}

#[test]
fn parity_money_consecutive_cn_digits() {
    let m = jio::parse_money("八九亿韩元").unwrap();
    assert!((m.num - 800_000_000.0).abs() < 1e-3);
    assert_eq!(m.end_num, Some(900_000_000.0));
    assert_eq!(m.case, "韩元");
    assert_eq!(m.definition, "blur");
}

// ── round 36 — all 54 Python test_money_parser cases ──

/// Assert helper with optional end_num.
fn money_case_full(input: &str, lo: f64, hi: Option<f64>, case: &str, def: &str) {
    let m = jio::parse_money(input).unwrap_or_else(|| panic!("parse_money({}) → None", input));
    assert!(
        (m.num - lo).abs() < 1e-3,
        "{} num: got {}, want {}",
        input,
        m.num,
        lo
    );
    match hi {
        Some(h) => {
            let got = m.end_num.unwrap_or(f64::NAN);
            assert!(
                (got - h).abs() < 1e-3,
                "{} end_num: got {:?}, want {}",
                input,
                m.end_num,
                h
            );
        }
        None => assert_eq!(m.end_num, None, "{} end_num", input),
    }
    assert_eq!(m.case, case, "{} case", input);
    assert_eq!(m.definition, def, "{} definition", input);
}

#[test]
fn parity_money_all_54_from_python() {
    let cases: &[(&str, f64, Option<f64>, &str, &str)] = &[
        ("82，225.00元", 82225.00, None, "元", "accurate"),
        ("25481港元", 25481.00, None, "港元", "accurate"),
        ("45564.44美元", 45564.44, None, "美元", "accurate"),
        (
            "233,333，333,434.344元",
            233333333434.34,
            None,
            "元",
            "accurate",
        ),
        ("1.2万元", 12000.0, None, "元", "accurate"),
        ("3千万亿日元", 3e15, None, "日元", "accurate"),
        (
            "新台币 177.1 亿元",
            17710000000.0,
            None,
            "新台币",
            "accurate",
        ),
        ("15k左右", 15000.0, None, "元", "blur"),
        ("30w上下", 300000.0, None, "元", "blur"),
        ("123元1角1分", 123.11, None, "元", "accurate"),
        (
            "六十四万零一百四十三元一角七分",
            640143.17,
            None,
            "元",
            "accurate",
        ),
        ("壹万二千三百四十五元", 12345.0, None, "元", "accurate"),
        ("三百万", 3000000.0, None, "元", "accurate"),
        ("肆佰叁拾萬", 4300000.0, None, "元", "accurate"),
        ("肆佰叁拾萬圆整", 4300000.0, None, "元", "accurate"),
        ("肆佰叁拾萬圆", 4300000.0, None, "元", "accurate"),
        ("二十五万三千二百泰铢", 253200.0, None, "泰铢", "accurate"),
        ("两个亿卢布", 200000000.0, None, "卢布", "accurate"),
        ("十块三毛", 10.30, None, "元", "accurate"),
        ("一百三十五块六角七分钱", 135.67, None, "元", "accurate"),
        ("港币两千九百六十元", 2960.0, None, "港元", "accurate"),
        ("一百二十三元1角1分", 123.11, None, "元", "accurate"),
        ("三万元欧元", 30000.0, None, "欧元", "accurate"),
        ("9000元日币", 9000.0, None, "日元", "accurate"),
        ("约4.287亿美元", 428700000.0, None, "美元", "blur"),
        ("近700万元", 7000000.0, None, "元", "blur-"),
        ("至少九千块钱以上", 9000.0, None, "元", "blur+"),
        ("不到1.9万台币", 19000.0, None, "新台币", "blur-"),
        ("小于40万", 400000.0, None, "元", "blur-"),
        ("3000多欧元", 3000.0, Some(4000.0), "欧元", "blur"),
        ("几十万块", 100000.0, Some(1000000.0), "元", "blur"),
        (
            "人民币数十亿元",
            1000000000.0,
            Some(10000000000.0),
            "元",
            "blur",
        ),
        (
            "数十亿元人民币",
            1000000000.0,
            Some(10000000000.0),
            "元",
            "blur",
        ),
        ("十几块钱", 10.0, Some(20.0), "元", "blur"),
        ("大约十多欧元", 10.0, Some(20.0), "欧元", "blur"),
        ("从8500到3万港元", 8500.0, Some(30000.0), "港元", "blur"),
        ("1万-5万元", 10000.0, Some(50000.0), "元", "blur"),
        ("1万元--5万元", 10000.0, Some(50000.0), "元", "blur"),
        ("10~15k元", 10000.0, Some(15000.0), "元", "blur"),
        ("2——3万港币", 20000.0, Some(30000.0), "港元", "blur"),
        ("两到三万港元", 20000.0, Some(30000.0), "港元", "blur"),
        ("十八至三十万日元", 180000.0, Some(300000.0), "日元", "blur"),
        ("两到三仟澳元", 2000.0, Some(3000.0), "澳元", "blur"),
        ("两~3百日元", 200.0, Some(300.0), "日元", "blur"),
        (
            "一百二十到一百五十万元",
            1200000.0,
            Some(1500000.0),
            "元",
            "blur",
        ),
        (
            "一千到两千万元人民币",
            10000000.0,
            Some(20000000.0),
            "元",
            "blur",
        ),
        (
            "七千到九千亿元",
            700000000000.0,
            Some(900000000000.0),
            "元",
            "blur",
        ),
        (
            "八到九百亿泰铢",
            800000000.0,
            Some(90000000000.0),
            "泰铢",
            "blur",
        ),
        (
            "八百到九百亿泰铢",
            80000000000.0,
            Some(90000000000.0),
            "泰铢",
            "blur",
        ),
        ("八九亿韩元", 800000000.0, Some(900000000.0), "韩元", "blur"),
        ("三五百块", 300.0, Some(500.0), "元", "blur"),
        ("四五千块钱", 4000.0, Some(5000.0), "元", "blur"),
        ("50万元（含）以上", 500000.0, None, "元", "blur+"),
        ("1万(含)-5万元", 10000.0, Some(50000.0), "元", "blur"),
    ];
    for (input, lo, hi, case, def) in cases {
        money_case_full(input, *lo, *hi, case, def);
    }
}

// ── extract_money ──

#[test]
fn parity_extract_money_from_text() {
    let text = "海航亏损7000万港元出售香港公寓。以2.6亿港元的价格出售";
    let r = jio::extract_money(text, false, false);
    let texts: Vec<&str> = r.iter().map(|e| e.text.as_str()).collect();
    assert!(
        texts.iter().any(|t| t.contains("7000万")),
        "expected 7000万 in {:?}",
        texts
    );
    assert!(
        texts.iter().any(|t| t.contains("2.6亿")),
        "expected 2.6亿 in {:?}",
        texts
    );
}
