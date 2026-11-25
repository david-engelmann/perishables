# perishables-semantic

`perishables-semantic` normalizes semantic-layer definitions across seven analytics ecosystems into one format-blind grading model and evaluates structural integrity, name collisions, verification currency, and ground-truth completeness.

## Supported ecosystems

- dbt semantic manifests in `semantic_manifest.json` and `manifest.json` shapes
- Apache Ossie models in JSON and YAML
- Snowflake semantic views exported as YAML
- Databricks Genie serialized spaces in JSON
- Cube models as REST meta JSON, YAML model files, and JavaScript model files
- Looker models as assembled API JSON and native LookML files with project scope
- AtScale model exports in JSON

## Usage

```rust
use perishables_semantic::{grade, notes, read_definition};

let manifest = r#"{"semantic_models": [], "metrics": []}"#;
let definition = read_definition(manifest).expect("valid manifest");
assert!(grade(&definition).is_empty());
assert!(notes(&definition).is_empty());
```

## Documentation

- [Cross-ecosystem semantic grading specification](https://github.com/david-engelmann/perishables/blob/main/docs/CROSS-ECOSYSTEM.md)
- [Native LookML reader specification](https://github.com/david-engelmann/perishables/blob/main/docs/LOOKML.md)
- [API reference on docs.rs](https://docs.rs/perishables-semantic)

## License

Licensed under either of MIT or Apache License, Version 2.0 at your option.
