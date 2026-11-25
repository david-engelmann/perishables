## Summary

Describe the defect fixed or capability added and reference any specification section or ADR affected.

## Verification checklist

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` passes
- [ ] `cargo test --workspace --all-features --locked` passes
- [ ] `cargo run --locked -- corpus` exits `0` with zero failures
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` passes
- [ ] Zero external dependencies added to `Cargo.lock`
