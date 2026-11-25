//! Cross ecosystem grading for dbt semantic manifests.
//!
//! This adapter is the first build from the cross ecosystem
//! design record at docs/DESIGN-CROSS-ECOSYSTEM.md. It reads a
//! dbt semantic manifest, the artifact `dbt parse` writes as
//! semantic_manifest.json, or the semantic_models and metrics
//! keys of a full manifest.json, into one normalized grading
//! model, and runs two rule families over it.
//!
//! Structural integrity, in its first form, is the dangling
//! reference check. A metric built on a measure or an input
//! metric that does not exist is the semantic layer form of a
//! dangling superseded_by target.
//!
//! Discoverability, in its first form, is the name collision
//! check plus coverage notes. Two gradeable items answering to
//! the same label are two live claims to one name. Missing
//! descriptions and unreferenced measures are reported as notes,
//! information and never findings, in the posture the OKF
//! auditor uses for a concept with no sources.
//!
//! Slice 2 adds the Apache Ossie reader beside the dbt one.
//! Ossie documents carry an ai_context block whose synonyms
//! are claims that an item answers to another name, so the
//! model gains alias claims and the collision family reads
//! them beside display names. See ossie.rs for the mapping and
//! its stated boundary, the JSON form only in this slice.
//!
//! The four locked OKF calls in perishables-core are untouched
//! by this crate. Cross ecosystem finding codes are a separate
//! closed set, versioned with this crate.

mod json;
mod ossie;

use std::collections::BTreeMap;

pub use json::Json;
pub use ossie::read_ossie_definition;

/// One measure inside a semantic model.
#[derive(Debug, Clone, PartialEq)]
pub struct Measure {
    pub name: String,
    pub label: Option<String>,
    pub description: Option<String>,
    pub model: String,
}

/// One dimension inside a semantic model.
#[derive(Debug, Clone, PartialEq)]
pub struct Dimension {
    pub name: String,
    pub label: Option<String>,
    pub description: Option<String>,
    pub model: String,
}

/// One metric. References are the names its type_params point
/// at, split into measures it reads directly and metrics it is
/// derived from.
#[derive(Debug, Clone, PartialEq)]
pub struct Metric {
    pub name: String,
    pub label: Option<String>,
    pub description: Option<String>,
    pub kind: String,
    pub measure_refs: Vec<String>,
    pub metric_refs: Vec<String>,
}

/// One alias claim. A synonym recorded for an item is a claim
/// that the item also answers to that term, and it enters the
/// collision family on the same footing as a display name.
#[derive(Debug, Clone, PartialEq)]
pub struct Alias {
    pub term: String,
    /// The claiming item, in the same owner wording the
    /// collision check uses, for example "field orders.region".
    pub owner: String,
}

/// The normalized grading model for one manifest.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Definition {
    pub model_names: Vec<String>,
    pub measures: Vec<Measure>,
    pub dimensions: Vec<Dimension>,
    pub metrics: Vec<Metric>,
    /// Alias claims from ecosystems that record synonyms. The
    /// dbt reader leaves this empty in this slice, because
    /// native dbt has no synonym field. The Ossie reader fills
    /// it from ai_context.
    pub aliases: Vec<Alias>,
}

/// The closed set of cross ecosystem finding codes this adapter
/// emits. A code once published never changes meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Code {
    DanglingMetricReference,
    NameCollision,
}

impl Code {
    pub fn label(self) -> &'static str {
        match self {
            Code::DanglingMetricReference => "DANGLING_METRIC_REFERENCE",
            Code::NameCollision => "NAME_COLLISION",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub code: Code,
    /// The gradeable item the finding is about, a metric name
    /// for a dangling reference, the shared label for a
    /// collision.
    pub item: String,
    pub detail: String,
}

/// A reported fact that is not a defect. Coverage and orphan
/// observations travel as notes so a reader sees them without
/// the gate tripping on them.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub item: String,
    pub text: String,
}

