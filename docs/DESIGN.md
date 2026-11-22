# Design record for the OKF trust auditor

Seat 51fcd693. Phase 1 design gate. Prepared 2026-11-20.
Working name history in this record is karat, then lydite. Both names fail the exact checks in section 1. No repository has been created and nothing in this record authorizes creation. The product definition is unchanged by the naming failure.

Normative source for every rule below is the OKF v0.3 SPEC.md read firsthand from the canonical repository open-knowledge-format/open-knowledge-format on 2026-11-20. Section numbers cite that document.

## 1. Name verification, exact results

Method. Each registry was checked directly on 2026-11-20. A 404 from a registry page means no entry exists under that exact name. Search results were used only for the collision scan.

### karat, the original working name

- GitHub david-engelmann/karat returns 404. The namespace is free.
- crates.io karat returns 404. The crate name is free.
- PyPI karat is TAKEN and active. The project is an evals-first prompt optimization tool by thekevinscott, version 0.2.4 released 2026-03-24, installed from github.com/thekevinscott/karat. It sits in the adjacent AI evaluation space.
- npm karat is TAKEN but dormant. Version 0.0.0, published about four years ago, about two downloads a week, no README, repository points at an unrelated project.
- Top collision. Karat is a large technical interviewing company operating at karat.com. The name also appears on a token audit report and in robotics projects. None of these audit OKF bundles, but the word is heavily owned in software hiring and in search results.

Verdict on karat. The Rust-first build could ship because the GitHub namespace and the crate name are free. The PyPI name is gone to an active adjacent project and the search surface belongs to an unrelated company. David rejected this name set on 2026-11-20 before these results landed. The evidence supports the rejection.

### fineness, the original fallback

- crates.io fineness returns 404. The crate name is free.
- PyPI fineness is TAKEN and active and conceptually adjacent. The project describes itself as a framework that certifies numerical and stochastic software, version 0.4.2 released 2026-07-27. Its stated doctrine is that a check certifies nothing until the check is shown able to fail. That is the same gate discipline this auditor ships, in a different domain.
- npm fineness could not be confirmed in this pass. The registry page refused the fetch with a 403. This line stays unverified rather than guessed.

Verdict on fineness. The fallback fails on PyPI against an active project whose pitch overlaps the trust pitch. Do not use it.

### lydite, the replacement name ordered 2026-11-20

- GitHub david-engelmann/lydite returns 404. The namespace is free.
- crates.io lydite returns 404. The crate name is free.
- PyPI lydite is TAKEN but dormant. Version 0.0.2 uploaded 2021-09-05, a 1.3 kB placeholder with no project description on the fetched page.
- npm lydite could not be confirmed in this pass for the same 403 reason as fineness. This line stays unverified.
- Top collision, severe. An active open source program already ships under this exact name at aoricloud/lydite. It is written in Rust under MIT, it installs a binary named lydite from releases attested by GitHub Actions, and it is listed on the Hermes Marketplace and in the Hermes plugin directory. Its job is to delegate a GitHub issue to isolated Claude Code workers, replay a proof command on every result, keep only what passes, and open a pull request with the proof result. That is agent work verification with proof replay. It is not an OKF auditor, but it owns the same binary name, the same language, and the neighbouring claim.

Verdict on lydite. The name fails the purpose of the exact check. Two Rust verification binaries named lydite cannot coexist in the same agent tooling ecosystem without permanent confusion, and the incumbent is active and attested. Recommendation to Main is in section 9. This seat does not create a repository under lydite.

## 2. The product in its sharpest form

Five structural validators already prove that an OKF bundle parses. This tool proves whether the trust claims inside a bundle that parses are true at a pinned moment in time. It recomputes every trust tier from current verifications only, it walks every supersession chain to its end, it flags stable claims that have passed their own expiry date, it resolves provenance and measures source drift, and it runs Attested Computations again instead of trusting the recorded result. It ships a corpus of bundles with planted trust defects so that any run can be checked against defects the gate is known to catch. A gate that has never failed on a planted defect certifies nothing, and the corpus is the standing proof that this one can fail.

