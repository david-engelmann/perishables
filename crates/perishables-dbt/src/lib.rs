//! Cross ecosystem grading for semantic layer definitions.
//!
//! This crate is the adapter from the cross ecosystem design
//! record at docs/DESIGN-CROSS-ECOSYSTEM.md. Each ecosystem
//! gets a reader that normalizes its definition into one
//! grading model, and the rule families below run over the
//! model, format blind. The crate keeps the name
//! perishables-dbt from its first slice. The name is
//! historical, and renaming it is a separate change.
//!
//! The readers. dbt semantic manifests. Apache Ossie
//! documents, JSON and YAML. Snowflake semantic views.
//! Databricks Genie serialized spaces. Cube models, meta
//! JSON and YAML model files. Looker models, API documents
//! and native LookML files with project scope. AtScale
//! model exports. Each reader's own module states its
//! mapping and its boundary.
//!
//! The families. Structural integrity grades references,
//! a metric built on a measure that does not exist, a
//! relationship or join naming a missing table or view, a
//! hierarchy level on a missing attribute. Discoverability
//! grades names and coverage, a display name or synonym
//! claimed by two definitions is a finding, and missing
//! descriptions and kindred gaps are notes, information and
//! never findings. Currency grades verification events
//! against the definition's last change, where the platform
//! records them. Ground truth shape grades stored question
//! and answer pairs for completeness. Running that ground
//! truth belongs to the vendor runtime and is outside every
//! reader here.
//!
//! The build history lives in the round records under
//! docs/, one per slice, and each reader module carries its
//! own slice notes where they matter.
//!
//! The four locked OKF calls in perishables-core are untouched
//! by this crate. Cross ecosystem finding codes are a separate
//! closed set, versioned with this crate.

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

pub use atscale::read_atscale_definition;
pub use cube::{read_cube_definition, read_cube_js, read_cube_yaml};
pub use databricks::read_databricks_definition;
pub use json::Json;
pub use looker::read_looker_definition;
pub use lookml::{Item, Value, parse_tree, read_lookml_definition, read_lookml_project};
pub use ossie::{read_ossie_definition, read_ossie_yaml};
pub use snowflake::read_snowflake_definition;

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

/// One verification event recorded inside a definition. A
/// Snowflake verified query carries who verified it and when,
/// as a free text name and an epoch second, which is a weaker
/// claim than an OKF actor scheme and grades accordingly. See
/// the currency check.
#[derive(Debug, Clone, PartialEq)]
pub struct Verification {
    /// The verified item, a query name for Snowflake.
    pub item: String,
    pub by: Option<String>,
    pub at: Option<i64>,
}

/// One declared relationship between two tables in a
/// definition, the join structure the structural family walks.
#[derive(Debug, Clone, PartialEq)]
pub struct Relationship {
    pub name: String,
    pub left_table: String,
    pub right_table: String,
}

/// One stored ground truth pair, a question and the answer
/// the platform is claimed to produce for it. Databricks
/// benchmarks and Snowflake verified queries both store these.
/// Running them is the behavioral family and needs the vendor
/// runtime. This model records the claim itself, so its shape
/// can be graded offline.
#[derive(Debug, Clone, PartialEq)]
pub struct GroundTruth {
    /// The pair's identifier, a benchmark id where the
    /// platform gives one.
    pub item: String,
    pub question: String,
    pub expected_sql: String,
}

/// One ai_context block recorded on an item, by character
/// count. Cube truncates ai_context at 2,000 characters
/// silently, so the count itself is a gradeable stated fact.
/// The text is not carried, because nothing in the families
/// reads prose.
#[derive(Debug, Clone, PartialEq)]
pub struct AiContext {
    pub owner: String,
    pub chars: usize,
}

/// The normalized grading model for one manifest.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Definition {
    pub model_names: Vec<String>,
    pub measures: Vec<Measure>,
    pub dimensions: Vec<Dimension>,
    pub metrics: Vec<Metric>,
    /// Alias claims from ecosystems that record synonyms. The
    /// dbt reader leaves this empty, because native dbt has
    /// no synonym field. The Ossie, Snowflake, and Looker
    /// readers fill it from their synonym surfaces.
    pub aliases: Vec<Alias>,
    /// Verification events the definition records. Only the
    /// Snowflake reader fills this.
    pub verifications: Vec<Verification>,
    /// Declared relationships between tables. The Snowflake,
    /// Cube, Looker, LookML, and Ossie readers fill this.
    pub relationships: Vec<Relationship>,
    /// The instant the definition last changed, where the
    /// caller knows it. No vendor stores this inside the
    /// definition, so it arrives from outside, object metadata
    /// for Snowflake, and the currency check grades against it
    /// only when it is present.
    pub changed_at: Option<i64>,
    /// Stored ground truth pairs the definition carries.
    /// The Databricks and Snowflake readers fill this, the
    /// two platforms that store ground truth in the
    /// definition.
    pub ground_truth: Vec<GroundTruth>,
    /// Questions the definition offers with no stored answer,
    /// Genie sample questions. They are coverage
    /// observations, because the platform never claimed an
    /// answer for them.
    pub unverifiable_questions: Vec<String>,
    /// ai_context blocks recorded on items, by owner and
    /// character count. The Cube readers fill this.
    pub ai_contexts: Vec<AiContext>,
    /// Attribute names a reference may resolve against, in
    /// the qualified form the referencing reader uses.
    /// AtScale hierarchy levels reference dimension
    /// attributes, which are neither measures nor metrics.
    pub attributes: Vec<String>,
    /// Qualified field names, view.field, for readers whose
    /// references are qualified. A dotted reference resolves
    /// only against this set. The LookML project reader and
    /// the Cube JS reader fill it.
    pub qualified_fields: Vec<String>,
    /// Include patterns from a project file that matched no
    /// file in the provided set. Coverage observations, since
    /// a partial set cannot prove the file absent.
    pub unmatched_includes: Vec<String>,
}

