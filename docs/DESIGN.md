# OKF trust model and design specification

`perishables` audits whether the trust claims inside an Open Knowledge Format v0.3 bundle hold at a pinned moment in time. Structural validators check whether frontmatter fields are well-formed. `perishables` checks whether those claims remain current, coherent, and supported by their sources and computations.

## Trust model

### Verification currency and tier recomputation

Under OKF v0.3 SPEC 5.2 and 5.3, trust tiers are derived from current verification events and never stored.

- A verification event is well-formed when it carries both `by` and a parseable `at` timestamp. Events missing either field emit `MALFORMED_VERIFICATION` and count toward no tier.
- A well-formed event is current when its `at` instant is equal to or later than `generated.at` on the same concept, or when `generated.at` is absent. Events earlier than `generated.at` emit `LAPSED_VERIFICATION`.
- Zero current events yield `unverified`. One or more current events with no `human:` actor yield `machine-confirmed` (including `process:` and `team:` actors, because a team handle does not identify an individual human reviewer). At least one current `human:` event yields `human-reviewed`.
- Every audit reports both `raw_tier` (from all well-formed events) and `recomputed_tier` (from current events only), exposing silent tier drops after regeneration.

### Supersession chains

Under SPEC 5.4 and SPEC 6, `superseded_by` holds a single in-bundle concept path and is valid only when `status` is `deprecated`.

- Leading `/` paths resolve from the bundle root. Relative paths resolve from the directory of the carrying concept. External URIs (`https://`, `urn:`, and other RFC 3986 schemes) and `..` segments that escape the bundle root fail resolution and emit `DANGLING_SUPERSEDED_BY`.
- Bundles declaring `okf_version` `0.2` or earlier, or omitting `okf_version`, receive the SPEC 12.1 root-relative fallback when relative resolution fails. Any hop resolved via that fallback is recorded in `ChainReport::fallback_targets` and surfaced as an informational `NOTE`.
- Chains that revisit a node or exceed 64 hops emit `SUPERSESSION_CYCLE`. Multi-hop chains and headless cycles are deduplicated so each defective chain head or canonical cycle representative emits one finding.
- Any concept carrying `superseded_by` while its status is not `deprecated` emits `SUPERSEDED_WITHOUT_DEPRECATED`.

### Freshness (`stale_after`)

Under SPEC 5.5 and SPEC 11.2, a concept is stale when the pinned `AuditClock` is equal to or later than `stale_after`.

- Stale means check again rather than false. When a stale concept still presents `status: stable` (or omits `status`, which defaults to `stable`), the auditor emits `STALE_BUT_STABLE`.
- A stale concept with `status: deprecated` no longer claims currency and is surfaced as an informational `NOTE` rather than a finding.
- An unparseable `stale_after` string is never treated as fresh and emits `UNPARSEABLE_STALE_AFTER`.

### Provenance and source drift

Under SPEC 5.1, provenance is checked strictly on stated bundle facts without network fetches.

- Inline Markdown footnote citations (`[^label]`) outside fenced code blocks, inline code spans, and footnote definition lines must match a `sources` entry `id`. Unmatched citations emit `UNRESOLVED_CITATION`.
- In-bundle `sources` file paths must exist in the bundle file inventory. Missing files emit `MISSING_SOURCE`. External URLs are never fetched offline.
- Concepts carrying no `sources` list at all are conformant and surface as an informational `NOTE` rather than a finding.
- When an in-bundle `sources` entry records a `sha256` digest, `check_drift` hashes the file bytes via FIPS 180-4 SHA-256 and emits `SOURCE_DRIFT` on mismatch.

## Locked library interface

`perishables-core` exposes four locked entry points for downstream upkeep engines.

- `recompute_tier(concept, clock) -> TierReport`
- `check_currency(bundle, clock) -> Vec<Finding>`
- `walk_supersession(bundle, start) -> ChainReport`
- `check_supersession(bundle) -> Vec<Finding>`

All twelve `FindingCode` labels are closed under semantic versioning. Changing the meaning of a published `FindingCode` requires a major version bump.

## Scope discipline

`perishables` audits trust claims rather than document syntax or world truth. The following checks remain intentionally outside scope.

- General SPEC 11.1 schema conformance on untyped Markdown files, which belongs to structural validators.
- Enforcing a closed `type` taxonomy, since OKF v0.3 allows open domain types.
- Checking ordinary prose hyperlinks outside `superseded_by` and `sources`.
- Assigning subjective numeric credibility scores to sources or authors.