First five minutes. A user runs the binary against a bundle with a pinned as-of date and gets a per concept table that shows the claimed tier beside the recomputed tier, every lapse, every broken chain, and every stale stable concept, with a non zero exit code when the configured gate trips. A user runs the corpus command and watches every must fail fixture fail while naming its planted rule.

## 3. Strongest counter case, read firsthand

The counter case has three parts and each part was checked against a primary source in this pass or in the dossier pass.

First, the spec already defines the tier rule, so a reader can compute tiers without this tool. True and beside the point. Readers derive tiers for display inside their own runtime. The dossier found no tool that audits a whole bundle against the rule, reports the delta between the raw list and the recomputed tier, and fails a build on the delta. Derivation is not auditing.

Second, provenance layers already exist. In this pass a new adjacent project was read firsthand. Lemma ships Verifiable OKF, an extension that attaches signing and condition proofs to bundles, with its own spec and a provenance schema. It answers whether a bundle was tampered with and who issued it. It does not recompute tiers from current verifications, walk supersession chains, enforce stale dates, or re execute computations. Signing a false trust claim produces a well signed false trust claim. The seats do not overlap.

Third, the okf-mcp position quoted in the dossier states that Attested Computation contracts are inspected statically, with nothing executed and nothing attested. The spec itself says in section 10 that OKF records the computation and the means to check it, and that OKF itself does not run anything. The empty seat is the runner. Section 10.5 of the spec describes the consumer flow of discover, load, parameterize, execute, attest, and gate. This tool implements that flow as a deterministic auditor instead of leaving it as an informative sketch.

One more neighbour deserves a line. The aoricloud lydite program replays proof commands for coding tasks. It proves a code change passes a stated command. It never reads an OKF bundle and it has no trust field semantics. The overlap is the verification instinct and the name, not the product.

## 4. Trust model

### 4.1 What counts as a current verification

The rule is SPEC 5.3 with no additions. A verification event counts when it carries both required keys, by and at, and when its at value is equal to or later than generated.at for the same concept. When generated is absent, every well formed event counts. Events missing by or at are ignored for tier purposes and reported as malformed history. Date only timestamps mean the start of that day in UTC, per SPEC 5. A bare mapping in place of a list counts as one event, per SPEC 5.2. The auditor never invents a verification and never repairs one silently.

### 4.2 Tier recompute rules

Tiers are derived and never stored, per SPEC 5.3. No current event gives unverified. Current events with no human actor give machine-confirmed. Any current event whose actor uses the human scheme gives human-reviewed. Only the human scheme qualifies, per SPEC 7. A team actor is not a human actor and gives machine-confirmed at best. The report always shows two values for each concept, the tier implied by the raw verified list and the tier recomputed from current events. A drop between the two is the lapse finding. Equal timestamps count as current, because the spec says equal to or later than. The auditor reports the full event history classification so a reader can see which events were excluded and why.

### 4.3 Supersession chain semantics

The field is SPEC 5.4. superseded_by holds one path and has meaning only with status deprecated. Path resolution follows SPEC 6 exactly. A leading slash resolves from the bundle root, any other value resolves from the directory of the file that holds it, and a value with a URI scheme is external and therefore a dangling target for audit purposes. The v0.2 fallback in SPEC 12.1 applies only when the bundle declares version 0.2 or earlier or declares no version, and the report marks any target found only through the fallback.

Chain rules. Every chain must resolve at each hop to a concept inside the bundle. Every chain must terminate at a concept with no superseded_by. No chain may revisit a node, and the walker caps traversal at a fixed hop limit and reports any chain that reaches the cap as a cycle suspect. A deprecated concept without superseded_by is valid and terminates its own chain, because retirement without a replacement is a defined state in SPEC 5.4. A concept that carries superseded_by while its status is not deprecated is a finding, because the field has no meaning there and current knowledge filters will misread it. A stable concept that is the target of a chain is the healthy end state and is not a finding. Two deprecated concepts that point at each other form the cycle defect in the corpus.

### 4.4 The stale but stable rule

