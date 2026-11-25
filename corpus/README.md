# Planted-defect corpus

This directory holds the `perishables` reference corpus of Open Knowledge Format bundles (`corpus_version = "0.2.0"`). Each `expect = "fail"` fixture plants a single trust defect and carries a `LABEL.toml` file naming the exact `FindingCode` and concept it trips. Each `expect = "pass"` fixture serves as a clean control that must produce zero findings.

`corpus/manifest.toml` inventories all nineteen fixtures alongside the pinned `as_of` date (`2026-11-20`) and compatible auditor version range. Any OKF auditor can evaluate these bundles directly against `manifest.toml` and `LABEL.toml` without depending on the `perishables` binary.

Run the reference gate from the repository root with `perishables corpus` or `perishables corpus --format json`. The runner verifies manifest completeness, checks every finding code and recomputed tier against `LABEL.toml`, and hashes each fixture directory before and after auditing to prove zero mutation.
