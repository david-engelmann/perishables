//! Cross-ecosystem grading for semantic-layer definitions.
//!
//! Each supported ecosystem provides a reader that normalizes its
//! definition into [`Definition`], and the rule families in this crate
//! run over that normalized model in a format-blind pass.
//!
//! Supported ecosystems and readers:
//! - dbt semantic manifests via [`read_definition`]
//! - Apache Ossie documents in JSON via [`read_ossie_definition`] and YAML via [`read_ossie_yaml`]
//! - Snowflake semantic views via [`read_snowflake_definition`]
//! - Databricks Genie serialized spaces via [`read_databricks_definition`]
//! - Cube models as meta JSON via [`read_cube_definition`], YAML via [`read_cube_yaml`], and JS via [`read_cube_js`]
//! - Looker models as API JSON via [`read_looker_definition`] and native LookML via [`read_lookml_definition`] and [`read_lookml_project`]
//! - AtScale model exports via [`read_atscale_definition`]
//!
//! # Example
//!
//! ```
//! use perishables_semantic::{Code, grade, read_definition};
//!
//! let manifest = r#"{
//!   "semantic_models": [{"name": "orders", "measures": [], "dimensions": []}],
//!   "metrics": [{"name": "gross_margin", "type": "simple", "type_params": {"measure": "missing"}}]
//! }"#;
//! let def = read_definition(manifest).expect("valid manifest");
//! let findings = grade(&def);
//! assert_eq!(findings.len(), 1);
//! assert_eq!(findings[0].code, Code::DanglingMetricReference);
//! ```

mod atscale;
mod cube;
mod databricks;
mod js;
mod json;
mod looker;
mod lookml;
mod ossie;
mod snowflake;
mod yaml;

use std::collections::{BTreeMap, BTreeSet};

use perishables_core::AuditClock;

pub use atscale::read_atscale_definition;
pub use cube::{read_cube_definition, read_cube_js, read_cube_yaml};
pub use databricks::read_databricks_definition;
pub use json::Json;
pub use looker::read_looker_definition;
pub use lookml::{Item, Value, parse_tree, read_lookml_definition, read_lookml_project};
pub use ossie::{read_ossie_definition, read_ossie_yaml};
pub use snowflake::read_snowflake_definition;

/// One measure inside a semantic model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Measure {
    /// Raw measure identifier.
    pub name: String,
    /// Optional human-facing display label or caption.
    pub label: Option<String>,
    /// Optional prose description.
    pub description: Option<String>,
    /// Parent model, table, cube, or view name.
    pub model: String,
}

/// One dimension inside a semantic model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dimension {
    /// Raw dimension or field identifier.
    pub name: String,
    /// Optional human-facing display label or caption.
    pub label: Option<String>,
    /// Optional prose description.
    pub description: Option<String>,
    /// Parent model, dataset, table, cube, or view name.
    pub model: String,
}

/// One metric and the measures or upstream metrics it references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metric {
    /// Raw metric identifier.
    pub name: String,
    /// Optional human-facing display label or caption.
    pub label: Option<String>,
    /// Optional prose description.
    pub description: Option<String>,
    /// Metric kind or ecosystem tag.
    pub kind: String,
    /// Measure or field names referenced directly by this metric.
    pub measure_refs: Vec<String>,
    /// Upstream metric or hierarchy attribute names referenced by this metric.
    pub metric_refs: Vec<String>,
}

/// One alias claim recorded as a synonym on an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alias {
    /// Synonym term claimed by the owner.
    pub term: String,
    /// Qualified owner description, such as `"field orders.region"`.
    pub owner: String,
}

/// One verification event recorded inside a semantic definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verification {
    /// Verified item identifier, such as a Snowflake verified query name.
    pub item: String,
    /// Optional free-text verifier name.
    pub by: Option<String>,
    /// Optional verification timestamp in UTC epoch seconds.
    pub at: Option<i64>,
}

