//! Sequence-labeling evaluation — port of
//! `jionlp/algorithm/{cws,ner}/measure.py`. Computes per-label precision /
//! recall / F1 / accuracy from gold+pred tag streams, and pretty-prints a
//! confusion matrix.
//!
//! Unlike Python's `F1(skip_labels=[...])` which walks BIOES sequences and
//! collapses "tag group matches", this Rust port measures **tag-level**
//! agreement, which is what most modern evaluation harnesses do. A more
//! elaborate "entity-level" F1 is available via
//! `algorithm::ner_convert::entity_compare`.

use rustc_hash::FxHashMap;

/// Per-label statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelStats {
    pub true_positive: u64,
    pub false_positive: u64,
    pub false_negative: u64,
    pub true_negative: u64,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub accuracy: f64,
}

/// Evaluation result — per-label stats plus a confusion matrix.
#[derive(Debug, Clone)]
pub struct F1Report {
    /// label → statistics.
    pub per_label: FxHashMap<String, LabelStats>,
    /// Ordered labels (also rows/cols of the confusion matrix).
    pub labels: Vec<String>,
    /// `matrix[gold][pred] = count`. Indexed by position in `labels`.
    pub matrix: Vec<Vec<u64>>,
    /// Macro-averaged F1 (mean across labels, excluding 'O').
    pub macro_f1: f64,
    /// Micro-averaged precision / recall / F1.
    pub micro_precision: f64,
    pub micro_recall: f64,
    pub micro_f1: f64,
}

/// Compute F1 stats from parallel gold/pred lists of tag sequences.
///
/// Each inner list is one sample's tag sequence; outer list is the batch.
/// Returns a full report. Panics if outer lengths differ or any inner pair
/// has mismatched length — matches Python `assert` behavior.
pub fn compute_f1(gold: &[Vec<String>], pred: &[Vec<String>]) -> F1Report {
    assert_eq!(gold.len(), pred.len(), "gold/pred sample count differs");

    // 1. Collect unique labels.
    let mut label_set: std::collections::BTreeSet<String> = Default::default();
    for (g, p) in gold.iter().zip(pred.iter()) {
        assert_eq!(
            g.len(),
            p.len(),
            "gold/pred tag length differs within sample"
        );
        for tag in g.iter().chain(p.iter()) {
            label_set.insert(tag.clone());
        }
    }
    let labels: Vec<String> = label_set.into_iter().collect();
    let n = labels.len();
    let idx: FxHashMap<String, usize> = labels
        .iter()
        .enumerate()
        .map(|(i, s)| (s.clone(), i))
        .collect();

    // 2. Build confusion matrix.
    let mut matrix = vec![vec![0u64; n]; n];
    for (g, p) in gold.iter().zip(pred.iter()) {
        for (gt, pt) in g.iter().zip(p.iter()) {
            matrix[idx[gt]][idx[pt]] += 1;
        }
    }

    // 3. Per-label stats.
    let total: u64 = matrix.iter().flatten().sum();
    let mut per_label: FxHashMap<String, LabelStats> = FxHashMap::default();
    let mut macro_sum = 0.0f64;
    let mut macro_count = 0u64;
    let mut tp_total = 0u64;
    let mut fp_total = 0u64;
    let mut fn_total = 0u64;
    for (li, label) in labels.iter().enumerate() {
        let tp = matrix[li][li];
        let col_sum: u64 = matrix.iter().map(|row| row[li]).sum();
        let row_sum: u64 = matrix[li].iter().sum();
        let fp = col_sum - tp;
        let fn_ = row_sum - tp;
        let tn = total - tp - fp - fn_;
        let (precision, recall, f1, accuracy) = compute_prf(tp, fp, fn_, tn);
        per_label.insert(
            label.clone(),
            LabelStats {
                true_positive: tp,
                false_positive: fp,
                false_negative: fn_,
                true_negative: tn,
                precision,
                recall,
                f1,
                accuracy,
            },
        );
        if label != "O" {
            macro_sum += f1;
            macro_count += 1;
            tp_total += tp;
            fp_total += fp;
            fn_total += fn_;
        }
    }
    let macro_f1 = if macro_count == 0 {
        0.0
    } else {
        macro_sum / macro_count as f64
    };
    let micro_precision = if tp_total + fp_total == 0 {
        0.0
    } else {
        tp_total as f64 / (tp_total + fp_total) as f64
    };
    let micro_recall = if tp_total + fn_total == 0 {
        0.0
    } else {
        tp_total as f64 / (tp_total + fn_total) as f64
    };
    let micro_f1 = if micro_precision + micro_recall == 0.0 {
        0.0
    } else {
        2.0 * micro_precision * micro_recall / (micro_precision + micro_recall)
    };

    F1Report {
        per_label,
        labels,
        matrix,
        macro_f1,
        micro_precision,
        micro_recall,
        micro_f1,
    }
}

