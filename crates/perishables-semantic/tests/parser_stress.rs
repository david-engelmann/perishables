//! Deterministic seeded stress tests proving panic-freedom and
//! determinism across all four `perishables-semantic` hand-rolled
//! parsers (`json`, `yaml`, `js`, and `lookml`).

use perishables_semantic::{
    grade, notes, parse_tree, read_atscale_definition, read_cube_definition, read_cube_js,
    read_cube_yaml, read_databricks_definition, read_definition, read_looker_definition,
    read_lookml_definition, read_ossie_definition, read_ossie_yaml, read_snowflake_definition,
};

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[(self.next_u64() as usize) % items.len()]
    }
}

#[test]
fn seeded_adversarial_inputs_never_panic_across_all_semantic_parsers() {
    let fragments: &[&str] = &[
        "{",
        "}",
        "[",
        "]",
        "(",
        ")",
        ":",
        ",",
        ";",
        ";;",
        "\"",
        "'",
        "`",
        "\\",
        "\\\"",
        "\\\\",
        "\\u0041",
        "\\uD800",
        "\\u12",
        "/*",
        "*/",
        "//",
        "#",
        "\n",
        "\r\n",
        "  ",
        "true",
        "false",
        "null",
        "-12.34e+5",
        "cube",
        "view",
        "explore",
        "dimension",
        "measure",
        "sql",
        "html",
        "extends",
        "include",
        "${amount}",
        "${orders.id}",
        "${TABLE}.x",
        "|",
        "|-",
        ">",
        ">-",
        "- name: orders",
        "verified_at: \"2026-11-20T00:00:00Z\"",
        "İstanbul",
        "🦀",
    ];
    let mut rng = Rng::new(0xA5A5_5A5A_F0F0_0F0F);

    for _ in 0..1_000 {
        let len = rng.next_u64() as usize % 24;
        let mut input = String::new();
        for _ in 0..len {
            input.push_str(rng.pick(fragments));
        }

        let j1 = read_definition(&input);
        let j2 = read_definition(&input);
        assert_eq!(j1, j2);
        if let Ok(def) = &j1 {
            let _ = grade(def);
            let _ = notes(def);
        }

        let _ = read_ossie_definition(&input);
        let _ = read_databricks_definition(&input);
        let _ = read_cube_definition(&input);
        let _ = read_looker_definition(&input);
        let _ = read_atscale_definition(&input);

        let y1 = read_snowflake_definition(&input);
        let y2 = read_snowflake_definition(&input);
        assert_eq!(y1, y2);
        let _ = read_ossie_yaml(&input);
        let _ = read_cube_yaml(&input);

        let js1 = read_cube_js(&input);
        let js2 = read_cube_js(&input);
        assert_eq!(js1, js2);

        let l1 = read_lookml_definition(&input);
        let l2 = read_lookml_definition(&input);
        assert_eq!(l1, l2);
        let _ = parse_tree(&input);
    }
}

#[test]
fn truncated_escapes_and_deep_nesting_return_clean_errors_not_panics() {
    // Trailing backslash inside JS and LookML strings (regression guard for E0499 fix).
    assert!(read_cube_js("cube(\"orders\\").is_err());
    assert!(read_cube_js("cube('orders\\").is_err());
    assert!(read_cube_js("cube(`orders\\").is_err());
    assert!(read_lookml_definition("view: \"orders\\").is_err());
    assert!(read_definition("{\"a\": \"abc\\").is_err());

    // Escaped quotes inside JS and LookML strings parse cleanly.
    let js = r#"cube("ord\"ers", { measures: { count: { title: 'Co\'unt', description: `De\`sc` } } });"#;
    let def = read_cube_js(js).expect("escaped JS strings must parse");
    assert_eq!(def.model_names, vec!["ord\"ers".to_string()]);

    let lookml = r#"view: orders { dimension: region { label: "Re\"gion" description: "Sales\\Region" sql: ${TABLE}.region ;; } }"#;
    let ldef = read_lookml_definition(lookml).expect("escaped LookML strings must parse");
    assert_eq!(ldef.dimensions[0].label.as_deref(), Some("Re\"gion"));
    assert_eq!(
        ldef.dimensions[0].description.as_deref(),
        Some("Sales\\Region")
    );
}