/// One declared relationship between two models, tables, cubes, or views.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship {
    /// Relationship identifier.
    pub name: String,
    /// Left-hand table, model, cube, or view name.
    pub left_table: String,
    /// Right-hand table, model, cube, or view name.
    pub right_table: String,
}

/// One stored ground-truth question and expected SQL pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundTruth {
    /// Pair identifier, such as a benchmark id or query name.
    pub item: String,
    /// Natural-language question text.
    pub question: String,
    /// Expected SQL text claimed to answer the question.
    pub expected_sql: String,
}

/// One `ai_context` block recorded on an item, tracked by character count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiContext {
    /// Qualified item owning the `ai_context` block.
    pub owner: String,
    /// Character count of the `ai_context` string.
    pub chars: usize,
}

/// Normalized cross-ecosystem semantic-layer grading model.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Definition {
    /// Declared model, dataset, table, cube, or view names.
    pub model_names: Vec<String>,
    /// Declared measures across all models.
    pub measures: Vec<Measure>,
    /// Declared dimensions across all models.
    pub dimensions: Vec<Dimension>,
    /// Declared metrics across the definition.
    pub metrics: Vec<Metric>,
    /// Synonym alias claims entering collision detection.
    pub aliases: Vec<Alias>,
    /// Recorded verification events, such as Snowflake verified queries.
    pub verifications: Vec<Verification>,
    /// Declared relationships and joins between models.
    pub relationships: Vec<Relationship>,
    /// Caller-supplied UTC epoch second when the definition last changed.
    pub changed_at: Option<i64>,
    /// Stored ground-truth question and SQL pairs.
    pub ground_truth: Vec<GroundTruth>,
    /// Sample questions carrying no stored ground-truth answer.
    pub unverifiable_questions: Vec<String>,
    /// Recorded `ai_context` character counts.
    pub ai_contexts: Vec<AiContext>,
    /// Qualified dimension attributes for hierarchy resolution.
    pub attributes: Vec<String>,
    /// Qualified `view.field` identifiers for project-scoped resolution.
    pub qualified_fields: Vec<String>,
    /// Project `include` patterns that matched no provided file path.
    pub unmatched_includes: Vec<String>,
}

/// Closed set of cross-ecosystem semantic-layer finding codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Code {
    /// A metric, LookML measure, Cube view, or AtScale hierarchy references a missing target.
    DanglingMetricReference,
    /// Two or more items claim the same case-insensitive display name or synonym.
    NameCollision,
    /// A verified query's timestamp predates the definition's `changed_at` instant.
    LapsedVerifiedQuery,
    /// A declared relationship or `extends` parent names a missing model, table, cube, or view.
    DanglingRelationship,
    /// A stored benchmark or verified query has an empty question or empty SQL answer.
    BenchmarkWithoutAnswer,
    /// Two models connect both directly and through an alternate join path that aggregates at a different grain.
    AmbiguousJoinPath,
}

impl Code {
    /// Stable string label for this finding code.
    pub fn label(self) -> &'static str {
        match self {
            Code::DanglingMetricReference => "DANGLING_METRIC_REFERENCE",
            Code::NameCollision => "NAME_COLLISION",
            Code::LapsedVerifiedQuery => "LAPSED_VERIFIED_QUERY",
            Code::DanglingRelationship => "DANGLING_RELATIONSHIP",
            Code::BenchmarkWithoutAnswer => "BENCHMARK_WITHOUT_ANSWER",
            Code::AmbiguousJoinPath => "AMBIGUOUS_JOIN_PATH",
        }
    }
}

/// One gate finding emitted while grading a semantic definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Finding rule code.
    pub code: Code,
    /// Gradeable item the finding concerns.
    pub item: String,
    /// Human-readable statement of the defect.
    pub detail: String,
}

/// One informational coverage or orphan observation that does not trip the gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// Item the observation concerns.
    pub item: String,
    /// Plain description of the observation.
    pub text: String,
}

