# perishables

`perishables` is the command-line auditor and adapter library for Open Knowledge Format bundles and cross-ecosystem semantic-layer definitions.

## Installation

```sh
# Prebuilt Linux and macOS release archives verified against SHA256SUMS
curl -fsSL https://raw.githubusercontent.com/david-engelmann/perishables/main/install.sh | sh

# PyPI wheel with bundled native binary and typed Python SDK
pip install perishables

# Source build from crates.io
cargo install perishables
```

## Commands

```sh
perishables audit ./bundle --as-of 2026-11-20
perishables corpus
perishables drift ./bundle
perishables grade model.yaml orders.view.lookml
perishables grade-dbt manifest.json
perishables grade-ossie model.yaml
perishables grade-snowflake view.yaml --changed-at 2026-11-20
perishables grade-databricks space.json
perishables grade-cube model.js
perishables grade-looker orders.view.lookml sales.model.lookml
perishables grade-atscale model.json
```

Structured output is available on audit, drift, and semantic grading commands with `--format text`, `--format json`, `--format sarif`, or `--format github`, plus `--output <path>` to write reports outside the audited bundle.

## Shell Completions

Generate static tab-completion scripts for `bash`, `zsh`, `fish`, or `powershell` from the Python SDK (`python -m perishables completions <shell>` or `perishables.completions(shell)`) or in Rust via `perishables::completions`:

```rust
use perishables::completions::{Shell, completions_script};

let script = completions_script(Shell::Zsh);
assert!(script.starts_with("#compdef perishables"));
```

## Documentation

- [Architecture overview](https://github.com/david-engelmann/perishables/blob/main/docs/ARCHITECTURE.md)
- [OKF trust model](https://github.com/david-engelmann/perishables/blob/main/docs/DESIGN.md)
- [Cross-ecosystem semantic grading](https://github.com/david-engelmann/perishables/blob/main/docs/CROSS-ECOSYSTEM.md)
- [Claims and corpus traceability](https://github.com/david-engelmann/perishables/blob/main/docs/CLAIMS.md)
- [API reference on docs.rs](https://docs.rs/perishables)

## License

Licensed under either of MIT or Apache License, Version 2.0 at your option.
