//! The Cube gate, on the meta JSON shape.

use perishables_dbt::{Code, grade, notes, read_cube_definition, read_cube_js, read_cube_yaml};

fn load(name: &str) -> perishables_dbt::Definition {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    read_cube_definition(&text).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

fn load_yaml(name: &str) -> perishables_dbt::Definition {
    let path = format!("{}/tests/fixtures/{name}.yaml", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    read_cube_yaml(&text).unwrap_or_else(|e| panic!("parse {name}: {e}"))
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
fn yaml_model_files_grade_like_the_meta_form() {
    // The YAML fixtures mirror the meta fixtures cube for
    // cube, so the two forms of the same model must produce
    // equal models and equal verdicts. The empty measures
    // list on customers also pins the YAML reader's empty
    // flow list handling.
    let from_yaml = load_yaml("cube-clean");
    let from_json = load("cube-clean");
    assert_eq!(from_yaml, from_json);
    assert!(grade(&from_yaml).is_empty());
    let dangling = load_yaml("cube-dangling");
    let findings = grade(&dangling);
    assert_eq!(findings.len(), 1, "got {findings:?}");
    assert_eq!(findings[0].code, Code::DanglingRelationship);
}

#[test]
fn js_model_files_grade() {
    // The JS fixtures mirror the meta fixtures' model, in
    // the keyed object form JS uses. The verdicts must
    // match the other forms.
    let path = format!(
        "{}/tests/fixtures/cube-clean.js",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let def = read_cube_js(&text).unwrap_or_else(|e| panic!("parse js: {e}"));
    assert_eq!(def.model_names.len(), 2);
    assert_eq!(def.relationships.len(), 1);
    assert_eq!(def.ai_contexts.len(), 1);
    assert!(grade(&def).is_empty(), "got {:?}", grade(&def));
    assert!(notes(&def).is_empty());

    let path = format!(
        "{}/tests/fixtures/cube-dangling.js",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let def = read_cube_js(&text).unwrap_or_else(|e| panic!("parse js: {e}"));
    let findings = grade(&def);
    assert_eq!(findings.len(), 1, "got {findings:?}");
    assert_eq!(findings[0].code, Code::DanglingRelationship);
    assert_eq!(findings[0].item, "orders_to_shipments");
}

#[test]
fn js_outside_the_subset_is_a_reader_error() {
    // A function value is outside the stated subset. The
    // reader must refuse, not grade partially.
    let text = "cube(`orders`, { measures: { x: { sql: () => 1 } } });";
    assert!(read_cube_js(text).is_err());
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
