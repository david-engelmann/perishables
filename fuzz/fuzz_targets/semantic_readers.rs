#![no_main]

use libfuzzer_sys::fuzz_target;
use perishables_semantic::{
    Definition, grade, notes, parse_tree, read_atscale_definition, read_cube_definition,
    read_cube_js, read_cube_yaml, read_databricks_definition, read_definition,
    read_looker_definition, read_lookml_definition, read_lookml_project, read_ossie_definition,
    read_ossie_yaml, read_snowflake_definition,
};

const MAX_INPUT_BYTES: usize = 64 * 1024;
const PINNED_CHANGED_AT: i64 = 1_795_132_800; // 2026-11-20T00:00:00Z

fn exercise_definition(mut def: Definition) {
    let _ = grade(&def);
    let _ = notes(&def);
    def.changed_at = Some(PINNED_CHANGED_AT);
    let _ = grade(&def);
    let _ = notes(&def);
}

fn split_at_char_boundary(s: &str, approx: usize) -> (&str, &str) {
    let mut idx = approx.min(s.len());
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    s.split_at(idx)
}

fn fuzz_str(text: &str) {
    for reader in [
        read_definition,
        read_ossie_definition,
        read_ossie_yaml,
        read_snowflake_definition,
        read_databricks_definition,
        read_cube_definition,
        read_cube_yaml,
        read_cube_js,
        read_looker_definition,
        read_lookml_definition,
        read_atscale_definition,
    ] {
        if let Ok(def) = reader(text) {
            exercise_definition(def);
        }
    }

    let _ = parse_tree(text);

    let (left, right) = split_at_char_boundary(text, text.len() / 2);
    let project_files = [
        ("views/orders.view.lookml".to_string(), left.to_string()),
        ("models/sales.model.lookml".to_string(), right.to_string()),
    ];
    if let Ok(def) = read_lookml_project(&project_files) {
        exercise_definition(def);
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    if let Ok(valid) = std::str::from_utf8(data) {
        fuzz_str(valid);
    } else {
        let lossy = String::from_utf8_lossy(data);
        fuzz_str(&lossy);
    }
});
