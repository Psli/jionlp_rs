//! Criterion micro-benchmarks for jionlp-core.
//!
//! Run from the bench crate directory:
//!   cd jionlp_rs/bench && cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jionlp_core::{
    bpe_decode, bpe_encode, char2num, char_radical, check_all_chinese_char, check_id_card,
    clean_html, dict, extract_chinese, extract_email, extract_id_card, extract_ip_address,
    extract_keyphrase, extract_keyphrase_textrank, extract_parentheses, extract_phone_number,
    extract_summary, extract_summary_mmr, extract_url, hamming_distance, homophone_substitution,
    num2char, parse_id_card, parse_location, parse_money, parse_motor_vehicle_licence_plate,
    parse_time, phone_location, pinyin, random_add_delete, recognize_location, remove_html_tag,
    remove_stopwords, sentiment_score, sim2tra, simhash, split_sentence, swap_char_position,
    tra2sim, Criterion as SplitCriterion, LexiconNer, NumStyle, PinyinFormat, RemoveOpts,
    TsMode,
};
use std::path::PathBuf;
use std::sync::Once;

static INIT: Once = Once::new();
fn setup() {
    INIT.call_once(|| {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let d = PathBuf::from(manifest).join("../../jionlp/dictionary");
        dict::init_from_path(&d).expect("init_from_path");
        let _ = dict::stopwords();
        let _ = dict::tra2sim_char();
        let _ = dict::sim2tra_char();
        let _ = dict::tra2sim_word();
        let _ = dict::sim2tra_word();
    });
}

const SAMPLE_TEXT: &str =
    "中华古汉语，泱泱大国，历史传承的瑰宝。今天天气真好，我打算去公园散步！\
     特朗普老友皮尔斯·摩根喊话特朗普：\"美国人的生命比你的选举更重要。\"";

const MIXED_TEXT: &str = concat!(
    "联系方式如下:邮箱 hello@example.com,电话 13912345678,座机 010-12345678。",
    " 访问 https://example.com/path?x=1 或 IP 192.168.1.1。",
    " 身份证 11010519900307123X,车牌 川A·23047B。QQ 123456789。"
);

fn bench_split_sentence(c: &mut Criterion) {
    setup();
    let mut g = c.benchmark_group("split_sentence");
    g.bench_function("coarse", |b| {
        b.iter(|| split_sentence(black_box(SAMPLE_TEXT), SplitCriterion::Coarse))
    });
    g.bench_function("fine", |b| {
        b.iter(|| split_sentence(black_box(SAMPLE_TEXT), SplitCriterion::Fine))
    });
    g.finish();
}

fn bench_ts_conversion(c: &mut Criterion) {
    setup();
    let tra = "今天天氣好晴朗，想喫速食麵。妳還在太空梭上工作嗎？";
    let sim = "今天天气好晴朗，想吃方便面。你还在航天飞机上工作吗？";
    let mut g = c.benchmark_group("ts_conversion");
    g.bench_function("tra2sim_char", |b| {
        b.iter(|| tra2sim(black_box(tra), TsMode::Char).unwrap())
    });
    g.bench_function("sim2tra_char", |b| {
        b.iter(|| sim2tra(black_box(sim), TsMode::Char).unwrap())
    });
    g.bench_function("tra2sim_word", |b| {
        b.iter(|| tra2sim(black_box(tra), TsMode::Word).unwrap())
    });
    g.bench_function("sim2tra_word", |b| {
        b.iter(|| sim2tra(black_box(sim), TsMode::Word).unwrap())
    });
    g.finish();
}

