# Build round 7. Cross ecosystem slice 1, the dbt adapter

Date. 2026-11-24. This round is the first build from the cross ecosystem design record, taken in the order the record sets, dbt and Ossie first.

What landed. A new adapter crate, perishables-dbt, with a hand written JSON reader, since the workspace carries no external dependencies. The reader accepts a dbt semantic manifest, semantic_manifest.json or the semantic_models and metrics keys of a full manifest.json, and builds the normalized grading model the design record specifies, semantic models, measures, dimensions, and metrics with their measure and metric references resolved out of type_params in both the string and the map forms.

Two rule families run over the model. Structural integrity as DANGLING_METRIC_REFERENCE, one finding for each measure or input metric a metric points at that does not exist. Discoverability as NAME_COLLISION, one finding for each display name, label where present else name, compared case insensitively, that two or more gradeable items claim. Coverage and orphan observations, missing descriptions and measures no metric references, are notes and never findings, in the posture the OKF auditor uses for a concept with no sources.

The gate. Three planted fixtures under the adapter tests assert exact finding sets. A clean manifest grades clean with no notes. A dangling manifest produces exactly its two dangling findings. A collision manifest, where one label is claimed by two metrics and one measure, produces exactly one collision finding naming all three owners.

The binary gains a grade-dbt command that reads one manifest file, prints findings and notes, and exits non zero on findings.

What did not change. The four locked OKF calls are untouched, and the OKF finding code set is untouched. Cross ecosystem codes are a separate closed set versioned with the adapter crate.

Landing route. The flagship repos joined the Trunk merge queue programme on 2026-11-24 and direct updates to main are refused by the branch ruleset, so this round lands as a branch and a pull request rather than a direct push. The branch is cross-ecosystem-dbt-1. The pull request opens from Main's side, where the working GitHub login lives, and the hosted checks run on the pull request under the workflow triggers.

Next in the order. The Ossie reader, whose ai_context synonyms feed the same collision family, and the entity graph, which carries multi path join ambiguity. Then Snowflake, then Databricks, as the design record orders them.
