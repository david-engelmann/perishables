# Cheapest failure test results

Prototype in prototype/audit.py. Checks implemented are verification currency under OKF v0.3 SPEC 5.3 and supersession integrity under SPEC 5.4 only. Clock pinned with as-of 2026-11-20 for every run. PyYAML 6.0.1. All bundles live under bundles/ in this directory.

## Hand built bundles

| Bundle | Planted defect | Expected from minimal script | Actual | Findings |
|---|---|---|---|---|
| 00-clean | none, 2 concepts | PASS | PASS exit 0 | 0 |
| 01-lapsed | verification predates last change | FAIL currency | FAIL exit 1 | 1 LAPSED_VERIFICATION, tier human-reviewed to unverified |
| 02-cycle | two node supersession cycle | FAIL supersession | FAIL exit 1 | 2 SUPERSESSION_CYCLE, one per entry point, same cycle |
| 03-dangling | superseded_by target missing | FAIL supersession | FAIL exit 1 | 1 DANGLING_SUPERSEDED_BY |
| 04-stale-stable | stale_after passed, status stable | PASS, miss by design | PASS exit 0 | 0 |
| 05-attested | claimed result 42, recompute gives 3 | PASS, miss by design | PASS exit 0 | 0 |

Coverage statement. The two cheapest checks catch 3 of the 5 planted defects, miss 2 by design because the stale rule and re execution are not in the minimal script, and produce 0 false positives on the clean control. Separation is deterministic. A repeat run of the lapsed bundle produced byte identical output under diff.

## Google published material

Material was reconstructed file for file from primary sources read firsthand in this pass, namely the SPEC.md Appendix A worked example and the acme_retail gross margin excerpt quoted from the Google sample bundle. The ga4, stackoverflow, and crypto_bitcoin bundle files listed in the okfs.sh catalog were not fetched file by file in this pass. That gap is recorded in the design record and no numbers are claimed for them.

| Bundle | Concepts | Actual | Tier agreement |
|---|---|---|---|
| google-spec-finance | 3 | PASS exit 0, 0 findings | 3 of 3 match the expected table in SPEC Appendix A. Income statement human-reviewed, revenue human-reviewed, profit machine-confirmed |
| google-acme-excerpt | 1 | PASS exit 0, 0 findings | 1 of 1. Gross margin human-reviewed, verification after generation |

Signal versus noise. Across 4 real published concepts the currency and supersession checks produced 0 findings and 0 tier disagreements. The profit concept in the spec bundle is stale at the pinned clock under its June stale_after date, and the minimal script correctly stays silent on it because freshness is outside its scope. The full auditor stale rule is expected to surface it as stale information, matching the spec table which marks profit as not fresh in that window.

## Reading of the result

The concept passes its cheapest failure test in the narrow sense the dossier defined. Currency and supersession alone separate clean from defective deterministically and stay quiet on real material. The test also proves the two checks are not the product on their own, since two of five defect classes pass straight through them. The full check set in the design record is therefore load bearing, not decoration.
