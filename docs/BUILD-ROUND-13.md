# Build round 13. Cross ecosystem slice 7, the AtScale reader

Date. 2026-11-25. The seventh build from the cross ecosystem design record, stacked on slices 1 to 6, taken in the same pass as slice 6 per the sweep's charge.

What landed. An AtScale reader in the adapter crate over the JSON reader, reading a semantic model export in the shape its fixtures state. The export shape varies by surface, and the reader treats anything outside the stated shape as absent rather than guessing, which the reader header says plainly.

The mapping. Dimensions carry attributes, and attributes do double duty. They are dimension items for coverage, and they enter the model's new attribute set, because hierarchy levels reference attributes by qualified name and a reference must resolve against something. The reference check now resolves metric references against metrics and attributes both, with the finding detail naming which kind was missed. Hierarchies become metrics whose references are their levels' attributes, so a level naming an attribute the dimension does not define grades as a dangling reference. Measures map as before with captions as display names, and calculation groups, AtScale's organization wide shared definitions, enter as measures under the model because their captions are display claims like any other and the collision family should see them.

The gate. A clean model grades clean, attributes, measures, calculation group, and hierarchy all resolving. A hierarchy level naming a missing attribute produces exactly one dangling finding whose detail names the attribute.

Landing route. Stacked on cross-ecosystem-looker-1 as branch cross-ecosystem-atscale-1, for the standing route.

What the programme queues next. The research ranking's gradeable surfaces are now all read, dbt, Ossie, Snowflake, Databricks, Cube, Looker, and AtScale. The stack stands seven deep and is the next checkpoint. When it lands, the three reviews run over the whole adapter surface as one checkpoint. Behind that stand the YAML native readers for the Ossie and Cube model files, a native LookML reader as its own larger project, and the behavioral harnesses, which need vendor accounts and remain Main's call to charge. The attestation note still stands with Main for review.
