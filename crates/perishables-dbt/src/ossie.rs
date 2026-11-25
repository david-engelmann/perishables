//! The Apache Ossie reader. Ossie documents carry datasets,
//! relationships, and metrics at the root, and an ai_context
//! block at model, dataset, and field level holding
//! instructions, synonyms, and examples for AI tools.
//!
//! The mapping into the grading model is the one the design
//! record sets. Datasets are the semantic models. Fields are
//! gradeable items in the dimension family. Metrics are
//! metrics, with no references extracted in this slice, because
//! an Ossie metric expression is a code string and parsing
//! references out of it is a design question of its own. The
//! synonyms inside ai_context are the new surface. Each one is
//! an alias claim by its owner, and alias claims enter the
//! collision family beside display names, which is how a
//! synonym that names two different fields gets caught.
//!
//! The first Ossie slice read the JSON form only. The YAML
//! form joins it in the slice that closes that boundary,
//! over the YAML reader written for Snowflake, with the
//! mapping shared so both forms grade identically.

use super::{Alias, Definition, Dimension, Json, Metric};

/// Parse Ossie JSON text into the grading model.
pub fn read_ossie_definition(text: &str) -> Result<Definition, String> {
    let root = super::json::parse(text)?;
    Ok(definition_from(&root))
}

/// Parse an Ossie document in YAML form into the grading
/// model, over the YAML reader from the Snowflake slice. The
/// mapping is the JSON reader's own, shared below, so the
/// two forms of the same document grade identically.
pub fn read_ossie_yaml(text: &str) -> Result<Definition, String> {
    let root = super::yaml::parse(text)?;
    Ok(definition_from(&root))
}

/// The mapping itself, over a parsed document of either
/// form.
fn definition_from(root: &Json) -> Definition {
    let mut def = Definition::default();
    let root_name = text_of(&root, "name");
    collect_aliases(&root, &format!("model {root_name}"), &mut def);
    if let Some(datasets) = root.get("datasets").and_then(Json::as_list) {
        for dataset in datasets {
            let dataset_name = text_of(dataset, "name");
            def.model_names.push(dataset_name.clone());
            collect_aliases(dataset, &format!("dataset {dataset_name}"), &mut def);
            if let Some(fields) = dataset.get("fields").and_then(Json::as_list) {
                for field in fields {
                    let field_name = text_of(field, "name");
                    collect_aliases(
                        field,
                        &format!("field {dataset_name}.{field_name}"),
                        &mut def,
                    );
                    def.dimensions.push(Dimension {
                        name: field_name,
                        label: opt_text_of(field, "label"),
                        description: opt_text_of(field, "description"),
                        model: dataset_name.clone(),
                    });
                }
            }
        }
    }
    if let Some(metrics) = root.get("metrics").and_then(Json::as_list) {
        for m in metrics {
            let metric_name = text_of(m, "name");
            collect_aliases(m, &format!("metric {metric_name}"), &mut def);
            def.metrics.push(Metric {
                name: metric_name,
                label: opt_text_of(m, "label"),
                description: opt_text_of(m, "description"),
                kind: "ossie".to_string(),
                measure_refs: Vec::new(),
                metric_refs: Vec::new(),
            });
        }
    }
    def
}

/// The synonyms inside one ai_context block become alias
/// claims owned by the item carrying the block. An ai_context
/// given as a bare string is instructions only and claims
/// nothing.
fn collect_aliases(item: &Json, owner: &str, def: &mut Definition) {
    let Some(context) = item.get("ai_context") else {
        return;
    };
    let Some(synonyms) = context.get("synonyms") else {
        return;
    };
    for term in synonyms.as_str_list() {
        def.aliases.push(Alias {
            term,
            owner: owner.to_string(),
        });
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
