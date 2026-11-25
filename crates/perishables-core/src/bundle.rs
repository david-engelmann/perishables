//! Loading a bundle from disk into the concepts the checks read.
//!
//! Loading never writes. Files are read in sorted path order so a
//! bundle always loads identically. Reserved files (`index.md` and
//! `log.md`), symlinks, and hidden paths are not concepts. A Markdown
//! file is a concept when its frontmatter parses and carries a `type`.
//! A bundle root that cannot be read is an error, never an empty
//! bundle, because an auditor that reports a clean pass over a bundle
//! it could not read has failed at its only job.

use std::fs;
use std::path::{Path, PathBuf};

use crate::frontmatter::{
    Value, body_of, citation_labels, claimed_result, code_blocks, parse_frontmatter,
};
use crate::time::parse_timestamp;

/// One entry of a concept `sources` list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRef {
    /// The source id, when the entry carries one.
    pub id: Option<String>,
    /// The resource the entry points at, when it carries one.
    pub resource: Option<String>,
    /// A recorded content hash for the resource, when the entry
    /// carries one under the `sha256` key.
    pub sha256: Option<String>,
}

/// One declared parameter of an Attested Computation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    /// The parameter name.
    pub name: String,
    /// The declared parameter type, such as `string`, `integer`,
    /// `number`, or `boolean`, when specified in frontmatter.
    pub param_type: Option<String>,
    /// The declared value, when the bundle binds one.
    pub value: Option<String>,
}

/// One verification event as recorded in frontmatter, before any
/// judgement about its currency or shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedEvent {
    /// The actor string, such as `human:ahormati`, when present.
    pub by: Option<String>,
    /// The raw timestamp text, when present.
    pub at_text: Option<String>,
    /// The parsed timestamp, when the text parses.
    pub at: Option<i64>,
}

impl VerifiedEvent {
    /// Whether this event has an actor and a parseable timestamp.
    pub fn is_well_formed(&self) -> bool {
        self.by.is_some() && self.at.is_some()
    }

    /// Why this event is malformed, in plain words.
    pub fn malformed_reason(&self) -> Option<String> {
        if self.by.is_none() {
            return Some("verification event has no by actor".to_string());
        }
        match (&self.at_text, &self.at) {
            (None, _) => Some("verification event has no at timestamp".to_string()),
            (Some(text), None) => Some(format!("verification timestamp {text} does not parse")),
            _ => None,
        }
    }

    /// Whether this event uses the `human:` actor scheme, the only
    /// scheme that qualifies for the `human-reviewed` tier.
    pub fn is_human(&self) -> bool {
        self.by
            .as_deref()
            .is_some_and(|by| by.starts_with("human:"))
    }
}

/// One concept and the trust fields the checks read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Concept {
    /// Bundle-relative path without the `.md` suffix, POSIX style.
    pub id: String,
    /// The concept type, such as `Metric` or `Attested Computation`.
    pub concept_type: String,
    /// The lifecycle status. Absent status means `stable`.
    pub status: String,
    /// The raw `superseded_by` value, when present.
    pub superseded_by: Option<String>,
    /// The parsed `generated.at`, when present and parseable.
    pub generated_at: Option<i64>,
    /// The raw `generated.at` text, when present.
    pub generated_text: Option<String>,
    /// The raw `stale_after` text, when present.
    pub stale_after_text: Option<String>,
    /// The parsed `stale_after`, when present and parseable.
    pub stale_after: Option<i64>,
    /// Recorded verification events, in document order.
    pub verified: Vec<VerifiedEvent>,
    /// The concept's recorded sources.
    pub sources: Vec<SourceRef>,
    /// The footnote labels the concept body cites.
    pub citations: Vec<String>,
    /// The declared runtime, for `Attested Computation` concepts.
    pub runtime: Option<String>,
    /// The declared parameters, for `Attested Computation` concepts.
    pub parameters: Vec<Parameter>,
    /// The fenced code blocks of the body, `(language, code)`.
    pub code_blocks: Vec<(String, String)>,
    /// The text of a computation file the body links to, when the
    /// computation lives in a file rather than inline.
    pub file_code: Option<String>,
    /// The attested result the body claims, when it states one.
    pub claimed_result: Option<String>,
}

/// A loaded bundle.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bundle {
    /// Concepts in sorted id order.
    pub concepts: Vec<Concept>,
    /// The `okf_version` declared by the bundle index, when any.
    pub okf_version: Option<String>,
    /// Every file in the bundle as a POSIX-relative path, when the
    /// bundle was loaded from disk. In-memory bundles carry `None`,
    /// and checks that need the inventory skip file existence checks.
    pub files: Option<std::collections::BTreeSet<String>>,
    /// The bytes of in-bundle sources that record a content hash,
    /// keyed by bundle-relative path.
    pub source_bytes: std::collections::BTreeMap<String, Vec<u8>>,
    /// Explicit source modification instants supplied by a caller
    /// (for example from version-control commit timestamps), as epoch
    /// seconds keyed by bundle-relative path. Disk loads leave this
    /// map empty so host filesystem `mtime` never alters audit output.
    pub source_modified: std::collections::BTreeMap<String, i64>,
}

