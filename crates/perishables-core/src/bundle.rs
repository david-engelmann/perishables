//! Loading a bundle from disk into the concepts the checks read.
//!
//! Loading never writes. Files are read in sorted path order so a
//! bundle always loads identically. Reserved files (`index.md` and
//! `log.md`) and hidden paths are not concepts. A Markdown file is a
//! concept when its frontmatter parses and carries a `type`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::frontmatter::{Value, parse_frontmatter};
use crate::time::parse_timestamp;

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
    /// A well formed event has an actor and a parseable timestamp.
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

    /// The human scheme is the only one that can give the
    /// human-reviewed tier.
    pub fn is_human(&self) -> bool {
        self.by
            .as_deref()
            .is_some_and(|by| by.starts_with("human:"))
    }
}

/// One concept and the trust fields the checks read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Concept {
    /// Bundle relative path without the `.md` suffix, posix style.
    pub id: String,
    /// The lifecycle status. Absent status means stable.
    pub status: String,
    /// The raw `superseded_by` value, when present.
    pub superseded_by: Option<String>,
    /// The parsed `generated.at`, when present and parseable.
    pub generated_at: Option<i64>,
    /// The raw `generated.at` text, when present.
    pub generated_text: Option<String>,
    /// Recorded verification events, in document order.
    pub verified: Vec<VerifiedEvent>,
}

/// A loaded bundle.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bundle {
    /// Concepts in sorted id order.
    pub concepts: Vec<Concept>,
    /// The `okf_version` declared by the bundle index, when any.
    pub okf_version: Option<String>,
}

impl Bundle {
    /// Whether the SPEC 12.1 fallbacks apply to this bundle. They
    /// apply when the bundle declares version 0.2 or earlier, or
    /// declares no version at all.
    pub fn uses_v02_fallback(&self) -> bool {
        match &self.okf_version {
            None => true,
            Some(version) => {
                let mut parts = version.split('.');
                let major: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
                let minor: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
                (major, minor) <= (0, 2)
            }
        }
    }
}

impl Bundle {
    /// Load every concept under a bundle root directory.
    pub fn load(root: &Path) -> std::io::Result<Bundle> {
        let okf_version = read_okf_version(root);
        let mut files = Vec::new();
        collect_markdown(root, &mut files);
        files.sort();
        let mut concepts = Vec::new();
        for path in files {
            let text = fs::read_to_string(&path)?;
            let Some(fm) = parse_frontmatter(&text) else {
                continue;
            };
            if fm.get("type").and_then(Value::as_str).is_none() {
                continue;
            }
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let id = to_posix_id(rel);
            concepts.push(concept_from_frontmatter(id, &fm));
        }
        concepts.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(Bundle {
            concepts,
            okf_version,
        })
    }

    /// Build a bundle from in memory concepts. The corpus and unit
    /// tests use this, and so can any consumer that parses bundles
    /// its own way.
    pub fn from_concepts(mut concepts: Vec<Concept>) -> Bundle {
        concepts.sort_by(|a, b| a.id.cmp(&b.id));
        Bundle {
            concepts,
            okf_version: None,
        }
    }

    /// One concept by id.
    pub fn concept(&self, id: &str) -> Option<&Concept> {
        self.concepts.iter().find(|c| c.id == id)
    }
}

fn collect_markdown(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_markdown(&path, out);
        } else if name.ends_with(".md") && name != "index.md" && name != "log.md" {
            out.push(path);
        }
    }
}

/// The bundle index is a reserved file, not a concept, but it
/// carries the declared `okf_version`.
fn read_okf_version(root: &Path) -> Option<String> {
    let text = fs::read_to_string(root.join("index.md")).ok()?;
    let fm = parse_frontmatter(&text)?;
    fm.get("okf_version")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn to_posix_id(rel: &Path) -> String {
    let text = rel.to_string_lossy().replace('\\', "/");
    text.strip_suffix(".md").unwrap_or(&text).to_string()
}

fn concept_from_frontmatter(id: String, fm: &Value) -> Concept {
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
    let mut verified = Vec::new();
    if let Some(value) = fm.get("verified") {
        for item in value.as_items() {
            let by = item.get("by").and_then(Value::as_str).map(str::to_string);
            let at_text = item.get("at").and_then(Value::as_str).map(str::to_string);
            let at = at_text.as_deref().and_then(parse_timestamp);
            verified.push(VerifiedEvent { by, at_text, at });
        }
    }
    Concept {
        id,
        status,
        superseded_by,
        generated_at,
        generated_text,
        verified,
    }
}
