.PHONY: ci fmt lint fuzz-check test corpus doc deny python-test examples book clean

ci: fmt lint fuzz-check test corpus doc deny python-test examples

fmt:
	cargo fmt --all -- --check
	cargo fmt --manifest-path fuzz/Cargo.toml --all -- --check

lint:
	cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

fuzz-check:
	CARGO_TARGET_DIR="$(CURDIR)/target" cargo check --manifest-path fuzz/Cargo.toml --locked --bins

test:
	cargo test --workspace --all-features --locked

corpus:
	cargo run --locked -- corpus

doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

deny:
	cargo deny check

python-test:
	cargo build --locked -p perishables --bin perishables
	python3 -m unittest discover -s python/tests -v

examples:
	cargo build --locked -p perishables --bin perishables
	python3 examples/audit_and_semantic_gate.py
	sh examples/ci_gate_walkthrough.sh

book:
	bash book/sync-docs.sh
	mdbook build book

clean:
	cargo clean
	rm -rf book/build book/src/docs
