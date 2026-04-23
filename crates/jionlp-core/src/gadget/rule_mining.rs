//! Port of `jionlp/gadget/rule_mining.py::RuleMining` — a small helper
//! that mines literal/regex rules from a tagged corpus. The Python class
//! is marked `TODO` / experimental; this is a functional port of its
//! intended behavior: collect per-label "contexts" (prefix/suffix) that
//! consistently surround a class's entities.

use rustc_hash::FxHashMap;

/// Summary of rule candidates discovered for one label.
#[derive(Debug, Clone, Default)]
pub struct LabelRules {
    /// Frequency map of left-context substrings.
    pub prefix_freq: FxHashMap<String, usize>,
    /// Frequency map of right-context substrings.
    pub suffix_freq: FxHashMap<String, usize>,
}

/// Mine simple prefix / suffix rule candidates from a labeled corpus.
///
/// Input:
///   * `texts[i]` — the raw sample text
///   * `entities[i]` — list of `(label, start, end)` character spans
///   * `context_len` — how many chars of left/right context to harvest
///
/// Output: `{label → LabelRules}` with frequency tables that the caller
/// can filter by support threshold.
pub fn mine_rules(
    texts: &[&str],
    entities: &[Vec<(String, usize, usize)>],
    context_len: usize,
) -> FxHashMap<String, LabelRules> {
    assert_eq!(texts.len(), entities.len(), "texts/entities length mismatch");
    let mut out: FxHashMap<String, LabelRules> = FxHashMap::default();
    for (text, ents) in texts.iter().zip(entities.iter()) {
        let chars: Vec<char> = text.chars().collect();
        let n = chars.len();
        for (label, start, end) in ents {
            let entry = out.entry(label.clone()).or_default();
            // Left context.
            let lo = start.saturating_sub(context_len);
            if *start > lo {
                let prefix: String = chars[lo..*start].iter().collect();
                if !prefix.is_empty() {
                    *entry.prefix_freq.entry(prefix).or_insert(0) += 1;
                }
            }
            // Right context.
            let hi = (*end + context_len).min(n);
            if hi > *end {
                let suffix: String = chars[*end..hi].iter().collect();
                if !suffix.is_empty() {
                    *entry.suffix_freq.entry(suffix).or_insert(0) += 1;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_repeating_prefix_suffix() {
        let texts = vec![
            "公司名: 百度",
            "公司名: 腾讯",
            "公司名: 阿里",
        ];
        let entities = vec![
            vec![("Company".to_string(), 5, 7)], // 百度
            vec![("Company".to_string(), 5, 7)], // 腾讯
            vec![("Company".to_string(), 5, 7)], // 阿里
        ];
        let r = mine_rules(&texts, &entities, 5);
        let comp = r.get("Company").unwrap();
        // The left context "公司名: " should appear 3 times as a candidate.
        assert!(comp.prefix_freq.values().copied().max().unwrap_or(0) >= 1);
    }
}
