# Changelog

All notable changes to `perishables` and its published planted-defect corpus are recorded in this file.

## [0.1.2] - 2026-11-27

### Semantic grading and CLI (`perishables-semantic`, `perishables`)

- Graded `${field}` references inside LookML `dimension` and `dimension_group` SQL blocks alongside measure SQL under `DANGLING_METRIC_REFERENCE` (`#15`).
- Updated `perishables corpus` to resolve either a corpus root directory (containing `manifest.toml` and `fixtures/`) or a `fixtures/` directory directly (`#14`).
- Added zero-dependency shell completion script generation in `perishables::completions` (`Shell`, `completions_script`, `write_completions`) covering all subcommands and flags across `bash`, `zsh`, `fish`, and `powershell`.

### Python SDK and distribution (`python/perishables`, `install.sh`)

- Added pure-Python `perishables.completions(shell)` and module entrypoint `python -m perishables completions <bash|zsh|fish|powershell>` (with CLI argument delegation via `python -m perishables`).
- Added POSIX `install.sh` (`--version`, `--install-dir`, `--base-url`, `--target`, `--help`) verifying prebuilt GitHub Release tarballs against `SHA256SUMS` before extraction.

### Supply chain, governance, and developer tooling

- Added `deny.toml` (`cargo-deny` v2) enforcing zero external dependencies and `MIT OR Apache-2.0` licensing, plus `cargo deny check` in the `hygiene` job of `.github/workflows/ci.yml`.
- Added `CITATION.cff` (CFF 1.2.0), `CODE_OF_CONDUCT.md` (Contributor Covenant v2.1), `.github/ISSUE_TEMPLATE/config.yml`, `Makefile`, and `.editorconfig`, and updated default version pins across `action.yml`, `.github/workflows/release.yml`, and `.github/workflows/verify-published.yml` to `0.1.2`.

## [0.1.1] - 2026-11-26

### Python SDK (`python/perishables`)

- Added typed ecosystem-specific grading wrappers `grade_dbt`, `grade_ossie`, `grade_snowflake`, `grade_databricks`, `grade_cube`, `grade_looker`, and `grade_atscale`.
- Added SARIF 2.1.0 pipeline helpers `audit_sarif` and `grade_sarif` with optional `output` file writing, returning parsed SARIF 2.1.0 dictionaries across both clean (`0`) and finding-producing (`1`) exit codes while raising `PerishablesError` on usage or parse errors (`2`).

### Packaging, documentation, and CI

- Replaced relative media sources and repository links in root `README.md` with canonical GitHub URLs so images and documentation links render cleanly on PyPI and crates.io.
- Updated composite GitHub Action `action.yml` default `version` input to `v0.1.1` and pinned `actions/checkout` to `v7.0.1` across workflow definitions.
- Added `.github/workflows/verify-published.yml` to verify published `0.1.1` crates.io libraries, CLI binary, and PyPI wheel on clean runner environments.

## v0.1.0 (2026-11-25)

### Workspace and packaging

- Published the three-crate workspace comprising `perishables-core`, `perishables-semantic`, and `perishables` with zero external runtime dependencies.
- Enforced `#![forbid(unsafe_code)]`, `missing_docs = "deny"`, and `clippy::all = "deny"` across all workspace crates on Rust `1.100`.
- Added composite GitHub Action `action.yml`, `.pre-commit-hooks.yaml`, `cargo-binstall` metadata, a typed Python SDK and precompiled `perishables` PyPI wheels (`pyproject.toml`, `python/perishables`), and GitHub Actions OIDC release automation for crates.io, PyPI, and five-platform GitHub Releases.

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
- Added `docs/README.md`, `docs/CLAIMS.md`, light and dark SVG lockups, architecture and shelf-life comparison diagrams, a `1200x630` social preview card, and a real-output `docs/media/money-shot.gif` demonstration.
