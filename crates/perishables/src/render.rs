//! Zero-dependency report serializers for `text`, `json`, `sarif`, and `github` output formats.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use perishables_core::{Finding, InfoNote, TierReport};
use perishables_semantic::Definition;

use crate::execute::ComputationReport;

/// Output formats supported by `perishables` reporting commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable deterministic line-oriented text.
    Text,
    /// Canonical RFC 8259 JSON report.
    Json,
    /// OASIS SARIF 2.1.0 report for code scanning integrations.
    Sarif,
    /// GitHub Actions workflow error annotations (`::error ...`).
    Github,
}

impl OutputFormat {
    /// Parse a `--format` value (`text`, `json`, `sarif`, or `github`).
    pub fn parse(raw: &str) -> Option<OutputFormat> {
        match raw.trim() {
            "text" => Some(OutputFormat::Text),
            "json" => Some(OutputFormat::Json),
            "sarif" => Some(OutputFormat::Sarif),
            "github" => Some(OutputFormat::Github),
            _ => None,
        }
    }
}

/// Escape a string as a RFC 8259 JSON string literal (including quotes).
pub fn json_str(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_opt_str(value: Option<&str>) -> String {
    match value {
        Some(v) => json_str(v),
        None => "null".to_string(),
    }
}

fn normalize_uri(path: &str) -> String {
    let s = path.replace('\\', "/");
    s.strip_prefix("./").unwrap_or(&s).to_string()
}

fn concept_uri(bundle_path: &str, concept_id: &str) -> String {
    let root = normalize_uri(bundle_path);
    let trimmed = root.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "." {
        format!("{concept_id}.md")
    } else {
        format!("{trimmed}/{concept_id}.md")
    }
}

fn gh_escape_data(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

fn gh_escape_prop(s: &str) -> String {
    gh_escape_data(s).replace(':', "%3A").replace(',', "%2C")
}

/// Render a SARIF 2.1.0 document from normalized `(rule_id, uri, message)` entries.
fn render_sarif(entries: &[(String, String, String)]) -> String {
    let mut rules: BTreeMap<&str, &str> = BTreeMap::new();
    for (rule_id, _, _) in entries {
        rules
            .entry(rule_id.as_str())
            .or_insert_with(|| rule_summary(rule_id));
    }
    let mut rules_json = String::new();
    for (idx, (id, desc)) in rules.iter().enumerate() {
        if idx > 0 {
            rules_json.push_str(",\n          ");
        }
        let _ = write!(
            rules_json,
            "{{\"id\": {}, \"shortDescription\": {{\"text\": {}}}}}",
            json_str(id),
            json_str(desc)
        );
    }
    let mut results_json = String::new();
    for (idx, (rule_id, uri, message)) in entries.iter().enumerate() {
        if idx > 0 {
            results_json.push_str(",\n        ");
        }
        let _ = write!(
            results_json,
            "{{\"ruleId\": {}, \"level\": \"error\", \"message\": {{\"text\": {}}}, \"locations\": [{{\"physicalLocation\": {{\"artifactLocation\": {{\"uri\": {}}}}}}}]}}",
            json_str(rule_id),
            json_str(message),
            json_str(uri)
        );
    }
    let rules_block = if rules_json.is_empty() {
        "[]".to_string()
    } else {
        format!("[\n          {rules_json}\n        ]")
    };
    let results_block = if results_json.is_empty() {
        "[]".to_string()
    } else {
        format!("[\n        {results_json}\n      ]")
    };
    format!(
        "{{\n  \"$schema\": \"https://json.schemastore.org/sarif-2.1.0.json\",\n  \"version\": \"2.1.0\",\n  \"runs\": [\n    {{\n      \"tool\": {{\n        \"driver\": {{\n          \"name\": \"perishables\",\n          \"version\": {},\n          \"informationUri\": \"https://github.com/david-engelmann/perishables\",\n          \"rules\": {}\n        }}\n      }},\n      \"results\": {}\n    }}\n  ]\n}}\n",
        json_str(env!("CARGO_PKG_VERSION")),
        rules_block,
        results_block
    )
}

fn rule_summary(code: &str) -> &'static str {
    match code {
        "LAPSED_VERIFICATION" => "Verification timestamp predates last concept generation",
        "MALFORMED_VERIFICATION" => "Verification event is missing actor or parseable timestamp",
        "DANGLING_SUPERSEDED_BY" => "Supersession chain points to a missing or external target",
        "SUPERSESSION_CYCLE" => "Supersession chain revisits a concept or exceeds hop limit",
        "SUPERSEDED_WITHOUT_DEPRECATED" => {
            "Concept declares superseded_by without deprecated status"
        }
        "STALE_BUT_STABLE" => "Concept stale_after instant has passed while status remains stable",
        "UNPARSEABLE_STALE_AFTER" => "Concept stale_after field cannot be parsed as RFC 3339",
        "UNRESOLVED_CITATION" => "Concept body cites a footnote label missing from sources",
        "MISSING_SOURCE" => "Source entry references an in-bundle file that does not exist",
        "ATTESTED_RESULT_MISMATCH" => {
            "Recomputed result disagrees with attested claim in concept body"
        }
        "SOURCE_DRIFT" => "Source file bytes disagree with recorded SHA-256 hash",
        "UNATTESTED_CLAIM" => "Concept claims an attested result without a verifiable computation",
        "DANGLING_METRIC_REFERENCE" => {
            "Semantic metric, view, or hierarchy references a missing field or measure"
        }
        "NAME_COLLISION" => {
            "Two or more semantic definitions claim the same display name or synonym"
        }
        "LAPSED_VERIFIED_QUERY" => "Verified query predates the semantic definition change instant",
        "DANGLING_RELATIONSHIP" => "Relationship or join names a missing model, table, or view",
        "BENCHMARK_WITHOUT_ANSWER" => "Stored ground-truth benchmark is missing question or SQL",
        "AMBIGUOUS_JOIN_PATH" => {
            "Models connect over multiple join routes with conflicting aggregation grains"
        }
        _ => "Perishables trust or semantic integrity finding",
    }
}

/// Render an `audit` report in the requested [`OutputFormat`].
#[allow(clippy::too_many_arguments)]
pub fn render_audit(
    format: OutputFormat,
    bundle_path: &str,
    as_of: &str,
    tiers: &[TierReport],
    computations: &[ComputationReport],
    notes: &[InfoNote],
    findings: &[Finding],
    include_timing: bool,
) -> String {
    match format {
        OutputFormat::Text => {
            let mut out = String::new();
            let _ = writeln!(
                out,
                "BUNDLE {} as-of {} concepts={}",
                bundle_path,
                as_of,
                tiers.len()
            );
            for tier in tiers {
                let _ = writeln!(
                    out,
                    "  TIER {} raw={} recomputed={}",
                    tier.concept_id,
                    tier.raw_tier.name(),
                    tier.recomputed_tier.name()
                );
            }
            for report in computations {
                let mut line = format!(
                    "  COMPUTATION {} {}",
                    report.concept_id,
                    report.verdict.name()
                );
                if let Some(reason) = report.verdict.reason() {
                    let _ = write!(line, " reason={reason}");
                }
                if let Some(hash) = &report.artifact_hash {
                    let _ = write!(line, " artifact={hash}");
                }
                if include_timing && let Some(elapsed) = report.elapsed_ms {
                    let _ = write!(line, " elapsed_ms={elapsed}");
                }
                if let Some(timeout) = report.timeout_ms {
                    let _ = write!(line, " timeout_ms={timeout}");
                }
                let _ = writeln!(out, "{line}");
            }
            for note in notes {
                let _ = writeln!(out, "  NOTE {} {}", note.concept_id, note.text);
            }
            for finding in findings {
                let _ = writeln!(
                    out,
                    "  FINDING {} {} {}",
                    finding.concept_id,
                    finding.code.label(),
                    finding.detail
                );
            }
            let _ = writeln!(
                out,
                "RESULT {} findings={}",
                if findings.is_empty() { "PASS" } else { "FAIL" },
                findings.len()
            );
            out
        }
        OutputFormat::Json => {
            let mut out = String::new();
            let result = if findings.is_empty() { "PASS" } else { "FAIL" };
            let _ = writeln!(out, "{{");
            let _ = writeln!(out, "  \"schema_version\": \"1\",");
            let _ = writeln!(out, "  \"tool\": \"perishables\",");
            let _ = writeln!(
                out,
                "  \"tool_version\": {},",
                json_str(env!("CARGO_PKG_VERSION"))
            );
            let _ = writeln!(out, "  \"command\": \"audit\",");
            let _ = writeln!(out, "  \"bundle\": {},", json_str(bundle_path));
            let _ = writeln!(out, "  \"as_of\": {},", json_str(as_of));
            let _ = writeln!(out, "  \"result\": {},", json_str(result));
            let _ = writeln!(out, "  \"concepts_count\": {},", tiers.len());

            out.push_str("  \"tiers\": [");
            for (i, t) in tiers.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let _ = write!(
                    out,
                    "\n    {{\"concept_id\": {}, \"raw_tier\": {}, \"recomputed_tier\": {}}}",
                    json_str(&t.concept_id),
                    json_str(t.raw_tier.name()),
                    json_str(t.recomputed_tier.name())
                );
            }
            if !tiers.is_empty() {
                out.push_str("\n  ");
            }
            out.push_str("],\n");

            out.push_str("  \"computations\": [");
            for (i, c) in computations.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let timeout_str = c
                    .timeout_ms
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "null".to_string());
                let timing_field = if include_timing {
                    let val = c
                        .elapsed_ms
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "null".to_string());
                    format!(", \"elapsed_ms\": {val}")
                } else {
                    String::new()
                };
                let _ = write!(
                    out,
                    "\n    {{\"concept_id\": {}, \"verdict\": {}, \"reason\": {}, \"claimed\": {}, \"recomputed\": {}, \"artifact_hash\": {}, \"timeout_ms\": {timeout_str}{timing_field}}}",
                    json_str(&c.concept_id),
                    json_str(c.verdict.name()),
                    json_opt_str(c.verdict.reason()),
                    json_opt_str(c.claimed.as_deref()),
                    json_opt_str(c.recomputed.as_deref()),
                    json_opt_str(c.artifact_hash.as_deref()),
                );
            }
            if !computations.is_empty() {
                out.push_str("\n  ");
            }
            out.push_str("],\n");

            out.push_str("  \"notes\": [");
            for (i, n) in notes.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let _ = write!(
                    out,
                    "\n    {{\"concept_id\": {}, \"text\": {}}}",
                    json_str(&n.concept_id),
                    json_str(&n.text)
                );
            }
            if !notes.is_empty() {
                out.push_str("\n  ");
            }
            out.push_str("],\n");

            out.push_str("  \"findings\": [");
            for (i, f) in findings.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let _ = write!(
                    out,
                    "\n    {{\"concept_id\": {}, \"code\": {}, \"detail\": {}}}",
                    json_str(&f.concept_id),
                    json_str(f.code.label()),
                    json_str(&f.detail)
                );
            }
            if !findings.is_empty() {
                out.push_str("\n  ");
            }
            out.push_str("]\n}\n");
            out
        }
        OutputFormat::Sarif => {
            let entries: Vec<(String, String, String)> = findings
                .iter()
                .map(|f| {
                    (
                        f.code.label().to_string(),
                        concept_uri(bundle_path, &f.concept_id),
                        format!("{}: {}", f.concept_id, f.detail),
                    )
                })
                .collect();
            render_sarif(&entries)
        }
        OutputFormat::Github => {
            let mut out = String::new();
            for f in findings {
                let file = concept_uri(bundle_path, &f.concept_id);
                let _ = writeln!(
                    out,
                    "::error file={},title={}::{} {}",
                    gh_escape_prop(&file),
                    gh_escape_prop(f.code.label()),
                    gh_escape_data(&f.concept_id),
                    gh_escape_data(&f.detail)
                );
            }
            out
        }
    }
}

