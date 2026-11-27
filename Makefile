.PHONY: ci fmt lint test corpus doc deny python-test clean

ci: fmt lint test corpus doc deny python-test

fmt:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

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

clean:
	cargo clean
