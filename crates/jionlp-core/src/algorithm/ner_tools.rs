//! NER support utilities — ports of:
//! * `ner/ner_data_converter.py::char2word` / `word2char`
//! * `ner/check_person_name.py::CheckPersonName`
//! * `ner/analyse_dataset.py` (entity-type counts)
//! * token-batching helpers
//!
//! These are data-shape utilities used to prepare, sanity-check, and
//! analyze NER training corpora. None of them drive model inference.

use super::tag_conversion::Entity;

/// Remap char-level entity offsets to word-level offsets, given a
/// pre-tokenized word list. Entities whose boundaries don't align with a
/// word split are dropped (matches Python's lenient behavior).
pub fn char2word(char_entities: &[Entity], word_tokens: &[String]) -> Vec<Entity> {
    let mut idx_list: Vec<usize> = vec![0];
    let mut flag = 0usize;
    for w in word_tokens {
        flag += w.chars().count();
        idx_list.push(flag);
    }
    let mut out = Vec::new();
    for e in char_entities {
        let (cs, ce) = e.offset;
        let start = idx_list.iter().position(|&x| x == cs);
        let end = idx_list.iter().position(|&x| x == ce);
        if let (Some(s), Some(ed)) = (start, end) {
            out.push(Entity {
                text: e.text.clone(),
                type_: e.type_.clone(),
                offset: (s, ed),
            });
        }
    }
    out
}

/// Inverse of `char2word` — remap word-level offsets to char offsets.
pub fn word2char(word_entities: &[Entity], word_tokens: &[String]) -> Vec<Entity> {
    let mut idx_list: Vec<usize> = vec![0];
    let mut flag = 0usize;
    for w in word_tokens {
        flag += w.chars().count();
        idx_list.push(flag);
    }
    let mut out = Vec::new();
    for e in word_entities {
        let (ws, we) = e.offset;
        if ws >= idx_list.len() || we >= idx_list.len() {
            continue;
        }
        out.push(Entity {
            text: e.text.clone(),
            type_: e.type_.clone(),
            offset: (idx_list[ws], idx_list[we]),
        });
    }
    out
}

// ───────────────────────── CheckPersonName ───────────────────────────────

/// The 100-surnames list (百家姓) — single-char surnames. Extended set can
/// be supplied by the caller via `CheckPersonName::with_surnames`.
const COMMON_SURNAMES: &[&str] = &[
    "赵", "钱", "孙", "李", "周", "吴", "郑", "王", "冯", "陈", "褚", "卫", "蒋", "沈", "韩", "杨",
    "朱", "秦", "尤", "许", "何", "吕", "施", "张", "孔", "曹", "严", "华", "金", "魏", "陶", "姜",
    "戚", "谢", "邹", "喻", "柏", "水", "窦", "章", "云", "苏", "潘", "葛", "奚", "范", "彭", "郎",
    "鲁", "韦", "昌", "马", "苗", "凤", "花", "方", "俞", "任", "袁", "柳", "酆", "鲍", "史", "唐",
    "费", "廉", "岑", "薛", "雷", "贺", "倪", "汤", "滕", "殷", "罗", "毕", "郝", "邬", "安", "常",
    "乐", "于", "时", "傅", "皮", "卞", "齐", "康", "伍", "余", "元", "卜", "顾", "孟", "平", "黄",
    "和", "穆", "萧", "尹", "姚", "邵", "湛", "汪", "祁", "毛", "禹", "狄", "米", "贝", "明", "臧",
    "计", "伏", "成", "戴", "谈", "宋", "茅", "庞", "熊", "纪", "舒", "屈", "项", "祝", "董", "梁",
    "杜", "阮", "蓝", "闵", "席", "季",
];

/// Two-char compound surnames, common enough to ignore the trailing char.
const DOUBLE_SURNAMES: &[&str] = &[
    "欧阳", "司马", "诸葛", "上官", "尉迟", "公孙", "皇甫", "慕容", "令狐", "夏侯", "轩辕", "东方",
    "宇文", "澹台",
];

/// Check whether `text` looks like a Chinese person name.
/// Rules (ported from Python `CheckPersonName`):
///   * length 2-4 Chinese chars
///   * first char (or first two, for 复姓) is a known surname
///   * no punctuation/latin letters
pub fn is_person_name(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 2 || chars.len() > 4 {
        return false;
    }
    if !chars.iter().all(|c| ('\u{4E00}'..='\u{9FA5}').contains(c)) {
        return false;
    }
    // Double surname first.
    if chars.len() >= 2 {
        let two: String = chars[..2].iter().collect();
        if DOUBLE_SURNAMES.contains(&two.as_str()) {
            return true;
        }
    }
    let one: String = chars[0].to_string();
    COMMON_SURNAMES.contains(&one.as_str())
}

// ───────────────────────── dataset analysis ──────────────────────────────

/// Per-entity-type count and sample-level statistics.
#[derive(Debug, Clone)]
pub struct NerDatasetAnalysis {
    pub total_samples: usize,
    pub total_entities: usize,
    /// entity-type → count.
    pub per_type_count: rustc_hash::FxHashMap<String, usize>,
    /// Average entity-count per sample.
    pub entities_per_sample: f64,
}

