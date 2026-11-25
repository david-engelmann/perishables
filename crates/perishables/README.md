# perishables

`perishables` is the command-line auditor for Open Knowledge Format bundles and cross-ecosystem semantic-layer definitions.

## Installation

```sh
cargo install perishables
```

## Commands

```sh
perishables audit ./bundle --as-of 2026-11-20
perishables corpus
perishables drift ./bundle
perishables grade-dbt manifest.json
perishables grade-ossie model.yaml
perishables grade-snowflake view.yaml --changed-at 2026-11-20
perishables grade-databricks space.json
perishables grade-cube model.js
perishables grade-looker orders.view.lookml sales.model.lookml
perishables grade-atscale model.json
```

Structured output is available on audit, drift, and semantic grading commands with `--format text`, `--format json`, `--format sarif`, or `--format github`, plus `--output <path>` to write reports outside the audited bundle.

## License

Licensed under either of MIT or Apache License, Version 2.0 at your option.
