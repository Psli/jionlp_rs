//! Python-Rust parity regression suite — location parser domain.
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

// ── recognize_location ──

#[test]
fn parity_recognize_location_finds_admin_units() {
    ensure_init();
    let r = jio::recognize_location("我出生在广东省广州市海珠区").unwrap();
    let names: Vec<&str> = r.iter().map(|m| m.name.as_str()).collect();
    assert!(names.contains(&"广东省"));
    assert!(names.contains(&"广州市"));
}

#[test]
fn parity_recognize_location_with_alias() {
    ensure_init();
    // 京 alias → 北京市 (Python: jionlp/rule/rule_pattern.py CHINA_PROVINCE_ALIAS)
    let r = jio::recognize_location("他籍贯京").unwrap();
    assert!(r.iter().any(|m| m.name == "北京市"));
}

// ── parse_location_full — round 21 cases ──
// Sourced from jionlp/test/test_location_parser.py.

#[test]
fn parity_parse_location_change2new_county() {
    ensure_init();
    // 柳州地区忻城县 was moved to 来宾市 — change2new=true remaps.
    let r = jio::parse_location_full("柳州地区忻城县", false, true).unwrap();
    assert_eq!(r.province.as_deref(), Some("广西壮族自治区"));
    assert_eq!(r.city.as_deref(), Some("来宾市"));
    assert_eq!(r.county.as_deref(), Some("忻城县"));
    assert_eq!(r.detail, "");
}

#[test]
fn parity_parse_location_change2new_city() {
    ensure_init();
    // 襄樊市 → 襄阳市 (city-level rename).
    let r = jio::parse_location_full("湖北省襄樊市小水街222号", false, true).unwrap();
    assert_eq!(r.province.as_deref(), Some("湖北省"));
    assert_eq!(r.city.as_deref(), Some("襄阳市"));
    assert_eq!(r.detail, "小水街222号");
}

#[test]
fn parity_parse_location_province_city() {
    ensure_init();
    let r = jio::parse_location_full("台湾省台北市", false, true).unwrap();
    assert_eq!(r.province.as_deref(), Some("台湾省"));
    assert_eq!(r.city.as_deref(), Some("台北市"));
    assert_eq!(r.county, None);
    assert_eq!(r.detail, "");
}

#[test]
fn parity_parse_location_province_abbrev() {
    ensure_init();
    // 青海 + 西宁 — the adjacent-offset guard rejects the bad 海西 match.
    let r = jio::parse_location_full("青海西宁", false, false).unwrap();
    assert_eq!(r.province.as_deref(), Some("青海省"));
    assert_eq!(r.city.as_deref(), Some("西宁市"));
    assert_eq!(r.county, None);
    assert_eq!(r.detail, "");
}

#[test]
fn parity_parse_location_alias_exception_suffix() {
    ensure_init();
    // 重庆路 should NOT trigger the 重庆 municipality alias.
    let r = jio::parse_location_full("北海市重庆路其仓11号", false, false).unwrap();
    assert_eq!(r.province.as_deref(), Some("广西壮族自治区"));
    assert_eq!(r.city.as_deref(), Some("北海市"));
    assert_eq!(r.detail, "重庆路其仓11号");
}

#[test]
fn parity_parse_location_autonomous_prefecture() {
    ensure_init();
    let r = jio::parse_location_full("海南藏族自治州", false, false).unwrap();
    assert_eq!(r.province.as_deref(), Some("青海省"));
    assert_eq!(r.city.as_deref(), Some("海南藏族自治州"));
}

#[test]
fn parity_parse_location_alias_ambiguity() {
    ensure_init();
    // 西安 should match 西安市 (city), not 西安区 in 吉林 (county).
    let r = jio::parse_location_full("西安交通大学", false, false).unwrap();
    assert_eq!(r.province.as_deref(), Some("陕西省"));
    assert_eq!(r.city.as_deref(), Some("西安市"));
    assert_eq!(r.detail, "交通大学");
}

