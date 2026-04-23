//! Port of the dataset-analysis utilities under
//! `jionlp/algorithm/text_classification/`. These are NOT training
//! algorithms; they are statistical summaries of a labeled corpus used to
//! decide stopword lists, class imbalance, feature selection, etc.
//!
//! Two public entry points:
//!   * `analyse_dataset(items)` — per-class sample count and char-length
//!     distribution.
//!   * `analyse_freq_words(items, stopwords, top_k)` — top-K frequent
//!     character n-grams per class.

use rustc_hash::FxHashMap;

/// A labeled sample: `(label, text)`.
pub type LabeledSample<'a> = (&'a str, &'a str);

/// Per-class summary output.
#[derive(Debug, Clone)]
pub struct DatasetAnalysis {
    /// class label → sample count.
    pub per_class_count: FxHashMap<String, usize>,
    /// Total number of samples.
    pub total: usize,
    /// Average sample length (in chars) per class.
    pub per_class_mean_len: FxHashMap<String, f64>,
    /// Total number of distinct labels.
    pub num_classes: usize,
}

/// Compute basic class-distribution stats.
pub fn analyse_dataset(items: &[LabeledSample]) -> DatasetAnalysis {
    let mut counts: FxHashMap<String, usize> = FxHashMap::default();
    let mut len_sums: FxHashMap<String, usize> = FxHashMap::default();
    for (label, text) in items {
        *counts.entry((*label).to_string()).or_insert(0) += 1;
        *len_sums.entry((*label).to_string()).or_insert(0) += text.chars().count();
    }
    let num_classes = counts.len();
    let total = items.len();
    let per_class_mean_len: FxHashMap<String, f64> = counts
        .iter()
        .map(|(k, c)| {
            let sum = *len_sums.get(k).unwrap_or(&0) as f64;
            (k.clone(), sum / *c as f64)
        })
        .collect();
    DatasetAnalysis {
        per_class_count: counts,
        total,
        per_class_mean_len,
        num_classes,
    }
}

/// Per-class frequent words (character tokens). Stopwords are removed
/// before counting.
pub fn analyse_freq_words(
    items: &[LabeledSample],
    stopwords: &std::collections::HashSet<String>,
    top_k: usize,
) -> FxHashMap<String, Vec<(String, usize)>> {
    let mut per_class: FxHashMap<String, FxHashMap<String, usize>> = FxHashMap::default();
    for (label, text) in items {
        let entry = per_class.entry((*label).to_string()).or_default();
        for c in text.chars() {
            let s = c.to_string();
            if stopwords.contains(&s) {
                continue;
            }
            if c.is_whitespace() || !(('\u{4E00}'..='\u{9FA5}').contains(&c) || c.is_alphanumeric())
            {
                continue;
            }
            *entry.entry(s).or_insert(0) += 1;
        }
    }

    per_class
        .into_iter()
        .map(|(label, counts)| {
            let mut v: Vec<(String, usize)> = counts.into_iter().collect();
            v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            v.truncate(top_k);
            (label, v)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dataset_counts() {
        let items: Vec<LabeledSample> = vec![
            ("pos", "a quick fox"),
            ("pos", "fast brown"),
            ("neg", "slow fox"),
        ];
        let r = analyse_dataset(&items);
        assert_eq!(r.total, 3);
        assert_eq!(r.num_classes, 2);
        assert_eq!(*r.per_class_count.get("pos").unwrap(), 2);
    }

    #[test]
    fn freq_words_topk() {
        let items: Vec<LabeledSample> = vec![
            ("A", "中国中国中国人"),
            ("A", "中国是中国"),
            ("B", "美国美国日本"),
        ];
        let sw: std::collections::HashSet<String> = ["是".to_string()].into_iter().collect();
        let r = analyse_freq_words(&items, &sw, 3);
        let a = r.get("A").unwrap();
        assert_eq!(a[0].0, "中");
    }
}