/// Render a `drift` report in the requested [`OutputFormat`].
pub fn render_drift(format: OutputFormat, bundle_path: &str, findings: &[Finding]) -> String {
    match format {
        OutputFormat::Text => {
            let mut out = String::new();
            for finding in findings {
                let _ = writeln!(
                    out,
                    "  FINDING {} {} {}",
                    finding.concept_id,
                    finding.code.label(),
                    finding.detail
                );
            }
            let _ = writeln!(
                out,
                "DRIFT {} findings={}",
                if findings.is_empty() { "NONE" } else { "FOUND" },
                findings.len()
            );
            out
        }
        OutputFormat::Json => {
            let mut out = String::new();
            let result = if findings.is_empty() { "NONE" } else { "FOUND" };
            let _ = writeln!(out, "{{");
            let _ = writeln!(out, "  \"schema_version\": \"1\",");
            let _ = writeln!(out, "  \"tool\": \"perishables\",");
            let _ = writeln!(
                out,
                "  \"tool_version\": {},",
                json_str(env!("CARGO_PKG_VERSION"))
            );
            let _ = writeln!(out, "  \"command\": \"drift\",");
            let _ = writeln!(out, "  \"bundle\": {},", json_str(bundle_path));
            let _ = writeln!(out, "  \"result\": {},", json_str(result));
            out.push_str("  \"findings\": [");
            for (i, f) in findings.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let _ = write!(
                    out,
                    "\n    {{\"concept_id\": {}, \"code\": {}, \"detail\": {}}}",
                    json_str(&f.concept_id),
                    json_str(f.code.label()),
                    json_str(&f.detail)
                );
            }
            if !findings.is_empty() {
                out.push_str("\n  ");
            }
            out.push_str("]\n}\n");
            out
        }
        OutputFormat::Sarif => {
            let entries: Vec<(String, String, String)> = findings
                .iter()
                .map(|f| {
                    (
                        f.code.label().to_string(),
                        concept_uri(bundle_path, &f.concept_id),
                        format!("{}: {}", f.concept_id, f.detail),
                    )
                })
                .collect();
            render_sarif(&entries)
        }
        OutputFormat::Github => {
            let mut out = String::new();
            for f in findings {
                let file = concept_uri(bundle_path, &f.concept_id);
                let _ = writeln!(
                    out,
                    "::error file={},title={}::{} {}",
                    gh_escape_prop(&file),
                    gh_escape_prop(f.code.label()),
                    gh_escape_data(&f.concept_id),
                    gh_escape_data(&f.detail)
                );
            }
            out
        }
    }
}

