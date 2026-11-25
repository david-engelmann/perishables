//! The Ossie gate. The fixtures plant one state each, and the
//! tests assert the exact finding set, the same discipline as
//! the dbt gate.

use perishables_dbt::{Code, grade, notes, read_ossie_definition};

fn load(name: &str) -> perishables_dbt::Definition {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    read_ossie_definition(&text).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

#[test]
fn clean_ossie_grades_clean() {
    let def = load("ossie-clean");
    assert_eq!(def.model_names, vec!["orders".to_string()]);
    assert_eq!(def.dimensions.len(), 2);
    assert_eq!(def.metrics.len(), 1);
    // Model, dataset, and field synonyms are all recorded as
    // alias claims, and none of them collide.
    assert_eq!(def.aliases.len(), 3);
    let findings = grade(&def);
    assert!(
        findings.is_empty(),
        "a clean Ossie document must grade clean, got {findings:?}"
    );
    let found_notes = notes(&def);
    assert!(
        found_notes.is_empty(),
        "every item is described, got {found_notes:?}"
    );
}

#[test]
fn a_synonym_claimed_by_two_fields_is_one_collision() {
    let def = load("ossie-collision");
    let findings = grade(&def);
    assert_eq!(
        findings.len(),
        1,
        "exactly one collision finding, got {findings:?}"
    );
    assert_eq!(findings[0].code, Code::NameCollision);
    assert_eq!(findings[0].item, "revenue");
    assert!(findings[0].detail.contains("field orders.amount"));
    assert!(findings[0].detail.contains("field orders.recognized"));
}