Freshness is SPEC 5.5. A concept is stale when the pinned as-of instant is equal to or later than stale_after. Stale means check again and does not mean false, and the report uses that wording. The audit finding fires when a concept is stale at the pinned clock and still presents as current, which means its status is stable or absent, since absent status defaults to stable under SPEC 5.4. A stale deprecated concept is reported as information and not as a gate finding, because it no longer claims currency. A stale_after value that cannot be parsed is never treated as fresh, per SPEC 11.2, and is reported as malformed. Concepts with no stale_after carry no freshness finding, because the spec forbids inventing a review date.

### 4.5 Provenance completeness and source drift

Provenance is SPEC 5.1. The auditor scores completeness per concept on stated facts only. A concept with no sources entry is reported as having no recorded provenance. A footnote label in the body with no matching sources id is a broken attribution. A sources entry whose resource is an in bundle path that does not resolve is a broken source. External URLs are never fetched by default, because an audit must stay deterministic and offline. An optional network mode is out of scope for v1 and is flagged in section 9.

Source drift applies to in bundle sources and to sources that carry a content hash in an extension key. When a producer records a hash, the auditor recomputes it and reports drift on mismatch. When no hash is recorded, the auditor compares the source last_modified signal against generated.at and reports only the ordering fact, never a verdict the data cannot support. The spec lists source drift signals as deferred future work in section 12.2, so this tool treats drift as an extension level check with its own severity and keeps it out of the default gate until the corpus proves it separates cleanly.

## 5. Attested Computation re execution contract

The contract follows SPEC 10. A computation is its own concept with type Attested Computation. The auditor discovers computations by scanning for that type and by following links from concepts that use one.

What runs. The auditor binds only declared parameter values to the declared computation, exactly as SPEC 10.3 requires. It never edits the computation and never accepts a computation supplied at run time. The bound artifact is hashed before execution and the hash is part of the report, so a replaced computation file or a changed dependency changes the hash and fails comparison.

Sandboxing. Execution happens with no network access, a read only view of the bundle, a scratch working directory that is discarded after the run, a wall clock limit, and a memory limit. The environment is scrubbed to a stated allowlist. In v1 the supported runtimes are a small allowlist of deterministic local runtimes, proposed as python with a pinned interpreter invocation and a shell free arithmetic profile for corpus fixtures. Runtimes that need a live external service, such as bigquery and dbt against a warehouse, are not executed in v1. Their contracts are checked statically and reported as not executable in this profile, which is an honest result rather than a silent skip. Expanding the allowlist is later work gated on determinism evidence per runtime.

Determinism. The same bundle, the same parameter values, and the same as-of clock must produce the same verdict and the same report bytes. Any runtime that reads the wall clock, randomness, or the network is outside the deterministic profile and its result is reported as non deterministic rather than compared. The corpus fixtures use only computations whose outputs are pure functions of declared inputs.

What a failed re execution means. Three outcomes are distinct and reported distinctly. A mismatch means the computation ran cleanly and its result differs from the attested or claimed result, and that is a gate finding. An execution failure means the computation did not run to completion, and that is reported as unattestable with the captured reason, gated by configuration because a broken executor is a bundle defect. A missing computation, per SPEC 10.3, means the concept is not attestable, and any concept that presents its value as attested anyway is a gate finding under SPEC 11.2, which forbids showing a value as attested without runtime and attester. A successful re execution whose result matches is the only outcome reported as attested.

## 6. Planted defect corpus design

The corpus is data inside the repository, not a separate repository. Each fixture is a complete minimal bundle that passes structural validation. Each must fail fixture carries exactly one planted trust defect, so a failure can be attributed to one rule and a pass can never hide behind a second defect. Each fixture carries a label file that names the rule it trips, the expected finding code, and the expected recomputed tier where a tier is involved.

Defect taxonomy for v1. One fixture per code, plus clean controls.

