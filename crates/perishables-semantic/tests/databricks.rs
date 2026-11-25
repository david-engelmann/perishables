//! The Databricks gate. Offline grading covers the shape of
//! the stored ground truth. Running it is the behavioral
//! family and belongs to a later, online slice.

use perishables_semantic::{Code, grade, notes, read_databricks_definition};

fn load(name: &str) -> perishables_semantic::Definition {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    read_databricks_definition(&text).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

#[test]
fn clean_space_grades_clean() {
    let def = load("databricks-clean");
    assert_eq!(def.model_names, vec!["sales.public.orders".to_string()]);
    assert_eq!(def.dimensions.len(), 2);
    // One benchmark and one example pair are stored ground
    // truth, both fully formed.
    assert_eq!(def.ground_truth.len(), 2);
    assert_eq!(def.ground_truth[0].item, "a1b2c3");
    assert!(def.ground_truth[0].expected_sql.contains("AVG(amount)"));
    let findings = grade(&def);
    assert!(
        findings.is_empty(),
        "a clean space must grade clean, got {findings:?}"
    );
    // The one coverage observation is the sample question,
    // which carries no stored answer by design. The columns
    // carry no descriptions either, and those are notes too.
    let found_notes = notes(&def);
    // Five notes. The sample question, the two undescribed
    // columns, and label coverage on those same two columns,
    // which carry no labels in the serialized form.
    assert_eq!(found_notes.len(), 5, "got {found_notes:?}");
    assert!(
        found_notes
            .iter()
            .any(|n| n.text.contains("carries no stored answer"))
    );
}

#[test]
fn benchmark_without_an_answer_is_a_finding() {
    let def = load("databricks-benchmark");
    let findings = grade(&def);
    assert_eq!(
        findings.len(),
        1,
        "exactly the benchmark finding, got {findings:?}"
    );
    assert_eq!(findings[0].code, Code::BenchmarkWithoutAnswer);
    assert_eq!(findings[0].item, "deadbeef");
}