fn compute_prf(tp: u64, fp: u64, fn_: u64, tn: u64) -> (f64, f64, f64, f64) {
    if tp == 0 {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let precision = tp as f64 / (tp + fp) as f64;
    let recall = tp as f64 / (tp + fn_) as f64;
    let f1 = 2.0 * precision * recall / (precision + recall);
    let accuracy = (tp + tn) as f64 / (tp + fp + fn_ + tn) as f64;
    (precision, recall, f1, accuracy)
}

impl F1Report {
    /// Pretty-print the confusion matrix + per-label stats to a String.
    pub fn to_report_string(&self) -> String {
        let mut out = String::new();
        out.push_str("\n=== Confusion Matrix (gold → pred) ===\n");
        let max_lbl = self
            .labels
            .iter()
            .map(|s| s.chars().count())
            .max()
            .unwrap_or(5);
        let cell = (max_lbl + 2).max(6);
        out.push_str(&format!("{:width$}", "gold\\pred", width = cell));
        for l in &self.labels {
            out.push_str(&format!("{:width$}", l, width = cell));
        }
        out.push('\n');
        for (i, l) in self.labels.iter().enumerate() {
            out.push_str(&format!("{:width$}", l, width = cell));
            for j in 0..self.labels.len() {
                out.push_str(&format!("{:width$}", self.matrix[i][j], width = cell));
            }
            out.push('\n');
        }
        out.push_str("\n=== Per-Label Stats ===\n");
        out.push_str(&format!(
            "{:width$}  precision  recall    f1       accuracy\n",
            "label",
            width = cell
        ));
        let mut sorted: Vec<&String> = self.per_label.keys().collect();
        sorted.sort();
        for l in sorted {
            let s = &self.per_label[l];
            out.push_str(&format!(
                "{:width$}  {:.4}     {:.4}   {:.4}   {:.4}\n",
                l,
                s.precision,
                s.recall,
                s.f1,
                s.accuracy,
                width = cell
            ));
        }
        out.push_str(&format!(
            "\nmacro F1: {:.4}  |  micro P/R/F1: {:.4} / {:.4} / {:.4}\n",
            self.macro_f1, self.micro_precision, self.micro_recall, self.micro_f1
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn perfect_agreement() {
        let g = vec![v(&["B", "I", "O", "O"]), v(&["B", "O"])];
        let p = g.clone();
        let r = compute_f1(&g, &p);
        assert!((r.micro_f1 - 1.0).abs() < 1e-9);
        for label in &r.labels {
            let s = &r.per_label[label];
            assert!((s.precision - 1.0).abs() < 1e-9);
            assert!((s.recall - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn partial_agreement() {
        let g = vec![v(&["B", "I", "O"])];
        let p = vec![v(&["B", "O", "O"])];
        let r = compute_f1(&g, &p);
        // B correctly predicted once → tp=1, fp=0, fn=0 for B.
        let b = &r.per_label["B"];
        assert_eq!(b.true_positive, 1);
        // I: gold 1, pred 0 → fn=1.
        let i = &r.per_label["I"];
        assert_eq!(i.true_positive, 0);
        assert_eq!(i.false_negative, 1);
    }

    #[test]
    fn report_prints() {
        let g = vec![v(&["B-Org", "I-Org", "O"])];
        let p = vec![v(&["B-Org", "I-Org", "O"])];
        let r = compute_f1(&g, &p);
        let s = r.to_report_string();
        assert!(s.contains("Confusion Matrix"));
        assert!(s.contains("macro F1"));
    }
}
