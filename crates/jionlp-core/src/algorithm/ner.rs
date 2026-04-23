//! Lexicon-based NER — port of `jionlp/algorithm/ner/lexicon_ner.py`.
//!
//! Given a lexicon shaped as `{entity_type: [term, …]}`, build an
//! Aho-Corasick multi-pattern index and scan text in linear time,
//! returning all matches with their byte offsets.
//!
//! This is a *lexicon* NER — no statistical model. Use it for:
//!   * Domain-specific term recognition when you have curated lists
//!     (drug names, ticker symbols, product SKUs…)
//!   * Bootstrapping training data for a downstream model.
//!
//! For statistical NER (person/location/organization from free text),
//! integrate a transformer in your app layer. That's out of scope.

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use rustc_hash::FxHashMap;

/// One recognized entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NerEntity {
    pub text: String,
    pub entity_type: String,
    /// Byte offsets into the input string (start inclusive, end exclusive).
    pub offset: (usize, usize),
}

/// Builder-style lexicon: one [`LexiconNer`] per lexicon. Construction is
/// O(Σ term lengths); scanning is O(text length) per call.
pub struct LexiconNer {
    ac: AhoCorasick,
    /// Parallel to AC pattern ids: (term, type) for each pattern.
    patterns: Vec<(String, String)>,
}

impl LexiconNer {
    /// Build an index from a `type → [term, …]` map. Duplicate terms
    /// (same string, different types) are allowed — all matches are
    /// returned at scan time.
    pub fn new<I, T>(lexicon: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = (T, Vec<String>)>,
        T: Into<String>,
    {
        let mut patterns: Vec<String> = Vec::new();
        let mut meta: Vec<(String, String)> = Vec::new();
        for (entity_type, terms) in lexicon {
            let type_str: String = entity_type.into();
            for term in terms {
                if !term.is_empty() {
                    patterns.push(term.clone());
                    meta.push((term, type_str.clone()));
                }
            }
        }
        if patterns.is_empty() {
            return Err("empty lexicon".to_string());
        }
        let ac = AhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostLongest)
            .build(&patterns)
            .map_err(|e| format!("AC build: {e}"))?;
        Ok(LexiconNer { ac, patterns: meta })
    }

    /// Shortcut: build from a `FxHashMap` lexicon.
    pub fn from_map(lexicon: &FxHashMap<String, Vec<String>>) -> Result<Self, String> {
        let iter = lexicon.iter().map(|(k, v)| (k.clone(), v.clone()));
        Self::new(iter)
    }

    /// Scan `text` and return all matches in leftmost-longest order.
    pub fn recognize(&self, text: &str) -> Vec<NerEntity> {
        self.ac
            .find_iter(text)
            .map(|m| {
                let (term, entity_type) = &self.patterns[m.pattern().as_usize()];
                NerEntity {
                    text: term.clone(),
                    entity_type: entity_type.clone(),
                    offset: (m.start(), m.end()),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo() -> LexiconNer {
        let mut m: FxHashMap<String, Vec<String>> = FxHashMap::default();
        m.insert(
            "Drug".to_string(),
            vec!["阿司匹林".to_string(), "布洛芬".to_string()],
        );
        m.insert(
            "Company".to_string(),
            vec!["阿里巴巴".to_string(), "腾讯".to_string()],
        );
        LexiconNer::from_map(&m).unwrap()
    }

    #[test]
    fn finds_known_terms() {
        let ner = demo();
        let r = ner.recognize("他买了阿司匹林,去阿里巴巴面试,顺路吃了布洛芬。");
        assert_eq!(r.len(), 3);
        let texts: Vec<&str> = r.iter().map(|e| e.text.as_str()).collect();
        assert!(texts.contains(&"阿司匹林"));
        assert!(texts.contains(&"阿里巴巴"));
        assert!(texts.contains(&"布洛芬"));
    }

    #[test]
    fn offsets_roundtrip() {
        let ner = demo();
        let text = "去阿里巴巴";
        let r = ner.recognize(text);
        assert_eq!(r.len(), 1);
        let e = &r[0];
        assert_eq!(&text[e.offset.0..e.offset.1], "阿里巴巴");
        assert_eq!(e.entity_type, "Company");
    }

    #[test]
    fn empty_input_returns_empty() {
        let ner = demo();
        assert!(ner.recognize("").is_empty());
    }

    #[test]
    fn no_match_returns_empty() {
        let ner = demo();
        assert!(ner.recognize("一段没有实体的普通文本").is_empty());
    }

    #[test]
    fn leftmost_longest_wins() {
        // Longer match should win over shorter prefix.
        let mut m: FxHashMap<String, Vec<String>> = FxHashMap::default();
        m.insert(
            "Thing".to_string(),
            vec!["北京".to_string(), "北京大学".to_string()],
        );
        let ner = LexiconNer::from_map(&m).unwrap();
        let r = ner.recognize("我在北京大学上学");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].text, "北京大学");
    }
}
