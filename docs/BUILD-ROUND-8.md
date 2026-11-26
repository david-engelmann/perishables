# Build round 8. Cross ecosystem slice 2, the Ossie reader

Date. 2026-11-24. The second build from the cross ecosystem design record, stacked on slice 1, which grades dbt manifests.

What landed. An Ossie reader in the same adapter crate, reading the JSON form of an Apache Ossie document into the same normalized grading model. Datasets map to semantic models, fields map to gradeable items in the dimension family, and metrics map to metrics with no references extracted, because an Ossie metric expression is a code string and parsing references out of code is a design question of its own, recorded here rather than improvised.

The new surface is ai_context. Its synonyms are claims that an item answers to another name, so the grading model gains alias claims, recorded at model, dataset, field, and metric level with their owners, and the collision family reads alias claims beside display names. A synonym claimed by two different fields is now one NAME_COLLISION finding naming both owners, which is the discoverability defect this ecosystem exists to catch.

The gate. Two planted fixtures assert exact results. A clean document, with model, dataset, and field synonyms that collide with nothing, grades clean with three alias claims recorded and no notes. A collision document, where two fields claim the same synonym, produces exactly one collision finding. The binary gains a grade-ossie command beside grade-dbt, sharing one grading path.

Stated boundary. This slice reads the JSON form of an Ossie document. JSON is valid YAML and the converter ecosystem emits it. YAML native reading is a later slice if a build is charged for it.

Landing route. This round is stacked on the slice 1 branch, cross-ecosystem-dbt-1, as branch cross-ecosystem-ossie-1, because slice 1 has not landed yet. Its pull request bases on the slice 1 branch when Main opens it from the standing route, and it retargets to main once slice 1 lands.

Next in the order. Snowflake semantic view YAML, the currency family on verified queries, then Databricks serialized spaces, as the design record orders them.
