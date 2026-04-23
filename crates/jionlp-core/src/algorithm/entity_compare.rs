//! Detailed entity diffing — port of
//! `jionlp/algorithm/ner/ner_entity_compare.py::entity_compare`.
//!
//! Produces a per-pair diff list showing for each disagreement:
//!   * `labeled_entity` (gold) — `Some` or `None`
//!   * `predicted_entity` — `Some` or `None`
//!   * `context` — a window of the original text around the span
//!
//! Complements the simpler `algorithm::ner_convert::entity_compare` that
//! returns only aggregate F1 stats.

use crate::algorithm::tag_conversion::Entity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityDiff {
    pub context: String,
    pub labeled_entity: Option<Entity>,
    pub predicted_entity: Option<Entity>,
}

/// Produce a diff list comparing `labeled` (gold) and `predicted` entities.
/// `context_pad` specifies how many chars of surrounding text to include
/// in each diff's `context` string (mirrors Python default 10).
pub fn entity_compare_detailed(
    text: &str,
    labeled: &[Entity],
    predicted: &[Entity],
    context_pad: usize,
) -> Vec<EntityDiff> {
    let text_chars: Vec<char> = text.chars().collect();
    let text_len = text_chars.len();
    let mut out = Vec::new();

    let mut labeled: Vec<Entity> = labeled.to_vec();
    let mut predicted: Vec<Entity> = predicted.to_vec();
    labeled.sort_by_key(|e| e.offset.0);
    predicted.sort_by_key(|e| e.offset.0);

    let ctx = |start: usize, end: usize| -> String {
        let lo = start.saturating_sub(context_pad);
        let hi = (end + context_pad).min(text_len);
        text_chars[lo..hi].iter().collect()
    };

    if labeled.is_empty() {
        for e in &predicted {
            out.push(EntityDiff {
                context: ctx(e.offset.0, e.offset.1),
                labeled_entity: None,
                predicted_entity: Some(e.clone()),
            });
        }
        return out;
    }
    if predicted.is_empty() {
        for e in &labeled {
            out.push(EntityDiff {
                context: ctx(e.offset.0, e.offset.1),
                labeled_entity: Some(e.clone()),
                predicted_entity: None,
            });
        }
        return out;
    }

    // Both non-empty: align by offset.
    let mut pred_consumed = vec![false; predicted.len()];
    for lab in &labeled {
        let mut any_overlap = false;
        for (pi, pred) in predicted.iter().enumerate() {
            if pred.offset.1 <= lab.offset.0 {
                continue; // predicted is entirely before labeled
            }
            if pred.offset.0 >= lab.offset.1 {
                break; // predicted is entirely after; no overlap
            }
            any_overlap = true;
            pred_consumed[pi] = true;
            // Check exact match (same offsets).
            if pred.offset == lab.offset {
                if pred.type_ == lab.type_ {
                    // Perfect match — no diff.
                } else {
                    out.push(EntityDiff {
                        context: ctx(lab.offset.0, lab.offset.1),
                        labeled_entity: Some(lab.clone()),
                        predicted_entity: Some(pred.clone()),
                    });
                }
            } else {
                // Partial overlap — boundary mismatch.
                let start = lab.offset.0.min(pred.offset.0);
                let end = lab.offset.1.max(pred.offset.1);
                out.push(EntityDiff {
                    context: ctx(start, end),
                    labeled_entity: Some(lab.clone()),
                    predicted_entity: Some(pred.clone()),
                });
            }
        }
        if !any_overlap {
            out.push(EntityDiff {
                context: ctx(lab.offset.0, lab.offset.1),
                labeled_entity: Some(lab.clone()),
                predicted_entity: None,
            });
        }
    }
    // Predicted entities not consumed by any labeled → false positives.
    for (pi, pred) in predicted.iter().enumerate() {
        if !pred_consumed[pi] {
            out.push(EntityDiff {
                context: ctx(pred.offset.0, pred.offset.1),
                labeled_entity: None,
                predicted_entity: Some(pred.clone()),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(text: &str, type_: &str, s: usize, eo: usize) -> Entity {
        Entity {
            text: text.to_string(),
            type_: type_.to_string(),
            offset: (s, eo),
        }
    }

    #[test]
    fn boundary_diff_surfaces() {
        let text = "张三在西藏拉萨游玩！之后去新疆。";
        // Per-char offsets (char index, NOT byte).
        let labeled = vec![
            e("张三", "Person", 0, 2),
            e("西藏拉萨", "Location", 3, 7),
        ];
        let predicted = vec![
            e("张三在", "Person", 0, 3),
            e("西藏拉萨", "Location", 3, 7),
            e("新疆", "Location", 13, 15),
        ];
        let diffs = entity_compare_detailed(text, &labeled, &predicted, 1);
        // Expect: boundary mismatch for 张三/张三在 + false positive 新疆.
        assert_eq!(diffs.len(), 2);
        let first = &diffs[0];
        assert_eq!(first.labeled_entity.as_ref().unwrap().text, "张三");
        assert_eq!(first.predicted_entity.as_ref().unwrap().text, "张三在");
        let second = &diffs[1];
        assert!(second.labeled_entity.is_none());
        assert_eq!(second.predicted_entity.as_ref().unwrap().text, "新疆");
    }

    #[test]
    fn perfect_match_no_diff() {
        let text = "李四喜欢北京";
        let labeled = vec![
            e("李四", "Person", 0, 2),
            e("北京", "Location", 4, 6),
        ];
        let predicted = labeled.clone();
        let diffs = entity_compare_detailed(text, &labeled, &predicted, 2);
        assert!(diffs.is_empty());
    }
}
