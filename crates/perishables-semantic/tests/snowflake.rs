//! The Snowflake gate. Currency is graded against a change
//! instant the caller supplies, because the definition itself
//! stores none, so the tests set it explicitly, the way the
//! command line will from object metadata.

use perishables_semantic::{Code, grade, notes, read_snowflake_definition};

fn load(name: &str) -> perishables_semantic::Definition {
    let path = format!("{}/tests/fixtures/{name}.yaml", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    read_snowflake_definition(&text).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

#[test]
fn clean_view_grades_clean_when_verification_is_current() {
    let mut def = load("snowflake-clean");
    assert_eq!(def.model_names.len(), 2);
    assert_eq!(def.verifications.len(), 1);
    assert_eq!(def.relationships.len(), 1);
    // The verified query is also stored ground truth, with
    // its SQL carried and its name standing as the question
    // where the view carries no separate question text.
    assert_eq!(def.ground_truth.len(), 1);
    assert_eq!(def.ground_truth[0].item, "total_revenue_check");
    assert!(def.ground_truth[0].expected_sql.contains("SUM"));
    // The view last changed before the verification, so the
    // verification is current and nothing grades.
    def.changed_at = Some(1_750_000_000);
    let findings = grade(&def);
    assert!(
        findings.is_empty(),
        "a current verification must grade clean, got {findings:?}"
    );
    let found_notes = notes(&def);
    // Everything is described and stamped. No item carries
    // a label, and the two dimensions carry no synonyms, so
    // the notes are the coverage family on those questions,
    // five in all.
    assert_eq!(found_notes.len(), 5, "got {found_notes:?}");
}

#[test]
fn verification_predating_the_change_is_lapsed() {
    let mut def = load("snowflake-lapsed");
    def.changed_at = Some(1_770_000_000);
    let findings = grade(&def);
    assert_eq!(
        findings.len(),
        1,
        "exactly the lapsed finding, got {findings:?}"
    );
    assert_eq!(findings[0].code, Code::LapsedVerifiedQuery);
    assert_eq!(findings[0].item, "total_revenue_check");
    // The unstamped query is a coverage matter, two notes, and
    // never a second finding. The fixture's one measure
    // carries no label, which is the third note.
    let found_notes = notes(&def);
    assert_eq!(found_notes.len(), 3, "got {found_notes:?}");
    assert_eq!(
        found_notes
            .iter()
            .filter(|n| n.item == "unstamped_check")
            .count(),
        2
    );
}

#[test]
fn no_change_instant_means_no_currency_verdict() {
    let def = load("snowflake-lapsed");
    let findings = grade(&def);
    assert!(
        findings.is_empty(),
        "without a change instant the currency family stays silent, got {findings:?}"
    );
}

#[test]
fn relationship_to_a_missing_table_dangles() {
    let def = load("snowflake-dangling");
    let findings = grade(&def);
    assert_eq!(findings.len(), 1, "exactly one finding, got {findings:?}");
    assert_eq!(findings[0].code, Code::DanglingRelationship);
    assert_eq!(findings[0].item, "orders_to_shipments");
    assert!(findings[0].detail.contains("shipments"));
}

#[test]
fn verified_at_accepts_rfc3339_timestamps_and_formats_rfc3339_details() {
    let yaml = "name: v\ntables:\n  - name: orders\n    description: Orders.\n    facts:\n      - name: rev\n        description: Revenue.\n        expr: AMOUNT\nverified_queries:\n  - name: q1\n    question: Total revenue?\n    sql: |\n      SELECT SUM(AMOUNT) FROM ORDERS\n    verified_by: reviewer\n    verified_at: \"2026-06-01T12:00:00Z\"\n";
    let mut def = read_snowflake_definition(yaml).expect("parses");
    def.changed_at = perishables_core::AuditClock::parse("2026-06-10").map(|c| c.epoch_seconds);
    let findings = grade(&def);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code, Code::LapsedVerifiedQuery);
    assert!(findings[0].detail.contains("2026-06-01T12:00:00Z"));
    assert!(findings[0].detail.contains("2026-06-10T00:00:00Z"));
}
