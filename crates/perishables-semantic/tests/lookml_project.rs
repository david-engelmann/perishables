//! The LookML project gate, slice 2. Project scope is what
//! resolves cross view references, inherits extends, and
//! matches includes, so the gate is a small real project on
//! disk, graded whole and graded with a file removed.

use perishables_semantic::{Code, grade, notes, read_lookml_project};

fn project(names: &[&str]) -> perishables_semantic::Definition {
    let base = format!("{}/tests/fixtures/project", env!("CARGO_MANIFEST_DIR"));
    let files: Vec<(String, String)> = names
        .iter()
        .map(|name| {
            let text = std::fs::read_to_string(format!("{base}/{name}"))
                .unwrap_or_else(|e| panic!("read {name}: {e}"));
            (format!("/project/{name}"), text)
        })
        .collect();
    read_lookml_project(&files).unwrap_or_else(|e| panic!("parse project: {e}"))
}

const ALL: &[&str] = &[
    "orders.view.lookml",
    "customers.view.lookml",
    "extended.view.lookml",
    "sales.model.lookml",
];

#[test]
fn whole_project_grades_with_only_the_planted_defects() {
    let def = project(ALL);
    // The cross view reference resolves against the project,
    // so customer_label is not a finding. Extends inherits,
    // so orders_extended carries region beside its own
    // channel. The one planted defect is ghost extending a
    // view the project does not carry.
    assert!(
        def.qualified_fields
            .contains(&"orders_extended.region".to_string()),
        "extends must make region addressable through the child, got {:?}",
        def.qualified_fields
    );
    let findings = grade(&def);
    assert_eq!(findings.len(), 1, "got {findings:?}");
    assert_eq!(findings[0].code, Code::DanglingRelationship);
    assert_eq!(findings[0].item, "ghost_to_missing_view");
    // The dashboards include matched nothing in the set and
    // is a coverage note. The views include matched, and is
    // silent. The rest of the notes are label coverage, no
    // field or measure in the project carries a label, and
    // inherited fields carry the note once per view that
    // carries them. Thirteen in all.
    let found_notes = notes(&def);
    assert_eq!(found_notes.len(), 13, "got {found_notes:?}");
    assert!(
        found_notes
            .iter()
            .any(|n| n.text.contains("/dashboards/*.lookml"))
    );
}

#[test]
fn removing_a_file_makes_its_references_dangle() {
    // The same project without the customers file. The
    // cross view reference and the explore join both lose
    // their target, which is what project scope means.
    let def = project(&["orders.view.lookml", "sales.model.lookml"]);
    let findings = grade(&def);
    assert!(
        findings
            .iter()
            .any(|f| f.code == Code::DanglingMetricReference && f.item == "customer_label"),
        "the cross view reference must dangle, got {findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|f| f.code == Code::DanglingRelationship && f.item == "orders_to_customers"),
        "the join must dangle, got {findings:?}"
    );
}
