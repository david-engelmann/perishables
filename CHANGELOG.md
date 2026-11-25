# Changelog

All notable changes to `perishables` and its published planted-defect corpus are recorded in this file.

## v0.1.0 (2026-11-25)

### Workspace and packaging

- Published the three-crate workspace comprising `perishables-core`, `perishables-semantic`, and `perishables` with zero external runtime dependencies.
- Enforced `#![forbid(unsafe_code)]`, `missing_docs = "deny"`, and `clippy::all = "deny"` across all workspace crates on Rust `1.100`.
- Added composite GitHub Action `action.yml`, `.pre-commit-hooks.yaml`, `cargo-binstall` metadata, and multi-OS release packaging for Linux, macOS, and Windows.

### OKF trust auditing (`perishables-core`)

- Implemented the four locked library calls `recompute_tier`, `check_currency`, `walk_supersession`, and `check_supersession`, plus `check_freshness`, `check_provenance`, `check_drift`, and `info_notes`.
- Added exact Gregorian leap-year calendar validation and RFC 3339 date-time parsing and formatting on `AuditClock`.
- Hardened Markdown and YAML frontmatter parsing for multi-byte UTF-8 bodies, YAML `#` comments, fenced and inline code exclusion during footnote citation scanning, info-string attributes on fenced blocks, and symlink-safe directory traversal.
- Deduplicated multi-hop and cyclic supersession findings to canonical chain heads and surfaced SPEC 12.1 root-relative fallbacks through `info_notes`.

### Attested Computation sandbox and CLI (`perishables`)

- Implemented cross-platform Python re-execution with `ast.parse` determinism pre-screening, concurrent bounded pipe readers, exclusive scratch directories, safe memory-limit negotiation across Linux, macOS, and Windows, typed parameter binding, and `--no-exec` static audit mode.
- Closed the attestation gate so failed or non-deterministic computations on concepts claiming an attested result emit `UNATTESTED_CLAIM`.
- Added `--help`, `--version`, `--flag=value` parsing, `--timing`, `--output`, unified `perishables grade`, and zero-dependency `text`, `json`, `sarif`, and `github` output formats.

### Cross-ecosystem semantic grading (`perishables-semantic`)

- Shipped format-blind grading rules `DANGLING_METRIC_REFERENCE`, `NAME_COLLISION`, `LAPSED_VERIFIED_QUERY`, `DANGLING_RELATIONSHIP`, `BENCHMARK_WITHOUT_ANSWER`, and `AMBIGUOUS_JOIN_PATH`, alongside coverage and orphan notes.
- Shipped readers for dbt semantic manifests, Apache Ossie JSON and YAML, Snowflake semantic views with epoch and RFC 3339 `verified_at` support, Databricks Genie serialized spaces, Cube JSON, YAML, and JS models, Looker API JSON and project-scoped native LookML, and AtScale JSON exports.

### Planted-defect corpus `v0.2.0`

- Incremented `corpus_version` from `0.1.0` to `0.2.0` on adding `t5-unattested-claim`, completing planted-defect coverage across all twelve `FindingCode` variants alongside the four clean control bundles.
- Hardened `perishables corpus` to cross-check `manifest.toml` against directories on disk and validate `LABEL.toml` tier and computation assertions.
