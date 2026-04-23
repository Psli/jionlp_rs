//! textaug — text augmentation utilities ported from `jionlp/textaug/`.

pub mod homophone_substitution;
pub mod prng;
pub mod random_add_delete;
pub mod replace_entity;
pub mod swap_char_position;

pub use homophone_substitution::homophone_substitution;
pub use random_add_delete::random_add_delete;
pub use replace_entity::{replace_entity, EntityReplacement, NamedEntity};
pub use swap_char_position::swap_char_position;