/// Render a cross-ecosystem semantic grading report in the requested [`OutputFormat`].
pub fn render_grade(
    format: OutputFormat,
    tag: &str,
    ecosystem: &str,
    primary_file: &str,
    def: &Definition,
    findings: &[perishables_semantic::Finding],
    notes: &[perishables_semantic::Note],
) -> String {
    match format {
        OutputFormat::Text => {
            let mut out = String::new();
            for finding in findings {
                let _ = writeln!(
                    out,
                    "FINDING {} {} {}",
                    finding.code.label(),
                    finding.item,
                    finding.detail
                );
            }
            for note in notes {
                let _ = writeln!(out, "NOTE {} {}", note.item, note.text);
            }
            let _ = writeln!(
                out,
                "{tag} models={} measures={} dimensions={} metrics={} findings={}",
                def.model_names.len(),
                def.measures.len(),
                def.dimensions.len(),
                def.metrics.len(),
                findings.len()
            );
            out
        }
        OutputFormat::Json => {
            let mut out = String::new();
            let result = if findings.is_empty() { "PASS" } else { "FAIL" };
            let _ = writeln!(out, "{{");
            let _ = writeln!(out, "  \"schema_version\": \"1\",");
            let _ = writeln!(out, "  \"tool\": \"perishables\",");
            let _ = writeln!(
                out,
                "  \"tool_version\": {},",
                json_str(env!("CARGO_PKG_VERSION"))
            );
            let _ = writeln!(
                out,
                "  \"command\": {},",
                json_str(&tag.to_ascii_lowercase())
            );
            let _ = writeln!(out, "  \"ecosystem\": {},", json_str(ecosystem));
            let _ = writeln!(out, "  \"result\": {},", json_str(result));
            let _ = writeln!(
                out,
                "  \"counts\": {{\"models\": {}, \"measures\": {}, \"dimensions\": {}, \"metrics\": {}}},",
                def.model_names.len(),
                def.measures.len(),
                def.dimensions.len(),
                def.metrics.len()
            );
            out.push_str("  \"findings\": [");
            for (i, f) in findings.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let _ = write!(
                    out,
                    "\n    {{\"code\": {}, \"item\": {}, \"detail\": {}}}",
                    json_str(f.code.label()),
                    json_str(&f.item),
                    json_str(&f.detail)
                );
            }
            if !findings.is_empty() {
                out.push_str("\n  ");
            }
            out.push_str("],\n  \"notes\": [");
            for (i, n) in notes.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let _ = write!(
                    out,
                    "\n    {{\"item\": {}, \"text\": {}}}",
                    json_str(&n.item),
                    json_str(&n.text)
                );
            }
            if !notes.is_empty() {
                out.push_str("\n  ");
            }
            out.push_str("]\n}\n");
            out
        }
        OutputFormat::Sarif => {
            let uri = normalize_uri(primary_file);
            let entries: Vec<(String, String, String)> = findings
                .iter()
                .map(|f| {
                    (
                        f.code.label().to_string(),
                        uri.clone(),
                        format!("{}: {}", f.item, f.detail),
                    )
                })
                .collect();
            render_sarif(&entries)
        }
        OutputFormat::Github => {
            let mut out = String::new();
            let uri = normalize_uri(primary_file);
            for f in findings {
                let _ = writeln!(
                    out,
                    "::error file={},title={}::{} {}",
                    gh_escape_prop(&uri),
                    gh_escape_prop(f.code.label()),
                    gh_escape_data(&f.item),
                    gh_escape_data(&f.detail)
                );
            }
            out
        }
    }
}

