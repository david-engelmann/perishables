# Contributing

Contributions to `perishables` follow strict invariants around determinism, dependency hygiene, and corpus-backed verification.

## Core invariants

- **Zero external runtime dependencies** across `perishables-core`, `perishables-semantic`, and `perishables`. All parsers, SHA-256 hashing, RFC 3339 calendar math, and JSON or SARIF serializers live in workspace code over `std`.
- **Zero ambient state** in `perishables-core`. Never read the system clock, environment variables, randomness, or host filesystem `mtime` inside `perishables-core`.
- **Zero bundle mutation** across every command. The auditor never writes inside an audited bundle directory.
- **Planted-defect proof** for every finding rule. Every `FindingCode` and semantic `Code` must have a minimal fixture that fails for that exact code and clean controls that stay silent.
- **Stable rule meanings** under semantic versioning. Published finding codes never change meaning and retired codes are never reused.

## Local verification suite

Run all gates before opening a pull request.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo run --locked -- corpus
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo run -p perishables-core --example audit_bundle --locked
```

## Documentation style

Public Markdown files use plain ASCII punctuation in prose. Avoid em dashes and en dashes anywhere in Markdown files, and keep colons and semicolons inside code spans or fenced code blocks rather than prose sentences.

## Licensing

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion is dual-licensed under MIT OR Apache-2.0 without additional terms.
