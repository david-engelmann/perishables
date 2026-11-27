# How perishables compares to other tools

`perishables` occupies a focused layer in the documentation and analytics stack: **offline, deterministic point-in-time trust and semantic integrity auditing**. It complements syntax validators, warehouse compilers, and live data observability tools rather than replacing them.

## Summary matrix

| Tool category | Examples | What those tools check | What `perishables` adds |
| --- | --- | --- | --- |
| Schema and syntax validators | JSON Schema, `yamllint`, Markdown linters | Whether YAML, JSON, or Markdown conforms to structural field types | Whether verification timestamps postdate generation (`LAPSED_VERIFICATION`), `stale_after` holds at `--as-of`, `superseded_by` chains are acyclic, `sources` SHA-256 hashes match disk (`SOURCE_DRIFT`), and `Attested Computation` results reproduce |
| Single-vendor semantic compilers | `dbt parse`, MetricFlow CLI | Syntax and single-tool DAG validity inside one vendor toolchain | Zero-dependency, sub-second offline grading across **seven** semantic ecosystems under one finding taxonomy, including case-insensitive label/synonym collisions (`NAME_COLLISION`), fan-trap routes (`AMBIGUOUS_JOIN_PATH`), ground-truth SQL completeness, and verified-query currency |
| Warehouse-connected BI validators | `spectacles` (Looker), live BI SQL runners | Whether generated SQL executes without database errors against a live cloud warehouse via API credentials | Offline `.view.lookml` and `.model.lookml` grading with zero Looker API or warehouse credentials, catching `${field}` / `${view.field}` dangling references, missing `extends` parents, and duplicate business synonyms before code leaves a developer laptop or PR |
| Live data observability platforms | Monte Carlo, Elementary, Soda, Great Expectations | Runtime warehouse table freshness, row-count anomalies, null rates, and distribution drift | Repository-time contract and provenance auditing over committed OKF bundles and semantic definitions, deterministic to the second via `--as-of` |

## Detailed comparisons

### Versus JSON Schema, YAML linters, and frontmatter validators

A JSON Schema or frontmatter validator confirms that `verified` is an array of objects with `by` and `at` strings and that `sources` entries have `sha256` strings. Every planted defect in `corpus/fixtures/` (`t1-lapsed` through `t9-source-drift`) is structurally valid Markdown and YAML that passes schema validation.

`perishables` evaluates the **relationships between those fields, time, files, and code**:

- Comparing `verified[].at` against `generated.at` to catch silently stale human sign-off (`LAPSED_VERIFICATION`).
- Comparing `stale_after` against a pinned `AuditClock` (`STALE_BUT_STABLE`, `UNPARSEABLE_STALE_AFTER`).
- Walking multi-hop `superseded_by` graphs across the bundle inventory (`SUPERSESSION_CYCLE`, `DANGLING_SUPERSEDED_BY`, `SUPERSEDED_WITHOUT_DEPRECATED`).
- Matching Markdown `[^footnote]` citations to `sources` entries and hashing referenced files on disk (`UNRESOLVED_CITATION`, `MISSING_SOURCE`, `SOURCE_DRIFT`).
- Re-running deterministic Python `Attested Computation` blocks in an AST-screened sandbox (`ATTESTED_RESULT_MISMATCH`, `UNATTESTED_CLAIM`).

### Versus `dbt parse` and MetricFlow

`dbt parse` and MetricFlow validate a dbt project within the Python dbt runtime. If your organization only uses dbt and already runs `dbt parse` in CI, `dbt parse` handles dbt-internal Jinja rendering and model compilation.

`perishables` addresses two gaps that single-vendor compilers leave open:

1. **Cross-ecosystem uniformity.** Analytics teams routinely pair dbt models with LookML views, Snowflake Cortex semantic views, Databricks Genie spaces, Cube models, AtScale exports, or Apache Ossie definitions. `perishables` grades all seven formats with one static binary, one exit-code contract, and one SARIF 2.1.0 ruleset ([CROSS-ECOSYSTEM.md](CROSS-ECOSYSTEM.md)).
2. **AI-analyst and business-vocabulary hazards.** Compilers rarely fail when two measures share a case-insensitive display label or business synonym (`NAME_COLLISION`), when a Databricks Genie benchmark question lacks expected SQL (`BENCHMARK_WITHOUT_ANSWER`), when a Snowflake verified query predates a semantic view edit (`LAPSED_VERIFIED_QUERY`), or when direct and transitive joins create an aggregation fan trap (`AMBIGUOUS_JOIN_PATH`).

### Versus `spectacles` and live Looker validators

Tools such as `spectacles` authenticate against a Looker instance and execute `EXPLAIN` or `SELECT` queries against your cloud warehouse to catch dialect-specific SQL errors and missing database columns.

`perishables grade-looker` ([LOOKML.md](LOOKML.md)) operates purely on `.lookml` files (or Looker API JSON) with zero network access and zero warehouse spend:

- Runs in milliseconds inside pre-commit hooks, air-gapped CI runners, or local agent loops.
- Catches broken `${field}` and `${view.field}` interpolations across `measure`, `dimension`, and `dimension_group` SQL blocks (`DANGLING_METRIC_REFERENCE`), missing join or `extends` targets (`DANGLING_RELATIONSHIP`), ambiguous join loops (`AMBIGUOUS_JOIN_PATH`), and duplicate labels or synonyms (`NAME_COLLISION`).
- Pairs naturally with warehouse SQL validators: run `perishables grade-looker` on every edit and PR commit, then run live warehouse SQL checks on staging merges.

### Versus live data-freshness monitors

Warehouse observability tools monitor whether physical ETL tables loaded on schedule and whether column values drifted in production. `perishables` audits whether **human-authored and AI-consumed definitions, metric contracts, and attested claims** remain trustworthy and internally consistent in version control. A warehouse table can load fresh rows every hour while the metric definition or executive memo built on top of it cites a drifted extract or a superseded formula; `perishables` catches the latter.

## Explicit non-goals

Keeping `perishables` fast, offline, and zero-dependency requires clear boundaries on what it will **not** do:

- **No live warehouse SQL execution.** `perishables` never opens network connections to Snowflake, BigQuery, Databricks, Postgres, or Looker APIs, and does not validate physical table columns against live database catalogs.
- **No Jinja or full-JS execution in semantic readers.** `perishables-semantic` grades compiled `manifest.json` / `semantic_manifest.json` for dbt and declarative `cube(...)` / `view(...)` models for Cube, returning explicit reader errors on arbitrary dynamic code rather than guessing partial grades.
- **No LLM or heuristic prose scoring.** Every finding in `perishables` is a deterministic boolean rule backed by a planted-defect fixture ([CLAIMS.md](CLAIMS.md)). It never calls an LLM or assigns probabilistic confidence scores.
- **No in-place bundle rewriting.** `perishables` diagnoses decay and outputs machine-readable verdicts (`text`, `json`, `sarif`, `github`); updating claims and recording new `human:` verifications belongs to explicit human review or downstream upkeep workflows.
