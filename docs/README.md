# perishables documentation

Specification and reference documentation for `perishables` across Open Knowledge Format auditing and cross-ecosystem semantic grading.

## Start here

| If you want to... | Read |
| --- | --- |
| Audit an OKF bundle or grade a semantic layer from the CLI | [Root README](../README.md) · [perishables CLI README](../crates/perishables/README.md) |
| Embed the zero-dependency OKF trust engine in Rust | [perishables-core README](../crates/perishables-core/README.md) |
| Normalize and grade semantic models in Rust | [perishables-semantic README](../crates/perishables-semantic/README.md) |
| Inspect the 19 planted-defect and clean-control bundles | [corpus/README.md](../corpus/README.md) · [CLAIMS.md](CLAIMS.md) |

## Specifications

| Document | What it covers |
| --- | --- |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Three-crate zero-dependency layout, in-tree parsers, FIPS 180-4 SHA-256, Howard Hinnant civil-day math, Python AST sandbox, and determinism invariants. |
| [DESIGN.md](DESIGN.md) | Open Knowledge Format trust model, `recompute_tier` semantics, currency, freshness, supersession graphs, and provenance rules. |
| [ATTESTATION.md](ATTESTATION.md) | `Attested Computation` execution contract, parameter binding, AST screening, resource limits, and in-toto provenance alignment. |
| [CROSS-ECOSYSTEM.md](CROSS-ECOSYSTEM.md) | Normalization and grading rules across dbt, Apache Ossie, Snowflake, Databricks Genie, Cube, Looker, and AtScale. |
| [LOOKML.md](LOOKML.md) | Native LookML lexer, block-tree parser, `${...}` substitution extraction, `extends` inheritance, and model `include` glob scoping. |
| [CLAIMS.md](CLAIMS.md) | Traceability table mapping every public claim and finding code to its corpus fixture, CI gate, or integration test. |

## Governance and operations

| Document | What it covers |
| --- | --- |
| [DECISIONS.md](../DECISIONS.md) | Architectural decision records behind zero dependencies, explicit clocks, and format-blind grading. |
| [SECURITY.md](../SECURITY.md) | Python subprocess boundary, `--no-exec` mode, symlink and path-escape defenses, and private vulnerability reporting. |
| [CONTRIBUTING.md](../CONTRIBUTING.md) · [CHANGELOG.md](../CHANGELOG.md) | Contribution invariants, prose guidelines, local verification commands, and release history. |
