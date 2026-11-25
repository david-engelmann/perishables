//! The Snowflake semantic view reader. It reads the YAML form
//! a semantic view exports through
//! SYSTEM$READ_YAML_FROM_SEMANTIC_VIEW, or the same shape
//! written by hand, into the grading model.
//!
//! The mapping. Tables are the models. Facts are measures,
//! dimensions and time dimensions are dimensions, and table
//! level metrics are metrics whose expressions are code and
//! carry no extracted references in this slice. Synonyms at
//! table and element level are alias claims, as in Ossie.
//! Relationships are declared joins, graded for dangling
//! tables. Verified queries are verification events, the
//! currency surface this ecosystem exists for in the design
//! record's ordering. verified_by is a free text name and
//! verified_at an epoch second, and the reader records them
//! as stated, with no identity meaning attached.
//!
//! The definition carries no last change instant, so
//! changed_at stays unset here. The caller supplies it from
//! object metadata, and the currency check grades only then.

use super::{Alias, Definition, Dimension, Json, Measure, Metric, Relationship, Verification};

/// Parse semantic view YAML text into the grading model.
pub fn read_snowflake_definition(text: &str) -> Result<Definition, String> {
    let root = super::yaml::parse(text)?;
    let mut def = Definition::default();
    if let Some(tables) = root.get("tables").and_then(Json::as_list) {
        for table in tables {
            let table_name = text_of(table, "name");
            def.model_names.push(table_name.clone());
            collect_synonyms(table, &format!("table {table_name}"), &mut def);
            if let Some(facts) = table.get("facts").and_then(Json::as_list) {
                for fact in facts {
                    let name = text_of(fact, "name");
                    collect_synonyms(fact, &format!("fact {table_name}.{name}"), &mut def);
                    def.measures.push(Measure {
                        name,
                        label: None,
                        description: opt_text_of(fact, "description"),
                        model: table_name.clone(),
                    });
                }
            }
            for key in ["dimensions", "time_dimensions"] {
                if let Some(dims) = table.get(key).and_then(Json::as_list) {
                    for dim in dims {
                        let name = text_of(dim, "name");
                        collect_synonyms(dim, &format!("dimension {table_name}.{name}"), &mut def);
                        def.dimensions.push(Dimension {
                            name,
                            label: None,
                            description: opt_text_of(dim, "description"),
                            model: table_name.clone(),
                        });
                    }
                }
            }
            if let Some(metrics) = table.get("metrics").and_then(Json::as_list) {
                for metric in metrics {
                    let name = text_of(metric, "name");
                    collect_synonyms(metric, &format!("metric {table_name}.{name}"), &mut def);
                    def.metrics.push(Metric {
                        name,
                        label: None,
                        description: opt_text_of(metric, "description"),
                        kind: "snowflake".to_string(),
                        measure_refs: Vec::new(),
                        metric_refs: Vec::new(),
                    });
                }
            }
        }
    }
    if let Some(rels) = root.get("relationships").and_then(Json::as_list) {
        for rel in rels {
            def.relationships.push(Relationship {
                name: text_of(rel, "name"),
                left_table: text_of(rel, "left_table"),
                right_table: text_of(rel, "right_table"),
            });
        }
    }
    if let Some(queries) = root.get("verified_queries").and_then(Json::as_list) {
        for query in queries {
            def.verifications.push(Verification {
                item: text_of(query, "name"),
                by: opt_text_of(query, "verified_by"),
                at: epoch_of(query.get("verified_at")),
            });
        }
    }
    Ok(def)
}

fn collect_synonyms(item: &Json, owner: &str, def: &mut Definition) {
    if let Some(synonyms) = item.get("synonyms") {
        for term in synonyms.as_str_list() {
            def.aliases.push(Alias {
                term,
                owner: owner.to_string(),
            });
        }
    }
}

fn epoch_of(value: Option<&Json>) -> Option<i64> {
    match value {
        Some(Json::Number(text)) => text.parse().ok(),
        Some(Json::Str(text)) => text.trim().parse().ok(),
        _ => None,
    }
}

fn text_of(value: &Json, key: &str) -> String {
    value
        .get(key)
        .and_then(Json::as_str)
        .unwrap_or_default()
        .to_string()
}

fn opt_text_of(value: &Json, key: &str) -> Option<String> {
    value.get(key).and_then(Json::as_str).map(str::to_owned)
}