/// Parse manifest JSON text into the grading model. Accepts the
/// semantic_manifest.json shape and the full manifest.json
/// shape, which carry the same semantic_models and metrics keys.
pub fn read_definition(text: &str) -> Result<Definition, String> {
    let root = json::parse(text)?;
    let mut def = Definition::default();
    if let Some(models) = root.get("semantic_models").and_then(Json::as_list) {
        for model in models {
            let model_name = text_of(model, "name");
            def.model_names.push(model_name.clone());
            if let Some(measures) = model.get("measures").and_then(Json::as_list) {
                for m in measures {
                    def.measures.push(Measure {
                        name: text_of(m, "name"),
                        label: opt_text_of(m, "label"),
                        description: opt_text_of(m, "description"),
                        model: model_name.clone(),
                    });
                }
            }
            if let Some(dims) = model.get("dimensions").and_then(Json::as_list) {
                for d in dims {
                    def.dimensions.push(Dimension {
                        name: text_of(d, "name"),
                        label: opt_text_of(d, "label"),
                        description: opt_text_of(d, "description"),
                        model: model_name.clone(),
                    });
                }
            }
        }
    }
    if let Some(metrics) = root.get("metrics").and_then(Json::as_list) {
        for m in metrics {
            let mut metric = Metric {
                name: text_of(m, "name"),
                label: opt_text_of(m, "label"),
                description: opt_text_of(m, "description"),
                kind: text_of(m, "type"),
                measure_refs: Vec::new(),
                metric_refs: Vec::new(),
            };
            if let Some(params) = m.get("type_params") {
                collect_refs(params, &mut metric);
            }
            def.metrics.push(metric);
        }
    }
    Ok(def)
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

/// A reference in type_params is a bare string in some shapes
/// and a map carrying a name in others. Both name the same
/// thing.
fn ref_name(value: &Json) -> Option<String> {
    match value {
        Json::Str(s) => Some(s.clone()),
        Json::Map(_) => value.get("name").and_then(Json::as_str).map(str::to_owned),
        _ => None,
    }
}

fn collect_refs(params: &Json, metric: &mut Metric) {
    if let Some(measure) = params.get("measure")
        && let Some(name) = ref_name(measure)
    {
        metric.measure_refs.push(name);
    }
    for side in ["numerator", "denominator"] {
        if let Some(part) = params.get(side) {
            if let Some(measure) = part.get("measure")
                && let Some(name) = ref_name(measure)
            {
                metric.measure_refs.push(name);
            }
            if let Some(list) = part.get("metrics").and_then(Json::as_list) {
                for entry in list {
                    if let Some(name) = ref_name(entry) {
                        metric.metric_refs.push(name);
                    }
                }
            }
        }
    }
    if let Some(list) = params.get("input_metrics").and_then(Json::as_list) {
        for entry in list {
            if let Some(name) = ref_name(entry) {
                metric.metric_refs.push(name);
            }
        }
    }
    if let Some(list) = params.get("metrics").and_then(Json::as_list) {
        for entry in list {
            if let Some(name) = ref_name(entry) {
                metric.metric_refs.push(name);
            }
        }
    }
    metric.measure_refs.sort();
    metric.measure_refs.dedup();
    metric.metric_refs.sort();
    metric.metric_refs.dedup();
}

/// Structural integrity. Every measure or metric a metric
/// points at must exist in the same definition.
pub fn check_dangling(def: &Definition) -> Vec<Finding> {
    let measure_names: Vec<&str> = def.measures.iter().map(|m| m.name.as_str()).collect();
    let metric_names: Vec<&str> = def.metrics.iter().map(|m| m.name.as_str()).collect();
    let mut findings = Vec::new();
    for metric in &def.metrics {
        for r in &metric.measure_refs {
            if !measure_names.contains(&r.as_str()) {
                findings.push(Finding {
                    code: Code::DanglingMetricReference,
                    item: metric.name.clone(),
                    detail: format!(
                        "metric {} is built on measure {r}, which does not exist",
                        metric.name
                    ),
                });
            }
        }
        for r in &metric.metric_refs {
            if !metric_names.contains(&r.as_str()) {
                findings.push(Finding {
                    code: Code::DanglingMetricReference,
                    item: metric.name.clone(),
                    detail: format!(
                        "metric {} is derived from metric {r}, which does not exist",
                        metric.name
                    ),
                });
            }
        }
    }
    sort_findings(&mut findings);
    findings
}

/// Discoverability. The display name of an item is its label
/// where it carries one, else its name. A display name claimed
/// by two or more items, compared case insensitively, is a
/// collision, two live claims to one name.
pub fn check_collisions(def: &Definition) -> Vec<Finding> {
    let mut claims: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut claim = |display: &str, owner: String| {
        claims.entry(display.to_lowercase()).or_default().push(owner);
    };
    for m in &def.metrics {
        claim(m.label.as_deref().unwrap_or(&m.name), format!("metric {}", m.name));
    }
    for m in &def.measures {
        claim(m.label.as_deref().unwrap_or(&m.name), format!("measure {}", m.name));
    }
    for d in &def.dimensions {
        claim(d.label.as_deref().unwrap_or(&d.name), format!("dimension {}", d.name));
    }
    for a in &def.aliases {
        claim(&a.term, a.owner.clone());
    }
    let mut findings = Vec::new();
    for (display, mut owners) in claims {
        owners.sort();
        owners.dedup();
        if owners.len() > 1 {
            findings.push(Finding {
                code: Code::NameCollision,
                item: display.clone(),
                detail: format!("the name {display} is claimed by {}", owners.join(", ")),
            });
        }
    }
    sort_findings(&mut findings);
    findings
}

/// Coverage and orphan observations, as notes.
pub fn notes(def: &Definition) -> Vec<Note> {
    let mut out = Vec::new();
    for m in &def.metrics {
        if m.description.is_none() {
            out.push(Note {
                item: m.name.clone(),
                text: format!("metric {} carries no description", m.name),
            });
        }
    }
    for m in &def.measures {
        if m.description.is_none() {
            out.push(Note {
                item: m.name.clone(),
                text: format!("measure {} carries no description", m.name),
            });
        }
    }
    for d in &def.dimensions {
        if d.description.is_none() {
            out.push(Note {
                item: d.name.clone(),
                text: format!("dimension {} carries no description", d.name),
            });
        }
    }
    let referenced: Vec<&str> = def
        .metrics
        .iter()
        .flat_map(|m| m.measure_refs.iter().map(String::as_str))
        .collect();
    for m in &def.measures {
        if !referenced.contains(&m.name.as_str()) {
            out.push(Note {
                item: m.name.clone(),
                text: format!("measure {} is referenced by no metric", m.name),
            });
        }
    }
    out.sort_by(|a, b| a.item.cmp(&b.item).then(a.text.cmp(&b.text)));
    out
}

/// Every finding this adapter produces, sorted by code then
/// item then detail, deterministic for a given definition.
pub fn grade(def: &Definition) -> Vec<Finding> {
    let mut findings = check_dangling(def);
    findings.extend(check_collisions(def));
    sort_findings(&mut findings);
    findings
}

fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by(|a, b| {
        a.code
            .label()
            .cmp(b.code.label())
            .then(a.item.cmp(&b.item))
            .then(a.detail.cmp(&b.detail))
    });
}
