# jionlp-rs

Rust + Elixir ports of [**JioNLP**](https://github.com/dongrixinyu/JioNLP),
a Chinese NLP preprocessing & parsing toolkit. Pure native code, no Python
runtime, no model downloads, no network calls — the full runtime dictionary
(~9 MB) is bundled with the package.

This repository is a Cargo workspace with two crates:

| Crate                                   | Purpose                                                    | Target                      |
| --------------------------------------- | ---------------------------------------------------------- | --------------------------- |
| [`jionlp-core`](crates/jionlp-core)     | Pure-Rust library, port of the Python non-LLM APIs         | [crates.io](https://crates.io/crates/jionlp-core) |
| [`jionlp_nif`](crates/jionlp_nif)       | Rustler NIF `cdylib` that wraps `jionlp-core`              | consumed by `jionlp_ex`      |

The Elixir package lives as a sibling at
[`jionlp_ex`](https://github.com/Psli/jionlp_ex) and is distributed on
[Hex.pm](https://hex.pm/packages/jionlp_ex).

## Quickstart (Rust)

```toml
[dependencies]
jionlp-core = "0.1"
```

```rust
use jionlp_core as jio;

fn main() -> jio::Result<()> {
    jio::init()?;
    let sents = jio::split_sentence("今天天气真好。我要去公园！", Default::default());
    let simp  = jio::tra2sim("今天天氣好晴朗", Default::default());
    let money = jio::parse_money("三千五百万港币")?;
    Ok(())
}
```

See [`crates/jionlp-core/README.md`](crates/jionlp-core/README.md) for the
full API surface.

## Quickstart (Elixir)

```elixir
# mix.exs
def deps do
  [{:jionlp_ex, "~> 0.1"}]
end
```

```elixir
JioNLP.split_sentence("今天天气真好。我要去公园！")
# => ["今天天气真好。", "我要去公园！"]

JioNLP.parse_money("三千五百万港币")
# => %JioNLP.MoneyInfo{num: "35000000.00", case: "2", definition: "accurate", unit: "HKD"}
```

Prebuilt NIF binaries are downloaded via `rustler_precompiled` for Linux
(gnu/musl × x86_64/aarch64), macOS (x86_64/aarch64), and Windows
(x86_64). End users do **not** need a Rust toolchain.

## Development

```bash
# Rust
cargo test -p jionlp-core
cargo clippy -p jionlp-core -- -D warnings

# Elixir (sibling directory layout: jionlp_rs/ + jionlp_ex/)
cd ../jionlp_ex
./scripts/sync_dictionary.sh   # copies data/ into priv/data/
JIONLP_BUILD=1 mix test         # force source build of NIF
```

## Repository layout

```
jionlp_rs/
├── Cargo.toml                  # workspace root
├── crates/
│   ├── jionlp-core/            # published to crates.io (bundles data/)
│   │   ├── src/
│   │   └── data/               # 22 dict files, single source of truth
│   └── jionlp_nif/             # Rustler NIF cdylib
└── .github/workflows/
    ├── ci.yml                  # fmt + clippy + test
    ├── release-data.yml        # data tarball on jionlp-data-v* tag
    ├── release-crate.yml       # cargo publish on jionlp-core-v* tag
    └── release-nif.yml         # 14-tarball NIF release on jionlp_ex-v* tag
```

## Release flow

Three **independent** tag-triggered workflows, all on this repo:

| Tag pattern           | Artifact                                  | Workflow              |
| --------------------- | ----------------------------------------- | --------------------- |
| `jionlp-data-v<DATE>` | `jionlp-data-v<DATE>.tar.gz` + `.sha256`  | `release-data.yml`    |
| `jionlp-core-vX.Y.Z`  | crates.io publish                         | `release-crate.yml`   |
| `jionlp_ex-vX.Y.Z`    | 14 precompiled NIF tarballs               | `release-nif.yml`     |

**Data releases are decoupled from code releases** — bumping the
dictionary doesn't require bumping `jionlp-core` or `jionlp_ex`.
Consumers (currently `jionlp_ex`; future Rust `fetch` feature) pull the
data tarball from GitHub Releases and verify the SHA-256.

### Cutting a data-only release

```bash
# 1. Edit crates/jionlp-core/data/ as needed
# 2. Pick a date-based tag
git tag jionlp-data-v2026.05.1
git push origin jionlp-data-v2026.05.1
# 3. (Optional) bump @default_data_version in
#    ../jionlp_ex/lib/jionlp/data_fetcher.ex so new Hex installs default
#    to the new data version.
```

### Cutting a Rust crate release

```bash
# Bump workspace.package.version in Cargo.toml, then:
git tag jionlp-core-v0.1.1
git push origin jionlp-core-v0.1.1
# release-crate.yml verifies version & runs `cargo publish`.
# Requires the CARGO_REGISTRY_TOKEN repo secret.
```

### Cutting an Elixir release

```bash
# Bump @version in jionlp_ex/mix.exs.
git tag jionlp_ex-v0.1.1
git push origin jionlp_ex-v0.1.1
# release-nif.yml builds 14 NIF tarballs.
# Then locally in jionlp_ex/:
mix rustler_precompiled.download JioNLP.Native --all --print
git add checksum-Elixir.JioNLP.Native.exs && git commit
mix hex.publish
# The Hex tarball is ~200 KB — data is fetched on first use, not bundled.
```

## Parity & scope

The Rust port targets **JioNLP 0.2.7.x non-LLM APIs**. All lexicon-driven
features (parsing, extraction, cleaning, keyphrase, summary, simhash, F1
evaluation, NER tooling, etc.) are ported. LLM-dependent helpers
(`llm_*`, `predict_entity`, OpenAI-backed functions) are out of scope.

See the TODO comments in [`crates/jionlp-core/src/lib.rs`](crates/jionlp-core/src/lib.rs)
for the current public API surface.

## License

Apache-2.0, matching the upstream Python project. See [LICENSE](LICENSE).