- T1 lapsed verification, where the only verification predates the last meaningful change and the raw tier overstates the recomputed tier.
- T2 supersession cycle, in two node and three node forms.
- T3 dangling superseded_by, covering a missing file, a path that escapes the bundle root, and an external URI used as a target.
- T4 stale but stable, where stale_after has passed at the fixture as-of date while status remains stable.
- T5 attested result mismatch, where re execution of the declared computation gives a different result from the claimed result.
- T6 provenance break, covering a missing in bundle source and a footnote label with no matching sources entry.
- T7 superseded without deprecated, where superseded_by appears on a stable concept.
- T8 malformed trust data, covering a verification event missing its at key and an unparseable stale_after presented as fresh.
- T9 source drift, where a recorded hash no longer matches the in bundle source. This fixture ships in the extension set until the drift check earns default gate status.
- Clean controls. At least three must pass bundles, covering a healthy deprecated chain that terminates, a machine-confirmed concept with a current process verification, and a concept with no trust fields at all, which is conformant and must never be flagged as defective for absence alone.

Corpus gate. The corpus command runs the auditor over every fixture and compares findings against labels. The build fails when a must fail fixture passes, when a must pass fixture fails, or when a finding names the wrong rule. That comparison is the proof that the gate can fail, and it runs in hosted CI on every change.

## 7. Library surface for the recension dependency

The upkeep engine seat, recension, formerly sediment, consumes the currency and supersession checks rather than reimplementing them. The checks are therefore a library crate from the first commit, with the binary as a thin shell over it. Working crate name follows the final product name once Main rules on naming. The interface below is name neutral and stable in shape.

Public surface, Rust signatures in sketch form.

- AuditClock, a value type wrapping a pinned instant, constructed once from the as-of argument. No library function reads the system clock.
- Bundle, a loaded bundle with its concept map and its declared okf_version.
- fn recompute_tier(concept, clock) returns a TierReport holding the raw tier, the recomputed tier, and the classification of every verification event as current, lapsed, or malformed, with reasons.
- fn check_currency(bundle, clock) returns a list of CurrencyFinding values, one per lapsed or malformed event, each carrying the concept id and the finding code.
- fn walk_supersession(bundle, start) returns a ChainReport holding the ordered hops, the terminal state as terminated, dangling, cycle, or malformed, and the finding code when the chain is defective.
- fn check_supersession(bundle) returns a list of SupersessionFinding values covering every chain head in the bundle.
- Finding codes are a closed enum shared with the corpus labels, so the upkeep engine, the binary, and the corpus all speak the same codes.

Contract for the consumer. The library performs no IO beyond reading the bundle it is handed, writes nothing, and returns data rather than printing. The upkeep engine decides what to queue for re verification from CurrencyFinding values and decides what to rewrite from ChainReport values. Any change to finding code meanings is a breaking change recorded with a version consequence, because a second product depends on them.

## 8. Stack proposal

Rust, one workspace, two crates. A library crate holds the trust model, the chain walker, the freshness rule, the provenance checks, and the re execution harness. A binary crate holds argument parsing, reporting, and exit codes. A single binary ships for the audit and corpus commands. The clock is pinned through a required as-of argument with no default to the system time in gate mode, because a defaulted clock destroys byte for byte reproducibility. Reports are sorted by concept id and rendered from data structures with fixed field order, so the same inputs give the same bytes. Dependencies stay minimal and pinned, with YAML parsing and hashing as the expected core needs. No database, no daemon, no network in the default profile. This shape needs no further justification under the fleet pattern, as the brief states.

Licensing and repository shape follow the flagship defaults already locked for the greenfield programme, meaning MIT OR Apache-2.0, Rust 2024 edition, a pinned toolchain file, and a committed lockfile, subject to Main confirming those defaults carry to this seat.

Access, when creation is approved. The repository gets its own per repo SSH deploy key at creation through the stored credential, following the inventory pattern in the fleet deploy key record. The existing GitHub App installation is at its repository limit and is not extended to this repository.

## 11. Perfection addendum, added 2026-11-20 under the perfection mandate

This addendum carries the four perfection requirements from Main. Where it sharpens an earlier section, the addendum governs.

### 11.1 Scope discipline, the refuse list

This tool is the trust layer only. It assumes a bundle has already passed structural validation elsewhere and it forms no opinion on structure. The auditor refuses to check the following, and a request to add any of them is a scope change that returns to Main.

