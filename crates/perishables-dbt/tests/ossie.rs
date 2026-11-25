//! The Ossie gate. The fixtures plant one state each, and the
//! tests assert the exact finding set, the same discipline as
//! the dbt gate.

use perishables_dbt::{Code, grade, notes, read_ossie_definition, read_ossie_yaml};

fn load(name: &str) -> perishables_dbt::Definition {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    read_ossie_definition(&text).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

fn load_yaml(name: &str) -> perishables_dbt::Definition {
    let path = format!("{}/tests/fixtures/{name}.yaml", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    read_ossie_yaml(&text).unwrap_or_else(|e| panic!("parse {name}: {e}"))
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
fn clean_ossie_yaml_grades_like_the_json_form() {
    // The YAML fixtures mirror the JSON ones field for
    // field, so the two forms of the same document must
    // produce the same model and the same verdict.
    let from_yaml = load_yaml("ossie-clean");
    let from_json = load("ossie-clean");
    assert_eq!(from_yaml, from_json);
    assert!(grade(&from_yaml).is_empty());
    assert!(notes(&from_yaml).is_empty());
}

#[test]
fn ossie_yaml_collision_matches_the_json_form() {
    let def = load_yaml("ossie-collision");
    let findings = grade(&def);
    assert_eq!(findings.len(), 1, "got {findings:?}");
    assert_eq!(findings[0].code, Code::NameCollision);
    assert_eq!(findings[0].item, "revenue");
}

#[test]
fn ossie_relationships_grade() {
    // Root level relationships map into the model. The one
    // naming a dataset the document does not carry dangles,
    // the other is silent.
    let def = load("ossie-relationships");
    assert_eq!(def.relationships.len(), 2);
    let findings = grade(&def);
    assert_eq!(findings.len(), 1, "got {findings:?}");
    assert_eq!(findings[0].code, Code::DanglingRelationship);
    assert_eq!(findings[0].item, "orders_shipment");
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
