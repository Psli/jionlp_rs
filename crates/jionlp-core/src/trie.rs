//! Trie abstraction used by ts_conversion (and later parse_location).
//!
//! Semantically equivalent to `jionlp/gadget/trie_tree.py`:
//! greedy longest-prefix match, returning the matched byte length and the
//! label attached to that entry.
//!
//! Implementation uses a nested `FxHashMap` tree keyed by Unicode scalar
//! values. For large pattern sets (> 10k entries, e.g. China location
//! dictionary) consider switching to `aho-corasick::AhoCorasick` with
//! `LeftmostLongest` match kind — same API surface, faster at scale.

use rustc_hash::FxHashMap;

/// A label-carrying trie. The label type `L` is attached to terminal nodes.
#[derive(Debug, Clone)]
pub struct LabeledTrie<L: Clone> {
    root: Node<L>,
    depth: usize,
}

#[derive(Debug, Clone)]
struct Node<L: Clone> {
    children: FxHashMap<char, Node<L>>,
    label: Option<L>,
}

impl<L: Clone> Default for Node<L> {
    fn default() -> Self {
        Self {
            children: FxHashMap::default(),
            label: None,
        }
    }
}

impl<L: Clone> Default for LabeledTrie<L> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L: Clone> LabeledTrie<L> {
    pub fn new() -> Self {
        Self {
            root: Node {
                children: FxHashMap::default(),
                label: None,
            },
            depth: 0,
        }
    }

    /// Maximum number of chars in any inserted pattern.
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Insert a pattern with its label. Overwrites any previous label.
    pub fn insert(&mut self, pattern: &str, label: L) {
        let pattern = pattern.trim();
        if pattern.is_empty() {
            return;
        }
        let chars: Vec<char> = pattern.chars().collect();
        if chars.len() > self.depth {
            self.depth = chars.len();
        }
        let mut node = &mut self.root;
        for c in chars {
            node = node.children.entry(c).or_default();
        }
        node.label = Some(label);
    }

    /// Search for the longest prefix of `text` that exists in the trie.
    /// Returns `(char_count, Some(label))` if a label was found, or
    /// `(1, None)` to indicate "no match — consumer should advance 1 char".
    ///
    /// Mirrors the Python `search` method's `(step, typing)` return shape.
    pub fn longest_prefix(&self, text: &str) -> (usize, Option<&L>) {
        let mut node = &self.root;
        let mut best: Option<(usize, &L)> = None;

        for (i, c) in text.chars().enumerate() {
            match node.children.get(&c) {
                Some(next) => {
                    node = next;
                    if let Some(lbl) = &node.label {
                        best = Some((i + 1, lbl));
                    }
                }
                None => break,
            }
        }
        match best {
            Some((n, l)) => (n, Some(l)),
            None => (1, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_match() {
        let mut t: LabeledTrie<&'static str> = LabeledTrie::new();
        t.insert("速食麵", "tra");
        t.insert("太空梭", "tra");
        assert_eq!(t.depth(), 3);

        let (n, l) = t.longest_prefix("速食麵好吃");
        assert_eq!(n, 3);
        assert_eq!(l, Some(&"tra"));
    }

    #[test]
    fn longest_match_preferred() {
        let mut t: LabeledTrie<&'static str> = LabeledTrie::new();
        t.insert("中国", "a");
        t.insert("中国人", "b");
        let (n, l) = t.longest_prefix("中国人民");
        assert_eq!(n, 3);
        assert_eq!(l, Some(&"b"));
    }

    #[test]
    fn no_match_returns_one_none() {
        let t: LabeledTrie<&'static str> = LabeledTrie::new();
        let (n, l) = t.longest_prefix("abc");
        assert_eq!(n, 1);
        assert_eq!(l, None);
    }

    #[test]
    fn empty_pattern_ignored() {
        let mut t: LabeledTrie<&'static str> = LabeledTrie::new();
        t.insert("", "x");
        t.insert("  ", "x");
        assert_eq!(t.depth(), 0);
    }

    #[test]
    fn ascii_and_cjk_mixed() {
        let mut t: LabeledTrie<i32> = LabeledTrie::new();
        t.insert("U盘", 1);
        let (n, l) = t.longest_prefix("U盘很快");
        assert_eq!(n, 2);
        assert_eq!(l, Some(&1));
    }
}