/// One graded target inside a multi-file `perishables grade` batch.
pub struct GradeBatchEntry {
    /// Summary tag (for example `GRADE-DBT`).
    pub tag: String,
    /// Ecosystem identifier (for example `dbt`).
    pub ecosystem: String,
    /// Primary file path for location reporting.
    pub primary_file: String,
    /// Normalized semantic definition.
    pub def: Definition,
    /// Gate findings emitted for this target.
    pub findings: Vec<perishables_semantic::Finding>,
    /// Informational notes emitted for this target.
    pub notes: Vec<perishables_semantic::Note>,
}

/// Render one or more graded semantic definitions in `format`.
pub fn render_grade_batch(format: OutputFormat, items: &[GradeBatchEntry]) -> String {
    if let [single] = items {
        return render_grade(
            format,
            &single.tag,
            &single.ecosystem,
            &single.primary_file,
            &single.def,
            &single.findings,
            &single.notes,
        );
    }
    match format {
        OutputFormat::Text | OutputFormat::Github => {
            let mut out = String::new();
            for item in items {
                out.push_str(&render_grade(
                    format,
                    &item.tag,
                    &item.ecosystem,
                    &item.primary_file,
                    &item.def,
                    &item.findings,
                    &item.notes,
                ));
            }
            out
        }
        OutputFormat::Sarif => {
            let mut entries = Vec::new();
            for item in items {
                let uri = normalize_uri(&item.primary_file);
                for f in &item.findings {
                    entries.push((
                        f.code.label().to_string(),
                        uri.clone(),
                        format!("{}: {}", f.item, f.detail),
                    ));
                }
            }
            render_sarif(&entries)
        }
        OutputFormat::Json => {
            let any_fail = items.iter().any(|i| !i.findings.is_empty());
            let mut out = String::new();
            let _ = writeln!(out, "{{");
            let _ = writeln!(out, "  \"schema_version\": \"1\",");
            let _ = writeln!(out, "  \"tool\": \"perishables\",");
            let _ = writeln!(
                out,
                "  \"tool_version\": {},",
                json_str(env!("CARGO_PKG_VERSION"))
            );
            let _ = writeln!(out, "  \"command\": \"grade\",");
            let _ = writeln!(
                out,
                "  \"result\": {},",
                json_str(if any_fail { "FAIL" } else { "PASS" })
            );
            out.push_str("  \"reports\": [\n");
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    out.push_str(",\n");
                }
                let sub = render_grade(
                    OutputFormat::Json,
                    &item.tag,
                    &item.ecosystem,
                    &item.primary_file,
                    &item.def,
                    &item.findings,
                    &item.notes,
                );
                for (li, line) in sub.trim_end().lines().enumerate() {
                    if li > 0 {
                        out.push('\n');
                    }
                    out.push_str("    ");
                    out.push_str(line);
                }
            }
            out.push_str("\n  ]\n}\n");
            out
        }
    }
}

