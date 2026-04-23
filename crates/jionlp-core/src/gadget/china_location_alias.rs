//! Port of `jionlp/gadget/china_location_alias.py` — compute the short-form
//! nickname (简称) for a given Chinese administrative-division name.
//!
//! Four levels: province / city / county / town. The Rust API mirrors
//! Python's four standalone methods but exposes them as free functions.

use regex::Regex;
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;

/// 省级 简称 pattern — matches any of the 34 province/municipality/SAR
/// short-name prefixes. Taken verbatim from Python's
/// `CHINA_PROVINCE_SHORT_PATTERN`.
static PROVINCE_SHORT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(香港|澳门|台湾|北京|天津|上海|重庆|黑龙江|吉林|辽宁|新疆|西藏|青海|内蒙古|山西|河北|河南|甘肃|陕西|四川|贵州|云南|宁夏|江苏|浙江|安徽|山东|江西|湖北|湖南|广东|福建|广西|海南)",
    )
    .unwrap()
});

/// 2+ 字 民族名 (with trailing 族).
static MINORITY_1: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(壮|满|回|苗|维吾尔|土家|彝|蒙古|藏|布依|侗|瑶|朝鲜|白|哈尼|哈萨克|黎|傣|畲|傈僳|仡佬|东乡|高山|拉祜|水|佤|纳西|羌|土|仫佬|锡伯|柯尔克孜|达斡尔|景颇|毛南|撒拉|布朗|塔吉克|阿昌|普米|鄂温克|怒|京|基诺|德昂|保安|俄罗斯|裕固|乌兹别克|门巴|鄂伦春|独龙|塔塔尔|赫哲|珞巴)族",
    )
    .unwrap()
});

/// 2+ 字 民族名 (without 族).
static MINORITY_2: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(维吾尔|土家|蒙古|布依|朝鲜|哈尼|哈萨克|傈僳|仡佬|东乡|高山|拉祜|纳西|仫佬|锡伯|柯尔克孜|达斡尔|景颇|毛南|撒拉|布朗|塔吉克|阿昌|普米|鄂温克|基诺|德昂|保安|俄罗斯|裕固|乌兹别克|门巴|鄂伦春|独龙|塔塔尔|赫哲|珞巴)",
    )
    .unwrap()
});

/// Return the province short-name (e.g. `山西省 → 山西`), or `None` when the
/// given text does not contain any recognized province name.
pub fn get_china_province_alias(name: &str) -> Option<&str> {
    let m = PROVINCE_SHORT.find(name)?;
    Some(&name[m.start()..m.end()])
}

/// Return the city short-name — handles 市/地区/盟/自治州.
pub fn get_china_city_alias(
    name: &str,
    dismiss_diqu: bool,
    dismiss_meng: bool,
) -> Option<String> {
    if let Some(rest) = name.strip_suffix('市') {
        return Some(rest.to_string());
    }
    if !dismiss_diqu {
        if let Some(rest) = name.strip_suffix("地区") {
            return Some(rest.to_string());
        }
    }
    if !dismiss_meng {
        if let Some(rest) = name.strip_suffix('盟') {
            return Some(rest.to_string());
        }
    }
    // 自治州 (with 族).
    if let Some(m) = MINORITY_1.find(name) {
        return Some(name[..m.start()].to_string());
    }
    if let Some(m) = MINORITY_2.find(name) {
        return Some(name[..m.start()].to_string());
    }
    None
}

