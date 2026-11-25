//! The Cube reader. It reads the model JSON the Cube REST
//! meta endpoint returns, which carries every cube and view
//! with its measures, dimensions, joins, descriptions, and
//! meta in one document, and which the research names as
//! offline gradeable beside the model files themselves. The
//! YAML and JS model files are the follow on surface, stated
//! here as this slice's boundary.
//!
//! The mapping. Cubes and views are the models. Their
//! measures and dimensions map as before. A cube's joins are
//! declared relationships to other cubes, graded by the
//! structural family for dangling targets. Cube's ai_context
//! is a prose field inside meta on measures and dimensions,
//! not a structured synonym list, so it yields no alias
//! claims. What grades is its stated platform behavior.
//! Cube truncates ai_context at 2,000 characters silently, so
//! the reader records each block's owner and character count
//! and the notes family flags any block at or past the limit,
//! because its tail may already be gone without a trace.

use super::{AiContext, Definition, Dimension, Json, Measure, Relationship};

/// Parse Cube meta JSON text into the grading model.
pub fn read_cube_definition(text: &str) -> Result<Definition, String> {
    let root = super::json::parse(text)?;
    let mut def = Definition::default();
    if let Some(cubes) = root.get("cubes").and_then(Json::as_list) {
        for cube in cubes {
            let cube_name = text_of(cube, "name");
            def.model_names.push(cube_name.clone());
            collect_members(cube, &cube_name, &mut def);
            if let Some(joins) = cube.get("joins").and_then(Json::as_list) {
                for join in joins {
                    let target = text_of(join, "name");
                    def.relationships.push(Relationship {
                        name: format!("{cube_name}_to_{target}"),
                        left_table: cube_name.clone(),
                        right_table: target,
                    });
                }
            }
        }
    }
    Ok(def)
}

/// Measures, dimensions, and ai_context counts for one cube
/// or view shaped object.
fn collect_members(item: &Json, model: &str, def: &mut Definition) {
    if let Some(measures) = item.get("measures").and_then(Json::as_list) {
        for measure in measures {
            let name = short_name(&text_of(measure, "name"));
            collect_context(measure, &format!("measure {model}.{name}"), def);
            def.measures.push(Measure {
                name,
                label: opt_text_of(measure, "title"),
                description: opt_text_of(measure, "description"),
                model: model.to_string(),
            });
        }
    }
    if let Some(dims) = item.get("dimensions").and_then(Json::as_list) {
        for dim in dims {
            let name = short_name(&text_of(dim, "name"));
            collect_context(dim, &format!("dimension {model}.{name}"), def);
            def.dimensions.push(Dimension {
                name,
                label: opt_text_of(dim, "title"),
                description: opt_text_of(dim, "description"),
                model: model.to_string(),
            });
        }
    }
}

/// Record one item's meta.ai_context by owner and character
/// count, where it carries one.
fn collect_context(item: &Json, owner: &str, def: &mut Definition) {
    if let Some(context) = item
        .get("meta")
        .and_then(|m| m.get("ai_context"))
        .and_then(Json::as_str)
    {
        def.ai_contexts.push(AiContext {
            owner: owner.to_string(),
            chars: context.chars().count(),
        });
    }
}

/// Meta names are qualified, cube.member. The grading model
/// keys items by their short member name and carries the cube
/// separately.
fn short_name(name: &str) -> String {
    name.rsplit('.').next().unwrap_or(name).to_string()
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
