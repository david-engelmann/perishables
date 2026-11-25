//! The Looker reader. LookML itself is a custom language and
//! parsing it is a project of its own, so this reader takes
//! the JSON the Looker API returns for a model's views and
//! explores, assembled into one document. That is the shape
//! the research names as offline gradeable beside the LookML
//! files. A native LookML reader is a later surface, stated
//! here as this slice's boundary.
//!
//! The mapping. Views are the models. Their dimensions and
//! measures carry labels, descriptions, and a structured
//! synonyms list, which becomes alias claims the collision
//! family grades, the same treatment Ossie synonyms get.
//! Explores declare joins to other views, and those are
//! relationships, graded for dangling targets by the
//! structural family.

use super::{Alias, Definition, Dimension, Json, Measure, Relationship};

/// Parse an assembled Looker model document, views and
/// explores, into the grading model.
pub fn read_looker_definition(text: &str) -> Result<Definition, String> {
    let root = super::json::parse(text)?;
    let mut def = Definition::default();
    if let Some(views) = root.get("views").and_then(Json::as_list) {
        for view in views {
            let view_name = text_of(view, "name");
            def.model_names.push(view_name.clone());
            collect_fields(view, &view_name, &mut def);
        }
    }
    if let Some(explores) = root.get("explores").and_then(Json::as_list) {
        for explore in explores {
            let base = opt_text_of(explore, "view_name")
                .unwrap_or_else(|| text_of(explore, "name"));
            if let Some(joins) = explore.get("joins").and_then(Json::as_list) {
                for join in joins {
                    let target = text_of(join, "name");
                    def.relationships.push(Relationship {
                        name: format!("{base}_to_{target}"),
                        left_table: base.clone(),
                        right_table: target,
                    });
                }
            }
        }
    }
    Ok(def)
}

/// Dimensions, measures, and their synonym claims for one
/// view.
fn collect_fields(view: &Json, view_name: &str, def: &mut Definition) {
    if let Some(dims) = view.get("dimensions").and_then(Json::as_list) {
        for dim in dims {
            let name = text_of(dim, "name");
            push_aliases(dim, &format!("dimension {view_name}.{name}"), def);
            def.dimensions.push(Dimension {
                name,
                label: opt_text_of(dim, "label"),
                description: opt_text_of(dim, "description"),
                model: view_name.to_string(),
            });
        }
    }
    if let Some(measures) = view.get("measures").and_then(Json::as_list) {
        for measure in measures {
            let name = text_of(measure, "name");
            push_aliases(measure, &format!("measure {view_name}.{name}"), def);
            def.measures.push(Measure {
                name,
                label: opt_text_of(measure, "label"),
                description: opt_text_of(measure, "description"),
                model: view_name.to_string(),
            });
        }
    }
}

/// A field's synonyms list becomes alias claims under the
/// field as owner.
fn push_aliases(field: &Json, owner: &str, def: &mut Definition) {
    for term in field.get("synonyms").map(Json::as_str_list).unwrap_or_default() {
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