/// Summarize a sample dataset of `(tokens, entities)` pairs.
pub fn analyse_ner_dataset<T: AsRef<[Entity]>>(samples: &[T]) -> NerDatasetAnalysis {
    let total_samples = samples.len();
    let mut per_type: rustc_hash::FxHashMap<String, usize> = rustc_hash::FxHashMap::default();
    let mut total_entities = 0usize;
    for ents in samples {
        for e in ents.as_ref() {
            total_entities += 1;
            *per_type.entry(e.type_.clone()).or_insert(0) += 1;
        }
    }
    let entities_per_sample = if total_samples == 0 {
        0.0
    } else {
        total_entities as f64 / total_samples as f64
    };
    NerDatasetAnalysis {
        total_samples,
        total_entities,
        per_type_count: per_type,
        entities_per_sample,
    }
}

/// Flatten all entities across the dataset, preserving order.
pub fn collect_dataset_entities<T: AsRef<[Entity]>>(samples: &[T]) -> Vec<Entity> {
    let mut out = Vec::new();
    for ents in samples {
        out.extend_from_slice(ents.as_ref());
    }
    out
}

// ───────────────────────── token batching ────────────────────────────────

/// Split one long token string into sentence-sized pieces by breaking on
/// terminal punctuation `。！？`. Returns the pieces as substrings.
pub fn token_split_sentence(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for c in text.chars() {
        buf.push(c);
        if matches!(c, '。' | '！' | '？' | '.' | '!' | '?') {
            out.push(std::mem::take(&mut buf));
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

/// Break a long sentence into overlapping `window`-char chunks with
/// `overlap` characters shared between consecutive chunks.
pub fn token_break_long_sentence(text: &str, window: usize, overlap: usize) -> Vec<String> {
    if window == 0 {
        return vec![text.to_string()];
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= window {
        return vec![text.to_string()];
    }
    let step = window.saturating_sub(overlap).max(1);
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let end = (i + window).min(chars.len());
        out.push(chars[i..end].iter().collect::<String>());
        if end == chars.len() {
            break;
        }
        i += step;
    }
    out
}

/// Bucket a list of items into fixed-size batches.
pub fn token_batch_bucket<T: Clone>(items: &[T], batch_size: usize) -> Vec<Vec<T>> {
    if batch_size == 0 {
        return vec![items.to_vec()];
    }
    items.chunks(batch_size).map(|c| c.to_vec()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char2word_roundtrip() {
        let words = vec![
            "胡静静".to_string(),
            "喜欢".to_string(),
            "江西".to_string(),
            "红叶".to_string(),
            "建筑".to_string(),
            "公司".to_string(),
        ];
        let char_ents = vec![
            Entity {
                text: "胡静静".into(),
                type_: "Person".into(),
                offset: (0, 3),
            },
            Entity {
                text: "江西红叶建筑公司".into(),
                type_: "Company".into(),
                offset: (5, 13),
            },
        ];
        let word_ents = char2word(&char_ents, &words);
        assert_eq!(word_ents.len(), 2);
        assert_eq!(word_ents[0].offset, (0, 1));
        assert_eq!(word_ents[1].offset, (2, 6));

        let back = word2char(&word_ents, &words);
        assert_eq!(back[0].offset, (0, 3));
        assert_eq!(back[1].offset, (5, 13));
    }

    #[test]
    fn check_person_name_positive() {
        assert!(is_person_name("张三"));
        assert!(is_person_name("李小明"));
        assert!(is_person_name("欧阳修"));
    }

    #[test]
    fn check_person_name_negative() {
        assert!(!is_person_name("Alice"));
        assert!(!is_person_name("X"));
        assert!(!is_person_name("好的你好"));
    }

    #[test]
    fn analyse_ner_dataset_basic() {
        let samples: Vec<Vec<Entity>> = vec![
            vec![
                Entity {
                    text: "A".into(),
                    type_: "Person".into(),
                    offset: (0, 1),
                },
                Entity {
                    text: "B".into(),
                    type_: "Org".into(),
                    offset: (2, 3),
                },
            ],
            vec![Entity {
                text: "C".into(),
                type_: "Person".into(),
                offset: (0, 1),
            }],
        ];
        let r = analyse_ner_dataset(&samples);
        assert_eq!(r.total_samples, 2);
        assert_eq!(r.total_entities, 3);
        assert_eq!(*r.per_type_count.get("Person").unwrap(), 2);
    }

    #[test]
    fn token_split_sentence_basic() {
        let r = token_split_sentence("你好。再见！明天见？");
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn token_break_long_sentence_overlap() {
        let r = token_break_long_sentence("一二三四五六七八九十", 4, 1);
        assert_eq!(r.len(), 3);
        assert_eq!(r[0], "一二三四");
        assert_eq!(r[1], "四五六七");
        assert_eq!(r[2], "七八九十");
    }

    #[test]
    fn token_batch_bucket_chunks() {
        let r = token_batch_bucket(&[1, 2, 3, 4, 5], 2);
        assert_eq!(r, vec![vec![1, 2], vec![3, 4], vec![5]]);
    }
}