/// The closed set of cross ecosystem finding codes this adapter
/// emits. A code once published never changes meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Code {
    DanglingMetricReference,
    NameCollision,
    LapsedVerifiedQuery,
    DanglingRelationship,
    BenchmarkWithoutAnswer,
    AmbiguousJoinPath,
}

impl Code {
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
/// Two dbt surfaces the design record names are not read here,
/// stated as the boundary. Meta dictionaries on models and
/// metrics, and freshness, which arrives from git provenance
/// and source freshness artifacts rather than from the manifest
/// itself, are later surfaces. This reader consumes the
/// manifest's own stated facts.
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

/// Push one alias claim per synonym in a synonyms value,
/// shared by the readers whose formats carry synonym lists.
pub(crate) fn push_synonym_aliases(synonyms: &Json, owner: &str, aliases: &mut Vec<Alias>) {
    for term in synonyms.as_str_list() {
        aliases.push(Alias {
            term,
            owner: owner.to_string(),
        });
    }
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
    let measure_names: BTreeSet<&str> = def.measures.iter().map(|m| m.name.as_str()).collect();
    let dimension_names: BTreeSet<&str> =
        def.dimensions.iter().map(|d| d.name.as_str()).collect();
    let metric_names: BTreeSet<&str> = def.metrics.iter().map(|m| m.name.as_str()).collect();
    let qualified: BTreeSet<&str> = def.qualified_fields.iter().map(String::as_str).collect();
    let attributes: BTreeSet<&str> = def.attributes.iter().map(String::as_str).collect();
    let mut findings = Vec::new();
    // A measure reference resolves against the model's
    // field set. Measures in the dbt sense, and dimensions
    // too, because a LookML measure's SQL references fields
    // of either kind and the reference is a field reference.
    // A dotted reference is a cross view reference and
    // resolves only against the qualified field set a
    // project scoped reader supplies.
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
                let kind = if r.contains('.') { "attribute" } else { "metric" };
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

/// Verification currency, the OKF currency rule in vendor
/// dress. A verification is current when it stands at or after
/// the last change to the definition it verifies, and lapsed
/// when it predates that change. The check grades only when
/// both instants are known. A verification with no timestamp
/// cannot be placed against the change and is a coverage note,
/// never a lapse, and a definition whose change instant the
/// caller did not supply grades nothing here.
pub fn check_currency(def: &Definition) -> Vec<Finding> {
    let mut findings = Vec::new();
    if let Some(changed_at) = def.changed_at {
        for v in &def.verifications {
            if let Some(at) = v.at
                && at < changed_at
            {
                findings.push(Finding {
                    code: Code::LapsedVerifiedQuery,
                    item: v.item.clone(),
                    detail: format!(
                        "verified query {} was verified at {at}, before the definition last changed at {changed_at}",
                        v.item
                    ),
                });
            }
        }
    }
    sort_findings(&mut findings);
    findings
}

/// Structural integrity for declared relationships. Each side
/// of a relationship must name a table the definition carries.
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

/// Join path ambiguity, the fan trap. Where two routes
/// connect the same pair of models at different lengths, a
/// query can travel either and the routes aggregate at
/// different grains, which is the wrong answer the vendor
/// cookbooks name first. The check builds the model graph
/// from declared relationships and, for every directly
/// connected pair, looks for an alternative route of two or
/// three edges that avoids the direct edge. One finding per
/// ambiguous pair, naming both routes.
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

/// One alternative route between two models, as the
/// intermediate models joined by arrows, over paths of two
/// or three edges that never use the direct edge.
fn alternative_route(
    adjacency: &BTreeMap<&str, Vec<&str>>,
    a: &str,
    b: &str,
) -> Option<String> {
    let empty: Vec<&str> = Vec::new();
    for mid1 in adjacency.get(a).unwrap_or(&empty) {
        if *mid1 == b || *mid1 == a {
            continue;
        }
        if adjacency
            .get(mid1)
            .unwrap_or(&empty)
            .contains(&b)
        {
            return Some(format!("{mid1}"));
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

/// Ground truth shape. A stored benchmark is a claim that
/// the platform answers this question with this SQL. A pair
/// with an empty question or an empty answer is a claim with
/// nothing behind it, and no behavioral run could ever grade
/// it, which is a defect in the record rather than a coverage
/// gap.
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
    // The orphan observation only makes sense where metrics
    // reference measures by name, which is the dbt shape. In
    // ecosystems whose metrics are expressions, a measure no
    // metric names is normal, and the note stays silent.
    let name_linked = def.metrics.iter().any(|m| !m.measure_refs.is_empty());
    if name_linked {
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
            text: format!(
                "include pattern {pattern} matched no file in the provided set"
            ),
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
    out.sort_by(|a, b| a.item.cmp(&b.item).then(a.text.cmp(&b.text)));
    out
}

/// Every finding this adapter produces, sorted by code then
/// item then detail, deterministic for a given definition.
/// The individual check functions are public as the family
/// level API, for callers who grade one family at a time.
/// This function is their composition.
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