- Whether frontmatter parses, whether type is present, and every other conformance rule in SPEC 11.1. That is the work of the official validate.py and the independent structural validators named in the dossier.
- Whether a type value belongs to any taxonomy. Unknown types are accepted, per SPEC 4.1, and are never a finding.
- Whether links are broken in the general graph sense. A broken ordinary link is valid knowledge that nobody has written yet, per SPEC 6.1. Only the superseded_by path field is walked, because that field carries a lifecycle claim.
- Whether the prose content of a concept is correct in the world. The auditor checks claims about the content, never the content against reality, with the single bounded exception of re execution in section 11.3, where a declared computation is compared against its own declared result.
- Whether a credibility score should be assigned to a source or a concept. The spec records signals and forbids the stored score in spirit, and this tool reports signals and orderings only.
- Claim grain supersession inside a concept. The faircopy seat established, and Main verified on 2026-11-20, that claim level supersession has no home in OKF v0.3 because superseded_by is per concept with a single target. This auditor works at concept grain, which is the grain the spec defines. A claim registry convention belongs to the faircopy and recension seats, and this tool may read such a registry as data in a later version only by a scope ruling.

The boundary test for any proposed check is one question. Does the check test whether a trust claim is true, current, and coherent. If it instead tests whether the bundle is well formed, it belongs to a structural validator and is refused.

### 11.2 The corpus as a published artifact

The corpus is the credibility engine of the product and is designed as a published artifact, not as test scaffolding.

- The corpus lives in the repository under a top level corpus directory and carries its own version, independent of the binary version, recorded in a corpus manifest that lists every fixture, its defect code from the taxonomy in section 6, its expected finding, and the auditor version range the expectation applies to.
- Any OKF tool may run the corpus. The manifest is machine readable, the fixtures are plain bundles, and the expected results are data, so a competing or complementary tool can score itself against the same planted defects without using this binary.
- A corpus release is cut alongside a tool release and never silently edited. Adding a fixture increments the corpus minor version. Changing an expected result increments the corpus major version and requires a note explaining which auditor behaviour changed and why the old expectation was wrong.
- The corpus command in the binary is the reference runner. Its output states the corpus version, the auditor version, and a per fixture verdict, so a stranger can reproduce a published score exactly.
- The faircopy converted dogfood bundle, approved by Main on 2026-11-20 as a shared test fixture for this seat and the recension seat, joins the corpus programme as a real world must audit target rather than a planted fixture. It is the first large bundle whose ground truth a human can judge by eye, and audit findings on it are reported as observations, never as planted defect verdicts.

### 11.3 Re execution in v1, shapes and verdicts

The re execution surface is bounded to shapes that are deterministic on a local machine. Every Attested Computation concept receives exactly one verdict from the table below, and no concept is silently skipped.

Re executable in v1.

- Inline or file computations in the python profile whose body is a pure expression or a short deterministic script over declared parameters, with no imports outside a stated allowlist, no file access outside the bundle read only view, and no network.
- Arithmetic and aggregation fixtures of the kind the corpus ships, where the result is a number or a short string and equality is exact after a stated normalization.

Verdicts for everything else.

- A computation whose runtime needs a live external service, including bigquery, postgres against a remote host, dbt against a warehouse, and Looker, receives the verdict NOT_EXECUTABLE_IN_PROFILE, with the runtime named. It is not a gate finding by itself. A concept that presents a value from such a computation as attested in the bundle receives a finding, because the bundle claims a state the audit cannot support.
- A computation that is missing, or whose file does not resolve, receives NOT_ATTESTABLE, per SPEC 10.3.
- A computation that runs and disagrees with its claimed result receives MISMATCH, the gate finding.
- A computation that runs and agrees receives RECOMPUTED_MATCH, the only verdict reported as attested.
- A computation that starts and fails receives EXECUTION_FAILED with the captured reason, gated by configuration as section 5 states.
- A computation that reaches for the wall clock, randomness, or the network during a run is stopped and receives NON_DETERMINISTIC_PROFILE, because its result can never be reproduced byte for byte.

Cost bound. Each run carries a wall clock limit and a memory limit set in configuration with conservative defaults, and the report records the elapsed time and the bound for every executed computation, so an expensive bundle is visible rather than surprising.

### 11.4 Library surface, publication note

