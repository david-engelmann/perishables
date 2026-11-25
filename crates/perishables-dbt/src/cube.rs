//! The Cube reader. It reads the model JSON the Cube REST
//! meta endpoint returns, which carries every cube and view
//! with its measures, dimensions, joins, descriptions, and
//! meta in one document, and which the research names as
//! offline gradeable beside the model files themselves. The
//! YAML model file form joins it over the YAML reader, with
//! the mapping shared, and the JS model file form joins it
//! over the JS subset reader in js.rs, with its own mapping
//! for the keyed object shape JS uses.
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

use super::{AiContext, Definition, Dimension, Json, Measure, Metric, Relationship};

/// Parse Cube meta JSON text into the grading model.
pub fn read_cube_definition(text: &str) -> Result<Definition, String> {
    let root = super::json::parse(text)?;
    Ok(definition_from(&root))
}

/// Parse a Cube model file in YAML form into the grading
/// model. The model file shape matches the meta shape the
/// mapping consumes, unqualified member names where meta
/// qualifies them, and the shared mapping shortens both.
/// The two forms of the same model grade identically.
pub fn read_cube_yaml(text: &str) -> Result<Definition, String> {
    let root = super::yaml::parse(text)?;
    Ok(definition_from(&root))
}

/// Parse a Cube model file in JavaScript form into the
/// grading model, over the JS subset reader. In the JS
/// form, measures, dimensions, and joins are objects keyed
/// by member name rather than lists. A view call contributes
/// its name as a model and its member selection as
/// references, graded by the dangling family against the
/// qualified field set this reader builds.
pub fn read_cube_js(text: &str) -> Result<Definition, String> {
    let calls = super::js::parse_calls(text)?;
    let mut def = Definition::default();
    for call in &calls {
        if call.name != "cube" && call.name != "view" {
            continue;
        }
        let Some(Json::Str(name)) = call.args.first() else {
            continue;
        };
        def.model_names.push(name.clone());
        let Some(Json::Map(object)) = call.args.get(1) else {
            continue;
        };
        if call.name == "view" {
            collect_view_selection(object, name, &mut def);
            continue;
        }
        if let Some(Json::Map(measures)) = object.get("measures") {
            for (member, item) in measures {
                def.qualified_fields.push(format!("{name}.{member}"));
                collect_context(item, &format!("measure {name}.{member}"), &mut def);
                def.measures.push(Measure {
                    name: member.clone(),
                    label: opt_text_of(item, "title"),
                    description: opt_text_of(item, "description"),
                    model: name.clone(),
                });
            }
        }
        if let Some(Json::Map(dims)) = object.get("dimensions") {
            for (member, item) in dims {
                def.qualified_fields.push(format!("{name}.{member}"));
                collect_context(item, &format!("dimension {name}.{member}"), &mut def);
                def.dimensions.push(Dimension {
                    name: member.clone(),
                    label: opt_text_of(item, "title"),
                    description: opt_text_of(item, "description"),
                    model: name.clone(),
                });
            }
        }
        if let Some(Json::Map(joins)) = object.get("joins") {
            for target in joins.keys() {
                def.relationships.push(Relationship {
                    name: format!("{name}_to_{target}"),
                    left_table: name.clone(),
                    right_table: target.clone(),
                });
            }
        }
    }
    Ok(def)
}

/// A view's member selection. Each cubes entry names a
/// join path whose terminal cube supplies the members, and
/// its includes and excludes name members of that cube,
/// qualified or bare. The selection becomes one metric
/// whose references are the qualified member names, so the
/// dangling family grades a selected member the cube does
/// not define. A star selection claims everything and
/// references nothing.
fn collect_view_selection(
    object: &std::collections::BTreeMap<String, Json>,
    view: &str,
    def: &mut Definition,
) {
    let Some(Json::List(entries)) = object.get("cubes") else {
        return;
    };
    let mut refs: Vec<String> = Vec::new();
    for entry in entries {
        let Some(path) = entry.get("join_path").and_then(Json::as_str) else {
            continue;
        };
        let terminal = path.rsplit('.').next().unwrap_or(path);
        for key in ["includes", "excludes"] {
            match entry.get(key) {
                Some(Json::List(members)) => {
                    for member in members.iter().filter_map(Json::as_str) {
                        if member.contains('.') {
                            refs.push(member.to_string());
                        } else {
                            refs.push(format!("{terminal}.{member}"));
                        }
                    }
                }
                Some(Json::Str(star)) if star == "*" => {}
                _ => {}
            }
        }
    }
    refs.sort();
    refs.dedup();
    if !refs.is_empty() {
        def.metrics.push(Metric {
            name: view.to_string(),
            label: None,
            // The selection is a reader construct, not a user
            // definition, so it carries a plain description
            // rather than tripping the coverage notes.
            description: Some("View member selection.".to_string()),
            kind: "view_selection".to_string(),
            measure_refs: refs,
            metric_refs: Vec::new(),
        });
    }
}

/// The mapping itself, over a parsed document of either
/// form.
fn definition_from(root: &Json) -> Definition {
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
    def
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
