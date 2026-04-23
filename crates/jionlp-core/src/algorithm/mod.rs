//! algorithm — ports of `jionlp/algorithm/`.

pub mod analyse_ner;
pub mod bpe;
pub mod data_correction;
pub mod entity_compare;
pub mod extractors;
pub mod keyphrase;
pub mod measure;
pub mod ner;
pub mod ner_accelerate;
pub mod ner_tools;
pub mod new_word;
pub mod sentiment;
pub mod simhash;
pub mod summary;
pub mod tag_conversion;
pub mod text_classification;

pub use analyse_ner::{analyse_ner_dataset_split, ClassStat, DatasetStats, SplitResult};
pub use bpe::{bpe_decode, bpe_encode};
pub use data_correction::{correct_cws_sample, correct_pos_sample};
pub use entity_compare::{entity_compare_detailed, EntityDiff};
pub use extractors::{extract_money, extract_time, MoneyEntity, TimeEntity};
pub use keyphrase::{extract_keyphrase, extract_keyphrase_textrank, KeyPhrase};
pub use measure::{compute_f1, F1Report, LabelStats};
pub use ner::{LexiconNer, NerEntity};
pub use ner_accelerate::{TokenBatchBucket, TokenBreakLongSentence, TokenSplitSentence};
pub use ner_tools::{
    analyse_ner_dataset, char2word, collect_dataset_entities, is_person_name, token_batch_bucket,
    token_break_long_sentence, token_split_sentence, word2char, NerDatasetAnalysis,
};
pub use new_word::new_word_discovery;
pub use sentiment::sentiment_score;
pub use simhash::{hamming_distance, simhash, simhash_similarity};
pub use summary::{extract_summary, extract_summary_mmr, SummarySentence};
pub use tag_conversion::{cws, ner as ner_convert, pos, Entity, F1};
