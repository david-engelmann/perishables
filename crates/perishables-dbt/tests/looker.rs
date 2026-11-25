//! The Looker gate, on the assembled API document shape.

use perishables_dbt::{Code, grade, notes, read_looker_definition};

fn load(name: &str) -> perishables_dbt::Definition {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    read_looker_definition(&text).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

#[test]
fn clean_model_grades_clean() {
    let def = load("looker-clean");
    assert_eq!(def.model_names.len(), 2);
    assert_eq!(def.relationships.len(), 1);
    // Two synonym claims, territory and sales, neither
    // colliding with a display name or each other.
    assert_eq!(def.aliases.len(), 2);
    let findings = grade(&def);
    assert!(
        findings.is_empty(),
        "a clean Looker model must grade clean, got {findings:?}"
    );
    assert!(notes(&def).is_empty());
}

#[test]
fn defects_grade_exactly() {
    let def = load("looker-defects");
    let findings = grade(&def);
    // The region dimension claims the synonym revenue, which
    // the total_revenue measure already answers to as its
    // label. And the explore joins a view the model does not
    // carry. Exactly those two findings.
    assert_eq!(findings.len(), 2, "got {findings:?}");
    assert_eq!(findings[0].code, Code::DanglingRelationship);
    assert_eq!(findings[0].item, "orders_to_shipments");
    assert_eq!(findings[1].code, Code::NameCollision);
    assert_eq!(findings[1].item, "revenue");
    assert!(findings[1].detail.contains("dimension orders.region"));
    assert!(findings[1].detail.contains("measure orders.total_revenue"));
}
