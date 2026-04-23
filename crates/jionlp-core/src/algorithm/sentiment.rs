//! Lexicon-based sentiment analysis — simplified port of
//! `jionlp/algorithm/sentiment/sentiment_analysis.py`.
//!
//! Per-sentence scoring, aggregated and squashed to [0, 1] via sigmoid:
//! 0 = strongly negative, 0.5 = neutral, 1 = strongly positive.
//!
//! ## Algorithm
//!
//! 1. Split input into sentences (coarse criterion).
//! 2. For each sentence, scan with Aho-Corasick over the union of
//!    sentiment / negative / adverb-multiplier lexicons.
//! 3. Walk matches left-to-right, accumulating a per-sentence sum while
//!    tracking two "modifier carries":
//!    * `not_mul = -1.0` when a recent negation word was seen (resets
//!      after the next sentiment word consumes it).
//!    * `adv_mul` = adverb multiplier from the most recent adverb.
//! 4. Negative scores are doubled (Python convention — negative emotions
//!    are felt more sharply than positive ones).
//! 5. Sentence scores are averaged, then sigmoided.

use crate::dict;
use crate::gadget::split_sentence::{split_sentence, Criterion as SplitCriterion};
use crate::Result;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use once_cell::sync::OnceCell;

#[derive(Debug, Clone, Copy, PartialEq)]
enum LexKind {
    Sentiment,
    Negation,
    Adverb,
}

struct SentimentIndex {
    ac: AhoCorasick,
    kinds: Vec<LexKind>,
    /// Parallel to patterns, the matched string so we can re-look it up
    /// without storing another table inside AC.
    patterns: Vec<String>,
}

static INDEX: OnceCell<SentimentIndex> = OnceCell::new();

fn index() -> Result<&'static SentimentIndex> {
    INDEX.get_or_try_init(|| {
        let senti = dict::sentiment_words()?;
        let neg = dict::negative_words()?;
        let adv = dict::sentiment_expand_words()?;

        let mut patterns: Vec<String> = Vec::with_capacity(senti.len() + neg.len() + adv.len());
        let mut kinds: Vec<LexKind> = Vec::with_capacity(patterns.capacity());

        for k in senti.keys() {
            patterns.push(k.clone());
            kinds.push(LexKind::Sentiment);
        }
        for k in neg.iter() {
            patterns.push(k.clone());
            kinds.push(LexKind::Negation);
        }
        for k in adv.keys() {
            patterns.push(k.clone());
            kinds.push(LexKind::Adverb);
        }

        let ac = AhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostLongest)
            .build(&patterns)
            .map_err(|e| crate::Error::InvalidArg(format!("AC build (sentiment): {e}")))?;

        Ok(SentimentIndex { ac, kinds, patterns })
    })
}

/// Score `text` sentiment on `[0, 1]`. Empty input returns 0.5 (neutral).
pub fn sentiment_score(text: &str) -> Result<f64> {
    if text.trim().is_empty() {
        return Ok(0.5);
    }
    let senti = dict::sentiment_words()?;
    let adv = dict::sentiment_expand_words()?;
    let idx = index()?;

    let sentences = split_sentence(text, SplitCriterion::Coarse);
    if sentences.is_empty() {
        return Ok(0.5);
    }

    let mut total_raw: f64 = 0.0;
    let mut seen_sentences = 0usize;

    for sentence in sentences {
        seen_sentences += 1;

        let mut not_mul: f64 = 1.0;
        let mut adv_mul: f64 = 1.0;
        let mut sentence_val: f64 = 0.0;

        for m in idx.ac.find_iter(sentence.as_str()) {
            let pid = m.pattern().as_usize();
            let word = &idx.patterns[pid];
            match idx.kinds[pid] {
                LexKind::Sentiment => {
                    let mut v = *senti.get(word).unwrap_or(&0.0);
                    if adv_mul != 1.0 {
                        v *= adv_mul;
                    }
                    if not_mul != 1.0 {
                        v *= not_mul;
                    }
                    // Python: amplify negative sentiments (felt more strongly).
                    if v < 0.0 {
                        v *= 2.0;
                    }
                    sentence_val += v;
                    not_mul = 1.0;
                    adv_mul = 1.0;
                }
                LexKind::Negation => {
                    not_mul = -1.0;
                }
                LexKind::Adverb => {
                    adv_mul = *adv.get(word).unwrap_or(&1.0);
                }
            }
        }

        total_raw += sentence_val;
    }

    let avg = total_raw / seen_sentences as f64;
    Ok(sigmoid(avg))
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict;
    use std::path::PathBuf;
    use std::sync::Once;

    static INIT: Once = Once::new();
    fn ensure_init() {
        INIT.call_once(|| {
            let manifest = env!("CARGO_MANIFEST_DIR");
            let d = PathBuf::from(manifest).join("data");
            dict::init_from_path(&d).expect("init");
        });
    }

    #[test]
    fn empty_text_is_neutral() {
        ensure_init();
        let s = sentiment_score("").unwrap();
        assert!((s - 0.5).abs() < 1e-9);
    }

    #[test]
    fn positive_text_scores_above_neutral() {
        ensure_init();
        // 美好 is in the sentiment dict with positive weight.
        let s = sentiment_score("今天是美好的一天,我非常开心。").unwrap();
        assert!(s > 0.5, "expected positive, got {s}");
    }

    #[test]
    fn negative_text_scores_below_neutral() {
        ensure_init();
        let s = sentiment_score("事故造成严重伤亡,令人悲痛万分。").unwrap();
        assert!(s < 0.5, "expected negative, got {s}");
    }

    #[test]
    fn negation_flips_sign() {
        ensure_init();
        let pos = sentiment_score("他很开心").unwrap();
        let neg = sentiment_score("他不开心").unwrap();
        // Python's negation multiplies by -1 which typically pushes past 0.5;
        // verify relative ordering rather than absolute thresholds.
        assert!(neg < pos, "{neg} should be < {pos}");
    }

    #[test]
    fn adverb_amplifies() {
        ensure_init();
        let base = sentiment_score("今天开心").unwrap();
        let amplified = sentiment_score("今天非常开心").unwrap();
        // The adverb multiplier (~1.6) on a positive word should push further above 0.5.
        assert!(amplified >= base, "{amplified} should be >= {base}");
    }
}