/// Parse dbt semantic manifest JSON (`semantic_manifest.json` or `manifest.json`) into [`Definition`].
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

pub(crate) fn text_of(value: &Json, key: &str) -> String {
    value
        .get(key)
        .and_then(Json::as_str)
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn opt_text_of(value: &Json, key: &str) -> Option<String> {
    value.get(key).and_then(Json::as_str).map(str::to_owned)
}

/// Push one alias claim per synonym in a synonyms value.
pub(crate) fn push_synonym_aliases(synonyms: &Json, owner: &str, aliases: &mut Vec<Alias>) {
    for term in synonyms.as_str_list() {
        aliases.push(Alias {
            term,
            owner: owner.to_string(),
        });
    }
}

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

/// Check structural integrity of metric, measure, field, and attribute references.
pub fn check_dangling(def: &Definition) -> Vec<Finding> {
    let measure_names: BTreeSet<&str> = def.measures.iter().map(|m| m.name.as_str()).collect();
    let dimension_names: BTreeSet<&str> = def.dimensions.iter().map(|d| d.name.as_str()).collect();
    let metric_names: BTreeSet<&str> = def.metrics.iter().map(|m| m.name.as_str()).collect();
    let qualified: BTreeSet<&str> = def.qualified_fields.iter().map(String::as_str).collect();
    let attributes: BTreeSet<&str> = def.attributes.iter().map(String::as_str).collect();
    let mut findings = Vec::new();
    for metric in &def.metrics {
        for r in &metric.measure_refs {
            let resolves = if r.contains('.') {
                qualified.contains(r.as_str())
            } else {
                measure_names.contains(r.as_str()) || dimension_names.contains(r.as_str())
            };
            if !resolves {
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
            if !metric_names.contains(r.as_str()) && !attributes.contains(r.as_str()) {
                let kind = if r.contains('.') {
                    "attribute"
                } else {
                    "metric"
                };
                findings.push(Finding {
                    code: Code::DanglingMetricReference,
                    item: metric.name.clone(),
                    detail: format!(
                        "metric {} references {kind} {r}, which does not exist",
                        metric.name
                    ),
                });
            }
        }
    }
    sort_findings(&mut findings);
    findings
}

/// Check case-insensitive display-name and synonym collisions across items.
pub fn check_collisions(def: &Definition) -> Vec<Finding> {
    let mut claims: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut claim = |display: &str, owner: String| {
        claims
            .entry(display.to_lowercase())
            .or_default()
            .push(owner);
    };
    for m in &def.metrics {
        claim(
            m.label.as_deref().unwrap_or(&m.name),
            format!("metric {}", m.name),
        );
    }
    for m in &def.measures {
        claim(
            m.label.as_deref().unwrap_or(&m.name),
            format!("measure {}", m.name),
        );
    }
    for d in &def.dimensions {
        claim(
            d.label.as_deref().unwrap_or(&d.name),
            format!("dimension {}", d.name),
        );
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

/// Check verification currency against [`Definition::changed_at`] when supplied.
pub fn check_currency(def: &Definition) -> Vec<Finding> {
    let mut findings = Vec::new();
    if let Some(changed_at) = def.changed_at {
        let changed_iso = (AuditClock {
            epoch_seconds: changed_at,
        })
        .to_rfc3339();
        for v in &def.verifications {
            if let Some(at) = v.at
                && at < changed_at
            {
                let verified_iso = (AuditClock { epoch_seconds: at }).to_rfc3339();
                findings.push(Finding {
                    code: Code::LapsedVerifiedQuery,
                    item: v.item.clone(),
                    detail: format!(
                        "verified query {} was verified at {verified_iso}, before the definition last changed at {changed_iso}",
                        v.item
                    ),
                });
            }
        }
    }
    sort_findings(&mut findings);
    findings
}

/// Check that every declared relationship names models carried by the definition.
pub fn check_relationships(def: &Definition) -> Vec<Finding> {
    let mut findings = Vec::new();
    for r in &def.relationships {
        for (side, table) in [("left", &r.left_table), ("right", &r.right_table)] {
            if !def.model_names.contains(table) {
                findings.push(Finding {
                    code: Code::DanglingRelationship,
                    item: r.name.clone(),
                    detail: format!(
                        "relationship {} names {side} table {table}, which the definition does not carry",
                        r.name
                    ),
                });
            }
        }
    }
    sort_findings(&mut findings);
    findings
}

/// Check join-path ambiguity (fan traps) across declared model relationships.
pub fn check_join_paths(def: &Definition) -> Vec<Finding> {
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for r in &def.relationships {
        adjacency
            .entry(r.left_table.as_str())
            .or_default()
            .push(r.right_table.as_str());
        adjacency
            .entry(r.right_table.as_str())
            .or_default()
            .push(r.left_table.as_str());
    }
    for neighbors in adjacency.values_mut() {
        neighbors.sort();
        neighbors.dedup();
    }
    let mut findings = Vec::new();
    let mut pairs: Vec<(&str, &str)> = Vec::new();
    for r in &def.relationships {
        let (a, b) = if r.left_table <= r.right_table {
            (r.left_table.as_str(), r.right_table.as_str())
        } else {
            (r.right_table.as_str(), r.left_table.as_str())
        };
        if !pairs.contains(&(a, b)) {
            pairs.push((a, b));
        }
    }
    pairs.sort();
    for (a, b) in pairs {
        if let Some(route) = alternative_route(&adjacency, a, b) {
            findings.push(Finding {
                code: Code::AmbiguousJoinPath,
                item: format!("{a}/{b}"),
                detail: format!(
                    "models {a} and {b} connect directly and again through {route}, routes that aggregate at different grains"
                ),
            });
        }
    }
    sort_findings(&mut findings);
    findings
}

fn alternative_route(adjacency: &BTreeMap<&str, Vec<&str>>, a: &str, b: &str) -> Option<String> {
    let empty: Vec<&str> = Vec::new();
    for mid1 in adjacency.get(a).unwrap_or(&empty) {
        if *mid1 == b || *mid1 == a {
            continue;
        }
        if adjacency.get(mid1).unwrap_or(&empty).contains(&b) {
            return Some((*mid1).to_string());
        }
        for mid2 in adjacency.get(mid1).unwrap_or(&empty) {
            if *mid2 == a || *mid2 == b || *mid2 == *mid1 {
                continue;
            }
            if adjacency.get(mid2).unwrap_or(&empty).contains(&b) {
                return Some(format!("{mid1} and {mid2}"));
            }
        }
    }
    None
}

/// Check stored ground-truth pairs for empty questions or empty SQL answers.
pub fn check_ground_truth(def: &Definition) -> Vec<Finding> {
    let mut findings = Vec::new();
    for pair in &def.ground_truth {
        if pair.question.trim().is_empty() || pair.expected_sql.trim().is_empty() {
            findings.push(Finding {
                code: Code::BenchmarkWithoutAnswer,
                item: pair.item.clone(),
                detail: format!(
                    "benchmark {} stores a question or an answer that is empty, so no run could grade it",
                    pair.item
                ),
            });
        }
    }
    sort_findings(&mut findings);
    findings
}

/// Compute informational coverage and orphan observations for a definition.
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
    let name_linked = def
        .metrics
        .iter()
        .any(|m| m.kind != "view_selection" && !m.measure_refs.is_empty());
    if name_linked {
        let referenced: BTreeSet<&str> = def
            .metrics
            .iter()
            .flat_map(|m| m.measure_refs.iter().map(String::as_str))
            .collect();
        for m in &def.measures {
            let qualified = format!("{}.{}", m.model, m.name);
            if !referenced.contains(m.name.as_str()) && !referenced.contains(qualified.as_str()) {
                out.push(Note {
                    item: m.name.clone(),
                    text: format!("measure {} is referenced by no metric", m.name),
                });
            }
        }
    }
    if def.model_names.len() >= 2 && !def.relationships.is_empty() {
        let reached: BTreeSet<&str> = def
            .relationships
            .iter()
            .flat_map(|r| [r.left_table.as_str(), r.right_table.as_str()])
            .collect();
        let carrying: BTreeSet<&str> = def
            .measures
            .iter()
            .map(|m| m.model.as_str())
            .chain(def.dimensions.iter().map(|d| d.model.as_str()))
            .collect();
        for name in &def.model_names {
            if carrying.contains(name.as_str()) && !reached.contains(name.as_str()) {
                out.push(Note {
                    item: name.clone(),
                    text: format!("model {name} is reached by no relationship"),
                });
            }
        }
    }
    for v in &def.verifications {
        if v.by.is_none() {
            out.push(Note {
                item: v.item.clone(),
                text: format!("verified query {} records no verified_by", v.item),
            });
        }
        if v.at.is_none() {
            out.push(Note {
                item: v.item.clone(),
                text: format!("verified query {} records no verified_at", v.item),
            });
        }
    }
    for q in &def.unverifiable_questions {
        out.push(Note {
            item: q.clone(),
            text: format!("sample question {q} carries no stored answer"),
        });
    }
    for pattern in &def.unmatched_includes {
        out.push(Note {
            item: pattern.clone(),
            text: format!("include pattern {pattern} matched no file in the provided set"),
        });
    }
    for context in &def.ai_contexts {
        if context.chars >= 2_000 {
            out.push(Note {
                item: context.owner.clone(),
                text: format!(
                    "ai_context on {} runs {} characters, at or past the 2000 character limit Cube truncates silently",
                    context.owner, context.chars
                ),
            });
        }
    }
    for m in &def.metrics {
        if m.label.is_none() {
            out.push(Note {
                item: m.name.clone(),
                text: format!("metric {} carries no label", m.name),
            });
        }
    }
    for m in &def.measures {
        if m.label.is_none() {
            out.push(Note {
                item: m.name.clone(),
                text: format!("measure {} carries no label", m.name),
            });
        }
    }
    for d in &def.dimensions {
        if d.label.is_none() {
            out.push(Note {
                item: d.name.clone(),
                text: format!("dimension {} carries no label", d.name),
            });
        }
    }
    if !def.aliases.is_empty() {
        let claimed = |name: &str| {
            def.aliases.iter().any(|a| {
                a.owner == name
                    || a.owner.ends_with(&format!(" {name}"))
                    || a.owner.ends_with(&format!(".{name}"))
            })
        };
        for m in &def.metrics {
            if !claimed(&m.name) {
                out.push(Note {
                    item: m.name.clone(),
                    text: format!("metric {} carries no synonyms", m.name),
                });
            }
        }
        for m in &def.measures {
            if !claimed(&m.name) {
                out.push(Note {
                    item: m.name.clone(),
                    text: format!("measure {} carries no synonyms", m.name),
                });
            }
        }
        for d in &def.dimensions {
            if !claimed(&d.name) {
                out.push(Note {
                    item: d.name.clone(),
                    text: format!("dimension {} carries no synonyms", d.name),
                });
            }
        }
    }
    out.sort_by(|a, b| a.item.cmp(&b.item).then(a.text.cmp(&b.text)));
    out
}

/// Run all semantic-layer grading families and return findings in deterministic order.
pub fn grade(def: &Definition) -> Vec<Finding> {
    let mut findings = check_dangling(def);
    findings.extend(check_collisions(def));
    findings.extend(check_currency(def));
    findings.extend(check_relationships(def));
    findings.extend(check_join_paths(def));
    findings.extend(check_ground_truth(def));
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
