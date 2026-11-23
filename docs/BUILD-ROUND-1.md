# Build round 1 gate report

Round scope. Implement the four locked checks in perishables-core against the golden corpus, with MALFORMED_VERIFICATION as a fixture backed requirement of the round.

Landed head. 411b6406def20a26cbe0e26354ff91d320d788b0 on main, in three commits over the scaffold head 7fe76bc1. The round commit cfcbe63f carries the implementation, 5d40185a collapses one nested if for clippy, and 411b6406 fixes flow collections used as block list items.

Hosted verdict. Actions run 45111301007 on head 411b6406 completed with conclusion success. Format, clippy with warnings denied, the full test suite, and the build all pass on the exact head. The test suite includes the corpus gate over all seventeen fixtures and the unit tests in the time and frontmatter modules.

Local gate note. Local cargo could not run in this round. This box is in a write fault wave and forty consecutive cargo attempts failed on fingerprint writes with operation not permitted, while shell and file tool writes in the same directories succeed. Formatting was verified per file by comparing rustfmt standard output against each file on disk. The merge verdict therefore rests on the hosted run, per the Actions first rule, and local test evidence is owed when the wave breaks.

Review record. Three reviews ran at the checkpoint.

The Standards axis found the round blocking defect. Three files, bundle.rs, the corpus test, and the CLI main, stood at zero bytes on disk after an early background format loop truncated them in the wave. The loop was killed, the files were restored from the authored text, and every write since has been size verified. The same axis cleared the std only rule, found no copy bar breach, and raised three judgement calls, all addressed. The duplicated dangling finding construction became one helper, the list item heuristic is now a named rule with its reason stated, and the unused clock parameter carries its reason in the doc comment.

The Spec axis found six gaps against the design record, and all six are closed in the landed head. TierReport now carries the classification of every verification event as current, lapsed, or malformed, with reasons. check_currency emits one finding per lapsed or malformed event rather than one per concept. ChainReport carries its terminal state as terminated, dangling, cycle, or malformed. The SPEC 12.1 fallback applies to bundles declaring version 0.2 or earlier or no version, and ChainReport marks every target found only through it. Library findings are sorted by concept id then rule id. The frontmatter block parser now adopts the indent of its first content line, so two space and four space block forms both parse.

My own review re-derived two further defects from the hosted failures and fixed both, the clippy collapsible if and the flow map list item that lost its keys. Both fixes are in the landed head and the hosted run is green over them.

Corpus behaviour on the landed head. T1 fails with LAPSED_VERIFICATION and the tier drop from human-reviewed to unverified. Both T2 forms fail with SUPERSESSION_CYCLE. All three T3 forms fail with DANGLING_SUPERSEDED_BY. T7 fails with SUPERSEDED_WITHOUT_DEPRECATED. T8 malformed verification fails with MALFORMED_VERIFICATION and counts for nothing. The three clean controls pass with their labelled tiers. The six later round fixtures stay silent under these four checks, asserted in the gate so a future round moves them deliberately. A further test hashes a fixture before and after an audit run to prove the bundle is byte unchanged.

Next rounds. The stale check with its pinned clock, the provenance checks, the attested computation re execution, and the source drift extension, in that order, each moving its fixtures into the expected fail table as it lands.
