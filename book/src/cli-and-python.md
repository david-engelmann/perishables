# CLI and Python SDK Reference

## CLI subcommands

| Command | Synopsis | Purpose |
| --- | --- | --- |
| `audit` | `perishables audit <BUNDLE> --as-of <TIMESTAMP> [OPTIONS]` | Recompute trust tiers, check currency, supersession, freshness, provenance, SHA-256 drift, and `Attested Computation` results |
| `drift` | `perishables drift <BUNDLE> [OPTIONS]` | Verify recorded `sources[].sha256` digests against on-disk bundle files |
| `corpus` | `perishables corpus [CORPUS_DIR \| FIXTURES_DIR] [OPTIONS]` | Run all 19 reference fixtures against `LABEL.toml` expectations and verify zero directory mutation |
| `grade` | `perishables grade <FILE>... [--ecosystem <NAME>] [OPTIONS]` | Grade one or more semantic-layer files with automatic or explicit ecosystem selection |
| `grade-dbt` | `perishables grade-dbt <MANIFEST.JSON> [OPTIONS]` | Grade dbt `manifest.json` or `semantic_manifest.json` |
| `grade-ossie` | `perishables grade-ossie <MODEL.JSON\|MODEL.YAML> [OPTIONS]` | Grade Apache Ossie JSON or YAML semantic models |
| `grade-snowflake` | `perishables grade-snowflake <VIEW.YAML> [--changed-at <TS>] [OPTIONS]` | Grade Snowflake semantic view YAML exports |
| `grade-databricks` | `perishables grade-databricks <SPACE.JSON> [OPTIONS]` | Grade Databricks Genie `serialized_space` JSON |
| `grade-cube` | `perishables grade-cube <FILE> [OPTIONS]` | Grade Cube `/v1/meta` JSON, `.yaml`, or declarative `.js` models |
| `grade-looker` | `perishables grade-looker <MODEL.JSON\|FILE.LOOKML...> [OPTIONS]` | Grade Looker API JSON or single/multi-file native `.lookml` projects |
| `grade-atscale` | `perishables grade-atscale <MODEL.JSON> [OPTIONS]` | Grade AtScale JSON model exports |

### Common CLI flags

- `--as-of <YYYY-MM-DD|RFC3339>`: Required on `audit`. Sets the pinned evaluation instant (`AuditClock`).
- `--changed-at <YYYY-MM-DD|RFC3339|EPOCH>`: Optional on `grade` and `grade-snowflake`. Supplies the semantic view's last-altered timestamp for `LAPSED_VERIFIED_QUERY`.
- `--ecosystem <dbt|ossie|snowflake|databricks|cube|looker|atscale>`: Pins the reader used by `perishables grade`.
- `--format <text|json|sarif|github>`: Selects output format (`corpus` supports `text` and `json`; all other commands support all four).
- `--output <PATH>` / `-o <PATH>`: Writes the rendered report to `PATH`. For `audit` and `drift`, `PATH` must lie outside the audited bundle directory.
- `--no-exec`: Skips Python subprocess execution of `Attested Computation` blocks during `audit`. Concepts claiming an attested result emit `UNATTESTED_CLAIM`.
- `--timing`: Includes wall-clock `elapsed_ms` on executed computations (omitted by default for byte-identical determinism).

### Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Clean pass (zero gate findings) |
| `1` | Gate failure (one or more trust findings, semantic findings, or corpus mismatches) |
| `2` | Usage error, unreadable path, or input syntax error |

## Finding code summary

### OKF bundle audit (`12` codes)

