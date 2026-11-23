//! The corpus gate. Runs the four locked checks over every
//! fixture in the golden corpus and compares the outcome with the
//! fixture label for the rules this round implements.
//!
//! Fixtures whose defects belong to later rounds (stale, attested
//! mismatch, provenance, drift, unparseable stale_after) must
//! produce no findings from these four checks today. When a later
//! round implements one of those rules, its fixture moves into the
//! expected-fail table in the same change.

use std::path::PathBuf;

use perishables_core::{
    AuditClock, Bundle, FindingCode, Tier, check_currency, check_freshness, check_supersession,
    recompute_tier,
};

fn corpus_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/fixtures")
        .join(name)
}

fn clock() -> AuditClock {
    AuditClock::parse("2026-11-20").expect("pinned date parses")
}

fn all_findings(bundle: &Bundle) -> Vec<perishables_core::Finding> {
    let mut findings = check_currency(bundle, &clock());
    findings.extend(check_supersession(bundle));
    findings.extend(check_freshness(bundle, &clock()));
    findings
}

fn load(name: &str) -> Bundle {
    Bundle::load(&corpus_fixture(name)).expect("fixture loads")
}

#[test]
fn t1_lapsed_verification() {
    let bundle = load("t1-lapsed");
    let findings = all_findings(&bundle);
    assert!(findings
        .iter()
        .any(|f| f.code == FindingCode::LapsedVerification && f.concept_id == "metrics/revenue"));
    let report = recompute_tier(
        bundle.concept("metrics/revenue").expect("concept"),
        &clock(),
    );
    assert_eq!(report.raw_tier, Tier::HumanReviewed);
    assert_eq!(report.recomputed_tier, Tier::Unverified);
}

#[test]
fn t2_cycles_in_both_forms() {
    for name in ["t2-cycle-two", "t2-cycle-three"] {
        let bundle = load(name);
        let findings = all_findings(&bundle);
        assert!(
            findings
                .iter()
                .any(|f| f.code == FindingCode::SupersessionCycle),
            "{name} must report a supersession cycle"
        );
    }
}

#[test]
fn t3_dangling_in_all_three_forms() {
    for name in [
        "t3-dangling-missing",
        "t3-dangling-escape",
        "t3-dangling-external",
    ] {
        let bundle = load(name);
        let findings = all_findings(&bundle);
        assert!(
            findings
                .iter()
                .any(|f| f.code == FindingCode::DanglingSupersededBy),
            "{name} must report a dangling superseded_by"
        );
    }
}

#[test]
fn t7_superseded_without_deprecated() {
    let bundle = load("t7-superseded-without-deprecated");
    let findings = all_findings(&bundle);
    assert!(
        findings
            .iter()
            .any(|f| f.code == FindingCode::SupersededWithoutDeprecated)
    );
}

#[test]
fn t8_malformed_verification() {
    let bundle = load("t8-malformed-verification");
    let findings = all_findings(&bundle);
    assert!(findings.iter().any(|f| {
        f.code == FindingCode::MalformedVerification && f.concept_id == "metrics/revenue"
    }));
    let report = recompute_tier(
        bundle.concept("metrics/revenue").expect("concept"),
        &clock(),
    );
    assert_eq!(report.recomputed_tier, Tier::Unverified);
}

#[test]
fn clean_controls_pass() {
    for name in [
        "clean-healthy-chain",
        "clean-machine-confirmed",
        "clean-no-trust-fields",
    ] {
        let bundle = load(name);
        let findings = all_findings(&bundle);
        assert!(
            findings.is_empty(),
            "{name} must be clean, got {findings:?}"
        );
    }
    let bundle = load("clean-healthy-chain");
    let report = recompute_tier(
        bundle.concept("metrics/revenue").expect("concept"),
        &clock(),
    );
    assert_eq!(report.recomputed_tier, Tier::HumanReviewed);
    let bundle = load("clean-machine-confirmed");
    let report = recompute_tier(
        bundle.concept("metrics/event-count").expect("concept"),
        &clock(),
    );
    assert_eq!(report.recomputed_tier, Tier::MachineConfirmed);
    let bundle = load("clean-no-trust-fields");
    let report = recompute_tier(bundle.concept("notes/glossary").expect("concept"), &clock());
    assert_eq!(report.recomputed_tier, Tier::Unverified);
}

#[test]
fn t4_stale_but_stable() {
    let bundle = load("t4-stale-stable");
    let findings = all_findings(&bundle);
    assert!(
        findings
            .iter()
            .any(|f| f.code == FindingCode::StaleButStable && f.concept_id == "metrics/revenue"),
        "t4 must report STALE_BUT_STABLE, got {findings:?}"
    );
}

#[test]
fn t8_unparseable_stale_after() {
    let bundle = load("t8-malformed-stale-after");
    let findings = all_findings(&bundle);
    assert!(
        findings
            .iter()
            .any(|f| f.code == FindingCode::UnparseableStaleAfter),
        "t8 stale_after must report UNPARSEABLE_STALE_AFTER, got {findings:?}"
    );
}

#[test]
fn later_round_fixtures_are_invisible_to_these_checks() {
    for name in [
        "t5-attested-mismatch",
        "t6-provenance-footnote",
        "t6-provenance-missing-file",
        "t9-source-drift",
    ] {
        let bundle = load(name);
        let findings = all_findings(&bundle);
        assert!(
            findings.is_empty(),
            "{name} belongs to a later round and must be silent here, got {findings:?}"
        );
    }
}

#[test]
fn bundle_unchanged_by_audit() {
    // The auditor never mutates. Hash a fixture before and after a
    // full run of the checks and require identical bytes.
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let dir = corpus_fixture("clean-healthy-chain");
    let hash_tree = |dir: &std::path::Path| -> u64 {
        let mut hasher = DefaultHasher::new();
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(dir.join("metrics")).expect("metrics dir") {
            paths.push(entry.expect("entry").path());
        }
        paths.sort();
        for path in paths {
            std::fs::read(&path).expect("read").hash(&mut hasher);
        }
        hasher.finish()
    };
    let before = hash_tree(&dir);
    let bundle = load("clean-healthy-chain");
    let _ = all_findings(&bundle);
    assert_eq!(before, hash_tree(&dir));
}
