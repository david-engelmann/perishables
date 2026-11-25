# Claims and evidence

Every load-bearing promise in `README.md` and `docs/` maps directly to a CI guard, a planted-defect fixture in `corpus/fixtures/`, or a named test suite in the workspace.

## Workspace guarantees

| Claim | Enforcement |
| --- | --- |
| Zero external runtime or build crates in the dependency tree | `.github/workflows/ci.yml` (`Verify zero external dependencies in Cargo.lock` asserts `Cargo.lock` lists only `perishables`, `perishables-core`, and `perishables-semantic`). |
| Zero ambient wall-clock or host filesystem `mtime` reads | `AuditClock` required explicitly on `recompute_tier`, `check_currency`, and `check_freshness`, plus sorted directory traversal in `crates/perishables-core/src/bundle.rs`. |
| Zero mutation of audited bundle directories | `perishables corpus` computes recursive pre-audit and post-audit SHA-256 digests of every fixture directory and asserts `unchanged=true` across all 19 fixtures. |
| Untrusted bundles can be audited with zero subprocess spawns | `--no-exec` CLI flag tested in `crates/perishables/tests/cli.rs`, emitting `UNATTESTED_CLAIM` (`execution skipped (--no-exec)`) whenever an attested result is claimed. |
| Deterministic byte-for-byte output across `text`, `json`, `sarif`, and `github` | Canonical finding and note sorting in `crates/perishables/src/render.rs`, verified in `crates/perishables/tests/cli.rs`. |

## OKF finding codes (`perishables-core` and `corpus/`)

| Finding code | Planted-defect fixture(s) | What the gate proves |
| --- | --- | --- |
| `LAPSED_VERIFICATION` | `corpus/fixtures/t1-lapsed` | Downgrades `raw=human-reviewed` to `recomputed=unverified` and fails when `generated_at` is newer than `verified_at`. |
| `SUPERSESSION_CYCLE` | `corpus/fixtures/t2-cycle-two`, `corpus/fixtures/t2-cycle-three` | Detects 2-node and 3-node cycles in `superseded_by` chains without looping. |
| `DANGLING_SUPERSEDED_BY` | `corpus/fixtures/t3-dangling-missing`, `t3-dangling-external`, `t3-dangling-escape` | Rejects missing targets, external URLs, and bundle-escaping `../../` paths in `superseded_by`. |
| `STALE_BUT_STABLE` | `corpus/fixtures/t4-stale-stable` | Flags `status: stable` concepts whose `stale_after` timestamp has passed at the pinned `AuditClock`. |
| `ATTESTED_RESULT_MISMATCH` | `corpus/fixtures/t5-attested-mismatch` | Re-runs deterministic Python `Attested Computation` in an isolated scratch directory and fails when output differs from `claimed_result`. |
| `UNATTESTED_CLAIM` | `corpus/fixtures/t5-unattested-claim` | Rejects attested result claims backed by non-local or disallowed runtimes (such as `bigquery`) or skipped under `--no-exec`. |
| `UNRESOLVED_CITATION` | `corpus/fixtures/t6-provenance-footnote` | Flags Markdown body `[^id]` citations outside code blocks that match no declared `sources` entry. |
| `MISSING_SOURCE` | `corpus/fixtures/t6-provenance-missing-file` | Flags in-bundle `sources.resource` paths missing from disk or escaping the bundle root. |
| `SUPERSEDED_WITHOUT_DEPRECATED` | `corpus/fixtures/t7-superseded-without-deprecated` | Flags concepts carrying `superseded_by` while still declaring `status: stable`. |
| `UNPARSEABLE_STALE_AFTER` | `corpus/fixtures/t8-malformed-stale-after` | Rejects unparseable or invalid calendar dates on `stale_after` so malformed dates never silently pass as fresh. |
| `MALFORMED_VERIFICATION` | `corpus/fixtures/t8-malformed-verification` | Rejects verification events missing timestamps or carrying invalid metadata. |
| `SOURCE_DRIFT` | `corpus/fixtures/t9-source-drift` | Hashes bundled source files with FIPS 180-4 SHA-256 and fails when on-disk bytes diverge from recorded digests. |
| Clean controls (`0` findings) | `clean-healthy-chain`, `clean-machine-confirmed`, `clean-no-trust-fields`, `clean-team-verified` | Proves zero false positives across healthy supersession chains, machine and team verifications, and plain unverified notes. |

## Cross-ecosystem semantic grading (`perishables-semantic`)

| Ecosystem | Input shapes | Enforcing test suites |
| --- | --- | --- |
| dbt Semantic Layer | `semantic_manifest.json` and full `manifest.json` | `crates/perishables-semantic/tests/` and `crates/perishables/tests/cli.rs` |
| Apache Ossie | JSON and YAML semantic models | `crates/perishables-semantic/src/ossie.rs` and integration tests |
| Snowflake | Exported Semantic View YAML (`--changed-at`) | `crates/perishables-semantic/src/snowflake.rs` and CLI tests |
| Databricks Genie | Serialized Genie space JSON | `crates/perishables-semantic/src/databricks.rs` and CLI tests |
| Cube | REST `/v1/meta` JSON, `.yaml` models, and `.js` `cube(...)` / `view(...)` files | `crates/perishables-semantic/src/cube.rs`, `js.rs`, and CLI tests |
| Looker and native LookML | Assembled API JSON and `.view.lookml` / `.model.lookml` files (`extends`, `include` globs) | `crates/perishables-semantic/src/looker.rs`, `lookml.rs`, and CLI tests |
| AtScale | Exported model JSON | `crates/perishables-semantic/src/atscale.rs` and CLI tests |