| Code | Trigger |
| --- | --- |
| `LAPSED_VERIFICATION` | Verification `at` predates `generated.at`, causing `recomputed_tier` to drop below `raw_tier` |
| `MALFORMED_VERIFICATION` | Verification entry lacks `by` actor or parseable `at` timestamp |
| `DANGLING_SUPERSEDED_BY` | `superseded_by` points to a missing concept, external URI, or root-escaping `../` path |
| `SUPERSESSION_CYCLE` | `superseded_by` chain revisits a concept or exceeds 64 hops |
| `SUPERSEDED_WITHOUT_DEPRECATED` | Concept carries `superseded_by` while `status` is not `deprecated` |
| `STALE_BUT_STABLE` | Pinned `AuditClock` is at or past `stale_after` while `status` is `stable` |
| `UNPARSEABLE_STALE_AFTER` | `stale_after` fails calendar or RFC 3339 parsing |
| `ATTESTED_RESULT_MISMATCH` | Deterministic Python re-execution succeeded and differed from `claimed_result` |
| `UNATTESTED_CLAIM` | Concept claims an attested result whose computation is missing, non-deterministic, failed, non-local, or skipped via `--no-exec` |
| `UNRESOLVED_CITATION` | Markdown body cites `[^label]` with no matching `sources[].id` |
| `MISSING_SOURCE` | In-bundle `sources[].resource` file does not exist in the bundle inventory |
| `SOURCE_DRIFT` | On-disk file SHA-256 digest differs from `sources[].sha256` |

### Cross-ecosystem semantic grading (`6` codes)

| Code | Trigger |
| --- | --- |
| `DANGLING_METRIC_REFERENCE` | Metric, LookML measure/dimension SQL, Cube view, or AtScale hierarchy references a missing field, measure, metric, or attribute |
| `NAME_COLLISION` | Two or more items or synonym aliases claim the same case-insensitive display name |
| `LAPSED_VERIFIED_QUERY` | Verified query `verified_at` predates `--changed-at` |
| `DANGLING_RELATIONSHIP` | Relationship, join, or LookML `extends` names a missing model, table, cube, or view |
| `BENCHMARK_WITHOUT_ANSWER` | Stored ground-truth benchmark or verified query has an empty question or empty SQL |
| `AMBIGUOUS_JOIN_PATH` | Direct and multi-hop relationships connect the same model pair at conflicting grains |

## Python SDK reference (`perishables`)

All functions live at the top level of `import perishables` and rely exclusively on the Python standard library.

### Core functions

- `audit(bundle_path, *, as_of, no_exec=False, timing=False, binary=None) -> AuditReport`
- `audit_sarif(bundle, *, as_of, no_exec=False, output=None, binary=None) -> dict[str, Any]`
- `drift(bundle_path, *, binary=None) -> DriftReport`
- `corpus(fixtures_dir=None, *, binary=None) -> CorpusReport`
- `grade(paths, *, ecosystem=None, changed_at=None, binary=None) -> GradeReport`
- `grade_dbt(manifest, *, binary=None) -> GradeReport`
- `grade_ossie(model, *, binary=None) -> GradeReport`
- `grade_snowflake(view, *, changed_at=None, binary=None) -> GradeReport`
- `grade_databricks(space, *, binary=None) -> GradeReport`
- `grade_cube(paths, *, binary=None) -> GradeReport`
- `grade_looker(paths, *, binary=None) -> GradeReport`
- `grade_atscale(model, *, binary=None) -> GradeReport`
- `grade_sarif(paths, *, ecosystem=None, changed_at=None, output=None, binary=None) -> dict[str, Any]`
- `completions(shell="bash") -> str` (`"bash"`, `"zsh"`, `"fish"`, `"powershell"`)
- `version(*, binary=None) -> str`
- `find_binary(binary=None) -> Path` (honors `PERISHABLES_BIN`)

### Report types and error handling

- `AuditReport.passed`, `GradeReport.passed`, `CorpusReport.passed`, and `DriftReport.clean` return `bool`.
- Calling `.raise_for_findings()` on an `AuditReport`, `GradeReport`, or `DriftReport` raises `PerishablesError` (`exit_code=1`) when any gate finding is present.
- Invalid CLI arguments, missing files, or malformed JSON/YAML/LookML inputs raise `PerishablesError` with `exit_code=2`.
