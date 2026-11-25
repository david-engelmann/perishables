//! The adapter gate. Each fixture is a small semantic manifest
//! with one planted state, and the tests assert the exact
//! finding set, so the gate fails when a must fail fixture
//! passes or a finding names the wrong rule.

use perishables_dbt::{Code, grade, notes, read_definition};

fn load(name: &str) -> perishables_dbt::Definition {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    read_definition(&text).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

fn codes(findings: &[perishables_dbt::Finding]) -> Vec<(String, String)> {
    findings
        .iter()
        .map(|f| (f.code.label().to_string(), f.item.clone()))
        .collect()
}

#[test]
fn clean_manifest_grades_clean() {
    let def = load("clean");
    assert_eq!(def.metrics.len(), 3);
    assert_eq!(def.measures.len(), 2);
    let findings = grade(&def);
    assert!(
        findings.is_empty(),
        "a clean manifest must grade clean, got {findings:?}"
    );
    let found_notes = notes(&def);
    // Everything is described, and nothing carries a label
    // except the metrics, so the notes are exactly the label
    // coverage on the two measures and two dimensions.
    assert_eq!(found_notes.len(), 4, "got {found_notes:?}");
    assert!(
        found_notes
            .iter()
            .all(|n| n.text.contains("carries no label"))
    );
}

#[test]
fn dangling_references_are_exact() {
    let def = load("dangling");
    let findings = grade(&def);
    assert_eq!(
        codes(&findings),
        vec![
            (
                Code::DanglingMetricReference.label().to_string(),
                "gross_margin".to_string()
            ),
            (
                Code::DanglingMetricReference.label().to_string(),
                "margin_rate".to_string()
            ),
        ],
        "one finding per dangling reference, got {findings:?}"
    );
}

#[test]
fn one_label_claimed_thrice_is_one_collision() {
    let def = load("collision");
    let findings = grade(&def);
    assert_eq!(
        findings.len(),
        1,
        "exactly one collision finding, got {findings:?}"
    );
    assert_eq!(findings[0].code, Code::NameCollision);
    assert_eq!(findings[0].item, "revenue");
    assert!(findings[0].detail.contains("measure revenue"));
    assert!(findings[0].detail.contains("metric total_revenue"));
    assert!(findings[0].detail.contains("metric recognized_revenue"));
}

#[test]
fn notes_report_coverage_without_tripping_the_gate() {
    // The dangling fixture describes everything it defines, so
    // its only note is the orphan. The revenue measure is
    // referenced by no metric in this fixture, because the one
    // metric that should read it points at a measure that does
    // not exist.
    let def = load("dangling");
    let found = notes(&def);
    // Two notes. The orphan, and the revenue measure's label
    // coverage, the one item in the fixture with no label.
    assert_eq!(found.len(), 2, "got {found:?}");
    assert!(
        found
            .iter()
            .any(|n| n.text.contains("referenced by no metric"))
    );
    assert!(
        found
            .iter()
            .any(|n| n.text.contains("measure revenue carries no label"))
    );
}
