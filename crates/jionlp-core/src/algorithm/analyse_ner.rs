//! Detailed NER dataset analyser — port of
//! `jionlp/algorithm/ner/analyse_dataset.py::analyse_dataset`.
//!
//! Splits a labeled NER corpus into train / valid / test sets, reports
//! per-class counts and proportion, and measures each split's KL divergence
//! against the whole dataset's class distribution. Retries up to 3 times
//! when the split produces a skewed distribution (Python behavior).

use crate::algorithm::tag_conversion::Entity;
use crate::textaug::prng::SplitMix64;
use rustc_hash::FxHashMap;

/// Per-class statistics: `count` and `proportion`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClassStat {
    pub count: usize,
    pub proportion: f64,
}

/// Aggregated dataset statistics — matches Python's `stats` dict shape.
#[derive(Debug, Clone)]
pub struct DatasetStats {
    pub total: FxHashMap<String, ClassStat>,
    pub train: FxHashMap<String, ClassStat>,
    pub valid: FxHashMap<String, ClassStat>,
    pub test: FxHashMap<String, ClassStat>,
    /// `Some((kl, info_dismatch_ratio))` when all three splits share the
    /// same class set; `None` when split was skewed (retry failed 3×).
    pub train_kl: Option<(f64, f64)>,
    pub valid_kl: Option<(f64, f64)>,
    pub test_kl: Option<(f64, f64)>,
}

/// Output of `analyse_ner_dataset_split`. Owns all three splits plus stats.
pub struct SplitResult<X> {
    pub train_x: Vec<X>,
    pub train_y: Vec<Vec<Entity>>,
    pub valid_x: Vec<X>,
    pub valid_y: Vec<Vec<Entity>>,
    pub test_x: Vec<X>,
    pub test_y: Vec<Vec<Entity>>,
    pub stats: DatasetStats,
}

/// Split a labeled NER dataset `(x, y)` into train/valid/test by `ratio`.
/// Retries up to 3 times when a split produces class-imbalance > 5% KL
/// info-loss. `seed` controls deterministic shuffling.
pub fn analyse_ner_dataset_split<X: Clone>(
    dataset_x: &[X],
    dataset_y: &[Vec<Entity>],
    ratio: (f64, f64, f64),
    seed: u64,
    shuffle: bool,
) -> SplitResult<X> {
    assert_eq!(dataset_x.len(), dataset_y.len(), "x/y length mismatch");
    let mut indices: Vec<usize> = (0..dataset_x.len()).collect();
    let mut rng = SplitMix64::new(seed);

    let total_stat = stat_class(dataset_y);

    for attempt in 0..3 {
        if shuffle {
            // Fisher-Yates shuffle deterministic via rng.
            for i in (1..indices.len()).rev() {
                let j = (rng.next_u64() as usize) % (i + 1);
                indices.swap(i, j);
            }
        }
        let n = indices.len();
        let n_train = (n as f64 * ratio.0) as usize;
        let n_valid = (n as f64 * ratio.1) as usize;
        let train_idx = &indices[..n_train];
        let valid_idx = &indices[n_train..n_train + n_valid];
        let test_idx = &indices[n_train + n_valid..];

        let train_y: Vec<Vec<Entity>> = train_idx.iter().map(|&i| dataset_y[i].clone()).collect();
        let valid_y: Vec<Vec<Entity>> = valid_idx.iter().map(|&i| dataset_y[i].clone()).collect();
        let test_y: Vec<Vec<Entity>> = test_idx.iter().map(|&i| dataset_y[i].clone()).collect();
        let train_stat = stat_class(&train_y);
        let valid_stat = stat_class(&valid_y);
        let test_stat = stat_class(&test_y);

        let same_classes = train_stat.len() == valid_stat.len()
            && valid_stat.len() == test_stat.len()
            && train_stat.len() == total_stat.len();
        if !same_classes && attempt < 2 {
            continue;
        }

        let (train_kl, valid_kl, test_kl) = if same_classes {
            (
                Some(kl_divergence(&total_stat, &train_stat)),
                Some(kl_divergence(&total_stat, &valid_stat)),
                Some(kl_divergence(&total_stat, &test_stat)),
            )
        } else {
            (None, None, None)
        };

        // If any split's info-loss > 5% AND we still have retries, redo.
        if let (Some((_, tr)), Some((_, vr)), Some((_, te))) =
            (train_kl, valid_kl, test_kl)
        {
            if (tr > 0.05 || vr > 0.05 || te > 0.05) && attempt < 2 {
                continue;
            }
        }

        let train_x: Vec<X> = train_idx.iter().map(|&i| dataset_x[i].clone()).collect();
        let valid_x: Vec<X> = valid_idx.iter().map(|&i| dataset_x[i].clone()).collect();
        let test_x: Vec<X> = test_idx.iter().map(|&i| dataset_x[i].clone()).collect();

        return SplitResult {
            train_x,
            train_y,
            valid_x,
            valid_y,
            test_x,
            test_y,
            stats: DatasetStats {
                total: total_stat,
                train: train_stat,
                valid: valid_stat,
                test: test_stat,
                train_kl,
                valid_kl,
                test_kl,
            },
        };
    }
    unreachable!("loop exits via return or falls through at attempt==2")
}

fn stat_class(dataset_y: &[Vec<Entity>]) -> FxHashMap<String, ClassStat> {
    let mut counts: FxHashMap<String, usize> = FxHashMap::default();
    for sample in dataset_y {
        for e in sample {
            *counts.entry(e.type_.clone()).or_insert(0) += 1;
        }
    }
    let total: usize = counts.values().sum();
    counts
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                ClassStat {
                    count: v,
                    proportion: if total == 0 { 0.0 } else { v as f64 / total as f64 },
                },
            )
        })
        .collect()
}

/// Compute KL(p || q) and normalize by p's entropy for an "info dismatch
/// ratio" (matches Python convention). Both maps must have the same keys.
fn kl_divergence(
    p: &FxHashMap<String, ClassStat>,
    q: &FxHashMap<String, ClassStat>,
) -> (f64, f64) {
    let mut kl = 0.0f64;
    let mut entropy = 0.0f64;
    for (k, ps) in p {
        let p1 = ps.proportion.max(1e-12);
        let q1 = q.get(k).map(|s| s.proportion).unwrap_or(1e-12).max(1e-12);
        kl += p1 * (p1 / q1).log2();
        entropy += p1 * (1.0 / p1).log2();
    }
    let ratio = if entropy.abs() < 1e-12 { 0.0 } else { kl / entropy };
    (kl, ratio)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(type_: &str) -> Entity {
        Entity {
            text: "x".into(),
            type_: type_.into(),
            offset: (0, 1),
        }
    }

    #[test]
    fn balanced_split_stats() {
        // Build a tiny uniform dataset: 10 samples, alternating types.
        let x: Vec<String> = (0..100).map(|i| format!("sample{}", i)).collect();
        let y: Vec<Vec<Entity>> = (0..100)
            .map(|i| {
                vec![if i % 2 == 0 { e("A") } else { e("B") }]
            })
            .collect();
        let r = analyse_ner_dataset_split(&x, &y, (0.8, 0.1, 0.1), 42, true);
        assert_eq!(r.train_x.len(), 80);
        assert_eq!(r.valid_x.len(), 10);
        assert_eq!(r.test_x.len(), 10);
        assert_eq!(r.stats.total.len(), 2);
        assert!(r.stats.train.contains_key("A"));
        assert!(r.stats.train.contains_key("B"));
    }
}
