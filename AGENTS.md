# Using and working on perishables with AI agents

This guide covers both **driving `perishables`** from an AI coding or analytics agent and **modifying the Rust or Python workspace**.

## Driving the CLI from an agent

Prefer `--format json` when inspecting structured findings programmatically, `--format sarif` when writing OASIS SARIF 2.1.0 artifacts for code-scanning pipelines, and `--format github` inside GitHub Actions jobs.

```sh
perishables audit ./bundle --as-of 2026-11-20 --format json
perishables audit ./untrusted-bundle --as-of 2026-11-20 --no-exec --format json
perishables drift ./bundle --format json
perishables grade model.yaml orders.view.lookml --format sarif --output semantic.sarif
perishables corpus --format json
```

### Exit code contract

Every command follows a strict three-code exit contract:

| Exit code | Meaning | Agent action |
| --- | --- | --- |
| `0` | Pass (zero gate findings) | Parse stdout JSON or SARIF; gate passed |
| `1` | Gate failure (one or more trust or semantic findings) | Parse stdout JSON or SARIF `findings` / `results` to inspect exact codes and items |
| `2` | Command usage, file I/O, or syntax parse error | Read stderr; fix invocation flags, missing path, or malformed input file |

### Untrusted bundle rule (`--no-exec`)

By default, `perishables audit` re-executes `Attested Computation` Python blocks inside an AST-screened, isolated `0700` scratch directory (`python3 -I -B`) to verify `claimed_result` values. When auditing **untrusted** bundles or running in environments where subprocess execution is disallowed, **always pass `--no-exec`** (or `no_exec=True` in Python). Under `--no-exec`, zero subprocesses are spawned and any concept claiming an attested result emits `UNATTESTED_CLAIM`.

## Driving the Python SDK from an agent

The `perishables` Python package wraps the native Rust binary with typed standard-library dataclasses (`AuditReport`, `DriftReport`, `GradeReport`, `CorpusReport`) and SARIF helpers:

```python
import perishables

# OKF bundle audit and SHA-256 drift check
audit_rep = perishables.audit("corpus/fixtures/clean-team-verified", as_of="2026-11-20", no_exec=True)
audit_rep.raise_for_findings()

drift_rep = perishables.drift("corpus/fixtures/clean-team-verified")
assert drift_rep.clean

# Ecosystem-specific semantic grading (dbt, ossie, snowflake, databricks, cube, looker, atscale)
dbt_rep = perishables.grade_dbt("manifest.json")
lookml_rep = perishables.grade_looker(["orders.view.lookml", "sales.model.lookml"])
sf_rep = perishables.grade_snowflake("view.yaml", changed_at="2026-11-20")

# SARIF 2.1.0 dictionaries (returned on both exit 0 and exit 1; raises PerishablesError on exit 2)
audit_sarif_doc = perishables.audit_sarif("corpus/fixtures/t1-lapsed", as_of="2026-11-20", no_exec=True)
grade_sarif_doc = perishables.grade_sarif(["orders.view.lookml"], ecosystem="looker")
```

Set `PERISHABLES_BIN=/path/to/perishables` if driving a custom build outside `target/release`, `target/debug`, or `PATH`.

## Modifying the repository

Read these specifications first:

| Document | What it covers |
| --- | --- |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Crate boundaries (`perishables-core`, `perishables-semantic`, `perishables`) and determinism rules |
| [docs/DESIGN.md](docs/DESIGN.md) | Open Knowledge Format v0.3 trust tiers, verification currency, supersession chains, freshness, and provenance |
| [docs/ATTESTATION.md](docs/ATTESTATION.md) | Deterministic Python AST pre-screening, sandbox execution, and in-toto mapping |
| [docs/CROSS-ECOSYSTEM.md](docs/CROSS-ECOSYSTEM.md) | Normalized `Definition` model and grading rules across dbt, Ossie, Snowflake, Databricks, Cube, Looker, and AtScale |
| [docs/LOOKML.md](docs/LOOKML.md) | Native LookML lexer, block-tree parser, `extends` inheritance, and project-scoped `${view.field}` resolution |
| [docs/CLAIMS.md](docs/CLAIMS.md) | Traceability from every public claim and finding code to corpus fixtures and test suites |
| [examples/README.md](examples/README.md) | Runnable Python SDK and CLI CI gate walkthroughs |

Skip `docs/BUILD-ROUND-*.md` unless specifically researching historical implementation notes. Those files are maintainer build logs rather than current user or contributor specifications.

### Non-negotiable workspace invariants

1. **Zero external crates in root `Cargo.lock`.** `Cargo.lock` may contain only `perishables`, `perishables-core`, and `perishables-semantic`. Never add third-party crates to workspace members (`fuzz/` is a standalone out-of-workspace crate with its own `fuzz/Cargo.lock`).
2. **Zero `unsafe` and zero missing docs.** Workspace lints enforce `unsafe_code = "forbid"`, `missing_docs = "deny"`, and `clippy::all = "deny"`.
3. **Zero ambient clock or `mtime` reads.** Never call `SystemTime::now()` or inspect filesystem modification timestamps inside audit or grading logic; thread an explicit `AuditClock` or `--changed-at` instant instead.
4. **Markdown typography bar.** CI rejects em dashes (`U+2014`) and en dashes (`U+2013`) in any `.md` file across the repository. Use ASCII hyphens, commas, colons, or parentheses.
5. **Canonical absolute links in `README.md`.** `README.md` is published directly to PyPI and crates.io, so every link and image source in `README.md` must use a full `https://` URL.

### Local verification gate

Run `make ci` (or the underlying commands directly) before finishing any change:

```sh
make ci
```