/// Classification of a `sources.resource` or path field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResourceTarget {
    /// External URI carrying an RFC 3986 scheme (such as `https://` or `s3://`).
    ExternalUri,
    /// Valid normalized POSIX path strictly within the bundle root.
    InBundle(String),
    /// Empty path, Windows drive path, or relative `..` path escaping the bundle root.
    InvalidOrEscapes,
}

/// Whether a value carries an external URI scheme (`https:`, `s3:`,
/// `mailto:`, `urn:`). Single-letter Windows drive prefixes (`C:\` or
/// `C:/`) are local filesystem paths rather than URI schemes.
pub(crate) fn has_uri_scheme(raw: &str) -> bool {
    let Some(colon) = raw.find(':') else {
        return false;
    };
    let scheme = &raw[..colon];
    if scheme.len() < 2 || scheme.contains(['/', '\\']) {
        return false;
    }
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
}

/// Classify a path or resource string relative to `from_concept`.
pub(crate) fn classify_resource_path(from_concept: &str, raw: &str) -> ResourceTarget {
    let raw = raw.trim();
    if raw.is_empty() {
        return ResourceTarget::InvalidOrEscapes;
    }
    if has_uri_scheme(raw) {
        return ResourceTarget::ExternalUri;
    }
    // Reject single-letter Windows drive paths such as `C:\secret.csv`.
    if raw.len() >= 2 && raw.as_bytes()[0].is_ascii_alphabetic() && raw.as_bytes()[1] == b':' {
        return ResourceTarget::InvalidOrEscapes;
    }
    let normalized = raw.replace('\\', "/");
    let stripped = normalized.strip_prefix('/').unwrap_or(&normalized);
    let combined = if normalized.starts_with('/') {
        stripped.to_string()
    } else {
        let parent = from_concept.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        if parent.is_empty() {
            stripped.to_string()
        } else {
            format!("{parent}/{stripped}")
        }
    };
    let mut parts: Vec<&str> = Vec::new();
    for segment in combined.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return ResourceTarget::InvalidOrEscapes;
                }
            }
            other => parts.push(other),
        }
    }
    if parts.is_empty() {
        ResourceTarget::InvalidOrEscapes
    } else {
        ResourceTarget::InBundle(parts.join("/"))
    }
}

/// Resolve a path field to a bundle-relative file path. External URIs
/// and paths that escape the bundle return `None`.
pub(crate) fn resolve_file_path(from_concept: &str, raw: &str) -> Option<String> {
    match classify_resource_path(from_concept, raw) {
        ResourceTarget::InBundle(path) => Some(path),
        ResourceTarget::ExternalUri | ResourceTarget::InvalidOrEscapes => None,
    }
}

impl Bundle {
    /// Whether the SPEC 12.1 fallbacks apply to this bundle. They
    /// apply when the bundle declares version `0.2` or earlier, or
    /// declares no version at all. A declared version that does not
    /// parse is treated as current, never as an old version.
    pub fn uses_v02_fallback(&self) -> bool {
        match &self.okf_version {
            None => true,
            Some(version) => {
                let mut parts = version.split('.');
                let (Some(Ok(major)), Some(Ok(minor))) = (
                    parts.next().map(|p| p.parse::<u32>()),
                    parts.next().map(|p| p.parse::<u32>()),
                ) else {
                    return false;
                };
                (major, minor) <= (0, 2)
            }
        }
    }

