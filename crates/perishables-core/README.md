# perishables-core

`perishables-core` is the zero-dependency Rust library that checks whether the trust claims inside an Open Knowledge Format bundle hold at a pinned instant in time.

## Guarantees

- Zero external dependencies, relying strictly on the Rust standard library.
- Zero ambient clock reads, taking every instant from an explicit `AuditClock`.
- Zero mutations, reading bundle files in sorted path order without writing to disk.
- Stable locked interface for downstream upkeep engines through `recompute_tier`, `check_currency`, `walk_supersession`, and `check_supersession`.

## Usage

```rust
use perishables_core::{AuditClock, Bundle, check_currency, check_supersession, recompute_tier};

let clock = AuditClock::parse("2026-11-20").expect("valid date");
let bundle = Bundle::from_concepts(Vec::new());
let currency_findings = check_currency(&bundle, &clock);
let chain_findings = check_supersession(&bundle);
assert!(currency_findings.is_empty());
assert!(chain_findings.is_empty());
```

## Documentation

- [OKF trust model and design specification](https://github.com/david-engelmann/perishables/blob/main/docs/DESIGN.md)
- [Architecture overview](https://github.com/david-engelmann/perishables/blob/main/docs/ARCHITECTURE.md)
- [API reference on docs.rs](https://docs.rs/perishables-core)

## License

Licensed under either of MIT or Apache License, Version 2.0 at your option.