#[test]
fn parity_parse_location_economic_zone() {
    ensure_init();
    // 经济技术开发区 normalized to just the suffix.
    let r = jio::parse_location_full("河北省秦皇岛市经济技术开发区", false, false).unwrap();
    assert_eq!(r.province.as_deref(), Some("河北省"));
    assert_eq!(r.city.as_deref(), Some("秦皇岛市"));
    assert_eq!(r.county.as_deref(), Some("经济技术开发区"));
    assert_eq!(r.detail, "");
}

#[test]
fn parity_parse_location_dedupe_city_county() {
    ensure_init();
    // 湖南省长沙市 — 长沙市 (city) full-name must win over 长沙 (county alias).
    let r = jio::parse_location_full("湖南省长沙市", false, false).unwrap();
    assert_eq!(r.province.as_deref(), Some("湖南省"));
    assert_eq!(r.city.as_deref(), Some("长沙市"));
    assert_eq!(r.county, None);
}

#[test]
fn parity_parse_location_full_address() {
    ensure_init();
    let r = jio::parse_location_full("山西长治潞州区山禾路2号", false, false).unwrap();
    assert_eq!(r.province.as_deref(), Some("山西省"));
    assert_eq!(r.city.as_deref(), Some("长治市"));
    assert_eq!(r.county.as_deref(), Some("潞州区"));
    assert_eq!(r.detail, "山禾路2号");
    assert_eq!(r.full_location, "山西省长治市潞州区山禾路2号");
    assert_eq!(r.orig_location, "山西长治潞州区山禾路2号");
}

#[test]
fn parity_parse_location_municipality() {
    ensure_init();
    let r = jio::parse_location_full("重庆解放碑", false, false).unwrap();
    assert_eq!(r.province.as_deref(), Some("重庆市"));
    assert_eq!(r.city.as_deref(), Some("重庆市"));
    assert_eq!(r.detail, "解放碑");
}

#[test]
fn parity_parse_location_nested() {
    ensure_init();
    let r = jio::parse_location_full("湖南湘潭市湘潭县城塘社区", false, false).unwrap();
    assert_eq!(r.province.as_deref(), Some("湖南省"));
    assert_eq!(r.city.as_deref(), Some("湘潭市"));
    assert_eq!(r.county.as_deref(), Some("湘潭县"));
    assert_eq!(r.detail, "城塘社区");
}

#[test]
fn parity_parse_location_no_match_preserves_text() {
    ensure_init();
    let r = jio::parse_location_full("hello world", false, false).unwrap();
    assert_eq!(r.province, None);
    assert_eq!(r.city, None);
    assert_eq!(r.county, None);
    assert_eq!(r.detail, "hello world");
    assert_eq!(r.full_location, "hello world");
    assert_eq!(r.orig_location, "hello world");
}

// ── alias helpers — round 22 ──

#[test]
fn parity_province_alias_full_names() {
    assert_eq!(jio::get_china_province_alias("山西省"), Some("山西"));
    assert_eq!(jio::get_china_province_alias("北京市"), Some("北京"));
    assert_eq!(
        jio::get_china_province_alias("新疆维吾尔自治区"),
        Some("新疆")
    );
}

#[test]
fn parity_city_alias() {
    assert_eq!(
        jio::get_china_city_alias("甘孜藏族自治州", false, false),
        Some("甘孜".to_string())
    );
    assert_eq!(
        jio::get_china_city_alias("锡林郭勒盟", false, false),
        Some("锡林郭勒".to_string())
    );
}

#[test]
fn parity_county_alias_qi() {
    // 科尔沁左翼后旗 → 科左后旗 via the Mongolian 旗 mapping.
    assert_eq!(
        jio::get_china_county_alias("科尔沁左翼后旗", false),
        Some("科左后旗".to_string())
    );
    // Generic 县 → drop 县.
    assert_eq!(
        jio::get_china_county_alias("忻城县", false),
        Some("忻城".to_string())
    );
}

#[test]
fn parity_town_alias() {
    assert_eq!(
        jio::get_china_town_alias("苏店镇"),
        Some("苏店".to_string())
    );
    assert_eq!(
        jio::get_china_town_alias("鼓楼街道"),
        Some("鼓楼".to_string())
    );
}

