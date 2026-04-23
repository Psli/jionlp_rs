//! BIO/BIOES tag converters — port of the CWS/POS/NER data utilities under
//! `jionlp/algorithm/{cws,pos,ner}/*_data_converter.py`. These are pure
//! data-shape transforms used to prepare training data for sequence models.
//!
//! Public API groups:
//!   * `cws::word2tag` / `cws::tag2word` — BI tags for word segmentation.
//!   * `pos::pos2tag` / `pos::tag2pos` — `B-<tag>` / `I-<tag>` for POS.
//!   * `ner::entity2tag` / `ner::tag2entity` — BIOES for NER.
//!   * `ner::entity_compare` + `F1` — entity-level evaluation.

// ───────────────────────── shared types ──────────────────────────────────

/// A named entity produced by the NER pipeline. `offset` is half-open
/// `[start, end)` over the *token list*, NOT byte positions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Entity {
    pub text: String,
    pub type_: String,
    pub offset: (usize, usize),
}

/// Classification F1 metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct F1 {
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
}

impl F1 {
    pub fn compute(tp: usize, fp: usize, fn_: usize) -> Self {
        let precision = if tp + fp == 0 {
            0.0
        } else {
            tp as f64 / (tp + fp) as f64
        };
        let recall = if tp + fn_ == 0 {
            0.0
        } else {
            tp as f64 / (tp + fn_) as f64
        };
        let f1 = if precision + recall == 0.0 {
            0.0
        } else {
            2.0 * precision * recall / (precision + recall)
        };
        F1 {
            precision,
            recall,
            f1,
            true_positives: tp,
            false_positives: fp,
            false_negatives: fn_,
        }
    }
}

// ───────────────────────── CWS (word segmentation) ───────────────────────

pub mod cws {
    /// Convert a word-segmented list into `(chars, tags)` where tags are `"B"`
    /// for word-start and `"I"` for inside — equivalent to Python's
    /// `cws.word2tag`.
    ///
    /// Tokens may be single-char or multi-char; Chinese chars count as one
    /// token per char.
    pub fn word2tag(words: &[String]) -> (String, Vec<&'static str>) {
        let mut chars = String::new();
        let mut tags = Vec::new();
        for w in words {
            let mut first = true;
            for _c in w.chars() {
                tags.push(if first { "B" } else { "I" });
                first = false;
            }
            chars.push_str(w);
        }
        (chars, tags)
    }

    /// Convert `(chars, tags)` back into a word list. Invalid tag sequences
    /// are handled leniently (same as the Python reference).
    pub fn tag2word(chars: &str, tags: &[&str]) -> Vec<String> {
        let ch_vec: Vec<char> = chars.chars().collect();
        assert_eq!(ch_vec.len(), tags.len(), "chars/tags length mismatch");

        let mut words: Vec<String> = Vec::new();
        let mut start: Option<usize> = None;

        for (idx, (tag, _c)) in tags.iter().zip(ch_vec.iter()).enumerate() {
            if *tag == "B" {
                if let Some(s) = start {
                    words.push(ch_vec[s..idx].iter().collect());
                }
                start = Some(idx);
            } else if *tag == "I" && start.is_none() {
                start = Some(idx);
            }
        }
        if let Some(s) = start {
            words.push(ch_vec[s..].iter().collect());
        }
        words
    }
}

// ───────────────────────── POS tagging ───────────────────────────────────

pub mod pos {
    /// Convert `(word, pos)` pairs into `(chars, tags)`. Tags use `B-<pos>`
    /// for word-start and `I-<pos>` for inside.
    pub fn pos2tag(pos_list: &[(String, String)]) -> (String, Vec<String>) {
        let mut chars = String::new();
        let mut tags: Vec<String> = Vec::new();
        for (w, p) in pos_list {
            let mut first = true;
            for _c in w.chars() {
                tags.push(format!("{}-{}", if first { "B" } else { "I" }, p));
                first = false;
            }
            chars.push_str(w);
        }
        (chars, tags)
    }

    /// Convert `(chars, tags)` back into `(word, pos)` pairs.
    pub fn tag2pos(chars: &str, tags: &[String]) -> Vec<(String, String)> {
        let ch_vec: Vec<char> = chars.chars().collect();
        assert_eq!(ch_vec.len(), tags.len(), "chars/tags length mismatch");

        let mut out: Vec<(String, String)> = Vec::new();
        let mut start: Option<usize> = None;
        let mut cur_pos: Option<String> = None;

        for (idx, tag) in tags.iter().enumerate() {
            if tag.starts_with('B') {
                if let Some(s) = start {
                    let w: String = ch_vec[s..idx].iter().collect();
                    out.push((w, cur_pos.clone().unwrap_or_default()));
                }
                start = Some(idx);
                cur_pos = Some(tag.split_once('-').map(|x| x.1).unwrap_or("").to_string());
            } else if tag.starts_with('I') && start.is_none() {
                start = Some(idx);
                cur_pos = Some(tag.split_once('-').map(|x| x.1).unwrap_or("").to_string());
            }
        }
        if let Some(s) = start {
            let w: String = ch_vec[s..].iter().collect();
            out.push((w, cur_pos.clone().unwrap_or_default()));
        }
        out
    }
}

// ───────────────────────── NER ───────────────────────────────────────────

pub mod ner {
    use super::{Entity, F1};