/// Summary row for a single fixture in `perishables corpus`.
pub struct CorpusFixtureSummary {
    /// Fixture directory name.
    pub name: String,
    /// Corpus gate set (`gate`).
    pub set: String,
    /// Whether the fixture matched all expected findings, tiers, and immutability checks.
    pub pass: bool,
    /// Whether the fixture directory bytes were identical before and after auditing.
    pub unchanged: bool,
    /// Findings emitted for the fixture.
    pub findings: Vec<Finding>,
    /// Diagnostic failure lines when `pass` is `false`.
    pub diagnostics: Vec<String>,
}

/// Render a `corpus` run summary in `text` or `json` format.
pub fn render_corpus(
    format: OutputFormat,
    corpus_version: &str,
    as_of: &str,
    fixtures: &[CorpusFixtureSummary],
    failures: usize,
) -> String {
    match format {
        OutputFormat::Json => {
            let mut out = String::new();
            let result = if failures == 0 { "PASS" } else { "FAIL" };
            let _ = writeln!(out, "{{");
            let _ = writeln!(out, "  \"schema_version\": \"1\",");
            let _ = writeln!(out, "  \"tool\": \"perishables\",");
            let _ = writeln!(
                out,
                "  \"tool_version\": {},",
                json_str(env!("CARGO_PKG_VERSION"))
            );
            let _ = writeln!(out, "  \"command\": \"corpus\",");
            let _ = writeln!(out, "  \"corpus_version\": {},", json_str(corpus_version));
            let _ = writeln!(out, "  \"as_of\": {},", json_str(as_of));
            let _ = writeln!(out, "  \"result\": {},", json_str(result));
            let _ = writeln!(out, "  \"fixtures_count\": {},", fixtures.len());
            let _ = writeln!(out, "  \"failures_count\": {failures},");
            out.push_str("  \"fixtures\": [");
            for (i, fx) in fixtures.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let mut codes_json = String::from("[");
                for (j, f) in fx.findings.iter().enumerate() {
                    if j > 0 {
                        codes_json.push_str(", ");
                    }
                    codes_json.push_str(&json_str(f.code.label()));
                }
                codes_json.push(']');
                let _ = write!(
                    out,
                    "\n    {{\"name\": {}, \"set\": {}, \"status\": {}, \"unchanged\": {}, \"findings\": {codes_json}}}",
                    json_str(&fx.name),
                    json_str(&fx.set),
                    json_str(if fx.pass { "PASS" } else { "FAIL" }),
                    fx.unchanged
                );
            }
            if !fixtures.is_empty() {
                out.push_str("\n  ");
            }
            out.push_str("]\n}\n");
            out
        }
        _ => {
            let mut out = String::new();
            let _ = writeln!(
                out,
                "CORPUS version={} auditor={} as-of={}",
                corpus_version,
                env!("CARGO_PKG_VERSION"),
                as_of
            );
            for fx in fixtures {
                let _ = writeln!(
                    out,
                    "FIXTURE {} set={} {} findings={} unchanged={}",
                    fx.name,
                    fx.set,
                    if fx.pass { "PASS" } else { "FAIL" },
                    fx.findings.len(),
                    fx.unchanged
                );
                for finding in &fx.findings {
                    let _ = writeln!(
                        out,
                        "  FINDING {} {} {}",
                        finding.concept_id,
                        finding.code.label(),
                        finding.detail
                    );
                }
                for diag in &fx.diagnostics {
                    let _ = writeln!(out, "  {diag}");
                }
            }
            let _ = writeln!(
                out,
                "CORPUS fixtures={} failures={failures}",
                fixtures.len()
            );
            out
        }
    }
}
