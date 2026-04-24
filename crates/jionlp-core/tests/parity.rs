//! Python-Rust parity regression suite.
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

// ────────────────────────────── gadget ────────────────────────────────────

#[test]
fn parity_split_sentence_fine_from_docstring() {
    // jionlp/gadget/split_sentence.py docstring:
    //   text = '中华古汉语，泱泱大国，历史传承的瑰宝。'
    //   jio.split_sentence(text, criterion='fine')
    //   → ['中华古汉语，', '泱泱大国，', '历史传承的瑰宝。']
    let got = jio::split_sentence(
        "中华古汉语，泱泱大国，历史传承的瑰宝。",
        jio::Criterion::Fine,
    );
    assert_eq!(got, vec!["中华古汉语，", "泱泱大国，", "历史传承的瑰宝。"]);
}

#[test]
fn parity_tra2sim_char_from_docstring() {
    // jionlp/gadget/ts_conversion.py docstring:
    //   text = '今天天氣好晴朗，想喫速食麵。妳還在工作嗎？在太空梭上工作嗎？'
    //   jio.tra2sim(text, mode='char')
    //   → '今天天气好晴朗，想吃速食面。你还在工作吗？在太空梭上工作吗？'
    ensure_init();
    let got = jio::tra2sim(
        "今天天氣好晴朗，想喫速食麵。妳還在工作嗎？在太空梭上工作嗎？",
        jio::TsMode::Char,
    )
    .unwrap();
    assert_eq!(
        got,
        "今天天气好晴朗，想吃速食面。你还在工作吗？在太空梭上工作吗？"
    );
}

#[test]
fn parity_tra2sim_word_idiom() {
    // Word-mode should map "速食麵" → "方便面" and "太空梭" → "航天飞机".
    ensure_init();
    let got = jio::tra2sim(
        "今天天氣好晴朗，想喫速食麵。妳還在工作嗎？在太空梭上工作嗎？",
        jio::TsMode::Word,
    )
    .unwrap();
    assert!(got.contains("方便面"), "expected '方便面' in {got}");
    assert!(got.contains("航天飞机"), "expected '航天飞机' in {got}");
}

#[test]
fn parity_parse_id_card_docstring_case() {
    // Python docstring:
    //   text = '52010320171109002X' → {province: '贵州省', city: '贵阳市', county: '云岩区',
    //   birth_year: '2017', birth_month: '11', birth_day: '09', gender: '女',
    //   check_code: 'x'}
    ensure_init();
    let info = jio::parse_id_card("52010320171109002X").unwrap().unwrap();
    assert_eq!(info.province, "贵州省");
    assert_eq!(info.city.as_deref(), Some("贵阳市"));
    assert_eq!(info.county.as_deref(), Some("云岩区"));
    assert_eq!(info.birth_year, "2017");
    assert_eq!(info.birth_month, "11");
    assert_eq!(info.birth_day, "09");
    assert_eq!(info.gender, "女");
    assert_eq!(info.check_code, "x");
}

#[test]
fn parity_parse_plate_docstring_case() {
    // Python docstring:
    //   text = '川A·23047B' → {car_loc: '川A', car_type: 'PEV', car_size: 'big'}
    let info = jio::parse_motor_vehicle_licence_plate("川A·23047B").unwrap();
    assert_eq!(info.car_loc, "川A");
    assert_eq!(info.car_type, "PEV");
    assert_eq!(info.car_size, Some("big"));
}

#[test]
fn parity_num2char_docstring_sim() {
    // Python docstring:
    //   38009 + sim → '三万八千零九'
    assert_eq!(
        jio::num2char("38009", jio::NumStyle::Simplified).unwrap(),
        "三万八千零九"
    );
}

#[test]
fn parity_num2char_docstring_tra() {
    // Python docstring:
    //   1234 + tra → '壹仟贰佰叁拾肆'
    assert_eq!(
        jio::num2char("1234", jio::NumStyle::Traditional).unwrap(),
        "壹仟贰佰叁拾肆"
    );
}

