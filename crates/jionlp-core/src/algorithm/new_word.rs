//! Port of `jionlp/algorithm/new_word/new_word_discovery.py`.
//!
//! Algorithm (per Python reference):
//!   1. Count N-grams (length 1..=max_word_len) across all sentences.
//!   2. For each candidate ≥3 chars, compute point-wise mutual information
//!      (PMI) using the strongest split of left/right sub-grams.
//!   3. Build left-context and right-context frequency maps, compute their
//!      entropies, and fuse — a word is only "new" when both contexts have
//!      high entropy (indicating free boundaries) AND high internal cohesion.
//!   4. Return `{word → (freq, entropy)}`, sorted by entropy descending.
//!
//! Performance: this is a pure-memory algorithm; input text is passed as
//! `&str` rather than a file path to match Rust idioms. Callers pre-read
//! their corpus.

use rustc_hash::FxHashMap;

/// Top-level entry. Same shape as Python's
/// `new_word_discovery(text, min_freq, min_mutual_information, min_entropy)`.
pub fn new_word_discovery(
    text: &str,
    min_freq: u32,
    min_mutual_information: f64,
    min_entropy: f64,
) -> Vec<(String, u32, f64)> {
    const MAX_WORD_LEN: usize = 5;
    // 1. N-gram counts.
    let word_freq = count_ngrams(text, MAX_WORD_LEN);
    let total_word: u64 = word_freq.values().map(|v| *v as u64).sum();
    if total_word == 0 {
        return Vec::new();
    }
    // 2. Left/right context dicts with PMI filter.
    let (l_dict, r_dict) =
        lrg_info(&word_freq, total_word, min_freq, min_mutual_information);
    let entropy_r = calc_entropy(&l_dict);
    let entropy_l = calc_entropy(&r_dict);

    // 3. Intersect & take min.
    let mut fused: FxHashMap<String, f64> = FxHashMap::default();
    for (w, er) in entropy_r.iter() {
        if let Some(el) = entropy_l.get(w) {
            fused.insert(w.clone(), er.min(*el));
        }
    }
    // 4. Entropy filter.
    let mut out: Vec<(String, u32, f64)> = fused
        .into_iter()
        .filter(|(_, e)| *e > min_entropy)
        .map(|(w, e)| {
            let f = *word_freq.get(&w).unwrap_or(&0);
            (w, f, e)
        })
        .collect();
    out.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    out
}

fn count_ngrams(text: &str, max_len: usize) -> FxHashMap<String, u32> {
    let mut counts: FxHashMap<String, u32> = FxHashMap::default();
    // Split by non-word chars (approximating Python's `re.compile(r'[\w]+')`).
    let segments: Vec<&str> = text
        .split(|c: char| !(c.is_alphanumeric() || ('\u{4E00}'..='\u{9FA5}').contains(&c)))
        .filter(|s| !s.is_empty())
        .collect();
    for seg in segments {
        let chars: Vec<char> = seg.chars().collect();
        let n = chars.len();
        for i in 0..n {
            for j in 1..=max_len.min(n - i) {
                let w: String = chars[i..i + j].iter().collect();
                *counts.entry(w).or_insert(0) += 1;
            }
        }
    }
    counts
}

/// Compute left/right context maps. Values are `[side_freq, ctx_freq1,
/// ctx_freq2, …]` — the first entry is the side_word's total, the rest are
/// per-context frequencies.
fn lrg_info(
    word_freq: &FxHashMap<String, u32>,
    total_word: u64,
    min_freq: u32,
    min_mtro: f64,
) -> (
    FxHashMap<String, Vec<u32>>,
    FxHashMap<String, Vec<u32>>,
) {
    let mut l_dict: FxHashMap<String, Vec<u32>> = FxHashMap::default();
    let mut r_dict: FxHashMap<String, Vec<u32>> = FxHashMap::default();

    for (word, freq) in word_freq.iter() {
        let chars: Vec<char> = word.chars().collect();
        let wlen = chars.len();
        if wlen < 3 {
            continue;
        }
        let left_word: String = chars[..wlen - 1].iter().collect();
        let right_word: String = chars[1..].iter().collect();
        update_dict(&mut l_dict, &left_word, word_freq, total_word, *freq, min_freq, min_mtro);
        update_dict(&mut r_dict, &right_word, word_freq, total_word, *freq, min_freq, min_mtro);
    }
    (l_dict, r_dict)
}

