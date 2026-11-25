# Cross-ecosystem semantic grading

Semantic layers used by AI analysts and BI tools make the same structural, naming, currency, and ground-truth claims as Open Knowledge Format bundles. `perishables-semantic` normalizes vendor definitions into a single `Definition` struct and runs six format-blind finding rules plus coverage and orphan observations over the result.

## Finding codes

| Code | Meaning |
|---|---|
| `DANGLING_METRIC_REFERENCE` | A metric, LookML measure, Cube view selection, or AtScale hierarchy level references a missing measure, field, metric, or attribute |
| `NAME_COLLISION` | Two or more metrics, measures, dimensions, or synonym aliases claim the same case-insensitive display name |
| `LAPSED_VERIFIED_QUERY` | A verified query records a `verified_at` instant earlier than the definition's `changed_at` timestamp |
| `DANGLING_RELATIONSHIP` | A declared relationship, join, or `extends` parent names a model, table, cube, or view absent from the definition |
| `BENCHMARK_WITHOUT_ANSWER` | A stored ground-truth benchmark or verified query has an empty question or empty expected SQL |
| `AMBIGUOUS_JOIN_PATH` | Two models connect both directly and over a two-edge or three-edge alternate join route that aggregates at a different grain |

## Informational notes

Observations that highlight coverage gaps or graph dead ends without representing contradictions travel as `Note` items and never fail the gate.

- Missing `description` or missing `label` on metrics, measures, and dimensions
- Missing `synonyms` on items within documents that actively use synonyms elsewhere
- Measures unreferenced by any metric in name-linked manifests such as dbt
- Multi-model definitions where a field-carrying model is reached by zero declared relationships
- Verified queries missing `verified_by` or `verified_at`, sample questions without stored answers, unmatched LookML `include` globs, and Cube `ai_context` strings at or past the 2,000-character silent truncation threshold

## Ecosystem readers and boundaries

### dbt (`grade-dbt`)

Reads `semantic_manifest.json` and full `manifest.json` (`semantic_models` and `metrics`), extracting simple, ratio, derived, and cumulative `type_params` references. Git-level freshness and free-form `meta` dictionaries lie outside the offline manifest reader.

### Apache Ossie (`grade-ossie`)

Reads Ossie JSON and YAML documents identically. Datasets map to models, fields map to dimensions, root `relationships` map to join edges, and `ai_context.synonyms` at model, dataset, field, and metric scope enter collision detection.

### Snowflake semantic views (`grade-snowflake`)

Reads semantic view YAML exported via `SYSTEM$READ_YAML_FROM_SEMANTIC_VIEW`. Tables map to models, `facts` map to measures, `dimensions` and `time_dimensions` map to dimensions, `synonyms` map to aliases, `relationships` map to joins, and `verified_queries` populate both `verifications` (accepting integer epoch seconds or RFC 3339 timestamps in `verified_at`) and `ground_truth`. Callers pass object-metadata modification time via `--changed-at` to enable `LAPSED_VERIFIED_QUERY`.

### Databricks Genie (`grade-databricks`)

Reads exported or API-fetched `serialized_space` JSON. Unity Catalog `data_sources.tables` and `column_configs` map to models and dimensions, `benchmarks.questions` and `instructions.example_question_sqls` map to `ground_truth`, and `config.sample_questions` map to `unverifiable_questions`. Live warehouse SQL execution and unexported OntoRank scores stay outside offline grading.

### Cube (`grade-cube`)

Reads REST `/v1/meta` JSON, YAML model files, and JavaScript `cube(...)` and `view(...)` files. Cube joins map to relationships, `meta.ai_context` character counts are checked against the 2,000-character truncation limit, and JS `view` `includes` and `excludes` selections validate against qualified `cube.member` fields. Arbitrary JavaScript functions or computed keys outside the declarative subset return reader errors rather than partial grades.

### Looker and LookML (`grade-looker`)

Reads assembled Looker API JSON (`views` and `explores`) as well as single or multi-file native `.lookml` projects. See `docs/LOOKML.md` for full grammar and project-scope rules.

### AtScale (`grade-atscale`)

Reads JSON model exports. Dimension attributes populate both `dimensions` and qualified `attributes`, `measures` and `calculation_groups` enter collision detection via `caption`, and `hierarchies` validate every level `attribute` reference against the parent dimension.
