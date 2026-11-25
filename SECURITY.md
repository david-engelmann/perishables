# Security policy

## Supported versions

| Version | Supported |
|---|---|
| `0.1.x` | Yes |

## Threat model and execution boundaries

- `perishables-core` and `perishables-semantic` are pure, safe Rust (`#![forbid(unsafe_code)]`) with zero external dependencies. Neither crate spawns subprocesses, opens network sockets, follows symlinks during directory traversal, or writes files.
- `perishables audit` re-executes `Attested Computation` concepts that declare `runtime: python` by default. Before executing any Python bytecode, the runner parses the bound source with Python `ast.parse` and rejects imports of operating-system, file, network, clock, randomness, and introspection modules as well as calls or references to `open`, `eval`, `exec`, `compile`, `__import__`, `input`, and `breakpoint` and dunder escape attributes.
- Python execution runs under `-I -B` in an exclusive `0700` temporary directory that is deleted immediately after completion, with cleared environment variables, bounded 1 MiB pipe capture, a configurable wall-clock timeout (`PERISHABLES_EXEC_TIMEOUT_MS`, default `5000`), and best-effort OS memory rlimits (`PERISHABLES_EXEC_MEMORY_MB`, default `256`).
- Python AST screening is a determinism guard rather than a kernel-level security boundary against hostile code. When auditing untrusted third-party bundles where arbitrary Python execution is unacceptable, pass `--no-exec` on `perishables audit`. Under `--no-exec`, zero subprocesses are spawned, every static trust check still runs, and computation concepts are reported as `SKIPPED`.
- `--output <path>` rejects target paths inside the audited bundle directory so report generation cannot overwrite bundle files.

## Reporting a vulnerability

Report suspected security vulnerabilities privately through GitHub Security Advisories on `david-engelmann/perishables` rather than opening a public issue. Include a minimal reproduction bundle or input file and the exact `perishables --version` output.
