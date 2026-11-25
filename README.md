# perishables

`perishables` is a deterministic trust-claim auditor for Open Knowledge Format bundles and cross-ecosystem semantic layers. Verified claims have a shelf life. Structural validators check whether trust fields parse. `perishables` checks whether those claims remain true, current, and coherent at a pinned instant in time, with zero external runtime dependencies.

## Capabilities

- **OKF bundle auditing** recomputes every concept trust tier from current verifications only, walks `superseded_by` chains for dangling targets and cycles, flags `stable` concepts whose `stale_after` window has elapsed, verifies footnote citations and in-bundle source paths, checks SHA-256 source drift, and re-executes deterministic Python `Attested Computation` blocks against claimed results.
- **Cross-ecosystem semantic grading** normalizes semantic definitions from dbt, Apache Ossie, Snowflake, Databricks Genie, Cube, Looker and native LookML, and AtScale into one format-blind model, grading dangling metric and field references, display-name and synonym collisions, fan-trap join paths, lapsed verified queries, and incomplete ground-truth benchmarks.
- **Planted-defect reference corpus** ships nineteen minimal OKF bundles in `corpus/fixtures`, covering all twelve OKF finding codes plus clean controls, proving the gate fails on every planted defect and leaves audited directories byte-for-byte unchanged.
- **Byte-for-byte determinism** takes every instant from an explicit `--as-of` clock, reads no ambient wall clock or host filesystem modification timestamps, sorts every finding and note canonically, and emits `text`, `json`, `sarif`, or `github` reports.

## Installation

```sh
cargo install perishables
```

Prebuilt binaries for Linux, macOS, and Windows are published on GitHub Releases and support `cargo binstall perishables`.

## Auditing OKF bundles

```sh
perishables audit ./bundle --as-of 2026-11-20
perishables audit ./bundle --as-of 2026-11-20 --format sarif --output audit.sarif
perishables audit ./untrusted-bundle --as-of 2026-11-20 --no-exec
perishables drift ./bundle
perishables corpus
```

`perishables audit` prints per-concept raw and recomputed trust tiers, computation verdicts, informational notes, and gate findings. Exit code `0` means zero findings, `1` means one or more gate findings, and `2` means a usage or parse error.

## Grading semantic layers

```sh
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

A unified `perishables grade <file>... [--ecosystem <name>]` command is also available and supports `--format text|json|sarif|github` and `--output <path>`.

## Workspace crates

| Crate | Role |
|---|---|
| `perishables-core` | Pure-Rust, zero-dependency OKF trust engine and locked four-call library surface |
| `perishables-semantic` | Cross-ecosystem readers and format-blind grading rules for seven semantic platforms |
| `perishables` | Command-line auditor, Python AST sandbox runner, and structured report formatters |

## Embedding as a library

```rust
use perishables_core::{
    AuditClock, Bundle, check_currency, check_drift, check_freshness,
    check_provenance, check_supersession, recompute_tier,
};
use std::path::Path;

let clock = AuditClock::parse("2026-11-20").expect("valid date");
let bundle = Bundle::load(Path::new("corpus/fixtures/clean-healthy-chain"))
    .expect("bundle loads");

for concept in &bundle.concepts {
    let tier = recompute_tier(concept, &clock);
    println!("{} -> {}", tier.concept_id, tier.recomputed_tier.name());
}
assert!(check_currency(&bundle, &clock).is_empty());
assert!(check_supersession(&bundle).is_empty());
assert!(check_freshness(&bundle, &clock).is_empty());
assert!(check_provenance(&bundle).is_empty());
assert!(check_drift(&bundle).is_empty());
```

## GitHub Actions integration

```yaml
- uses: david-engelmann/perishables@v0.1.0
  with:
    command: audit
    path: ./knowledge-bundle
    as-of: "2026-11-20"
    format: github
```

## Documentation

- [Architecture overview](docs/ARCHITECTURE.md)
- [OKF trust model and design specification](docs/DESIGN.md)
- [Cross-ecosystem semantic grading specification](docs/CROSS-ECOSYSTEM.md)
- [Native LookML reader specification](docs/LOOKML.md)
- [Attested Computation and in-toto design](docs/ATTESTATION.md)
- [Architectural decision records](DECISIONS.md)
- [Security policy and execution sandbox](SECURITY.md)

## License

Licensed under either of MIT or Apache License, Version 2.0 at your option.