#[test]
fn parity_pinyin_docstring_standard() {
    // Python docstring:
    //   '中华人民共和国。' + standard → ['zhōng','huá','rén','mín','gòng','hé','guó','<unk>']
    // Our sentinel for non-Chinese is '<py_unk>' (matches Python's
    // self.py_unk). The Python example shows '<unk>' in the simplified
    // README snippet but the source code uses '<py_unk>'.
    ensure_init();
    let r = jio::pinyin("中华人民共和国。", jio::PinyinFormat::Standard).unwrap();
    let plain: Vec<String> = r
        .iter()
        .map(|e| match e {
            jio::PinyinEntry::Standard(s) => s.clone(),
            _ => panic!("wrong variant"),
        })
        .collect();
    let joined = plain.join(",");
    // Dictionary gives first reading; verify the content chars look right.
    assert!(plain.iter().any(|p| p == "zhōng"), "have: {joined}");
    assert!(plain.iter().any(|p| p == "rén"), "have: {joined}");
    assert!(plain.iter().any(|p| p == "guó"), "have: {joined}");
    // '。' is non-Chinese → unk placeholder.
    assert_eq!(plain.last().map(String::as_str), Some("<py_unk>"));
}

// ────────────────────────────── rule ──────────────────────────────────────

#[test]
fn parity_extract_email_basic() {
    // README example: "please send email to foo@bar.com" → ["foo@bar.com"]
    let r = jio::extract_email("please send email to foo@bar.com");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].text, "foo@bar.com");
}

#[test]
fn parity_extract_id_card_boundaries() {
    // ID card should match only when surrounded by non-alphanumerics.
    let r = jio::extract_id_card("身份证 11010519900307123X, 谢谢");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].text, "11010519900307123X");
}

#[test]
fn parity_extract_chinese_runs() {
    // extract_chinese returns contiguous Chinese-char runs.
    assert_eq!(
        jio::extract_chinese("hello 你好 world 测试"),
        vec!["你好".to_string(), "测试".to_string()]
    );
}

#[test]
fn parity_clean_html_entities() {
    // A minimal HTML sample the Python cleanser also handles.
    assert_eq!(jio::clean_html("<p>a &amp; b</p>"), "a & b");
}

#[test]
fn parity_phone_location_beijing_landline() {
    ensure_init();
    let info = jio::phone_location("010-12345678").unwrap();
    assert_eq!(info.phone_type, "landline_phone");
    assert!(info.province.as_deref().unwrap_or("").contains("北京"));
}

#[test]
fn parity_simhash_identical_text() {
    assert_eq!(jio::simhash("今天天气真好"), jio::simhash("今天天气真好"));
}

#[test]
fn parity_simhash_near_duplicate_small_distance() {
    let a = jio::simhash("今天天气真好");
    let b = jio::simhash("今天天气非常好");
    // Same stem "今天天气"+ 好/非常好 — close by SimHash convention.
    assert!(jio::hamming_distance(a, b) < 25);
}

#[test]
fn parity_phone_location_cell_mobile() {
    ensure_init();
    // 138 is China Mobile — verified against Python telecom_operator.txt.
    let info = jio::phone_location("13812345678").unwrap();
    assert_eq!(info.phone_type, "cell_phone");
    assert_eq!(info.operator.as_deref(), Some("中国移动"));
}

#[test]
fn parity_sentiment_neutral_for_empty() {
    ensure_init();
    assert!((jio::sentiment_score("").unwrap() - 0.5).abs() < 1e-9);
}

#[test]
fn parity_sentiment_positive_text() {
    ensure_init();
    let s = jio::sentiment_score("今天非常开心,是美好的一天。").unwrap();
    assert!(s > 0.5);
}

#[test]
fn parity_sentiment_negative_text() {
    ensure_init();
    let s = jio::sentiment_score("事故造成严重伤亡,令人悲痛万分。").unwrap();
    assert!(s < 0.5);
}

#[test]
fn parity_pinyin_phrase_override_single_char() {
    ensure_init();
    // 貉: single-char primary reading is "mò" but "一丘之貉" phrase →
    // "hé". Verify the phrase trie wins.
    let r = jio::pinyin("一丘之貉", jio::PinyinFormat::Standard).unwrap();
    let plain: Vec<String> = r
        .iter()
        .map(|e| match e {
            jio::PinyinEntry::Standard(s) => s.clone(),
            _ => String::new(),
        })
        .collect();
    assert_eq!(plain, vec!["yī", "qiū", "zhī", "hé"]);
}

