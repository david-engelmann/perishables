# Build round 9. Cross ecosystem slice 3, the Snowflake reader and the currency family

Date. 2026-11-24. The third build from the cross ecosystem design record, stacked on slices 1 and 2.

What landed. A Snowflake semantic view reader in the adapter crate, over a YAML block structure reader written for this slice, since the workspace carries no external dependencies. The YAML reader covers the subset semantic view YAML uses, block mappings and sequences, plain and quoted scalars, flow sequences, and literal block scalars, which is how verified query SQL is carried. Its boundary is stated in its own header. Anchors, tags, and folded scalars beyond the simple form are outside it, and block scalars survive in shape rather than byte for byte, which this slice accepts because the SQL is carried as a stated fact and never executed.

The mapping. Tables are the models, facts are measures, dimensions and time dimensions are dimensions, and table level metrics are metrics whose expressions carry no extracted references in this slice. Synonyms at table and element level are alias claims, graded by the collision family beside the Ossie ones. Relationships are declared joins, and the structural family gains DANGLING_RELATIONSHIP for a relationship naming a table the definition does not carry.

The currency family arrives with this slice, which is the ecosystem's reason for ranking second in the design record. A verified query is a verification event. It is current when its verified_at stands at or after the last change to the view, and LAPSED_VERIFIED_QUERY when it predates that change, the OKF currency rule in vendor dress. Two disciplines from the design record are enforced in code. verified_by is free text and earns no identity meaning, grading as coverage only. And the definition stores no change instant, so changed_at arrives from the caller, from object metadata, and the currency family grades only when it is present. A verification with no timestamp is a coverage note and never a lapse. The command line takes it as grade-snowflake with a --changed-at date.

The gate. Three planted fixtures. A clean view with a current verification grades clean. A lapsed view produces exactly one LAPSED_VERIFIED_QUERY, with its unstamped second query producing exactly two coverage notes and no finding, and the same fixture graded without a change instant produces no currency verdict at all. A dangling view produces exactly one DANGLING_RELATIONSHIP.

One correction inside the slice. The orphan measure note from slice 1 now fires only where metrics reference measures by name, the dbt shape. In expression based ecosystems an unreferenced measure is normal, and the note stays silent rather than teaching readers to ignore it.

Landing route. Stacked on cross-ecosystem-ossie-1 as branch cross-ecosystem-snowflake-1, for the standing route. Next in the order is Databricks serialized spaces, the last of the three charged ecosystems.