// ── town / village 5-level — round 34 ──

#[test]
fn parity_location_town_village() {
    ensure_init();
    // Python case: `老河口市天气` + town_village=true → town=None
    // (Python dict may have "老河口市" mapped under 湖北省襄阳市 with town
    // "城关镇" etc., but detail="天气" has no town substring, so town=None).
    let r = jio::parse_location_full("老河口市天气", true, true).unwrap();
    assert_eq!(r.province.as_deref(), Some("湖北省"));
    assert_eq!(r.county.as_deref(), Some("老河口市"));
    // Detail has no matching town — so both fields are None.
    assert_eq!(r.town, None);
    assert_eq!(r.village, None);
}

#[test]
fn parity_location_town_village_matched() {
    ensure_init();
    // Python test case:
    //   云南省红河哈尼族彝族自治州元阳县黄茅岭乡 → town=黄茅岭乡
    let r =
        jio::parse_location_full("云南省红河哈尼族彝族自治州元阳县黄茅岭乡", true, true).unwrap();
    assert_eq!(r.province.as_deref(), Some("云南省"));
    assert_eq!(r.city.as_deref(), Some("红河哈尼族彝族自治州"));
    assert_eq!(r.county.as_deref(), Some("元阳县"));
    assert_eq!(r.town.as_deref(), Some("黄茅岭乡"));
}

// ── all Python test_location_parser cases — round 37 ──

/// Fields we assert on for each Python location parity case. `None`
/// slots in `province/city/county` mean the Python expected-null entry;
/// `town`/`village` fields are only checked when town_village=true.
struct LocExpect<'a> {
    input: &'a str,
    town_village: bool,
    change2new: bool,
    province: Option<&'a str>,
    city: Option<&'a str>,
    county: Option<&'a str>,
    detail: &'a str,
    town: Option<&'a str>,
    village: Option<&'a str>,
}

fn loc_case(e: &LocExpect) {
    let r = jio::parse_location_full(e.input, e.town_village, e.change2new).unwrap();
    assert_eq!(
        r.province.as_deref(),
        e.province,
        "{} province: got {:?} want {:?}",
        e.input,
        r.province,
        e.province
    );
    assert_eq!(
        r.city.as_deref(),
        e.city,
        "{} city: got {:?} want {:?}",
        e.input,
        r.city,
        e.city
    );
    assert_eq!(
        r.county.as_deref(),
        e.county,
        "{} county: got {:?} want {:?}",
        e.input,
        r.county,
        e.county
    );
    assert_eq!(
        r.detail, e.detail,
        "{} detail: got {:?} want {:?}",
        e.input, r.detail, e.detail
    );
    if e.town_village {
        assert_eq!(
            r.town.as_deref(),
            e.town,
            "{} town: got {:?} want {:?}",
            e.input,
            r.town,
            e.town
        );
        assert_eq!(
            r.village.as_deref(),
            e.village,
            "{} village: got {:?} want {:?}",
            e.input,
            r.village,
            e.village
        );
    }
}