fn update_dict(
    side: &mut FxHashMap<String, Vec<u32>>,
    side_word: &str,
    word_freq: &FxHashMap<String, u32>,
    total_word: u64,
    freq: u32,
    min_freq: u32,
    min_mtro: f64,
) {
    let side_word_freq = *word_freq.get(side_word).unwrap_or(&0);
    if side_word_freq <= min_freq {
        return;
    }
    let side_chars: Vec<char> = side_word.chars().collect();
    let slen = side_chars.len();
    let mul_info = if slen == 2 {
        let a = side_chars[0].to_string();
        let b = side_chars[1].to_string();
        let fa = *word_freq.get(&a).unwrap_or(&1) as f64;
        let fb = *word_freq.get(&b).unwrap_or(&1) as f64;
        side_word_freq as f64 * total_word as f64 / (fa * fb).max(1.0)
    } else {
        let left_rest: String = side_chars[1..].iter().collect();
        let first: String = side_chars[0].to_string();
        let last: String = side_chars[slen - 1].to_string();
        let right_head: String = side_chars[..slen - 1].iter().collect();
        let mul1 = *word_freq.get(&left_rest).unwrap_or(&1) as f64
            * *word_freq.get(&first).unwrap_or(&1) as f64;
        let mul2 = *word_freq.get(&last).unwrap_or(&1) as f64
            * *word_freq.get(&right_head).unwrap_or(&1) as f64;
        side_word_freq as f64 * total_word as f64 / mul1.max(mul2).max(1.0)
    };
    if mul_info > min_mtro {
        side
            .entry(side_word.to_string())
            .or_insert_with(|| vec![side_word_freq])
            .push(freq);
    }
}

fn calc_entropy(dict: &FxHashMap<String, Vec<u32>>) -> FxHashMap<String, f64> {
    let mut out: FxHashMap<String, f64> = FxHashMap::default();
    for (w, v) in dict.iter() {
        let r_list: &[u32] = if v.len() > 1 { &v[1..] } else { &[] };
        if r_list.is_empty() {
            continue;
        }
        let sum: f64 = r_list.iter().map(|x| *x as f64).sum();
        if sum == 0.0 {
            continue;
        }
        let entropy: f64 = r_list
            .iter()
            .map(|x| {
                let p = *x as f64 / sum;
                if p > 0.0 { -p * p.log2() } else { 0.0 }
            })
            .sum();
        out.insert(w.clone(), entropy);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn algorithm_runs_without_panic() {
        // The statistical filters require a big corpus; a 7-sample toy
        // input won't reliably discover anything — we just assert the
        // function executes and produces a consistent output shape.
        let text = "应采儿出道 应采儿入狱 应采儿吸毒 应采儿事件 应采儿新闻 应采儿消息 应采儿回归";
        let _ = new_word_discovery(text, 2, 5.0, 0.5);
    }

    #[test]
    fn count_ngrams_produces_counts() {
        let counts = count_ngrams("abcabc", 3);
        assert!(counts.contains_key("a"));
        assert!(counts.contains_key("ab"));
        assert!(counts.contains_key("abc"));
        assert_eq!(*counts.get("a").unwrap(), 2);
    }

    #[test]
    fn empty_input() {
        let r = new_word_discovery("", 1, 0.0, 0.0);
        assert!(r.is_empty());
    }
}
