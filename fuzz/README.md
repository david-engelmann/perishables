# Fuzzing perishables parsers and trust evaluators

`perishables` implements all seven semantic-layer readers (JSON, YAML, Cube JS, LookML, dbt, Apache Ossie, Snowflake, Databricks Genie, and AtScale), OKF YAML frontmatter and Markdown fence extraction, proleptic Gregorian RFC 3339 calendar parsing, and FIPS 180-4 SHA-256 directly on the Rust standard library.

This directory contains an **isolated Cargo workspace** (`[workspace] members = ["."]`) for `cargo-fuzz` (`libfuzzer-sys`) so that fuzzing harnesses compile and run against `perishables-core` and `perishables-semantic` while keeping the root workspace `Cargo.lock` strictly at zero external crates.

## Targets

| Target | Entry point | Surface covered |
| --- | --- | --- |
| `semantic_readers` | `fuzz_targets/semantic_readers.rs` | Feeds arbitrary byte slices and UTF-8 strings through `read_definition` (dbt), `read_ossie_definition`, `read_ossie_yaml`, `read_snowflake_definition`, `read_databricks_definition`, `read_cube_definition`, `read_cube_yaml`, `read_cube_js`, `read_looker_definition`, `parse_tree`, `read_lookml_definition`, `read_lookml_project`, and `read_atscale_definition`, then grades every parsed `Definition` via `grade` and `notes` |
| `okf_frontmatter_and_clock` | `fuzz_targets/okf_frontmatter_and_clock.rs` | Feeds arbitrary strings through `AuditClock::parse` (verifying RFC 3339 round-trip formatting), `sha256_hex`, `Bundle::load` (YAML frontmatter, `#` comment stripping, fenced code blocks, footnote citations, `claimed_result`, and path containment), and all trust evaluators (`recompute_tier`, `check_currency`, `walk_supersession`, `check_supersession`, `check_freshness`, `check_provenance`, `check_drift`, `info_notes`) |

## Compile check on stable Rust (runs in CI)

Every pull request checks that all fuzz targets compile cleanly against the current crate APIs on stable Rust:

```sh
cargo check --manifest-path fuzz/Cargo.toml --locked --bins
```

## Running `cargo-fuzz` locally (requires nightly Rust)

Install `cargo-fuzz` once and run either target against its seed corpus in `fuzz/seeds/`:

```sh
cargo +nightly fuzz run --fuzz-dir fuzz semantic_readers fuzz/seeds/semantic_readers -- -max_total_time=60
cargo +nightly fuzz run --fuzz-dir fuzz okf_frontmatter_and_clock fuzz/seeds/okf_frontmatter_and_clock -- -max_total_time=60
```