/// Internal map for Mongolian 旗 shortenings — copied verbatim from
/// Python reference (`_prepare_qi_alias_map`).
static QI_MAP: Lazy<FxHashMap<&'static str, &'static str>> = Lazy::new(|| {
    let pairs: &[(&str, &str)] = &[
        ("土默特右旗", "土右旗"), ("土默特左旗", "土左旗"),
        ("杭锦后旗", "杭后旗"), ("杭锦旗", "杭旗"),
        ("乌拉特后旗", "乌后旗"), ("乌拉特中旗", "乌中旗"),
        ("阿鲁科尔沁旗", "阿旗"),
        ("敖汉旗", "敖旗"),
        ("巴林右旗", "巴右旗"), ("巴林左旗", "巴左旗"),
        ("喀喇沁旗", "喀旗"),
        ("克什克腾旗", "克旗"),
        ("翁牛特旗", "翁旗"),
        ("达拉特旗", "达旗"),
        ("鄂托克旗", "鄂旗"), ("鄂托克前旗", "鄂前旗"),
        ("乌审旗", "乌旗"),
        ("伊金霍洛旗", "伊旗"),
        ("准格尔旗", "准旗"),
        ("阿荣旗", "阿荣旗"),
        ("陈巴尔虎旗", "陈旗"),
        ("鄂伦春自治旗", "鄂伦春"), ("鄂温克族自治旗", "鄂温克"),
        ("莫力达瓦达斡尔族自治旗", "莫旗"),
        ("新巴尔虎右旗", "新右旗"), ("新巴尔虎左旗", "新左旗"),
        ("科尔沁左翼后旗", "科左后旗"), ("科尔沁左翼中旗", "科左中旗"),
        ("科尔沁右翼前旗", "科右前旗"), ("科尔沁右翼中旗", "科右中旗"),
        ("库伦旗", "库旗"), ("奈曼旗", "奈旗"),
        ("扎鲁特旗", "扎旗"), ("扎赉特旗", "扎赉特旗"),
        ("察哈尔右翼后旗", "察右后旗"), ("察哈尔右翼前旗", "察右前旗"),
        ("察哈尔右翼中旗", "察右中旗"),
        ("四子王旗", "四子王旗"), ("阿巴嘎旗", "阿巴嘎旗"),
        ("东乌珠穆沁旗", "东乌旗"), ("西乌珠穆沁旗", "西乌旗"),
        ("苏尼特右旗", "苏右旗"), ("苏尼特左旗", "苏左旗"),
        ("太仆寺旗", "太仆寺旗"),
        ("镶黄旗", "镶黄旗"), ("正蓝旗", "正蓝旗"), ("正镶白旗", "正镶白旗"),
        ("阿拉善右旗", "阿右旗"), ("阿拉善左旗", "阿左旗"),
        ("额济纳旗", "额旗"),
        ("达尔罕茂明安联合旗", "达旗"),
    ];
    pairs.iter().copied().collect()
});

/// Return the county short-name (县/区/旗/林区/自治县).
pub fn get_china_county_alias(name: &str, dismiss_qi: bool) -> Option<String> {
    let char_len = name.chars().count();
    if let Some(rest) = name.strip_suffix('县') {
        if char_len == 2 {
            return Some(name.to_string());
        }
        return Some(rest.to_string());
    }
    if let Some(rest) = name.strip_suffix("林区") {
        return Some(rest.to_string());
    }
    if let Some(rest) = name.strip_suffix('区') {
        if char_len == 2 {
            return Some(name.to_string());
        }
        return Some(rest.to_string());
    }
    if let Some(rest) = name.strip_suffix('市') {
        return Some(rest.to_string());
    }
    if !dismiss_qi && name.ends_with('旗') {
        return Some(
            QI_MAP.get(name).copied().unwrap_or(name).to_string(),
        );
    }
    if let Some(m) = MINORITY_1.find(name) {
        return Some(name[..m.start()].to_string());
    }
    if let Some(m) = MINORITY_2.find(name) {
        return Some(name[..m.start()].to_string());
    }
    None
}

/// Return the town short-name (镇/乡/街道/地区).
pub fn get_china_town_alias(name: &str) -> Option<String> {
    let char_len = name.chars().count();
    if let Some(rest) = name.strip_suffix('镇') {
        if char_len == 2 {
            return Some(name.to_string());
        }
        return Some(rest.to_string());
    }
    if let Some(rest) = name.strip_suffix('乡') {
        if char_len == 2 {
            return Some(name.to_string());
        }
        return Some(rest.to_string());
    }
    if let Some(rest) = name.strip_suffix("地区") {
        return Some(rest.to_string());
    }
    if let Some(rest) = name.strip_suffix("街道") {
        return Some(rest.to_string());
    }
    if let Some(m) = MINORITY_1.find(name) {
        return Some(name[..m.start()].to_string());
    }
    if let Some(m) = MINORITY_2.find(name) {
        return Some(name[..m.start()].to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn province_short() {
        assert_eq!(get_china_province_alias("山西省"), Some("山西"));
        assert_eq!(get_china_province_alias("北京市"), Some("北京"));
        assert_eq!(get_china_province_alias("内蒙古自治区"), Some("内蒙古"));
        assert_eq!(get_china_province_alias("新疆维吾尔自治区"), Some("新疆"));
        assert_eq!(get_china_province_alias("XYZ"), None);
    }

    #[test]
    fn city_short_shi_and_diqu() {
        assert_eq!(get_china_city_alias("仙桃市", false, false), Some("仙桃".to_string()));
        assert_eq!(get_china_city_alias("柳州地区", false, false), Some("柳州".to_string()));
    }

    #[test]
    fn city_short_autonomous() {
        assert_eq!(
            get_china_city_alias("甘孜藏族自治州", false, false),
            Some("甘孜".to_string())
        );
    }

    #[test]
    fn county_short_qi_mapping() {
        assert_eq!(
            get_china_county_alias("科尔沁左翼后旗", false),
            Some("科左后旗".to_string())
        );
    }

    #[test]
    fn town_short_jie() {
        assert_eq!(get_china_town_alias("苏店镇"), Some("苏店".to_string()));
        assert_eq!(get_china_town_alias("鼓楼街道"), Some("鼓楼".to_string()));
    }
}
