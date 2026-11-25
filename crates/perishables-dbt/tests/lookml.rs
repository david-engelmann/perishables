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
    let def = load("lookml-defects");
    let findings = grade(&def);
    // Three findings, in sorted order. The dangling join and
    // the synonym collision are the API slice's pair. The
    // third is native to the files, the measure referencing
    // gross_profit, which no field in the view defines.
    assert_eq!(findings.len(), 3, "got {findings:?}");
    assert_eq!(findings[0].code, Code::DanglingMetricReference);
    assert_eq!(findings[0].item, "total_revenue");
    assert_eq!(findings[1].code, Code::DanglingRelationship);
    assert_eq!(findings[1].item, "orders_to_shipments");
    assert_eq!(findings[2].code, Code::NameCollision);
    assert_eq!(findings[2].item, "revenue");
}

#[test]
fn a_file_outside_the_subset_is_a_reader_error() {
    // No double semicolon terminator, so the SQL chunk never
    // ends. The reader must refuse, not grade partially.
    let text = "view: v { dimension: d { sql: ${TABLE}.x } }";
    assert!(read_lookml_definition(text).is_err());
}
