//! Entity replacement augmentation — port of
//! `jionlp/textaug/replace_entity.py`.
//!
//! Given a text, a list of its entities (`NamedEntity { text, entity_type,
//! offset }`), and an `EntityReplacement` lexicon (mapping entity type →
//! weighted list of replacement strings), produce `n` distinct augmented
//! variants. Each entity is independently, with probability `replace_ratio`,
//! swapped for a weighted-random replacement from the same type. Offsets in
//! the returned entities are recomputed for the new text.

use crate::textaug::prng::SplitMix64;
use rustc_hash::FxHashMap;

/// Replacement lexicon: map each entity type to `(name, weight)` pairs.
/// Weights are non-normalized — just relative preferences.
pub type EntityReplacement = FxHashMap<String, Vec<(String, f64)>>;

/// An entity occurrence in the source text. Offsets are byte offsets.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedEntity {
    pub text: String,
    pub entity_type: String,
    pub offset: (usize, usize),
}

/// One augmented variant: the new text and the updated entities list
/// (same length as input entities, offsets re-computed).
#[derive(Debug, Clone, PartialEq)]
pub struct EntityAugmented {
    pub text: String,
    pub entities: Vec<NamedEntity>,
}

/// Produce up to `n` distinct augmented variants. `entities` must be in
/// increasing `offset.0` order; overlapping entities are not supported.
pub fn replace_entity(
    text: &str,
    entities: &[NamedEntity],
    replacements: &EntityReplacement,
    n: usize,
    replace_ratio: f64,
    seed: u64,
) -> Vec<EntityAugmented> {
    if n == 0 || entities.is_empty() {
        return Vec::new();
    }
    let ratio = replace_ratio.clamp(0.0, 1.0);
    let mut rng = SplitMix64::from_opt(seed);

    let mut out: Vec<EntityAugmented> = Vec::with_capacity(n);
    // Cap attempts so we don't loop forever when the lexicon makes it hard
    // to generate distinct outputs.
    let cap = (n as f64 / ratio.max(1e-6)).min((entities.len() * 8 + 4) as f64) as usize + n;
    let mut attempts = 0usize;

    while out.len() < n && attempts < cap {
        attempts += 1;
        let cand = augment_once(text, entities, replacements, ratio, &mut rng);
        // Reject unchanged and exact duplicates.
        if cand.text == text {
            continue;
        }
        if !out.iter().any(|o| o.text == cand.text) {
            out.push(cand);
        }
    }
    out
}

fn augment_once(
    text: &str,
    entities: &[NamedEntity],
    replacements: &EntityReplacement,
    ratio: f64,
    rng: &mut SplitMix64,
) -> EntityAugmented {
    let mut new_text = String::with_capacity(text.len());
    let mut new_entities: Vec<NamedEntity> = Vec::with_capacity(entities.len());
    let mut cursor = 0usize;

    for ent in entities {
        // Guard: skip broken or overlapping entities.
        if ent.offset.0 < cursor || ent.offset.1 > text.len() {
            continue;
        }
        new_text.push_str(&text[cursor..ent.offset.0]);

        let replacement = if rng.uniform01() < ratio {
            pool_pick(replacements.get(&ent.entity_type), rng)
        } else {
            None
        };

        let (chosen, chosen_type) = match replacement {
            Some(s) => (s, ent.entity_type.clone()),
            None => (ent.text.clone(), ent.entity_type.clone()),
        };

        let new_start = new_text.len();
        new_text.push_str(&chosen);
        let new_end = new_text.len();
        new_entities.push(NamedEntity {
            text: chosen,
            entity_type: chosen_type,
            offset: (new_start, new_end),
        });

        cursor = ent.offset.1;
    }
    // Tail after last entity.
    if cursor < text.len() {
        new_text.push_str(&text[cursor..]);
    }

    EntityAugmented {
        text: new_text,
        entities: new_entities,
    }
}

fn pool_pick(pool: Option<&Vec<(String, f64)>>, rng: &mut SplitMix64) -> Option<String> {
    let pool = pool?;
    if pool.is_empty() {
        return None;
    }
    let weights: Vec<f64> = pool.iter().map(|(_, w)| *w).collect();
    let idx = rng.weighted_choice(&weights);
    Some(pool[idx].0.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lexicon() -> EntityReplacement {
        let mut m: EntityReplacement = FxHashMap::default();
        m.insert(
            "Person".to_string(),
            vec![
                ("张三".to_string(), 3.0),
                ("李四".to_string(), 2.0),
                ("王五".to_string(), 1.0),
            ],
        );
        m.insert(
            "Country".to_string(),
            vec![("美国".to_string(), 2.0), ("英国".to_string(), 1.0)],
        );
        m
    }

    fn mk_entities() -> Vec<NamedEntity> {
        vec![
            // "伊藤慧太" starts at byte offset of "名叫“" + quote.
            NamedEntity {
                text: "伊藤慧太".to_string(),
                entity_type: "Person".to_string(),
                offset: (15, 27), // 4 * 3 bytes for CJK
            },
            NamedEntity {
                text: "日本".to_string(),
                entity_type: "Country".to_string(),
                offset: (33, 39), // 2 * 3 bytes for CJK
            },
        ]
    }

    #[test]
    fn produces_variants() {
        let text = "一位名叫“伊藤慧太”的男子身着日本匠人常穿的作务衣";
        let ents = mk_entities();
        let r = replace_entity(text, &ents, &lexicon(), 3, 0.8, 42);
        assert!(!r.is_empty());
    }

    #[test]
    fn preserves_entity_count_and_types() {
        let text = "一位名叫“伊藤慧太”的男子身着日本匠人常穿的作务衣";
        let ents = mk_entities();
        let r = replace_entity(text, &ents, &lexicon(), 3, 0.9, 1);
        for v in &r {
            assert_eq!(v.entities.len(), 2);
            assert_eq!(v.entities[0].entity_type, "Person");
            assert_eq!(v.entities[1].entity_type, "Country");
        }
    }

    #[test]
    fn offsets_are_consistent_with_text() {
        let text = "一位名叫“伊藤慧太”的男子身着日本匠人常穿的作务衣";
        let ents = mk_entities();
        let r = replace_entity(text, &ents, &lexicon(), 5, 0.9, 1);
        for v in &r {
            for e in &v.entities {
                assert_eq!(&v.text[e.offset.0..e.offset.1], e.text);
            }
        }
    }

    #[test]
    fn deterministic_with_seed() {
        let text = "一位名叫“伊藤慧太”的男子身着日本匠人常穿的作务衣";
        let ents = mk_entities();
        let a = replace_entity(text, &ents, &lexicon(), 3, 0.9, 123);
        let b = replace_entity(text, &ents, &lexicon(), 3, 0.9, 123);
        assert_eq!(a, b);
    }

    #[test]
    fn empty_entities_returns_empty() {
        let r = replace_entity("some text", &[], &lexicon(), 3, 0.9, 1);
        assert!(r.is_empty());
    }
}
