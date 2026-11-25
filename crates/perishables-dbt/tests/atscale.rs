//! The AtScale gate, on the JSON export shape the reader
//! states.

use perishables_dbt::{Code, grade, notes, read_atscale_definition};

fn load(name: &str) -> perishables_dbt::Definition {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    read_atscale_definition(&text).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

#[test]
fn clean_model_grades_clean() {
    let def = load("atscale-clean");
    assert_eq!(def.attributes, vec!["orders.region", "orders.city"]);
    // The revenue measure and the calculation group both
    // enter as display claiming items.
    assert_eq!(def.measures.len(), 2);
    let findings = grade(&def);
    assert!(
        findings.is_empty(),
        "a clean AtScale model must grade clean, got {findings:?}"
    );
    assert!(notes(&def).is_empty());
}

#[test]
fn hierarchy_level_on_a_missing_attribute_dangles() {
    let def = load("atscale-dangling");
    let findings = grade(&def);
    assert_eq!(findings.len(), 1, "exactly one finding, got {findings:?}");
    assert_eq!(findings[0].code, Code::DanglingMetricReference);
    assert_eq!(findings[0].item, "geography");
    assert!(findings[0].detail.contains("orders.country"));
    assert!(findings[0].detail.contains("attribute"));
}
