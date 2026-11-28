# perishables Documentation Index

Specifications, corpus traceability, and ecosystem guides for `perishables`. The rendered documentation book is published at <https://david-engelmann.github.io/perishables/>.

## Core & Semantic Specifications

| Document | Contents |
| --- | --- |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Three-crate zero-external-dependency design, in-tree parsers, FIPS 180-4 SHA-256, Howard Hinnant civil-day calendar math, Python AST sandbox, and determinism rules. |
| [DESIGN.md](DESIGN.md) | Open Knowledge Format v0.3 trust model, `recompute_tier`, verification currency, `stale_after` freshness, supersession chains, and provenance rules. |
| [ATTESTATION.md](ATTESTATION.md) | `Attested Computation` execution contract, `ast.parse` pre-screening, `0700` scratch isolation, `--no-exec` untrusted mode, and in-toto mapping. |
| [CROSS-ECOSYSTEM.md](CROSS-ECOSYSTEM.md) | Normalized `Definition` graph and six finding rules across dbt, Apache Ossie, Snowflake, Databricks Genie, Cube, Looker, and AtScale. |
| [LOOKML.md](LOOKML.md) | Native LookML lexer, block-tree parser, `${...}` reference extraction, `extends` inheritance, and model `include` glob scoping. |
| [CLAIMS.md](CLAIMS.md) | 1-to-1 mapping from every public claim and all 18 `FindingCode` values to planted-defect fixtures in `corpus/` and test suites. |
| [COMPARISON.md](COMPARISON.md) · [FAQ.md](FAQ.md) | Comparison against JSON Schema linters, `dbt parse`, `spectacles`, and data monitors, plus operational FAQs. |

## Crate, Corpus, and Agent Guides

| Document | Contents |
| --- | --- |
| [Root README](../README.md) | Decay timeline and semantic graph diagrams, CLI/Python/SARIF quickstart, and complete 18-code finding reference. |
| [perishables CLI README](../crates/perishables/README.md) | CLI flags, exit codes (`0`, `1`, `2`), output formats (`text`, `json`, `sarif`, `github`), and shell completions. |
| [perishables-core README](../crates/perishables-core/README.md) · [perishables-semantic README](../crates/perishables-semantic/README.md) | Zero-dependency Rust library APIs for OKF auditing and semantic-layer grading. |
| [corpus/README.md](../corpus/README.md) · [examples/README.md](../examples/README.md) · [AGENTS.md](../AGENTS.md) | Reference 19-bundle test corpus, runnable Python/shell CI gate examples, and agent integration guide. |

## Governance & Build Records

| Document | Contents |
| --- | --- |
| [DECISIONS.md](../DECISIONS.md) · [SECURITY.md](../SECURITY.md) | Zero-dependency and explicit-clock decisions, subprocess sandbox boundaries, and vulnerability reporting. |
| [CONTRIBUTING.md](../CONTRIBUTING.md) · [CHANGELOG.md](../CHANGELOG.md) | Zero-external-crate `Cargo.lock` rule, local verification commands, and release history. |
| [BUILD-ROUNDS.md](BUILD-ROUNDS.md) | Index of the maintainer build round records, rounds 7 through 24. |
