# jionlp-core

[![crates.io](https://img.shields.io/crates/v/jionlp-core.svg)](https://crates.io/crates/jionlp-core)
[![docs.rs](https://docs.rs/jionlp-core/badge.svg)](https://docs.rs/jionlp-core)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](../../LICENSE)

Rust port of [**JioNLP**](https://github.com/dongrixinyu/JioNLP) — a Chinese
NLP preprocessing & parsing toolkit. Pure Rust, no Python runtime, no model
downloads, no network calls.

The published crate bundles the full runtime dictionary (~9 MB of `.txt` and
`.zip` files under `data/`) so `jionlp_core::init()` works out of the box.

## At a glance

| Capability          | Functions                                                                            |
| ------------------- | ------------------------------------------------------------------------------------ |
| Sentence splitting  | `split_sentence`                                                                     |
| Stopwords           | `remove_stopwords`                                                                   |
| Traditional ⇄ Simp. | `tra2sim`, `sim2tra`                                                                 |
| Pinyin              | `pinyin`                                                                             |
| Money & time        | `parse_money`, `parse_time`, `extract_money`, `extract_time`                         |
| Location            | `parse_location_full`, `recognize_location`, `phone_location`                        |
| ID / plate / QQ     | `parse_id_card`, `parse_motor_vehicle_licence_plate`, `extract_qq`, `extract_email`  |
| Character utilities | `char_radical`, `num2char`, `char2num`                                               |
| Text cleaning       | `clean_text`, `clean_html`, `remove_html_tag`, `remove_exception_char`, …            |
| Algorithms          | `extract_keyphrase`, `extract_summary`, `simhash`, `sentiment_score`, `LexiconNer`   |
| Data utilities      | `compute_f1`, `correct_cws_sample`, `analyse_ner_dataset_split`, `mine_rules`, …     |

Full list: see `src/lib.rs` or [docs.rs](https://docs.rs/jionlp-core).

## Install

```toml
[dependencies]
jionlp-core = "0.1"
```

MSRV: Rust 1.85 (required by `jieba-rs` 0.9 transitively — it uses edition2024).

## Quickstart

```rust
use jionlp_core as jio;

fn main() -> jio::Result<()> {
    jio::init()?; // Loads the bundled dictionary. Call once.

    // Sentence splitting
    let sents = jio::split_sentence("今天天气真好。我要去公园！", jio::gadget::split_sentence::Criterion::Coarse);
    // → ["今天天气真好。", "我要去公园！"]

    // Traditional → Simplified
    let simp = jio::tra2sim("今天天氣好晴朗", jio::gadget::ts_conversion::TsMode::Char);
    // → "今天天气好晴朗"

    // Money parsing
    let money = jio::parse_money("三千五百万港币")?;
    // MoneyInfo { num: "35000000.00", case: "2", definition: "accurate", unit: "HKD" }

    // Time parsing
    let t = jio::parse_time("明天下午三点", None)?;

    // Keyphrase extraction
    let phrases = jio::extract_keyphrase("...长文本...", 10);

    Ok(())
}
```

## Using a custom dictionary directory

`init()` loads from the bundled `data/`. To point at an external directory —
e.g. to override the ID-card admin-code table for older data:

```rust
jionlp_core::dict::init_from_path("/opt/jionlp/data")?;
```

## Design

- **Lexicon-driven**, no neural inference. Parity target: Python JioNLP
  0.2.7.x (non-LLM APIs).
- **Dictionaries as data**, loaded once into `once_cell::Lazy` statics.
- **Regex compiled at first use** (`Lazy<Regex>`), zero cost after warm-up.
- **No I/O in the hot path** once `init()` returns.

## What's *not* ported

By design, LLM-dependent features from upstream are out of scope: `llm_*`,
`predict_entity`, OpenAI-backed helpers. The Rust port focuses on
deterministic, offline preprocessing and parsing.

## License

Apache-2.0, matching the upstream Python project.