fn bench_remove_stopwords(c: &mut Criterion) {
    setup();
    let words: Vec<String> = [
        "我", "今天", "的", "心情", "非常", "好", "，", "打算", "去", "公园", "散步", "并", "吃",
        "一", "个", "苹果", "。",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    c.bench_function("remove_stopwords", |b| {
        b.iter(|| remove_stopwords(black_box(&words), RemoveOpts::default()).unwrap())
    });
}

fn bench_extractors(c: &mut Criterion) {
    let mut g = c.benchmark_group("extract");
    g.bench_function("email", |b| b.iter(|| extract_email(black_box(MIXED_TEXT))));
    g.bench_function("phone_number", |b| {
        b.iter(|| extract_phone_number(black_box(MIXED_TEXT)))
    });
    g.bench_function("ip_address", |b| {
        b.iter(|| extract_ip_address(black_box(MIXED_TEXT)))
    });
    g.bench_function("id_card", |b| b.iter(|| extract_id_card(black_box(MIXED_TEXT))));
    g.bench_function("url", |b| b.iter(|| extract_url(black_box(MIXED_TEXT))));
    g.bench_function("chinese", |b| {
        b.iter(|| extract_chinese(black_box(MIXED_TEXT)))
    });
    g.bench_function("parentheses", |b| {
        b.iter(|| extract_parentheses(black_box("a (b (c) d) e【foo(bar)】"), "()[]（）【】"))
    });
    g.finish();
}

fn bench_checkers(c: &mut Criterion) {
    let mut g = c.benchmark_group("check");
    g.bench_function("id_card", |b| {
        b.iter(|| check_id_card(black_box("11010519900307123X")))
    });
    g.bench_function("all_chinese_char", |b| {
        b.iter(|| check_all_chinese_char(black_box("全部都是中文没有别的字符")))
    });
    g.finish();
}

fn bench_plate_parser(c: &mut Criterion) {
    c.bench_function("parse_plate", |b| {
        b.iter(|| parse_motor_vehicle_licence_plate(black_box("川A·23047B")))
    });
}

fn bench_id_card_parser(c: &mut Criterion) {
    setup();
    // Touch the dictionary once to warm the cache.
    let _ = parse_id_card("440105199001012345").unwrap();
    c.bench_function("parse_id_card", |b| {
        b.iter(|| parse_id_card(black_box("440105199001012345")).unwrap())
    });
}

fn bench_char_radical(c: &mut Criterion) {
    setup();
    let _ = char_radical("预热一下").unwrap();
    c.bench_function("char_radical_mixed_short", |b| {
        b.iter(|| char_radical(black_box("今天L.A.洛杉矶天气好晴朗")).unwrap())
    });
}

fn bench_num_char(c: &mut Criterion) {
    let mut g = c.benchmark_group("num_char");
    g.bench_function("num2char_sim", |b| {
        b.iter(|| num2char(black_box("1234567890"), NumStyle::Simplified).unwrap())
    });
    g.bench_function("num2char_tra", |b| {
        b.iter(|| num2char(black_box("1234567890.12"), NumStyle::Traditional).unwrap())
    });
    g.bench_function("char2num_wan", |b| {
        b.iter(|| char2num(black_box("三千五百万")).unwrap())
    });
    g.finish();
}

fn bench_html(c: &mut Criterion) {
    let html = "<html><head><style>body{color:red}</style></head>\
                <body><p>hello &amp; <b>world</b></p><!-- c --></body></html>";
    let mut g = c.benchmark_group("html");
    g.bench_function("remove_html_tag", |b| {
        b.iter(|| remove_html_tag(black_box(html)))
    });
    g.bench_function("clean_html", |b| b.iter(|| clean_html(black_box(html))));
    g.finish();
}

fn bench_phone_location(c: &mut Criterion) {
    setup();
    let _ = phone_location("13812345678").unwrap();
    let mut g = c.benchmark_group("phone_location");
    g.bench_function("cell", |b| {
        b.iter(|| phone_location(black_box("13812345678")).unwrap())
    });
    g.bench_function("landline", |b| {
        b.iter(|| phone_location(black_box("010-12345678")).unwrap())
    });
    g.finish();
}

fn bench_location(c: &mut Criterion) {
    setup();
    // Warm the Aho-Corasick index.
    let _ = recognize_location("预热").unwrap();
    let text = "我出生在广东省广州市海珠区，后来搬到四川省成都市锦江区。";
    let mut g = c.benchmark_group("location");
    g.bench_function("recognize", |b| {
        b.iter(|| recognize_location(black_box(text)).unwrap())
    });
    g.bench_function("parse", |b| {
        b.iter(|| parse_location(black_box(text)).unwrap())
    });
    g.finish();
}

fn bench_pinyin(c: &mut Criterion) {
    setup();
    let _ = pinyin("中国", PinyinFormat::Standard).unwrap();
    let mut g = c.benchmark_group("pinyin");
    g.bench_function("standard_short", |b| {
        b.iter(|| pinyin(black_box("中华人民共和国"), PinyinFormat::Standard).unwrap())
    });
    g.bench_function("simple_short", |b| {
        b.iter(|| pinyin(black_box("中华人民共和国"), PinyinFormat::Simple).unwrap())
    });
    g.finish();
}

fn bench_parse_money(c: &mut Criterion) {
    let mut g = c.benchmark_group("parse_money");
    g.bench_function("arabic_yuan", |b| {
        b.iter(|| parse_money(black_box("1,234.56元")))
    });
    g.bench_function("chinese_wan", |b| {
        b.iter(|| parse_money(black_box("三千五百万元")))
    });
    g.bench_function("mixed_usd", |b| {
        b.iter(|| parse_money(black_box("1.5万美元")))
    });
    g.bench_function("symbol_prefix", |b| {
        b.iter(|| parse_money(black_box("$100.5")))
    });
    g.bench_function("yuan_jiao_fen", |b| {
        b.iter(|| parse_money(black_box("100元5角3分")))
    });
    g.bench_function("range_dash", |b| {
        b.iter(|| parse_money(black_box("100-200元")))
    });
    g.bench_function("blur", |b| b.iter(|| parse_money(black_box("约100元"))));
    g.finish();
}

fn bench_parse_time(c: &mut Criterion) {
    let mut g = c.benchmark_group("parse_time");
    g.bench_function("absolute_cn", |b| {
        b.iter(|| parse_time(black_box("2024年3月5日")))
    });
    g.bench_function("absolute_dash", |b| {
        b.iter(|| parse_time(black_box("2024-03-05")))
    });
    g.bench_function("with_clock", |b| {
        b.iter(|| parse_time(black_box("2024年3月5日下午3点30分")))
    });
    g.bench_function("relative_tomorrow", |b| {
        b.iter(|| parse_time(black_box("明天")))
    });
    g.finish();
}

fn bench_simhash(c: &mut Criterion) {
    let short = "今天天气真好";
    let long = "机器学习是人工智能的一个分支,研究如何从数据中自动学习规律和模式";
    let mut g = c.benchmark_group("simhash");
    g.bench_function("short", |b| b.iter(|| simhash(black_box(short))));
    g.bench_function("long", |b| b.iter(|| simhash(black_box(long))));
    let a = simhash(short);
    let b_hash = simhash(long);
    g.bench_function("hamming", |b| {
        b.iter(|| hamming_distance(black_box(a), black_box(b_hash)))
    });
    g.finish();
}

fn bench_parse_time_stage2(c: &mut Criterion) {
    let mut g = c.benchmark_group("parse_time_s2");
    g.bench_function("holiday_guoqing", |b| {
        b.iter(|| parse_time(black_box("国庆节")))
    });
    g.bench_function("range_same_month", |b| {
        b.iter(|| parse_time(black_box("2024年3月5日到8日")))
    });
    g.finish();
}

fn bench_parse_time_stage3(c: &mut Criterion) {
    let mut g = c.benchmark_group("parse_time_s3");
    g.bench_function("timespan_bare", |b| {
        b.iter(|| parse_time(black_box("8点到12点")))
    });
    g.bench_function("timespan_with_day", |b| {
        b.iter(|| parse_time(black_box("明天下午3点到5点")))
    });
    g.bench_function("recurring_weekday", |b| {
        b.iter(|| parse_time(black_box("每周一")))
    });
    g.bench_function("recurring_day_of_month", |b| {
        b.iter(|| parse_time(black_box("每月15号")))
    });
    g.finish();
}

fn bench_parse_time_stage4(c: &mut Criterion) {
    let mut g = c.benchmark_group("parse_time_s4");
    g.bench_function("delta_days", |b| {
        b.iter(|| parse_time(black_box("三天后")))
    });
    g.bench_function("delta_half_hour", |b| {
        b.iter(|| parse_time(black_box("半小时后")))
    });
    g.bench_function("named_week", |b| {
        b.iter(|| parse_time(black_box("本周")))
    });
    g.bench_function("named_quarter", |b| {
        b.iter(|| parse_time(black_box("下季度")))
    });
    g.finish();
}

fn bench_parse_time_stage5(c: &mut Criterion) {
    let mut g = c.benchmark_group("parse_time_s5");
    g.bench_function("fuzzy_gangcai", |b| {
        b.iter(|| parse_time(black_box("刚才")))
    });
    g.bench_function("fuzzy_zuijin", |b| {
        b.iter(|| parse_time(black_box("最近")))
    });
    g.finish();
}

fn bench_bpe(c: &mut Criterion) {
    let src = "Hello 世界 🌍 テスト 今天天气真好";
    let enc = bpe_encode(src);
    let mut g = c.benchmark_group("bpe");
    g.bench_function("encode", |b| b.iter(|| bpe_encode(black_box(src))));
    g.bench_function("decode", |b| b.iter(|| bpe_decode(black_box(&enc))));
    g.finish();
}

fn bench_summary_mmr(c: &mut Criterion) {
    setup();
    let _ = extract_summary_mmr("预热。", 1, 0.7).unwrap();
    let text = "北京是中国的首都。上海是金融中心。广州是南方大都市。\
                深圳是科技创新之都。成都是西南地区的枢纽城市。杭州以互联网产业闻名。";
    c.bench_function("summary_mmr_lambda_07", |b| {
        b.iter(|| extract_summary_mmr(black_box(text), 3, 0.7).unwrap())
    });
}

fn bench_parse_time_stage6(c: &mut Criterion) {
    let mut g = c.benchmark_group("parse_time_s6");
    g.bench_function("lunar_chunjie", |b| {
        b.iter(|| parse_time(black_box("2024年春节")))
    });
    g.bench_function("lunar_mid_autumn_alias", |b| {
        b.iter(|| parse_time(black_box("2025年中秋")))
    });
    g.finish();
}

fn bench_keyphrase_textrank(c: &mut Criterion) {
    setup();
    let _ = extract_keyphrase_textrank("预热", 1, 2, 3).unwrap();
    let text = "机器学习是人工智能的一个分支。机器学习研究如何从数据中学习。\
                机器学习广泛应用于自然语言处理、计算机视觉和推荐系统。";
    c.bench_function("textrank_short_doc", |b| {
        b.iter(|| extract_keyphrase_textrank(black_box(text), 5, 2, 4).unwrap())
    });
}

fn bench_pinyin_phrase(c: &mut Criterion) {
    setup();
    let _ = pinyin("预热", PinyinFormat::Standard).unwrap();
    let mut g = c.benchmark_group("pinyin_phrase");
    g.bench_function("idiom", |b| {
        b.iter(|| pinyin(black_box("一丘之貉"), PinyinFormat::Standard).unwrap())
    });
    g.bench_function("mixed_phrase_and_char", |b| {
        b.iter(|| pinyin(black_box("中华人民共和国"), PinyinFormat::Standard).unwrap())
    });
    g.finish();
}

fn bench_keyphrase(c: &mut Criterion) {
    setup();
    let _ = extract_keyphrase("预热", 3, 2, 4).unwrap();
    let text =
        "机器学习是人工智能的一个分支,研究如何从数据中自动学习规律和模式。\
         机器学习广泛应用于自然语言处理、计算机视觉和推荐系统。";
    c.bench_function("keyphrase_short_doc", |b| {
        b.iter(|| extract_keyphrase(black_box(text), 5, 2, 4).unwrap())
    });
}

fn bench_sentiment(c: &mut Criterion) {
    setup();
    let _ = sentiment_score("预热一下").unwrap();
    let pos = "今天是美好的一天,我非常开心,万事如意。";
    let neg = "事故造成严重伤亡,令人悲痛万分。";
    let mut g = c.benchmark_group("sentiment");
    g.bench_function("positive", |b| {
        b.iter(|| sentiment_score(black_box(pos)).unwrap())
    });
    g.bench_function("negative", |b| {
        b.iter(|| sentiment_score(black_box(neg)).unwrap())
    });
    g.finish();
}

fn bench_summary(c: &mut Criterion) {
    setup();
    let _ = extract_summary("预热。", 1).unwrap();
    let text = "北京是中国的首都。上海是金融中心。广州是南方大都市。\
                深圳是科技创新之都。成都是西南地区的枢纽城市。杭州以互联网产业闻名。";
    c.bench_function("summary_6_sentences", |b| {
        b.iter(|| extract_summary(black_box(text), 3).unwrap())
    });
}

fn bench_swap_char(c: &mut Criterion) {
    c.bench_function("swap_char_3_variants", |b| {
        b.iter(|| {
            swap_char_position(
                black_box("中华人民共和国是美好的家园"),
                3,
                0.1,
                42,
                1.0,
            )
        })
    });
}

fn bench_textaug_extras(c: &mut Criterion) {
    setup();
    // Warm homophone reverse index.
    let _ = homophone_substitution("预热", 1, 0.5, 42).unwrap();
    let src = "中华人民共和国是美好的家园";
    let mut g = c.benchmark_group("textaug");
    g.bench_function("random_add_delete", |b| {
        b.iter(|| random_add_delete(black_box(src), 3, 0.1, 0.1, 42))
    });
    g.bench_function("homophone", |b| {
        b.iter(|| homophone_substitution(black_box(src), 3, 0.3, 42).unwrap())
    });
    g.finish();
}

fn bench_lexicon_ner(c: &mut Criterion) {
    use rustc_hash::FxHashMap;
    let mut lex: FxHashMap<String, Vec<String>> = FxHashMap::default();
    lex.insert(
        "Drug".to_string(),
        vec![
            "阿司匹林".to_string(),
            "布洛芬".to_string(),
            "对乙酰氨基酚".to_string(),
        ],
    );
    lex.insert(
        "Company".to_string(),
        vec![
            "阿里巴巴".to_string(),
            "腾讯".to_string(),
            "字节跳动".to_string(),
        ],
    );
    let ner = LexiconNer::from_map(&lex).unwrap();
    let text = "他在阿里巴巴上班,生病时服用了阿司匹林和布洛芬。";
    c.bench_function("lexicon_ner_scan", |b| {
        b.iter(|| ner.recognize(black_box(text)))
    });
}

criterion_group!(
    benches,
    bench_split_sentence,
    bench_ts_conversion,
    bench_remove_stopwords,
    bench_extractors,
    bench_checkers,
    bench_plate_parser,
    bench_id_card_parser,
    bench_char_radical,
    bench_num_char,
    bench_html,
    bench_phone_location,
    bench_location,
    bench_pinyin,
    bench_parse_money,
    bench_parse_time,
    bench_simhash,
    bench_parse_time_stage2,
    bench_pinyin_phrase,
    bench_keyphrase,
    bench_sentiment,
    bench_summary,
    bench_swap_char,
    bench_textaug_extras,
    bench_lexicon_ner,
    bench_parse_time_stage3,
    bench_parse_time_stage4,
    bench_parse_time_stage5,
    bench_bpe,
    bench_summary_mmr,
    bench_parse_time_stage6,
    bench_keyphrase_textrank,
);
criterion_main!(benches);
