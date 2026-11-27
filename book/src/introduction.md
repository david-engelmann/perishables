# perishables

<p>
  <img class="theme-light-only" src="docs/media/perishables-lockup-light.svg" alt="perishables" height="50">
  <img class="theme-dark-only" src="docs/media/perishables-lockup-dark.svg" alt="perishables" height="50">
</p>

`perishables` is a deterministic, point-in-time trust auditor for [Open Knowledge Format (OKF)](docs/DESIGN.md) bundles and [seven analytics semantic layers](docs/CROSS-ECOSYSTEM.md), implemented in safe Rust over `std` with **zero external runtime or build crates**.

<p>
  <img src="docs/media/money-shot.gif" alt="perishables auditing lapsed verifications, sandboxed Python attestation mismatches, SHA-256 source drift, native LookML semantic errors, and all 19 reference corpus fixtures" width="860">
</p>

## Why Syntax Validation Passes Decayed Truth

<p>
  <img class="theme-light-only" src="docs/media/syntax-vs-truth-light.svg" alt="Side-by-side comparison showing five structurally valid OKF and LookML lines that pass syntax linters while failing perishables point-in-time verification" width="880">
  <img class="theme-dark-only" src="docs/media/syntax-vs-truth-dark.svg" alt="Side-by-side comparison showing five structurally valid OKF and LookML lines that pass syntax linters while failing perishables point-in-time verification" width="880">
</p>

Schema linters verify that YAML, JSON, or LookML parses structurally. `perishables` evaluates whether verification timestamps postdate generation, stability horizons hold at `--as-of`, referenced source files match their FIPS 180-4 SHA-256 digests, AST-screened `Attested Computation` blocks reproduce claimed numbers, and semantic metric graphs resolve cleanly.

## Point-in-Time Bundle Audit & Seven-Ecosystem Semantic Graph

<p>
  <img class="theme-light-only" src="docs/media/audit-timeline-and-graph-light.svg" alt="How perishables evaluates OKF bundle decay against an explicit AuditClock and grades seven semantic ecosystems through one Definition graph" width="880">
  <img class="theme-dark-only" src="docs/media/audit-timeline-and-graph-dark.svg" alt="How perishables evaluates OKF bundle decay against an explicit AuditClock and grades seven semantic ecosystems through one Definition graph" width="880">
</p>

- **Pinned `AuditClock` (`perishables-core` + `perishables`).** Zero ambient clock or `mtime` reads; derives `unverified`, `machine-confirmed`, or `human-reviewed` strictly from current events, checks `stale_after` and `superseded_by` graphs, hashes bundled sources, and runs `python3 -I -B` inside an isolated `0700` scratch directory (`--no-exec` for untrusted bundles).
- **Unified `Definition` grading (`perishables-semantic`).** Native zero-dependency readers normalize **dbt**, **Apache Ossie**, **Snowflake**, **Databricks Genie**, **Cube**, **Looker / native LookML**, and **AtScale** models into one graph to convict dangling references, case-folded synonym collisions, lapsed verified queries, broken joins, empty benchmarks, and join fan-traps.

## Chapters in This Book

- **[Quickstart](./quickstart.md):** Install `perishables`, audit OKF bundles, grade semantic models, and configure GitHub Actions.
- **[CLI and Python SDK Reference](./cli-and-python.md):** Exit codes (`0`, `1`, `2`), `text` / `json` / `sarif` / `github` output schemas, and typed Python dataclasses.
- **[OKF Trust Model](docs/DESIGN.md) & [Attested Computation](docs/ATTESTATION.md):** Normative rules for currency, tiers, supersession, provenance, and Python AST sandboxing.
- **[Seven-Ecosystem Grading](docs/CROSS-ECOSYSTEM.md) & [Native LookML Reader](docs/LOOKML.md):** Vendor normalization rules, LookML lexer/parser scope, and `extends` / `include` resolution.
- **[Claims & Corpus](docs/CLAIMS.md), [Comparison](docs/COMPARISON.md) & [FAQ](docs/FAQ.md):** 19-fixture traceability matrix, tool comparisons, and operational FAQs.
