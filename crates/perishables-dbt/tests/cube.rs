//! The Cube gate, on the meta JSON shape.

use perishables_dbt::{Code, grade, notes, read_cube_definition};

fn load(name: &str) -> perishables_dbt::Definition {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    read_cube_definition(&text).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

#[test]
fn clean_model_grades_clean() {
    let def = load("cube-clean");
    assert_eq!(def.model_names.len(), 2);
    assert_eq!(def.relationships.len(), 1);
    assert_eq!(def.ai_contexts.len(), 1);
    let findings = grade(&def);
    assert!(
        findings.is_empty(),
        "a clean Cube model must grade clean, got {findings:?}"
    );
    let found_notes = notes(&def);
    assert!(
        found_notes.is_empty(),
        "described, short context, got {found_notes:?}"
    );
}

#[test]
fn join_to_a_missing_cube_dangles() {
    let def = load("cube-dangling");
    let findings = grade(&def);
    assert_eq!(findings.len(), 1, "exactly one finding, got {findings:?}");
    assert_eq!(findings[0].code, Code::DanglingRelationship);
    assert_eq!(findings[0].item, "orders_to_shipments");
}

#[test]
fn ai_context_at_the_truncation_limit_is_a_note() {
    // Built inline because a 2,000 character fixture file
    // would be noise. The note is the gradeable fact. The
    // finding set stays empty, because truncation risk is a
    // platform behavior to surface, not a defect to gate.
    let long = "x".repeat(2_001);
    let text = format!(
        r#"{{"cubes": [{{"name": "orders", "measures": [{{"name": "orders.revenue", "description": "Gross revenue.", "meta": {{"ai_context": "{long}"}}}}], "dimensions": []}}]}}"#
    );
    let def = read_cube_definition(&text).expect("parses");
    assert!(grade(&def).is_empty());
    let found_notes = notes(&def);
    assert_eq!(found_notes.len(), 1);
    assert!(found_notes[0].text.contains("truncates silently"));
    assert_eq!(found_notes[0].item, "measure orders.revenue");
}