    /// Load every concept under a bundle root directory.
    pub fn load(root: &Path) -> std::io::Result<Bundle> {
        let okf_version = read_okf_version(root);
        let mut inventory = Vec::new();
        collect_files(root, &mut inventory)?;
        inventory.sort();
        let files: std::collections::BTreeSet<String> = inventory
            .iter()
            .map(|path| {
                path.strip_prefix(root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        let mut markdown: Vec<&PathBuf> = inventory
            .iter()
            .filter(|path| {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                name.ends_with(".md") && name != "index.md" && name != "log.md"
            })
            .collect();
        markdown.sort();
        let mut concepts = Vec::new();
        for path in markdown {
            let text = fs::read_to_string(path)?;
            let Some(fm) = parse_frontmatter(&text) else {
                continue;
            };
            if fm.get("type").and_then(Value::as_str).is_none() {
                continue;
            }
            let rel = path.strip_prefix(root).unwrap_or(path);
            let id = to_posix_id(rel);
            concepts.push(concept_from_frontmatter(id, &fm, &text, root, &files));
        }
        concepts.sort_by(|a, b| a.id.cmp(&b.id));
        let mut source_bytes = std::collections::BTreeMap::new();
        for concept in &concepts {
            for source in &concept.sources {
                let Some(resource) = &source.resource else {
                    continue;
                };
                let Some(path) = resolve_file_path(&concept.id, resource) else {
                    continue;
                };
                if !files.contains(&path) {
                    continue;
                }
                if source.sha256.is_some() && !source_bytes.contains_key(&path) {
                    let disk_path = root.join(path_from_posix(&path));
                    source_bytes.insert(path, fs::read(disk_path)?);
                }
            }
        }
        Ok(Bundle {
            concepts,
            okf_version,
            files: Some(files),
            source_bytes,
            source_modified: std::collections::BTreeMap::new(),
        })
    }

    /// Build a bundle from in-memory concepts.
    pub fn from_concepts(mut concepts: Vec<Concept>) -> Bundle {
        concepts.sort_by(|a, b| a.id.cmp(&b.id));
        Bundle {
            concepts,
            okf_version: None,
            files: None,
            source_bytes: std::collections::BTreeMap::new(),
            source_modified: std::collections::BTreeMap::new(),
        }
    }

    /// Look up one concept by id.
    pub fn concept(&self, id: &str) -> Option<&Concept> {
        self.concepts.iter().find(|c| c.id == id)
    }
}

fn path_from_posix(posix: &str) -> PathBuf {
    let mut pb = PathBuf::new();
    for seg in posix.split('/') {
        if !seg.is_empty() {
            pb.push(seg);
        }
    }
    pb
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let ft = entry.file_type()?;
        if ft.is_symlink() {
            continue;
        }
        let path = entry.path();
        if ft.is_dir() {
            collect_files(&path, out)?;
        } else if ft.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

/// The bundle index is a reserved file, not a concept, and carries
/// the declared `okf_version`.
fn read_okf_version(root: &Path) -> Option<String> {
    let index_path = root.join("index.md");
    let meta = fs::symlink_metadata(&index_path).ok()?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return None;
    }
    let text = fs::read_to_string(index_path).ok()?;
    let fm = parse_frontmatter(&text)?;
    fm.get("okf_version")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn to_posix_id(rel: &Path) -> String {
    let text = rel.to_string_lossy().replace('\\', "/");
    text.strip_suffix(".md").unwrap_or(&text).to_string()
}

/// The first in-bundle Python file a body links to, read as text.
fn linked_python_file(
    id: &str,
    body: &str,
    root: &Path,
    files: &std::collections::BTreeSet<String>,
) -> Option<String> {
    let mut rest = body;
    while let Some(start) = rest.find("](") {
        let after = &rest[start + 2..];
        let Some(end) = after.find(')') else {
            break;
        };
        let target = after[..end].trim();
        if target.ends_with(".py")
            && let Some(path) = resolve_file_path(id, target)
            && files.contains(&path)
        {
            return fs::read_to_string(root.join(path_from_posix(&path))).ok();
        }
        rest = &after[end + 1..];
    }
    None
}

fn concept_from_frontmatter(
    id: String,
    fm: &Value,
    text: &str,
    root: &Path,
    files: &std::collections::BTreeSet<String>,
) -> Concept {
    let status = fm
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("stable")
        .to_string();
    let superseded_by = fm
        .get("superseded_by")
        .and_then(Value::as_str)
        .map(str::to_string);
    let generated_text = fm
        .get("generated")
        .and_then(|g| g.get("at"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let generated_at = generated_text.as_deref().and_then(parse_timestamp);
    let stale_after_text = fm
        .get("stale_after")
        .and_then(Value::as_str)
        .map(str::to_string);
    let stale_after = stale_after_text.as_deref().and_then(parse_timestamp);
    let mut verified = Vec::new();
    if let Some(value) = fm.get("verified") {
        for item in value.as_items() {
            let by = item.get("by").and_then(Value::as_str).map(str::to_string);
            let at_text = item.get("at").and_then(Value::as_str).map(str::to_string);
            let at = at_text.as_deref().and_then(parse_timestamp);
            verified.push(VerifiedEvent { by, at_text, at });
        }
    }
    let mut sources = Vec::new();
    if let Some(value) = fm.get("sources") {
        for item in value.as_items() {
            sources.push(SourceRef {
                id: item.get("id").and_then(Value::as_str).map(str::to_string),
                resource: item
                    .get("resource")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                sha256: item
                    .get("sha256")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        }
    }
    let mut parameters = Vec::new();
    if let Some(value) = fm.get("parameters") {
        for item in value.as_items() {
            if let Some(name) = item.get("name").and_then(Value::as_str) {
                parameters.push(Parameter {
                    name: name.to_string(),
                    param_type: item.get("type").and_then(Value::as_str).map(str::to_string),
                    value: item
                        .get("value")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                });
            }
        }
    }
    let body = body_of(text);
    let citations = citation_labels(body);
    let runtime = fm
        .get("runtime")
        .and_then(Value::as_str)
        .map(str::to_string);
    let blocks = code_blocks(body);
    let file_code = linked_python_file(&id, body, root, files);
    let claimed = claimed_result(body);
    let concept_type = fm
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Concept {
        id,
        concept_type,
        status,
        superseded_by,
        generated_at,
        generated_text,
        stale_after_text,
        stale_after,
        verified,
        sources,
        citations,
        runtime,
        parameters,
        code_blocks: blocks,
        file_code,
        claimed_result: claimed,
    }
}
