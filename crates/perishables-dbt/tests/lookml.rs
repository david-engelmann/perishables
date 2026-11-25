//! The native LookML gate, slice 1. The defect fixture is
//! the API slice's defect model re-expressed as files, plus
//! the one defect only the files can show, a measure built
//! on a field its view does not define.

use perishables_dbt::{Code, grade, notes, read_lookml_definition};

fn load(name: &str) -> perishables_dbt::Definition {
    let path = format!(
        "{}/tests/fixtures/{name}.lookml",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    read_lookml_definition(&text).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

#[test]
fn clean_project_grades_clean() {
    let def = load("lookml-clean");
    assert_eq!(def.model_names.len(), 2);
    assert_eq!(def.dimensions.len(), 3);
    assert_eq!(def.relationships.len(), 1);
    // The measure's field reference resolves against its
    // view's dimension, and the TABLE pseudo references in
    // dimension SQL claim nothing.
    assert_eq!(def.metrics.len(), 1);
    assert_eq!(def.metrics[0].measure_refs, vec!["amount".to_string()]);
    let findings = grade(&def);
    assert!(
        findings.is_empty(),
        "a clean project must grade clean, got {findings:?}"
    );
    assert!(notes(&def).is_empty());
}

#[test]
fn file_defects_grade_exactly() {
    // The API slice's defect model re-expressed as files,
    // grading to the same two findings the API form grades
    // to, as the design record's gate states.
    let def = load("lookml-defects");
    let findings = grade(&def);
    assert_eq!(findings.len(), 2, "got {findings:?}");
    assert_eq!(findings[0].code, Code::DanglingRelationship);
    assert_eq!(findings[0].item, "orders_to_shipments");
    assert_eq!(findings[1].code, Code::NameCollision);
    assert_eq!(findings[1].item, "revenue");
}

#[test]
fn a_measure_on_an_undefined_field_is_exactly_one_finding() {
    // The design record's first gate element, on its own
    // fixture. The measure references gross_profit, which
    // no field in the view defines.
    let def = load("lookml-field");
    let findings = grade(&def);
    assert_eq!(findings.len(), 1, "got {findings:?}");
    assert_eq!(findings[0].code, Code::DanglingMetricReference);
    assert_eq!(findings[0].item, "margin");
}

#[test]
fn a_file_outside_the_subset_is_a_reader_error() {
    // No double semicolon terminator, so the SQL chunk never
    // ends. The reader must refuse, not grade partially.
    let text = "view: v { dimension: d { sql: ${TABLE}.x } }";
    assert!(read_lookml_definition(text).is_err());
}
