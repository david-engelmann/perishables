# Build round 15. Cross ecosystem slice 9, the Cube YAML form

Date. 2026-11-25. The ninth build from the cross ecosystem design record, stacked on slices 1 to 8, taken in the same pass as slice 8.

What landed. The Cube reader gains its YAML model file form, closing the second JSON first boundary. The mapping is shared exactly as the Ossie slice shared its own, the meta JSON and the YAML file differing only in the parser that feeds them, and grade-cube picks the reader by extension. The JS model files remain a later surface, stated in the reader header.

One hazard was checked while building the fixtures and found already handled. An empty flow list, which YAML model files use constantly for a cube with no measures, would plant a phantom measure with no name if the parser read it as one empty string. The YAML reader as written for the Snowflake slice already returns an empty list for it, and the clean fixture pins that behavior, since its customers cube carries an empty measures list and the model equality assertion would catch a phantom.

The gate is equivalence again. The YAML fixtures mirror the meta fixtures cube for cube, the tests assert equal models between the forms, and the dangling join grades identically in YAML.

Landing route. Stacked on cross-ecosystem-ossie-yaml-1 as branch cross-ecosystem-cube-yaml-1, for the standing route.

What the programme queues next. Both stated form boundaries in the adapter are now closed. The stack stands nine deep and is the next checkpoint, and the three reviews over the whole adapter surface matter more with every slice, because the surface is now seven ecosystems in two forms each where applicable. Behind the checkpoint stand a native LookML reader as its own larger project, the behavioral harnesses needing vendor accounts and Main's charge, and the attestation note still standing with Main for review.
