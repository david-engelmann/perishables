# Checkpoint 1 gate report, rounds 1 to 5 under review

Scope. The checkpoint review of the complete auditor, run under the house rule of three reviews before landing past a major checkpoint. Two independent axes reviewed the tree at head 08fc72da in parallel, Standards and Spec fidelity, each reporting separately. This report records what they found, what was fixed, and what stands as a documented deviation with its reason.

The fix pass landed as commit 67942a21. Its first hosted run, 45114822071, failed at clippy on a stray helper that never had a call site, written against a misremembered parser shape. Two repair commits followed, 9d5b9206 removing the helper and 5d39aed6 collapsing a match arm clippy flagged, and the checkpoint state is hosted green on the exact head 5d39aed6, run 45186081113, format, clippy, the full test suite, and the build all passing.

## What the Spec fidelity axis found and what changed

The corpus command was not the reference runner the design specifies. It now reads the manifest and every label, states the corpus and auditor versions, compares finding code sets exactly per fixture, and hashes each fixture before and after the run, failing when any fixture disagrees or changes.

Computation verdicts were computed and dropped. The audit now prints one line per computation with its verdict, reason, artifact hash, elapsed time, and bound. The raw tier now prints beside the recomputed tier.

Parameter binding was absent. Declared parameter values now bind as variables before the computation runs, and the bound artifact is hashed before execution and reported.

A claimed result on an unattestable or out of profile computation produced no finding. It now produces UNATTESTED_CLAIM.

The cost bound was a bare timeout. Runs now record elapsed time and the bound, the limits are configurable through environment variables with conservative defaults, and a memory limit is enforced through the resource limits the runner sets before the computation starts.

The three reported as information statements had no surface. The library now returns information notes covering stale deprecated concepts, concepts with no recorded provenance, and the source ordering fact for sources without a recorded hash.

The as-of argument had a hardcoded default. It is now required.

A declared okf_version that does not parse no longer receives the v0.2 fallback. URI scheme detection now covers schemes without slashes. Corpus tests assert exact code sets, and the never mutates test covers the whole fixture tree.

## What the Standards axis found and what changed

Bundle loading silently returned an empty bundle for an unreadable root and the CLI reported a clean pass. Loading now propagates the error and the CLI exits with a usage error. Drift byte read failures propagate the same way.

The executor scratch directory was shared per process and raced under parallel calls. It is unique per call.

Path resolution existed twice. The supersession resolver now delegates to the single file path resolver. Finding sorting existed twice at different strengths. The CLI uses the one core ordering. The CLI print and load blocks are shared helpers.

Dead surface is wired or corrected. The verdict name and reason functions drive the computation lines. The from_concepts documentation now says what the constructor is.

## Documented deviations, with reasons

The sandbox contract is met in the v1 profile by stronger and weaker means at once. Computations receive no bundle access at all, which is stronger than a read only view, so computations over bundle files are outside the profile. Network reach is screened statically, not blocked by the operating system. The interpreter invocation is pinned as python3 -I.

Discovery follows the type scan. Link following from concepts that use a computation would matter for claims made by using concepts, and no corpus fixture or label covers that shape yet, so it waits for a fixture.

Non deterministic verdicts are assigned by static screening, as they have been since round 4. Dynamic detection would need syscall tracing and is out of the v1 profile.

The faircopy dogfood bundle named in the design record is not in this corpus. It is a shared fixture owned with the faircopy seat and joins when that seat supplies it. Nothing in this repo can fabricate it.

The section 7 sketch names distinct CurrencyFinding and SupersessionFinding types. Both calls return the shared Finding type carrying the required concept id and code. Dismissed as a naming difference with the substance intact.

The Status enum suggestion from the Standards axis was not taken. The loader must preserve unknown status values verbatim, and the reviewer noted the call as defensible either way.
