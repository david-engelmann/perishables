# Runnable examples

Every script in `examples/` runs offline against checked-in reference fixtures in `corpus/fixtures/` and `crates/perishables-semantic/tests/fixtures/`, writes temporary artifacts exclusively inside `mktemp` / `tempfile` directories, and is executed on every pull request in `.github/workflows/ci.yml`.

## Prerequisites

Build the `perishables` binary once from the repository root (or install the wheel / release binary on `PATH`):

```sh
cargo build --locked -p perishables --bin perishables
```

## Available walkthroughs

| Script | Surface | What it demonstrates |
| --- | --- | --- |
| [`audit_and_semantic_gate.py`](audit_and_semantic_gate.py) | Typed Python SDK (`python/perishables`) | Audits clean (`clean-team-verified`) and lapsed (`t1-lapsed`) OKF bundles at `--as-of 2026-11-20`, checks SHA-256 source drift (`t9-source-drift`), writes a SARIF 2.1.0 report via `audit_sarif`, and grades dbt, LookML, and Snowflake fixtures via `grade_dbt`, `grade_looker`, and `grade_snowflake` while asserting exact finding codes |
| [`ci_gate_walkthrough.sh`](ci_gate_walkthrough.sh) | POSIX CLI (`perishables`) | Runs `perishables corpus`, `perishables audit --format json`, `perishables drift`, and `perishables grade --format sarif` into an isolated `mktemp -d` output directory, verifying exit codes `0` (clean) and `1` (convicted findings) |

## Run locally

```sh
python3 examples/audit_and_semantic_gate.py
sh examples/ci_gate_walkthrough.sh
```

Or run the Rust library embedding example in `crates/perishables-core`:

```sh
cargo run -p perishables-core --example audit_bundle --locked
```
