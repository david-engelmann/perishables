# Frequently asked questions

## General and determinism

### Why does `perishables audit` require `--as-of` instead of reading the system clock?

Point-in-time reproducibility is a core design invariant. If `perishables` called `SystemTime::now()`, the same commit could pass CI on Friday and fail on Monday with zero repository changes, and historical audits could not be reproduced byte-for-byte. Passing an explicit date (`--as-of 2026-11-20` or `--as-of "$(date -u +%Y-%m-%d)"` in a scheduled CI job) makes the evaluation instant an explicit, logged input to the audit. See [ARCHITECTURE.md](ARCHITECTURE.md) for the full determinism contract.

### Why does the workspace have zero external Rust dependencies?

`perishables` sits in CI gates and audits provenance, SHA-256 source hashes, and attested claims. Keeping `Cargo.lock` restricted to the three first-party crates (`perishables-core`, `perishables-semantic`, and `perishables`) eliminates transitive supply-chain churn, advisory noise, proc-macro build latency, and parser behavior shifts across dependency upgrades. Every parser (YAML frontmatter, JSON, YAML subset, Cube JS subset, native LookML), RFC 3339 proleptic Gregorian calendar conversion, and FIPS 180-4 SHA-256 digest is implemented in safe Rust (`#![forbid(unsafe_code)]`) and fuzz-tested under `fuzz/`.

### Does `perishables` ever modify files inside a bundle or semantic project?

No. Loading and auditing are strictly read-only, and `perishables audit` / `perishables drift` reject `--output` paths located inside the target bundle directory. Every run of `perishables corpus` computes recursive SHA-256 directory digests before and after auditing all 19 reference fixtures to prove zero file creation or mutation.

## OKF bundle auditing

### When should I pass `--no-exec`?

Pass `--no-exec` (or `no_exec=True` in the Python SDK) whenever auditing an **untrusted** bundle from an external contributor or running in a locked-down environment where spawning `python3` subprocesses is undesirable. Without `--no-exec`, `perishables` pre-screens `Attested Computation` blocks with `ast.parse` and runs them under `python3 -I -B` in an isolated `0700` scratch directory with timeout and memory limits, as detailed in [ATTESTATION.md](ATTESTATION.md). With `--no-exec`, zero subprocesses run; any concept that claims an attested result emits `UNATTESTED_CLAIM` (`execution skipped (--no-exec)`) rather than silently passing unverified claims.

### Why did a concept with `verified` events drop to `unverified` or `machine-confirmed`?

Under the Open Knowledge Format v0.3 trust model ([DESIGN.md](DESIGN.md)), trust tiers are recomputed from **current** verification events rather than stored:

- If a verification event's `at` timestamp is earlier than `generated.at` on the same concept, the content changed after the review, so the event is classified as lapsed (`LAPSED_VERIFICATION`) and excluded from `recomputed_tier`.
- Only actors with the `human:` prefix (such as `human:alice@example.com`) qualify for `human-reviewed`. Automated (`process:`) and group (`team:`) handles yield `machine-confirmed` because a group alias does not identify an accountable individual human reviewer.
- Events missing `by` or a valid calendar `at` timestamp emit `MALFORMED_VERIFICATION` and count toward no tier.

### Does `STALE_BUT_STABLE` mean the metric or claim is wrong?

No. `stale_after` marks a review horizon: stale means **check again**, not false. `STALE_BUT_STABLE` fires when the pinned `--as-of` instant is at or past `stale_after` while the concept still declares `status: stable` (or omits `status`). Re-verifying the concept and advancing `stale_after`, or marking a retired concept `status: deprecated`, clears the finding.

## Semantic-layer grading

### Should I run `perishables grade` or `perishables grade-<ecosystem>`?

Both run the exact same normalized `Definition` checks ([CROSS-ECOSYSTEM.md](CROSS-ECOSYSTEM.md)):

- Use `perishables grade-<ecosystem>` (`grade-dbt`, `grade-ossie`, `grade-snowflake`, `grade-databricks`, `grade-cube`, `grade-looker`, `grade-atscale`) when grading a single known ecosystem explicitly.
- Use `perishables grade <file>...` when grading mixed files in one pass (auto-detected by file extension and JSON/YAML structure, or pinned via `--ecosystem <name>`) and emitting a unified JSON or SARIF 2.1.0 report.

### Why does `grade-looker` accept multiple `.lookml` files at once?

Single-file LookML linting misses cross-view defects. Passing all `.view.lookml` and `.model.lookml` files together (`perishables grade-looker views/*.view.lookml models/*.model.lookml`) activates project-scoped resolution ([LOOKML.md](LOOKML.md)): `extends` inheritance is resolved across views, qualified `${other_view.field}` references in measure and dimension SQL are checked project-wide, explore joins validate target views, and unmatched `include` globs surface as coverage notes.

### Why did `grade-snowflake` not flag `LAPSED_VERIFIED_QUERY` until I passed `--changed-at`?

Snowflake semantic view YAML exports (`SYSTEM$READ_YAML_FROM_SEMANTIC_VIEW`) carry `verified_at` timestamps on individual `verified_queries`, whereas the last-altered timestamp of the semantic view object lives in Snowflake catalog metadata or git history. Passing `--changed-at <YYYY-MM-DD|RFC3339|epoch>` supplies that comparison timestamp explicitly without consulting host file `mtime`.

## CI, SARIF, and Python packaging

### Do I need a Rust toolchain to use `perishables` from Python or CI?

No. Prebuilt wheels on PyPI (`pip install perishables` or `uvx perishables`) bundle the compiled native Rust `perishables` binary alongside the typed Python SDK. On Linux and macOS runners without Python or Rust, `install.sh` downloads the prebuilt release archive and verifies it against `SHA256SUMS` before installation. In GitHub Actions, `david-engelmann/perishables@v0.1.3` installs the release binary automatically.

### Why does `perishables` exit with code `1` when generating a SARIF file?

Exit code `1` signals that one or more trust or semantic findings were convicted, while still writing the complete SARIF 2.1.0 document to `--output <path>` (or stdout). In GitHub Actions, upload the SARIF artifact with `if: always()` (or `if: success() || failure()`) so findings appear in GitHub Code Scanning even when the gate fails the pull request. From Python, `perishables.audit_sarif(...)` and `perishables.grade_sarif(...)` return the parsed SARIF dictionary cleanly on both exit codes `0` and `1`, raising `PerishablesError` only on usage or parse errors (exit code `2`).