Section 7 defines the interface that the recension seat consumes. Two publication rules are added here. The library crate is published as the primary artifact and the binary depends on it, never the reverse. The finding code enum and the TierReport and ChainReport shapes are the compatibility surface, they follow semantic versioning from the first release, and any change to a finding code meaning is a major version change with a migration note addressed to the recension seat.

### 11.5 Name evidence update

Two lines left unverified in section 1 are now closed. The npm registry API returns 404 for lydite, so the name is free on npm. The GitHub API confirms the incumbent repository aoricloud/lydite exists as a public repository. Neither result changes the verdict in section 1. The name still fails on the binary collision, and the naming blocker in section 9 stands as the single decision that gates creation.

## 12. Rename note, stilltrue checks, added 2026-11-20

Main renamed this seat to stilltrue on 2026-11-20 and David rejected that name minutes later as one he does not love, with a replacement to come from a fresh list. The exact checks were still run in full and are recorded here so the name is never revived without its evidence.

Channels the Rust first build needs, all clean. The GitHub namespace david-engelmann/stilltrue returns 404. The crates.io API states that stilltrue, stilltrue-core, and stilltrue-cli do not exist. PyPI returns 404.

For the record channels. npm stilltrue is taken and active. The package is sg-bfoster/stilltrue at version 0.8.0, created 2026-08-16 and modified 2026-10-30, shipping a CLI binary named stilltrue. Its description reads that tests check code while stilltrue checks facts, with CI checks for AI in the loop apps covering drift of curated facts against live sources, regression evals, and judging answers against sources. Its keywords are drift, facts, eval, llm, ci, regression, and groundedness.

Collision scan. GitHub search finds twelve repositories with stilltrue in the name, all at zero stars, and the cluster is thematically dense. PhiBao/stilltrue is a Rust project created 2026-11-07 that calls itself a freshness firewall for AI answers with claim level audit against live sources. A Python project under cyh7789 promises to find documentation in DataHub that stopped being true. A Rust project under uk0064 is a linter that binds claims made in Markdown to the real repository, aimed at CLAUDE.md and AGENTS.md files, which is the closest shape to this auditor in the cluster. An organisation account holds a stilltrue.io landing page.

Assessment. Under the precedent Main set for retrial and refrain, a taken npm name alone does not block a Rust first build, and every channel this build needs is clean. The honest difference here is density. At least four independent projects picked this exact name for fact freshness and claim checking in the last four months, which means the phrase is becoming a category label in the adjacent niche rather than a distinctive mark. The mechanisms differ from this tool, since the cluster judges live answers with models while this auditor is deterministic, offline, and bundle internal, but a reader meeting the name first would reasonably expect the other product. Combined with David having already rejected the name on his own call, the recommendation is that stilltrue stays rejected and the seat awaits the fresh list pick. No name is substituted by this seat.

Boundary names in this record should be read with the latest ruling. The memory product is looseleaf and the feedback tool is earful and the ARD kit is beacon, all settled. The verifier name receipts and the upkeep engine name upkeep were both rejected in the same ruling as stilltrue, so references in this record to the upkeep seat mean the seat itself, under whatever name Main settles next. The library interface in section 7 is name neutral and is unaffected.

## 13. Name settled and interface ruling, added 2026-11-20

### 13.1 Perishables, final name

David settled the name as perishables on 2026-11-20. Verified claims are perishables, stock with a shelf life, and the auditor walks the shelves. This seat ran its own exact checks in the creation pass discipline and found no contradiction with the registry state Main reported.

- The GitHub namespace david-engelmann/perishables returns 404.
- The crates.io API states that perishables, perishables-core, and perishables-cli do not exist.
- PyPI returns 404.
- The npm registry returns 404.
- The GitHub name scan finds fifty two repositories with perishables in the name, and every one of them concerns food, supply chains, game mods, or student inventory projects. None sits in software verification, knowledge formats, or agent tooling, and none carries an audience.

The name is clean on every channel and empty in the field that matters. Earlier names in this record, karat, fineness, lydite, and stilltrue, all stand rejected on their recorded evidence and are not revived. Crate naming follows the settled pattern, with the library as perishables-core and the binary crate as perishables-cli under a perishables workspace, matching the names verified above.

### 13.2 Ruling on the upkeep engine interface terms

