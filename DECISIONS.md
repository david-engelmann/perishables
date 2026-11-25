# Architectural decisions

This file records the load-bearing architectural decisions behind `perishables`.

## ADR-001 Zero external runtime dependencies

`perishables-core`, `perishables-semantic`, and `perishables` depend only on the Rust standard library. OKF frontmatter YAML, RFC 3339 timestamps, FIPS 180-4 SHA-256, JSON, semantic YAML, Cube JavaScript object literals, native LookML block trees, and JSON or SARIF report emitters are implemented directly in safe Rust. Eliminating third-party crates removes supply-chain churn, guarantees reproducible builds with `--offline`, and keeps compile times under seconds.

## ADR-002 Pinned AuditClock with no system-clock fallback

`perishables audit` requires an explicit `--as-of` date or RFC 3339 timestamp and refuses to default to `SystemTime::now()`. Host filesystem `mtime` is excluded from `Bundle::load` and default wall-clock `elapsed_ms` is excluded unless `--timing` is requested, ensuring that two runs over the same bundle and `--as-of` date produce identical bytes on any machine or git checkout.

## ADR-003 Locked four-call library surface in perishables-core

Downstream upkeep engines consume `recompute_tier`, `check_currency`, `walk_supersession`, and `check_supersession` directly from `perishables-core`. Their signatures and the meaning of every variant in `FindingCode` are locked under semantic versioning. Auditor-side checks (`check_freshness`, `check_provenance`, `check_drift`, and `info_notes`) sit beside those four functions without altering them, and process execution lives strictly in the `perishables` adapter crate.

## ADR-004 AST-screened deterministic Python re-execution

OKF v0.3 defines `Attested Computation` contracts while leaving execution to consumers. `perishables` executes only the local `python` runtime in v1, pre-screening the parsed Python AST to block non-deterministic and I/O modules before compilation while ignoring comments and string literals. Any claimed attested result whose computation fails, times out, uses an unsupported warehouse runtime, or violates the deterministic AST profile trips `UNATTESTED_CLAIM` so broken claims never pass silently.

## ADR-005 Normalized format-blind semantic grading

Vendor semantic layers express the same trust claims as OKF bundles in platform-specific syntax. Rather than duplicating grading logic per vendor, `perishables-semantic` normalizes dbt, Apache Ossie, Snowflake, Databricks Genie, Cube, Looker and native LookML, and AtScale documents into a single `Definition` struct and runs shared structural, collision, currency, and ground-truth rule families over that normalized representation.