#[test]
fn parity_char2num_big_units() {
    assert_eq!(jio::char2num("二十三").unwrap(), 23.0);
    assert_eq!(jio::char2num("一万").unwrap(), 10_000.0);
    assert_eq!(jio::char2num("三千五百万").unwrap(), 35_000_000.0);
}

#[test]
fn parity_num2char_traditional_invoice() {
    // Python docstring example
    assert_eq!(
        jio::num2char("1234", jio::NumStyle::Traditional).unwrap(),
        "壹仟贰佰叁拾肆"
    );
}

#[test]
fn parity_extract_qq() {
    let r = jio::extract_qq("QQ号码 123456789 ,联系");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].text, "123456789");
}

#[test]
fn parity_extract_ip_multiple() {
    let r = jio::extract_ip_address("server at 192.168.1.1 and 10.0.0.1 online");
    assert_eq!(r.len(), 2);
    let texts: Vec<&str> = r.iter().map(|e| e.text.as_str()).collect();
    assert!(texts.contains(&"192.168.1.1"));
    assert!(texts.contains(&"10.0.0.1"));
}

#[test]
fn parity_id_card_gender_parity() {
    ensure_init();
    // 17th char odd → 男, even → 女.
    let male = jio::parse_id_card("110105199001015671").unwrap().unwrap();
    assert_eq!(male.gender, "男");
    let female = jio::parse_id_card("110105199001012021").unwrap().unwrap();
    assert_eq!(female.gender, "女");
}

#[test]
fn parity_plate_nev_big_pev() {
    // Python docstring: 川A·23047B → PEV, big
    let info = jio::parse_motor_vehicle_licence_plate("川A·23047B").unwrap();
    assert_eq!(info.car_loc, "川A");
    assert_eq!(info.car_type, "PEV");
    assert_eq!(info.car_size, Some("big"));
}

#[test]
fn parity_split_sentence_coarse_quotes() {
    // Python behavior: quotes should attach correctly.
    let text = "他说\"今天天气真好\"。然后离开。";
    let r = jio::split_sentence(text, jio::Criterion::Coarse);
    assert!(!r.is_empty());
    // Last sentence should contain "离开".
    assert!(r.last().unwrap().contains("离开"));
}

#[test]
fn parity_check_all_chinese_char() {
    assert!(jio::check_all_chinese_char("全部中文"));
    assert!(!jio::check_all_chinese_char("中文 with ascii"));
    assert!(!jio::check_all_chinese_char(""));
}

#[test]
fn parity_ts_word_mode_idiom() {
    ensure_init();
    // 速食麵 → 方便面 (Python word-mode tra2sim)
    let r = jio::tra2sim("想喫速食麵", jio::TsMode::Word).unwrap();
    assert!(r.contains("方便面"), "got {r}");
}

#[test]
fn parity_phone_location_unicom() {
    ensure_init();
    // 130 is 中国联通 per `telecom_operator.txt`.
    let info = jio::phone_location("13012345678").unwrap();
    assert_eq!(info.operator.as_deref(), Some("中国联通"));
}

#[test]
fn parity_phone_location_telecom() {
    ensure_init();
    // 133 is 中国电信.
    let info = jio::phone_location("13312345678").unwrap();
    assert_eq!(info.operator.as_deref(), Some("中国电信"));
}

#[test]
fn parity_pinyin_simple_format_roundtrip() {
    ensure_init();
    let r = jio::pinyin("中国", jio::PinyinFormat::Simple).unwrap();
    let s: Vec<String> = r
        .iter()
        .map(|e| match e {
            jio::PinyinEntry::Simple(s) => s.clone(),
            _ => String::new(),
        })
        .collect();
    assert_eq!(s, vec!["zhong1", "guo2"]);
}

