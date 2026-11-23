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

## Current state

The design is settled and recorded in docs/DESIGN.md. The library crate implements the four core checks, tier recompute, verification currency, the supersession chain walk, and the supersession check, plus the freshness and provenance checks, and the corpus gate runs them against every fixture in corpus/. The command line adapter re executes Attested Computations under the deterministic python profile and reports the verdict table the design record specifies. Source drift began as an extension level check and joined the default gate on 2026-11-23, after the corpus proved it separates cleanly, one finding on its own fixture and silence on every other fixture. Nothing here is released yet.

## License

Licensed under either of MIT or Apache License, Version 2.0 at your option.