    /// Convert an entity list to BIOES tags. Entities are expected to use
    /// token-index offsets `[start, end)`.
    pub fn entity2tag(token_len: usize, entities: &[Entity]) -> Vec<String> {
        let mut tags = vec!["O".to_string(); token_len];
        let mut sorted = entities.to_vec();
        sorted.sort_by_key(|e| e.offset.0);
        let mut end_flag: usize = 0;
        for e in &sorted {
            if e.offset.1 <= end_flag {
                continue; // overlapping — drop the later one.
            }
            let (s, eend) = (e.offset.0, e.offset.1);
            if eend - s == 1 {
                tags[s] = format!("S-{}", e.type_);
            } else {
                tags[s] = format!("B-{}", e.type_);
                if eend - s > 2 {
                    for j in (s + 1)..(eend - 1) {
                        tags[j] = format!("I-{}", e.type_);
                    }
                }
                tags[eend - 1] = format!("E-{}", e.type_);
            }
            end_flag = eend;
        }
        tags
    }

    /// Convert BIOES tags back into an entity list. `tokens` supplies the
    /// token strings (one char or word per entry) used to reconstruct
    /// `entity.text`.
    pub fn tag2entity(tokens: &[String], tags: &[String]) -> Vec<Entity> {
        assert_eq!(tokens.len(), tags.len(), "tokens/tags length mismatch");
        let mut out: Vec<Entity> = Vec::new();
        let mut start: Option<usize> = None;

        for (idx, tag) in tags.iter().enumerate() {
            let prefix = tag.chars().next().unwrap_or('O');
            match prefix {
                'O' => start = None,
                'I' => { /* continue collecting */ }
                'E' => {
                    if let Some(s) = start {
                        let t: String = tokens[s..=idx].join("");
                        let type_ = tags[s]
                            .split_once('-')
                            .map(|x| x.1)
                            .unwrap_or("")
                            .to_string();
                        out.push(Entity {
                            text: t,
                            type_,
                            offset: (s, idx + 1),
                        });
                        start = None;
                    }
                }
                'S' => {
                    let type_ = tag.split_once('-').map(|x| x.1).unwrap_or("").to_string();
                    out.push(Entity {
                        text: tokens[idx].clone(),
                        type_,
                        offset: (idx, idx + 1),
                    });
                    start = None;
                }
                'B' => {
                    start = Some(idx);
                }
                _ => { /* invalid; skip */ }
            }
        }
        out
    }

    /// Compute entity-level F1 between a prediction and a gold reference.
    pub fn entity_compare(pred: &[Entity], gold: &[Entity]) -> F1 {
        let pred_set: std::collections::HashSet<&Entity> = pred.iter().collect();
        let gold_set: std::collections::HashSet<&Entity> = gold.iter().collect();
        let tp = pred_set.intersection(&gold_set).count();
        let fp = pred_set.len() - tp;
        let fn_ = gold_set.len() - tp;
        F1::compute(tp, fp, fn_)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cws_word_tag_roundtrip() {
        let words = vec![
            "他".to_string(),
            "指出".to_string(),
            "：".to_string(),
            "近".to_string(),
            "几".to_string(),
            "年".to_string(),
            "来".to_string(),
            "，".to_string(),
            "足球场".to_string(),
            "风气".to_string(),
            "差劲".to_string(),
            "。".to_string(),
        ];
        let (chars, tags) = cws::word2tag(&words);
        assert_eq!(chars, "他指出：近几年来，足球场风气差劲。");
        assert_eq!(tags.len(), 17);
        let back = cws::tag2word(&chars, &tags);
        assert_eq!(back, words);
    }

    #[test]
    fn pos_tag_roundtrip() {
        let pairs = vec![
            ("他".to_string(), "r".to_string()),
            ("指出".to_string(), "v".to_string()),
            ("：".to_string(), "w".to_string()),
        ];
        let (chars, tags) = pos::pos2tag(&pairs);
        assert_eq!(chars, "他指出：");
        assert_eq!(tags, vec!["B-r", "B-v", "I-v", "B-w"]);
        let back = pos::tag2pos(&chars, &tags);
        assert_eq!(back, pairs);
    }

    #[test]
    fn ner_bioes_roundtrip() {
        let tokens: Vec<String> = "胡静静在水利局工作。"
            .chars()
            .map(|c| c.to_string())
            .collect();
        let ents = vec![
            Entity {
                text: "胡静静".into(),
                type_: "Person".into(),
                offset: (0, 3),
            },
            Entity {
                text: "水利局".into(),
                type_: "Orgnization".into(),
                offset: (4, 7),
            },
        ];
        let tags = ner::entity2tag(tokens.len(), &ents);
        assert_eq!(
            tags,
            vec![
                "B-Person",
                "I-Person",
                "E-Person",
                "O",
                "B-Orgnization",
                "I-Orgnization",
                "E-Orgnization",
                "O",
                "O",
                "O"
            ]
        );
        let back = ner::tag2entity(&tokens, &tags);
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].text, "胡静静");
        assert_eq!(back[1].offset, (4, 7));
    }

    #[test]
    fn ner_f1() {
        let gold = vec![
            Entity {
                text: "甲".into(),
                type_: "A".into(),
                offset: (0, 1),
            },
            Entity {
                text: "乙".into(),
                type_: "B".into(),
                offset: (2, 3),
            },
        ];
        let pred = vec![
            Entity {
                text: "甲".into(),
                type_: "A".into(),
                offset: (0, 1),
            },
            Entity {
                text: "丙".into(),
                type_: "C".into(),
                offset: (5, 6),
            },
        ];
        let r = ner::entity_compare(&pred, &gold);
        assert_eq!(r.true_positives, 1);
        assert_eq!(r.false_positives, 1);
        assert_eq!(r.false_negatives, 1);
        assert!((r.f1 - 0.5).abs() < 1e-9);
    }
}
