# Attested Computation execution and attestation design

Open Knowledge Format v0.3 section 10 defines `Attested Computation` concepts that pair a declared computation and parameter contract with a claimed result. `perishables` implements deterministic local re-execution in `crates/perishables/src/execute.rs`.

## Verdict vocabulary and gate behavior

Every concept with `type: Attested Computation` receives one verdict.

| Verdict | Gate effect | Meaning |
|---|---|---|
| `RECOMPUTED_MATCH` | Pass | Computation ran cleanly under the deterministic profile and matched the claimed result |
| `MISMATCH` | `ATTESTED_RESULT_MISMATCH` | Computation ran cleanly and produced a value different from the claimed result |
| `NOT_ATTESTABLE` | `UNATTESTED_CLAIM` when claimed | Missing runtime, missing code block or linked `.py` file, invalid parameter identifier or typed literal, or missing claimed result |
| `NOT_EXECUTABLE_IN_PROFILE` | `UNATTESTED_CLAIM` when claimed | Declared runtime (such as `bigquery` or `snowflake`) lies outside the local deterministic allowlist |
| `NON_DETERMINISTIC_PROFILE` | `UNATTESTED_CLAIM` when claimed | AST pre-screen detected a restricted module import, builtin call, or dunder attribute |
| `EXECUTION_FAILED` | `UNATTESTED_CLAIM` when claimed | Interpreter exited non-zero, failed to assign `result`, exceeded output capture bounds, or timed out |
| `SKIPPED` | None | Caller explicitly passed `--no-exec` to disable subprocess execution |

`RECOMPUTED_MATCH` is the sole verdict that confirms an attested claim. Whenever a bundle states a claimed result whose computation cannot be verified under the profile, `UNATTESTED_CLAIM` trips the gate so broken or unverifiable claims never pass silently.

## Deterministic Python profile

1. **Parameter validation and binding** checks every declared parameter name against ASCII Python identifier rules, rejecting reserved keywords, `result`, and blocked builtins. Parameter values bind ahead of the computation code as validated `int`, `float`, `bool`, or control-escaped Python string literals, and the SHA-256 digest of that bound source is recorded as `artifact_hash` before execution.
2. **AST determinism pre-screen** parses the bound source with Python `ast.parse` prior to bytecode compilation. Comments and string literals are ignored by construction. Imports of OS, filesystem, clock, RNG, network, threading, subprocess, serialization, and introspection modules, relative imports, references to `open`, `eval`, `exec`, `compile`, `__import__`, `input`, and `breakpoint`, and dunder escape attributes (`__builtins__`, `__globals__`, `__subclasses__`, `__loader__`, `__spec__`, and `__import__`) exit immediately as `NON_DETERMINISTIC_PROFILE` before user code runs.
3. **Isolated subprocess execution** invokes Python with `-I -B` inside an exclusive `0700` scratch directory that is deleted immediately upon completion, draining stdout and stderr concurrently with a 1 MiB cap under a wall-clock timeout (`5000` ms default) and best-effort `RLIMIT_AS` or `RLIMIT_DATA` bounds.

## Travelling verdicts and in-toto mapping

`ComputationReport` records every field required to serialize a verifiable [in-toto Statement v1](https://in-toto.io/Statement/v1) predicate without re-running the computation.

- **Subject** pairs the concept id with the `sha256` `artifact_hash` of the bound parameters plus computation text, so any edit to parameters or code invalidates stale statements without executing code.
- **Predicate** carries the verdict name, optional reason, claimed and recomputed tokens, `python` profile identifier, `perishables` version, pinned `as_of` timestamp, and configured `timeout_ms` bound, deliberately omitting machine-dependent `elapsed_ms` so statements remain byte-comparable across runs.
