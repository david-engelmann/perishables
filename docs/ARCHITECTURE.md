# Architecture

`perishables` is organized as a three-crate Rust workspace with zero external runtime dependencies. Every parser, cryptographic hash, calendar calculation, graph check, and report serializer is implemented directly on the Rust standard library.

## Crate boundaries

```text
perishables-core      <- pure std library, OKF bundle loader, trust checks, SHA-256, RFC 3339
perishables-semantic  <- 7-ecosystem semantic-layer readers + format-blind grading engine
perishables           <- CLI binary, Python AST sandbox executor, text/json/sarif/github formatters
```

### `perishables-core`

`perishables-core` loads Open Knowledge Format bundles from disk or in-memory concept slices and evaluates trust claims against a pinned `AuditClock`.

- `frontmatter.rs` parses the YAML frontmatter subset used in OKF Markdown files, strips unquoted `#` comments, extracts fenced code blocks (` ``` ` and `~~~`) with info-string attributes, extracts footnote citations outside code blocks and footnote definitions, and scans UTF-8 bodies for claimed attested results.
- `time.rs` validates proleptic Gregorian calendar dates (including 4-year, 100-year, and 400-year leap rules) and RFC 3339 timestamps, converting to and from UTC epoch seconds via Howard Hinnant's civil-day algorithms.
- `hash.rs` implements FIPS 180-4 SHA-256 over byte slices for deterministic source drift verification and bound computation artifact digests.
- `bundle.rs` inventories bundle files in sorted path order while skipping hidden files, reserved files (`index.md` and `log.md`), and symlinks so directory cycles cannot overflow the stack and symlinks cannot escape the bundle root.
- `checks.rs` implements the four locked calls (`recompute_tier`, `check_currency`, `walk_supersession`, and `check_supersession`) alongside `check_freshness`, `check_provenance`, `check_drift`, and `info_notes`.

### `perishables-semantic`

`perishables-semantic` normalizes semantic-layer definitions from seven analytics platforms into a single `Definition` model and evaluates six closed finding codes plus coverage and orphan notes.

- Hand-written format readers live in `json.rs`, `yaml.rs`, `js.rs`, and `lookml.rs`. Every reader rejects inputs outside its stated subset with an explicit `Err(String)` rather than producing partial grades.
- Ecosystem adapters live in `lib.rs` (dbt), `ossie.rs`, `snowflake.rs`, `databricks.rs`, `cube.rs`, `looker.rs`, `lookml.rs`, and `atscale.rs`.
- Shared rule families (`check_dangling`, `check_collisions`, `check_currency`, `check_relationships`, `check_join_paths`, `check_ground_truth`, and `notes`) operate strictly on `Definition` without consulting vendor syntax.

### `perishables`

`perishables` provides the CLI dispatcher (`main.rs`), the deterministic Python re-execution harness (`execute.rs`), and zero-dependency structured output formatters (`render.rs`).

- `execute.rs` validates parameter identifiers, binds typed or string-escaped parameter literals ahead of user code, hashes the bound source before execution, pre-screens the Python AST via `ast.parse`, enforces timeouts and concurrent 1 MiB pipe draining across threads, and cleans up an exclusive `0700` scratch directory after each run.
- `render.rs` emits canonical `text`, RFC 8259 `json`, OASIS `sarif` `2.1.0`, and GitHub Actions `github` workflow annotations.

## Determinism guarantees

1. Every timestamp comparison uses an explicit `AuditClock` supplied on the command line or API. No check calls `SystemTime::now()`.
2. `Bundle::load` traverses directories in lexicographical order and ignores host filesystem `mtime` metadata so fresh clones and historical checkouts produce identical reports.
3. Findings and notes are canonically sorted before rendering.
4. Wall-clock execution duration (`elapsed_ms`) is omitted from default `audit` output and included only when `--timing` is explicitly passed.
