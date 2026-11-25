# perishables

perishables is a deterministic auditor for Open Knowledge Format bundles. An OKF bundle is a folder of Markdown files whose frontmatter carries trust fields that say where a concept came from, who confirmed it, and when it stops being fresh. Existing tools check that those fields are well formed. perishables checks whether they are true at a pinned moment in time.

## What it does

Given a bundle and a pinned as-of date, perishables recomputes every trust tier counting only current verifications, so a confirmation that predates the last change to a concept no longer counts. It walks every superseded_by chain and reports targets that do not exist, chains that never end, and cycles. It flags concepts that still claim stable status after their stale_after date has passed. It checks that recorded sources resolve and that cited claims have sources to cite. Where a concept declares an Attested Computation in a deterministic local runtime, it runs the computation again and compares the result with the claimed one. Every run with the same bundle and the same as-of date gives the same report, byte for byte.

The repository also ships a corpus of small bundles with planted trust defects, one defect per bundle, each labelled with the rule it trips. Running the corpus shows which defects the auditor catches and proves the gate can fail.

## How it is used

```text
perishables audit ./bundle --as-of 2026-11-20
perishables corpus
```

The audit command prints a per concept report and exits non zero when the gate trips. The corpus command runs the auditor over the shipped corpus and compares every result with its label.

The checks also live in a library crate named perishables-core, so other tools can call the tier recompute, the currency check, the chain walk, and the supersession check directly instead of shelling out to the binary. The auditor never changes the bundle it reads.

## Grading semantic layers

The same discipline applies to the semantic layers AI tools answer from, and a second crate grades them. A semantic definition makes claims of the same kinds. A name claimed by two definitions, a reference to a measure or a field that does not exist, a verification recorded before the thing it verifies last changed, a stored benchmark with no answer behind it. Each ecosystem gets a reader that normalizes its definition into one grading model, and the rule families run over the model, format blind. The readers cover dbt semantic manifests, Apache Ossie documents in JSON and YAML, Snowflake semantic views, Databricks Genie serialized spaces, Cube models as meta JSON, YAML model files, and JS model files, Looker models as API documents and as native LookML files with project scope, and AtScale model exports.

```text
perishables grade-dbt manifest.json
perishables grade-ossie model.json
perishables grade-ossie model.yaml
perishables grade-snowflake view.yaml --changed-at 2026-11-20
perishables grade-databricks space.json
perishables grade-cube meta.json
perishables grade-cube model.yaml
perishables grade-cube model.js
perishables grade-looker model.json
perishables grade-looker orders.view.lookml customers.view.lookml sales.model.lookml
perishables grade-atscale model.json
```

Each command prints its findings and coverage notes and exits non zero when a finding trips. What grades offline is the shape and currency of the claims. Running stored ground truth against a live warehouse belongs to the vendor runtime and is outside these readers, stated in each reader rather than smoothed over. The adapter crate keeps the name perishables-dbt from its first slice. The name is historical, the crate is the cross ecosystem adapter, and renaming it is a separate change from the grading work.

## Current state

The design is settled and recorded in docs/DESIGN.md. The library crate implements the four core checks, tier recompute, verification currency, the supersession chain walk, and the supersession check, plus the freshness and provenance checks, and the corpus gate runs them against every fixture in corpus/. The command line adapter re executes Attested Computations under the deterministic python profile and reports the verdict table the design record specifies. Source drift began as an extension level check and joined the default gate on 2026-11-23, after the corpus proved it separates cleanly, one finding on its own fixture and silence on every other fixture. The cross ecosystem adapter implements the design record at docs/DESIGN-CROSS-ECOSYSTEM.md, with a planted defect gate behind every reader, and the native LookML reader follows its own design record at docs/DESIGN-LOOKML.md. Nothing here is released yet.

## License

Licensed under either of MIT or Apache License, Version 2.0 at your option.