The upkeep seat proposed four terms, and Main recorded them as accepted as its proposal, locking when this gate confirms the same terms from the auditor side. This seat rules as follows, and the terms lock on gate confirmation.

First, the four calls. Confirmed, and they are the four calls defined in section 7. The tier recompute call takes a concept and the pinned clock and returns a tier report. The currency check takes a bundle and the clock and returns currency findings. The chain walk takes a bundle and a starting concept and returns a chain report. The supersession check takes a bundle and returns supersession findings for every chain head. No fifth call is added in v1, and any later addition is a minor version change announced to the upkeep seat.

Second, stable rule ids. Confirmed. Finding codes form a closed enum shared with the corpus labels, a code once published never changes meaning, a retired code is never reused, and a meaning change is a major version change with a migration note, as section 11.4 already states.

Third, deterministic output. Confirmed. The library reads no system clock, takes every instant from the pinned audit clock, sorts its findings by concept id and then by rule id, and returns the same values for the same bundle and clock on any machine. Report rendering inherits the same property in the binary.

Fourth, the auditor never mutates. Confirmed without qualification. The library performs no writes of any kind. The binary reads the bundle and writes only its own report to standard output or to a report path outside the bundle that the user names. An audit run leaves the bundle byte for byte unchanged, and the corpus runner verifies that property on its fixtures by hashing each fixture before and after the run.

## 9. Scope amendments and decisions flagged to Main

These items need a ruling from Main or David. None of them blocks the design, and work does not proceed past the gate on any of them without a ruling.

- Naming blocker. karat and fineness fail on PyPI and lydite collides with an active Rust verification binary of the same name. Recommendation is that Main runs a fresh naming pass for this seat and this seat re runs the exact registry checks on the survivor before any creation. No fallback from the earlier sets should be reused, because fineness is taken by an adjacent certification project.
- npm verification gap. The npm registry pages for fineness and lydite refused fetches in this pass, so those two npm lines are unverified. The gap does not change any verdict, because each name already fails on other evidence. The seat will close the npm line through the registry API during the next naming pass.
- Drift gate status. Recommendation is that source drift ships as an extension level check outside the default gate in v1, moving into the gate only when its corpus fixtures prove clean separation. The alternative is to hold v1 until drift is gate grade.
- Executable runtime allowlist. Recommendation is that v1 re executes only deterministic local runtimes and reports warehouse runtimes as not executable in this profile. The alternative of executing against live services would break determinism and offline auditing.
- Google sample coverage. The cheapest failure run in section 10 covers the bundles that were retrieved from primary sources in this pass. The full published sets under the old GoogleCloudPlatform repository, including the ga4, stackoverflow, and crypto_bitcoin bundles listed by the okfs.sh catalog as OKF 0.2, were not fetched file by file in this pass. Recommendation is a follow up run over those files before the corpus is declared final, recorded here so the gap is on the record rather than smoothed over.

## 10. Cheapest failure test, executed

Full method and numbers are in RESULTS.md beside this record. Summary follows.

Six hand built bundles were created, one clean and five with one planted defect each, covering lapsed verification, a supersession cycle, a dangling superseded_by, stale but stable, and an attested result that does not recompute. A minimal script checking only verification currency and supersession integrity was run over all six with the clock pinned at 2026-11-20.

The script separated the bundles deterministically. The clean bundle passed with zero findings. The lapsed, cycle, and dangling bundles failed, each naming its planted rule, and the lapsed concept tier dropped from human-reviewed to unverified. The stale but stable bundle and the attested mismatch bundle passed, which is the expected result for a script that does not implement those two rules, and it measures exactly how much of the defect surface the two cheapest checks cover, three of five planted defects with zero false positives. Repeat runs produced byte identical output.

The same script ran over Google published material retrieved from primary sources. The SPEC Appendix A finance bundle, three concepts, passed with zero findings, and the recomputed tiers matched the expected table printed in the spec itself on all three concepts. The acme_retail gross margin excerpt passed with its human-reviewed tier intact. Across four real concepts the two checks produced zero noise. The signal versus noise result is therefore three true failures on planted defects, zero false failures on the clean control, and zero false failures on published material, with two known misses that define the next checks to build.
