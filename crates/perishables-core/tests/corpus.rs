//! The corpus gate. Runs the four locked checks over every
//! fixture in the golden corpus and compares the outcome with the
//! fixture label for the rules this round implements.
//!
//! Fixtures whose defects belong to later rounds must produce no
//! findings from the library checks. When a later round implements
//! one of those rules, its fixture moves into the expected-fail
//! table in the same change. The attested mismatch fixture is the
//! one exception in shape only. Its defect is invisible to the
//! library checks by design, because re execution lives in the
//! adapter crate, and its gate is the CLI corpus test there.

use std::path::PathBuf;

use perishables_core::{
    AuditClock, Bundle, FindingCode, Tier, check_currency, check_drift, check_freshness,
    check_provenance, check_supersession, recompute_tier,
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
    findings.extend(check_provenance(bundle));
    findings.extend(check_drift(bundle));
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
fn t6_provenance_defects() {
    let bundle = load("t6-provenance-footnote");
    let findings = all_findings(&bundle);
    assert!(
        findings
            .iter()
            .any(|f| f.code == FindingCode::UnresolvedCitation),
        "footnote fixture must report UNRESOLVED_CITATION, got {findings:?}"
    );
    let bundle = load("t6-provenance-missing-file");
    let findings = all_findings(&bundle);
    assert!(
        findings
            .iter()
            .any(|f| f.code == FindingCode::MissingSource),
        "missing file fixture must report MISSING_SOURCE, got {findings:?}"
    );
}

#[test]
fn clean_team_verified() {
    // SPEC 5.2 and 5.3, read firsthand for this pin. A team actor
    // in verified gives the machine-confirmed tier, because a team
    // name does not identify the person who did the check, and
    // only the human scheme gives human-reviewed. The fixture
    // carries one current team verification and nothing else, so
    // both tiers must land on machine-confirmed with no findings.
    let bundle = load("clean-team-verified");
    let findings = all_findings(&bundle);
    assert!(
        findings.is_empty(),
        "a current team verification is not a defect, got {findings:?}"
    );
    let concept = bundle
        .concept("metrics/team-metric")
        .expect("concept exists");
    let report = recompute_tier(concept, &clock());
    assert_eq!(report.raw_tier, Tier::MachineConfirmed);
    assert_eq!(report.recomputed_tier, Tier::MachineConfirmed);
    assert_eq!(report.events.len(), 1);
    assert_eq!(
        report.events[0].class,
        perishables_core::EventClass::Current
    );
}

#[test]
fn t9_source_drift() {
    let bundle = load("t9-source-drift");
    let findings = check_drift(&bundle);
    assert_eq!(
        findings.len(),
        1,
        "exactly one drift finding, got {findings:?}"
    );
    assert_eq!(findings[0].code, FindingCode::SourceDrift);
    assert_eq!(findings[0].concept_id, "metrics/revenue");
}

#[test]
fn drift_separates_cleanly() {
    // The check must stay silent on every other fixture. This was
    // the separation evidence the design record asked for before
    // drift could join the default gate, and it stands as the
    // regression guard now that the promotion has happened.
    for name in [
        "t1-lapsed",
        "t2-cycle-two",
        "t2-cycle-three",
        "t3-dangling-missing",
        "t3-dangling-escape",
        "t3-dangling-external",
        "t4-stale-stable",
        "t5-attested-mismatch",
        "t5-unattested-claim",
        "t6-provenance-footnote",
        "t6-provenance-missing-file",
        "t7-superseded-without-deprecated",
        "t8-malformed-verification",
        "t8-malformed-stale-after",
        "clean-healthy-chain",
        "clean-machine-confirmed",
        "clean-no-trust-fields",
        "clean-team-verified",
    ] {
        let bundle = load(name);
        let findings = check_drift(&bundle);
        assert!(
            findings.is_empty(),
            "{name} must show no drift, got {findings:?}"
        );
    }
}

#[test]
fn later_round_fixtures_are_invisible_to_these_checks() {
    // Attested computation execution defects are evaluated by the
    // adapter crate rather than the pure library checks.
    for name in ["t5-attested-mismatch", "t5-unattested-claim"] {
        let bundle = load(name);
        let findings = all_findings(&bundle);
        assert!(
            findings.is_empty(),
            "{name} belongs to the adapter execution gate and must be silent here, got {findings:?}"
        );
    }
}

#[test]
fn exact_code_sets_for_must_fail_fixtures() {
    // The design rule is that the build fails when a finding names
    // the wrong rule, so presence is not enough. Each must fail
    // fixture must produce exactly its labelled code set from the
    // library checks, no more and no less.
    let cases: [(&str, &[&str]); 13] = [
        ("t1-lapsed", &["LAPSED_VERIFICATION"]),
        ("t2-cycle-two", &["SUPERSESSION_CYCLE"]),
        ("t2-cycle-three", &["SUPERSESSION_CYCLE"]),
        ("t3-dangling-missing", &["DANGLING_SUPERSEDED_BY"]),
        ("t3-dangling-escape", &["DANGLING_SUPERSEDED_BY"]),
        ("t3-dangling-external", &["DANGLING_SUPERSEDED_BY"]),
        ("t4-stale-stable", &["STALE_BUT_STABLE"]),
        ("t6-provenance-footnote", &["UNRESOLVED_CITATION"]),
        ("t6-provenance-missing-file", &["MISSING_SOURCE"]),
        (
            "t7-superseded-without-deprecated",
            &["SUPERSEDED_WITHOUT_DEPRECATED"],
        ),
        ("t8-malformed-verification", &["MALFORMED_VERIFICATION"]),
        ("t8-malformed-stale-after", &["UNPARSEABLE_STALE_AFTER"]),
        ("t9-source-drift", &["SOURCE_DRIFT"]),
    ];
    for (name, expected) in cases {
        let bundle = load(name);
        let findings = all_findings(&bundle);
        let codes: std::collections::BTreeSet<&str> =
            findings.iter().map(|f| f.code.label()).collect();
        let expected: std::collections::BTreeSet<&str> = expected.iter().copied().collect();
        assert_eq!(
            codes, expected,
            "{name} must produce exactly its labelled codes"
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
        let mut stack = vec![dir.to_path_buf()];
        while let Some(current) = stack.pop() {
            for entry in std::fs::read_dir(&current).expect("fixture dir") {
                let entry = entry.expect("entry");
                let ft = entry.file_type().expect("file type");
                if ft.is_symlink() {
                    continue;
                }
                let path = entry.path();
                if ft.is_dir() {
                    stack.push(path);
                } else if ft.is_file() {
                    paths.push(path);
                }
            }
        }
        paths.sort();
        for path in paths {
            path.to_string_lossy().hash(&mut hasher);
            std::fs::read(&path).expect("read").hash(&mut hasher);
        }
        hasher.finish()
    };
    let before = hash_tree(&dir);
    let bundle = load("clean-healthy-chain");
    let _ = all_findings(&bundle);
    assert_eq!(before, hash_tree(&dir));
}
