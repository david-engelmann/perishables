//! The AtScale reader. It reads a semantic model export in
//! JSON, the form AtScale models travel in through the API
//! and git based design workflows. The export shape varies
//! by surface, so this reader states the shape it consumes
//! in its fixtures and treats anything outside it as absent
//! rather than guessing.
//!
//! The mapping. The model is the single model name.
//! Dimensions carry attributes, which become dimension items
//! and also enter the model's attribute set, because
//! hierarchy levels reference attributes by qualified name
//! and the reference family resolves against them. Measures
//! map as before, with captions as their display names.
//! Calculation groups are organization wide shared
//! definitions in AtScale, and their captions are display
//! claims like any other, so they enter as measures under
//! the model and the collision family grades them.
//! Hierarchies become metrics whose references are their
//! levels' attributes, so a level naming an attribute the
//! dimension does not define grades as a dangling reference.

use super::{Definition, Dimension, Json, Measure, Metric, opt_text_of, text_of};

/// Parse an AtScale model export into the grading model.
pub fn read_atscale_definition(text: &str) -> Result<Definition, String> {
    let root = super::json::parse(text)?;
    let mut def = Definition::default();
    let model = text_of(&root, "name");
    def.model_names.push(model.clone());
    if let Some(dims) = root.get("dimensions").and_then(Json::as_list) {
        for dim in dims {
            let dim_name = text_of(dim, "unique_name");
            if let Some(attrs) = dim.get("attributes").and_then(Json::as_list) {
                for attr in attrs {
                    let attr_name = text_of(attr, "unique_name");
                    def.attributes.push(format!("{dim_name}.{attr_name}"));
                    def.dimensions.push(Dimension {
                        name: attr_name,
                        label: opt_text_of(attr, "caption"),
                        description: opt_text_of(attr, "description"),
                        model: dim_name.clone(),
                        refs: Vec::new(),
                    });
                }
            }
        }
    }
    if let Some(measures) = root.get("measures").and_then(Json::as_list) {
        for measure in measures {
            def.measures.push(Measure {
                name: text_of(measure, "unique_name"),
                label: opt_text_of(measure, "caption"),
                description: opt_text_of(measure, "description"),
                model: model.clone(),
            });
        }
    }
    if let Some(groups) = root.get("calculation_groups").and_then(Json::as_list) {
        for group in groups {
            def.measures.push(Measure {
                name: text_of(group, "unique_name"),
                label: opt_text_of(group, "caption"),
                description: opt_text_of(group, "description"),
                model: model.clone(),
            });
        }
    }
    if let Some(hierarchies) = root.get("hierarchies").and_then(Json::as_list) {
        for hierarchy in hierarchies {
            let dim_name = text_of(hierarchy, "dimension");
            let mut refs = Vec::new();
            if let Some(levels) = hierarchy.get("levels").and_then(Json::as_list) {
                for level in levels {
                    refs.push(format!("{dim_name}.{}", text_of(level, "attribute")));
                }
            }
            refs.sort();
            refs.dedup();
            def.metrics.push(Metric {
                name: text_of(hierarchy, "unique_name"),
                label: opt_text_of(hierarchy, "caption"),
                description: opt_text_of(hierarchy, "description")
                    .or_else(|| Some("Dimension hierarchy.".to_string())),
                kind: "hierarchy".to_string(),
                measure_refs: Vec::new(),
                metric_refs: refs,
            });
        }
    }
    Ok(def)
}