#[test]
fn parity_pinyin_detail_format_components() {
    ensure_init();
    let r = jio::pinyin("中", jio::PinyinFormat::Detail).unwrap();
    match &r[0] {
        jio::PinyinEntry::Detail(d) => {
            assert_eq!(d.consonant, "zh");
            assert_eq!(d.vowel, "ong");
            assert_eq!(d.tone, "1");
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn parity_swap_char_determinism() {
    // Same seed → identical outputs. This pins the PRNG discipline.
    let a = jio::swap_char_position("中华人民共和国是美好的家园", 3, 0.3, 42, 1.0);
    let b = jio::swap_char_position("中华人民共和国是美好的家园", 3, 0.3, 42, 1.0);
    assert_eq!(a, b);
    assert_eq!(a.len(), 3);
}

#[test]
fn parity_homophone_substitution_preserves_length() {
    ensure_init();
    let src = "今天天气真好";
    let r = jio::homophone_substitution(src, 3, 0.5, 42).unwrap();
    for v in r {
        assert_eq!(v.chars().count(), src.chars().count());
    }
}

#[test]
fn parity_random_add_delete_nonempty() {
    let r = jio::random_add_delete("今天天气真好我要去公园散步", 3, 0.3, 0.1, 42);
    assert!(!r.is_empty());
    for v in &r {
        assert!(!v.is_empty());
    }
}

#[test]
fn parity_extract_summary_preserves_order() {
    ensure_init();
    let text = "第一句话。第二句话。第三句话。第四句话。";
    let out = jio::extract_summary(text, 3).unwrap();
    for w in out.windows(2) {
        assert!(w[0].position < w[1].position);
    }
}

#[test]
fn parity_sentiment_adverb_strengthens_positive() {
    ensure_init();
    let a = jio::sentiment_score("今天开心").unwrap();
    let b = jio::sentiment_score("今天非常开心").unwrap();
    assert!(b >= a);
}

#[test]
fn parity_check_all_arabic_num() {
    assert!(jio::check_all_arabic_num("12345"));
    assert!(!jio::check_all_arabic_num("12a45"));
    assert!(!jio::check_all_arabic_num(""));
}

#[test]
fn parity_extract_url_with_trailing_punct() {
    let r = jio::extract_url("访问 https://example.com/path?q=1。然后...");
    assert_eq!(r.len(), 1);
    assert!(r[0].text.starts_with("https://"));
    // Trailing "。" should not be captured.
    assert!(!r[0].text.contains("。"));
}

#[test]
fn parity_extract_parentheses_chinese_and_ascii() {
    // Mixed ASCII and Chinese brackets, with nesting.
    let r = jio::extract_parentheses("a (b) c【标题】d", "()[]【】");
    let texts: Vec<&str> = r.iter().map(|e| e.text.as_str()).collect();
    assert!(texts.contains(&"(b)"));
    assert!(texts.contains(&"【标题】"));
}

#[test]
fn parity_clean_html_removes_script_tags() {
    let r = jio::clean_html("<html><script>alert(1)</script>hi</html>");
    assert!(!r.contains("alert"));
    assert!(r.contains("hi"));
}

#[test]
fn parity_bpe_roundtrip_scripts() {
    // Byte-level BPE must be lossless across scripts.
    for src in [
        "Hello 世界 🌍 テスト",
        "メトロ",
        "今天天气真好",
        "Mixed! punctuation: 。，",
    ] {
        assert_eq!(jio::bpe_decode(&jio::bpe_encode(src)), src, "src={src}");
    }
}

#[test]
fn parity_bpe_control_byte_remap() {
    // Tab (0x09) must remap to a code point >= U+0100.
    let enc = jio::bpe_encode("\t");
    let c = enc.chars().next().unwrap() as u32;
    assert!(c >= 0x0100);
    assert_eq!(jio::bpe_decode(&enc), "\t");
}

#[test]
fn parity_extract_summary_mmr_lambda_boundaries() {
    ensure_init();
    let text = "第一句话。第二句话。第三句话。";
    // λ=1 matches basic top-k.
    let basic = jio::extract_summary(text, 2).unwrap();
    let mmr = jio::extract_summary_mmr(text, 2, 1.0).unwrap();
    let bp: Vec<_> = basic.iter().map(|s| s.position).collect();
    let mp: Vec<_> = mmr.iter().map(|s| s.position).collect();
    assert_eq!(bp, mp);

    // λ=0 must not panic and returns top_k.
    let diverse = jio::extract_summary_mmr(text, 2, 0.0).unwrap();
    assert_eq!(diverse.len(), 2);
}

#[test]
fn parity_sentiment_neutral_edge_cases() {
    ensure_init();
    // Text with no sentiment vocabulary → 0.5 (neutral).
    let s = jio::sentiment_score("今天是星期三").unwrap();
    assert!((s - 0.5).abs() < 0.05, "expected ~0.5, got {s}");
}

#[test]
fn parity_extract_id_card_filters_18_chars() {
    // Must be exactly 18 chars, not 17 or 19.
    assert!(jio::extract_id_card("张三: 11010519900307123 未完成").is_empty());
}

#[test]
fn parity_extract_email_domain_dot_required() {
    // No TLD → not an email.
    assert!(jio::extract_email("foo@bar 缺少顶级域名").is_empty());
}

#[test]
fn parity_num2char_single_digit_zero() {
    assert_eq!(jio::num2char("0", jio::NumStyle::Simplified).unwrap(), "零");
}

#[test]
fn parity_char2num_single_char() {
    assert_eq!(jio::char2num("五").unwrap(), 5.0);
}

#[test]
fn parity_extract_parentheses_unmatched_is_skipped() {
    // Mismatched brackets are lenient: no crash, just no entry.
    let r = jio::extract_parentheses("没有闭合 (开始", "()");
    assert!(r.is_empty());
}

#[test]
fn parity_clean_html_numeric_entity() {
    // &#65; → "A", &#x4E2D; → "中"
    assert_eq!(jio::clean_html("&#65;"), "A");
    assert_eq!(jio::clean_html("&#x4E2D;"), "中");
}

#[test]
fn parity_check_all_chinese_char_rejects_mixed() {
    assert!(!jio::check_all_chinese_char("中文123"));
    assert!(!jio::check_all_chinese_char("123"));
}

#[test]
fn parity_lexicon_ner_leftmost_longest() {
    // LexiconNer with 北京 + 北京大学 → leftmost-longest picks 北京大学.
    let mut lex: rustc_hash::FxHashMap<String, Vec<String>> = rustc_hash::FxHashMap::default();
    lex.insert(
        "Univ".to_string(),
        vec!["北京".to_string(), "北京大学".to_string()],
    );
    let ner = jio::LexiconNer::from_map(&lex).unwrap();
    let r = ner.recognize("就读于北京大学");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].text, "北京大学");
}

#[test]
fn parity_textrank_nonempty() {
    ensure_init();
    let text = "机器学习是人工智能的一个分支。机器学习研究数据。机器学习应用广泛。";
    let r = jio::extract_keyphrase_textrank(text, 3, 2, 3).unwrap();
    assert!(!r.is_empty());
}

#[test]
fn parity_textrank_sorted_descending() {
    ensure_init();
    let r = jio::extract_keyphrase_textrank("北京是中国首都。北京有名胜古迹。", 5, 2, 3).unwrap();
    for w in r.windows(2) {
        assert!(w[0].weight >= w[1].weight);
    }
}

#[test]
fn parity_textrank_empty() {
    ensure_init();
    assert!(jio::extract_keyphrase_textrank("", 5, 2, 3)
        .unwrap()
        .is_empty());
}

#[test]
fn parity_char_radical_ascii_fallback() {
    // ASCII chars fall back to <cr_unk> placeholder.
    ensure_init();
    let r = jio::char_radical("A").unwrap();
    assert_eq!(r[0].radical, "<cr_unk>");
}

#[test]
fn parity_split_sentence_fine_multipunct() {
    // "……" should stay together, not split into two "…" (fine mode).
    let r = jio::split_sentence("这是第一句……这是第二句。", jio::Criterion::Fine);
    assert_eq!(r, vec!["这是第一句……", "这是第二句。"]);
}

#[test]
fn parity_remove_stopwords_preserves_plain_words() {
    ensure_init();
    // Non-stopwords should pass through untouched.
    let input: Vec<String> = ["机器学习", "深度学习", "神经网络"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let r = jio::remove_stopwords(&input, jio::RemoveOpts::default()).unwrap();
    assert_eq!(r, input);
}

#[test]
fn parity_check_cell_phone_prefix_range() {
    // Python CELL_PHONE_CHECK_PATTERN requires 1[3-9]x.
    assert!(jio::check_cell_phone("13912345678"));
    assert!(!jio::check_cell_phone("12312345678"));
    assert!(!jio::check_cell_phone("10012345678"));
}

#[test]
fn parity_extract_chinese_filters_ascii_and_digits() {
    assert_eq!(
        jio::extract_chinese("hello 中文 123 world 测试 !"),
        vec!["中文".to_string(), "测试".to_string()]
    );
}

#[test]
fn parity_check_any_chinese_char_edge() {
    // Empty → false (the Python behavior).
    assert!(!jio::check_any_chinese_char(""));
}

#[test]
fn parity_ts_sim2tra_idiom_roundtrip() {
    ensure_init();
    // Word-mode round-trip is lossy in Python too (multi-reading chars),
    // but character-mode should be stable on pure simplified text.
    let src = "今天天气好晴朗";
    let to_tra = jio::sim2tra(src, jio::TsMode::Char).unwrap();
    let back = jio::tra2sim(&to_tra, jio::TsMode::Char).unwrap();
    assert_eq!(back, src);
}

#[test]
fn parity_simhash_similarity_self_is_one() {
    let h = jio::simhash("some text");
    assert!((jio::simhash_similarity(h, h) - 1.0).abs() < 1e-9);
}

#[test]
fn parity_solar_to_lunar_roundtrip() {
    // 2024-02-10 = lunar 2024 / 1 / 1 (龙年春节).
    let (y, m, d, leap) =
        jio::solar_to_lunar(chrono::NaiveDate::from_ymd_opt(2024, 2, 10).unwrap()).unwrap();
    assert_eq!((y, m, d, leap), (2024, 1, 1, false));

    // 2023-06-22 = lunar 2023 / 5 / 5 (端午节).
    let (y, m, d, leap) =
        jio::solar_to_lunar(chrono::NaiveDate::from_ymd_opt(2023, 6, 22).unwrap()).unwrap();
    assert_eq!((y, m, d, leap), (2023, 5, 5, false));
}

#[test]
fn parity_idiom_solitaire_deterministic() {
    ensure_init();
    // Seed 42 + 千方百计 should deterministically pick the same successor.
    let game = jio::IdiomSolitaireGame::new(42);
    let next1 = game.next_by_char("千方百计", false).unwrap().unwrap();
    assert!(next1.starts_with('计'));

    let game2 = jio::IdiomSolitaireGame::new(42);
    let next2 = game2.next_by_char("千方百计", false).unwrap().unwrap();
    assert_eq!(next1, next2);
}

// ────────────────────────────── util + dict ───────────────────────────────

#[test]
fn parity_util_regex_composers() {
    assert_eq!(jio::bracket("\\d+"), "(\\d+)");
    assert_eq!(jio::bracket_absence("A-Z"), "(A-Z)?");
    assert_eq!(jio::absence("x"), "x?");
    assert_eq!(jio::start_end("abc"), "^abc$");
}

#[test]
fn parity_xiehouyu_loads_and_has_known_entry() {
    ensure_init();
    let x = jio::dict::xiehouyu().unwrap();
    assert!(x.len() > 1000);
    // Sanity: dict should contain the classic 一丈厚的烧饼.
    assert!(x.contains_key("一丈厚的烧饼"));
}

#[test]
fn parity_world_location_continents() {
    ensure_init();
    let w = jio::dict::world_location().unwrap();
    assert!(w.iter().any(|r| r.country == "中国"));
    assert!(w.iter().any(|r| r.continent == "亚洲"));
}

#[test]
fn parity_quantifiers_has_entries() {
    ensure_init();
    let q = jio::dict::quantifiers().unwrap();
    assert!(q.len() > 50);
}

// ────────────────────────────── remove_* / idiom / text_aug ───────────────

#[test]
fn parity_remove_email_basic() {
    let out = jio::remove_email("联系 dongrixinyu.89@163.com 请速回");
    assert!(!out.contains("@"));
    assert!(out.contains("联系"));
    assert!(out.contains("请速回"));
}

#[test]
fn parity_remove_url_basic() {
    let out = jio::remove_url("抖音 https://v.douyin.com/RtKFFah/ 看一下");
    assert!(!out.contains("http"));
    assert!(out.contains("抖音"));
    assert!(out.contains("看一下"));
}

#[test]
fn parity_remove_phone_basic() {
    let out = jio::remove_phone_number("打电话到 13812345678 咨询");
    assert!(!out.contains("13812345678"));
    assert!(out.contains("打电话到"));
}

#[test]
fn parity_remove_email_with_prefix() {
    // Python: remove_email(text, delete_prefix=True) removes `E-mail: ` too.
    let out = jio::remove_email_with_prefix("请联系 E-mail: alice@ex.com 或 邮箱: bob@ex.com 都行");
    assert!(!out.contains("alice"));
    assert!(!out.contains("bob"));
    assert!(!out.contains("E-mail"));
    assert!(!out.contains("邮箱"));
}

#[test]
fn parity_remove_url_with_prefix() {
    let out = jio::remove_url_with_prefix("详情 网址: https://example.com 查看");
    assert!(!out.contains("https"));
    assert!(!out.contains("网址"));
    assert!(out.contains("详情"));
}

#[test]
fn parity_remove_phone_with_prefix() {
    let out = jio::remove_phone_number_with_prefix("电话: 13812345678 联系");
    assert!(!out.contains("13812345678"));
    assert!(!out.contains("电话"));
}

#[test]
fn parity_analyse_ner_dataset_split() {
    let x: Vec<String> = (0..100).map(|i| format!("s{}", i)).collect();
    let y: Vec<Vec<jio::Entity>> = (0..100)
        .map(|i| {
            vec![jio::Entity {
                text: "x".into(),
                type_: if i % 2 == 0 { "A".into() } else { "B".into() },
                offset: (0, 1),
            }]
        })
        .collect();
    let r = jio::analyse_ner_dataset_split(&x, &y, (0.8, 0.1, 0.1), 42, true);
    assert_eq!(r.train_x.len(), 80);
    assert_eq!(r.valid_x.len(), 10);
    assert_eq!(r.test_x.len(), 10);
    assert!(r.stats.total.contains_key("A") && r.stats.total.contains_key("B"));
}

#[test]
fn parity_rule_mining_context_capture() {
    let texts = ["公司: 百度", "公司: 腾讯"];
    let entities = vec![
        vec![("Company".to_string(), 4, 6)],
        vec![("Company".to_string(), 4, 6)],
    ];
    let r = jio::mine_rules(&texts, &entities, 4);
    let comp = r.get("Company").unwrap();
    assert!(!comp.prefix_freq.is_empty());
}

#[test]
fn parity_idiom_solitaire_empty_input() {
    // Python: `jio.idiom_solitaire('', ...)` returns ''. Our helper treats
    // empty cur as missing last char and returns Ok(None) → we surface None.
    let game = jio::IdiomSolitaireGame::new(1);
    let r = game.next_by_char("", false);
    assert!(r.is_ok(), "empty input should not panic");
}

#[test]
fn parity_idiom_solitaire_daozuqichang() {
    ensure_init();
    // Python: 道阻且长 → idiom starting with 长.
    let game = jio::IdiomSolitaireGame::new(7);
    let n = game.next_by_char("道阻且长", false).unwrap().unwrap();
    assert!(n.starts_with('长'), "expected 长-prefix idiom, got {}", n);
}

#[test]
fn parity_text_aug_swap_runs() {
    // test_text_aug.py verifies the augmenter runs and produces outputs.
    let r = jio::swap_char_position("今天天气真好", 2, 0.3, 12345, 0.5);
    assert_eq!(r.len(), 2);
    for out in &r {
        assert_eq!(out.chars().count(), "今天天气真好".chars().count());
    }
}

#[test]
fn parity_text_aug_homophone() {
    ensure_init();
    let r = jio::homophone_substitution("今天天气真好", 2, 0.5, 12345).unwrap();
    assert_eq!(r.len(), 2);
    for out in &r {
        assert_eq!(out.chars().count(), "今天天气真好".chars().count());
    }
}