#[test]
fn parity_location_all_from_python() {
    ensure_init();
    use LocExpect as L;
    let cases: &[LocExpect] = &[
        L {
            input: "柳州地区忻城县",
            town_village: false,
            change2new: true,
            province: Some("广西壮族自治区"),
            city: Some("来宾市"),
            county: Some("忻城县"),
            detail: "",
            town: None,
            village: None,
        },
        L {
            input: "台湾省台北市",
            town_village: false,
            change2new: true,
            province: Some("台湾省"),
            city: Some("台北市"),
            county: None,
            detail: "",
            town: None,
            village: None,
        },
        L {
            input: "湖北省襄樊市小水街222号",
            town_village: false,
            change2new: true,
            province: Some("湖北省"),
            city: Some("襄阳市"),
            county: None,
            detail: "小水街222号",
            town: None,
            village: None,
        },
        L {
            input: "老河口市天气",
            town_village: true,
            change2new: true,
            province: Some("湖北省"),
            city: Some("襄阳市"),
            county: Some("老河口市"),
            detail: "天气",
            town: None,
            village: None,
        },
        L {
            input: "河北区",
            town_village: true,
            change2new: true,
            province: Some("天津市"),
            city: Some("天津市"),
            county: Some("河北区"),
            detail: "",
            town: None,
            village: None,
        },
        L {
            input: "湘潭城塘社区",
            town_village: true,
            change2new: true,
            province: Some("湖南省"),
            city: Some("湘潭市"),
            county: None,
            detail: "城塘社区",
            town: None,
            village: None,
        },
        L {
            input: "湘潭县城塘社区",
            town_village: true,
            change2new: true,
            province: Some("湖南省"),
            city: Some("湘潭市"),
            county: Some("湘潭县"),
            detail: "城塘社区",
            town: None,
            village: None,
        },
        L {
            input: "云南省红河哈尼族彝族自治州元阳县黄茅岭乡",
            town_village: true,
            change2new: true,
            province: Some("云南省"),
            city: Some("红河哈尼族彝族自治州"),
            county: Some("元阳县"),
            detail: "黄茅岭乡",
            town: Some("黄茅岭乡"),
            village: None,
        },
        L {
            input: "吉林省吉林市小皇村",
            town_village: true,
            change2new: true,
            province: Some("吉林省"),
            city: Some("吉林市"),
            county: None,
            detail: "小皇村",
            town: None,
            village: None,
        },
        L {
            input: "重庆解放碑",
            town_village: true,
            change2new: true,
            province: Some("重庆市"),
            city: Some("重庆市"),
            county: None,
            detail: "解放碑",
            town: None,
            village: None,
        },
        L {
            input: "湖南湘潭城塘社区",
            town_village: true,
            change2new: true,
            province: Some("湖南省"),
            city: Some("湘潭市"),
            county: None,
            detail: "城塘社区",
            town: None,
            village: None,
        },
        L {
            input: "湖南湘潭市湘潭县城塘社区",
            town_village: true,
            change2new: true,
            province: Some("湖南省"),
            city: Some("湘潭市"),
            county: Some("湘潭县"),
            detail: "城塘社区",
            town: None,
            village: None,
        },
        L {
            input: "西湖区蒋村花园小区管局农贸市场",
            town_village: true,
            change2new: true,
            province: None,
            city: None,
            county: Some("西湖区"),
            detail: "蒋村花园小区管局农贸市场",
            town: None,
            village: None,
        },
        L {
            input: "山西长治潞州区山禾路2号",
            town_village: false,
            change2new: false,
            province: Some("山西省"),
            city: Some("长治市"),
            county: Some("潞州区"),
            detail: "山禾路2号",
            town: None,
            village: None,
        },
        L {
            input: "青海西宁",
            town_village: false,
            change2new: false,
            province: Some("青海省"),
            city: Some("西宁市"),
            county: None,
            detail: "",
            town: None,
            village: None,
        },
        L {
            input: "东兴市北仑大道59号",
            town_village: true,
            change2new: false,
            province: Some("广西壮族自治区"),
            city: Some("防城港市"),
            county: Some("东兴市"),
            detail: "北仑大道59号",
            town: None,
            village: None,
        },
        L {
            input: "北海市重庆路其仓11号",
            town_village: true,
            change2new: false,
            province: Some("广西壮族自治区"),
            city: Some("北海市"),
            county: None,
            detail: "重庆路其仓11号",
            town: None,
            village: None,
        },
        L {
            input: "海南藏族自治州",
            town_village: true,
            change2new: false,
            province: Some("青海省"),
            city: Some("海南藏族自治州"),
            county: None,
            detail: "",
            town: None,
            village: None,
        },
        L {
            input: "西安交通大学",
            town_village: true,
            change2new: false,
            province: Some("陕西省"),
            city: Some("西安市"),
            county: None,
            detail: "交通大学",
            town: None,
            village: None,
        },
        L {
            input: "河北省秦皇岛市经济技术开发区",
            town_village: true,
            change2new: false,
            province: Some("河北省"),
            city: Some("秦皇岛市"),
            county: Some("经济技术开发区"),
            detail: "",
            town: None,
            village: None,
        },
        L {
            input: "江西南昌市新建区松湖镇江西省南昌市新建区松湖镇松湖中心小学",
            town_village: true,
            change2new: true,
            province: Some("江西省"),
            city: Some("南昌市"),
            county: Some("新建区"),
            detail: "松湖镇江西省南昌市新建区松湖镇松湖中心小学",
            town: Some("松湖镇"),
            village: None,
        },
        L {
            input: "湖南省长沙市",
            town_village: true,
            change2new: false,
            province: Some("湖南省"),
            city: Some("长沙市"),
            county: None,
            detail: "",
            town: None,
            village: None,
        },
        L {
            input: "香港九龙半岛清水湾香港科技大学",
            town_village: false,
            change2new: false,
            province: Some("香港特别行政区"),
            city: Some("香港"),
            county: Some("九龙城区"),
            detail: "半岛清水湾香港科技大学",
            town: None,
            village: None,
        },
        L {
            input: "莱芜",
            town_village: false,
            change2new: false,
            province: Some("山东省"),
            city: Some("济南市"),
            county: Some("莱芜区"),
            detail: "",
            town: None,
            village: None,
        },
        L {
            input: "成都市新津县金华镇清云北路2号（四川新津工业园）",
            town_village: false,
            change2new: false,
            province: Some("四川省"),
            city: Some("成都市"),
            county: Some("新津区"),
            detail: "金华镇清云北路2号（四川新津工业园）",
            town: None,
            village: None,
        },
        L {
            input: "青岛市市南区香港中路18号",
            town_village: false,
            change2new: false,
            province: Some("山东省"),
            city: Some("青岛市"),
            county: Some("市南区"),
            detail: "香港中路18号",
            town: None,
            village: None,
        },
        L {
            input: "石首市笔架山办事处建设路香港城西街",
            town_village: false,
            change2new: false,
            province: Some("湖北省"),
            city: Some("荆州市"),
            county: Some("石首市"),
            detail: "笔架山办事处建设路香港城西街",
            town: None,
            village: None,
        },
        L {
            input: "新疆巴音郭楞",
            town_village: false,
            change2new: false,
            province: Some("新疆维吾尔自治区"),
            city: Some("巴音郭楞蒙古自治州"),
            county: None,
            detail: "",
            town: None,
            village: None,
        },
        L {
            input: "内蒙古自治区通辽市科尔沁左翼后旗甘蓝子街98号",
            town_village: true,
            change2new: true,
            province: Some("内蒙古自治区"),
            city: Some("通辽市"),
            county: Some("科尔沁左翼后旗"),
            detail: "甘蓝子街98号",
            town: None,
            village: None,
        },
        L {
            input: "内蒙古自治区通辽市科尔沁甘蓝子街98号",
            town_village: true,
            change2new: true,
            province: Some("内蒙古自治区"),
            city: Some("通辽市"),
            county: Some("科尔沁区"),
            detail: "甘蓝子街98号",
            town: None,
            village: None,
        },
        L {
            input: "内蒙古通辽市科尔沁左翼后旗",
            town_village: true,
            change2new: true,
            province: Some("内蒙古自治区"),
            city: Some("通辽市"),
            county: Some("科尔沁左翼后旗"),
            detail: "",
            town: None,
            village: None,
        },
        L {
            input: "库尔勒市祥和镇",
            town_village: false,
            change2new: true,
            province: Some("新疆维吾尔自治区"),
            city: Some("巴音郭楞蒙古自治州"),
            county: Some("库尔勒市"),
            detail: "祥和镇",
            town: None,
            village: None,
        },
        L {
            input: "台湾省屏东县屏东市太原1路84号",
            town_village: false,
            change2new: true,
            province: Some("台湾省"),
            city: None,
            county: Some("屏东县"),
            detail: "屏东市太原1路84号",
            town: None,
            village: None,
        },
    ];
    for c in cases {
        loc_case(c);
    }
}
