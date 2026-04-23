//! Training-data correction for CWS and POS. Ports:
//!   * `algorithm/cws/cws_data_correction.py::CWSDCWithStandardWords`
//!   * `algorithm/pos/pos_data_correction.py::POSDCWithStandardWords`
//!
//! Idea: given a word-segmented sample and a list of "standard words" that
//! should always appear as a single token, re-tag the sample so standard
//! words override their local boundaries. Uses `LabeledTrie` for matching.

use crate::algorithm::tag_conversion::{cws, pos};
use crate::trie::LabeledTrie;

/// Correct a CWS sample: if a `standard_words` entry appears in the raw
/// concatenated text, force that span to be a single word.
pub fn correct_cws_sample(
    word_list: &[String],
    standard_words: &[String],
) -> Vec<String> {
    let mut trie: LabeledTrie<()> = LabeledTrie::new();
    for w in standard_words {
        trie.insert(w, ());
    }

    let (chars, tags) = cws::word2tag(word_list);
    let char_vec: Vec<char> = chars.chars().collect();
    let mut new_tags: Vec<&'static str> = tags.to_vec();

    let mut i = 0usize;
    let n = char_vec.len();
    while i < n {
        let remaining: String = char_vec[i..].iter().collect();
        let (step, matched) = trie.longest_prefix(&remaining);
        if matched.is_some() && step > 0 {
            new_tags[i] = "B";
            for k in (i + 1)..(i + step) {
                if k < n {
                    new_tags[k] = "I";
                }
            }
            if i + step < n {
                new_tags[i + step] = "B";
            }
            i += step;
        } else {
            i += 1;
        }
    }
    cws::tag2word(&chars, &new_tags)
}

/// Correct a POS sample. `standard_words` is a list of `(word, pos_tag)`
/// pairs — when `word` appears in the sample, its POS is forced to
/// `pos_tag` and its boundaries are re-asserted.
pub fn correct_pos_sample(
    pos_list: &[(String, String)],
    standard_words: &[(String, String)],
) -> Vec<(String, String)> {
    let mut trie: LabeledTrie<String> = LabeledTrie::new();
    for (w, p) in standard_words {
        trie.insert(w, p.clone());
    }

    let (chars, tags) = pos::pos2tag(pos_list);
    let char_vec: Vec<char> = chars.chars().collect();
    let mut new_tags: Vec<String> = tags.to_vec();

    let mut i = 0usize;
    let n = char_vec.len();
    while i < n {
        let remaining: String = char_vec[i..].iter().collect();
        let (step, label) = trie.longest_prefix(&remaining);
        if let Some(p_tag) = label {
            let p_tag = p_tag.clone();
            new_tags[i] = format!("B-{}", p_tag);
            for k in (i + 1)..(i + step) {
                if k < n {
                    new_tags[k] = format!("I-{}", p_tag);
                }
            }
            if i + step < n {
                // Re-seed next word as fresh "B-" (preserving existing POS).
                if let Some(rest) = new_tags[i + step].strip_prefix('I') {
                    new_tags[i + step] = format!("B{}", rest);
                }
            }
            i += step;
        } else {
            i += 1;
        }
    }
    pos::tag2pos(&chars, &new_tags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cws_correction_merges_standard_word() {
        // Input: ["学习", "区", "块链", "。"], standard 区块链 should merge.
        let words: Vec<String> = ["学习", "区", "块链", "。"]
            .iter().map(|s| s.to_string()).collect();
        let standards: Vec<String> = ["区块链".to_string(), "有条不紊".to_string()].to_vec();
        let out = correct_cws_sample(&words, &standards);
        assert!(out.contains(&"区块链".to_string()), "got {:?}", out);
    }

    #[test]
    fn pos_correction_sets_tag() {
        let input: Vec<(String, String)> = vec![
            ("学习".into(), "v".into()),
            ("区".into(), "n".into()),
            ("块链".into(), "n".into()),
            ("。".into(), "w".into()),
        ];
        let standards = vec![("区块链".to_string(), "n".to_string())];
        let out = correct_pos_sample(&input, &standards);
        assert!(
            out.iter().any(|(w, p)| w == "区块链" && p == "n"),
            "got {:?}",
            out
        );
    }
}
