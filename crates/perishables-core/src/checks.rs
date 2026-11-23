//! The four locked checks.
//!
//! Tier recompute and the currency check implement verification
//! currency from SPEC 5.3. A verification counts only when it is
//! current, meaning its timestamp is at or after `generated.at`, or
//! the concept carries no `generated.at`. Tiers are derived from
//! current events only, and only the human scheme reaches the
//! human-reviewed tier. Findings are per event, one for each lapsed
//! or malformed event, and sorted by concept id then rule id.
//!
//! The chain walk and the supersession check implement SPEC 5.4 and
//! the path rules of SPEC 6. `superseded_by` carries meaning only on
//! a deprecated concept, a chain must stay inside the bundle, it
//! must terminate, and it must not cycle. Bundles that declare
//! version 0.2 or earlier, or no version, also get the SPEC 12.1
//! fallback, and any target found only through it is marked.

use crate::bundle::{Bundle, Concept, VerifiedEvent};
use crate::{
    AuditClock, ChainReport, ChainTerminal, EventClass, EventReport, Finding, FindingCode, Tier,
    TierReport,
};

const MAX_HOPS: usize = 64;

fn tier_of(events: &[&VerifiedEvent]) -> Tier {
    if events.is_empty() {
        return Tier::Unverified;
    }
    if events.iter().any(|e| e.is_human()) {
        Tier::HumanReviewed
    } else {
        Tier::MachineConfirmed
    }
}

/// Classify one event against its concept generation instant.
fn classify(concept: &Concept, event: &VerifiedEvent) -> EventReport {
    let (class, reason) = if let Some(reason) = event.malformed_reason() {
        (EventClass::Malformed, reason)
    } else {
        match concept.generated_at {
            None => (
                EventClass::Current,
                "no generated.at recorded, so every well formed event counts".to_string(),
            ),
            Some(generated) if event.at.is_some_and(|at| at >= generated) => (
                EventClass::Current,
                format!(
                    "verified at {} is at or after generated at {}",
                    event.at_text.as_deref().unwrap_or(""),
                    concept.generated_text.as_deref().unwrap_or("")
                ),
            ),
            Some(_) => (
                EventClass::Lapsed,
                format!(
                    "verified at {} predates generated at {}",
                    event.at_text.as_deref().unwrap_or(""),
                    concept.generated_text.as_deref().unwrap_or("")
                ),
            ),
        }
    };
    EventReport {
        by: event.by.clone(),
        at_text: event.at_text.clone(),
        class,
        reason,
    }
}

fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by(|a, b| {
        a.concept_id
            .cmp(&b.concept_id)
            .then(a.code.label().cmp(b.code.label()))
            .then(a.detail.cmp(&b.detail))
    });
}

/// Call one of the locked interface. Recompute the trust tier of
/// one concept from its current verifications only, classifying
/// every recorded event.
///
/// The clock belongs to the locked signature. Currency is defined
/// against `generated.at`, not against the audit instant, so this
/// call does not consult it. The stale rule, which does read the
/// clock, arrives in its own round.
pub fn recompute_tier(concept: &Concept, _clock: &AuditClock) -> TierReport {
    let events: Vec<EventReport> = concept
        .verified
        .iter()
        .map(|event| classify(concept, event))
        .collect();
    let well_formed: Vec<&VerifiedEvent> = concept
        .verified
        .iter()
        .filter(|e| e.is_well_formed())
        .collect();
    let current: Vec<&VerifiedEvent> = concept
        .verified
        .iter()
        .zip(events.iter())
        .filter(|(_, report)| report.class == EventClass::Current)
        .map(|(event, _)| event)
        .collect();
    TierReport {
        concept_id: concept.id.clone(),
        raw_tier: tier_of(&well_formed),
        recomputed_tier: tier_of(&current),
        events,
    }
}

/// Call two of the locked interface. Check verification currency
/// across a whole bundle, one finding per lapsed or malformed
/// event.
pub fn check_currency(bundle: &Bundle, clock: &AuditClock) -> Vec<Finding> {
    let mut findings = Vec::new();
    for concept in &bundle.concepts {
        let report = recompute_tier(concept, clock);
        for event in &report.events {
            let code = match event.class {
                EventClass::Current => continue,
                EventClass::Lapsed => FindingCode::LapsedVerification,
                EventClass::Malformed => FindingCode::MalformedVerification,
            };
            findings.push(Finding {
                concept_id: concept.id.clone(),
                code,
                detail: event.reason.clone(),
            });
        }
    }
    sort_findings(&mut findings);
    findings
}

/// Resolve a `superseded_by` value to a bundle concept id, per
/// SPEC 6. A leading slash resolves from the bundle root, anything
/// else from the directory of the concept that carries the field.
/// External URIs and paths that escape the bundle do not resolve.
/// Returns the resolved id and whether the path was root relative.
fn resolve_path(from_concept: &str, raw: &str, root_relative: bool) -> Option<String> {
    let raw = raw.trim();
    if raw.contains("://") {
        return None;
    }
    let stripped = raw.strip_prefix('/').unwrap_or(raw);
    let combined = if raw.starts_with('/') || root_relative {
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
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    let joined = parts.join("/");
    Some(joined.strip_suffix(".md").unwrap_or(&joined).to_string())
}

fn dangling(start: &str, detail: String) -> Finding {
    Finding {
        concept_id: start.to_string(),
        code: FindingCode::DanglingSupersededBy,
        detail,
    }
}

/// Call three of the locked interface. Walk one supersession chain
/// to its end and report its shape.
pub fn walk_supersession(bundle: &Bundle, start: &str) -> ChainReport {
    let mut hops: Vec<String> = Vec::new();
    let mut fallback_targets: Vec<String> = Vec::new();
    let mut finding = None;
    let mut terminal = ChainTerminal::Terminated;
    let mut current = start.to_string();
    loop {
        let Some(concept) = bundle.concept(&current) else {
            terminal = ChainTerminal::Dangling;
            finding = Some(dangling(
                start,
                format!("chain reaches missing concept {current}"),
            ));
            break;
        };
        let Some(raw) = concept.superseded_by.clone() else {
            break;
        };
        if raw.trim().is_empty() {
            terminal = ChainTerminal::Malformed;
            finding = Some(dangling(start, "superseded_by is empty".to_string()));
            break;
        }
        // Primary resolution per SPEC 6, then the SPEC 12.1
        // fallback for older or undeclared bundles: a path without
        // a leading slash that fails from its file may resolve
        // from the bundle root, and the target is marked.
        let mut next =
            resolve_path(&current, &raw, false).filter(|id| bundle.concept(id).is_some());
        let mut via_fallback = false;
        if next.is_none()
            && bundle.uses_v02_fallback()
            && !raw.trim().starts_with('/')
            && let Some(id) =
                resolve_path(&current, &raw, true).filter(|id| bundle.concept(id).is_some())
        {
            next = Some(id);
            via_fallback = true;
        }
        let Some(next) = next else {
            terminal = ChainTerminal::Dangling;
            finding = Some(dangling(
                start,
                format!("superseded_by {raw} does not resolve to a concept in the bundle"),
            ));
            break;
        };
        if via_fallback {
            fallback_targets.push(next.clone());
        }
        hops.push(current.clone());
        if next == start || hops.contains(&next) {
            terminal = ChainTerminal::Cycle;
            finding = Some(Finding {
                concept_id: start.to_string(),
                code: FindingCode::SupersessionCycle,
                detail: format!("chain revisits {next} via {}", hops.join(", ")),
            });
            break;
        }
        if hops.len() > MAX_HOPS {
            terminal = ChainTerminal::Cycle;
            finding = Some(Finding {
                concept_id: start.to_string(),
                code: FindingCode::SupersessionCycle,
                detail: "chain exceeds 64 hops".to_string(),
            });
            break;
        }
        current = next;
    }
    ChainReport {
        start: start.to_string(),
        hops,
        terminal,
        fallback_targets,
        finding,
    }
}

/// The freshness check, implementing SPEC 5.5 as the design
/// record states it. Stale means check again and does not mean
/// false, and the finding wording says so. A stale deprecated
/// concept no longer claims currency, so it is not a finding.
pub fn check_freshness(bundle: &Bundle, clock: &AuditClock) -> Vec<Finding> {
    let mut findings = Vec::new();
    for concept in &bundle.concepts {
        let Some(text) = &concept.stale_after_text else {
            continue;
        };
        match concept.stale_after {
            None => findings.push(Finding {
                concept_id: concept.id.clone(),
                code: FindingCode::UnparseableStaleAfter,
                detail: format!(
                    "stale_after {text} does not parse, and an unparseable stale_after is never fresh"
                ),
            }),
            Some(stale_after) => {
                if clock.epoch_seconds >= stale_after && concept.status == "stable" {
                    findings.push(Finding {
                        concept_id: concept.id.clone(),
                        code: FindingCode::StaleButStable,
                        detail: format!(
                            "stale_after {text} has passed at the pinned clock and the concept still presents as stable. Stale means check again, it does not mean false"
                        ),
                    });
                }
            }
        }
    }
    sort_findings(&mut findings);
    findings
}

/// Resolve a source resource to a bundle relative file path.
/// External resources do not resolve, by design, because the audit
/// never fetches them.
fn resolve_resource(from_concept: &str, raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.contains("://") {
        return None;
    }
    let stripped = raw.strip_prefix('/').unwrap_or(raw);
    let combined = if raw.starts_with('/') {
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
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

/// The provenance check, implementing the design record section
/// on stated facts only. Citations resolve against source ids, and
/// in bundle resources resolve against the file inventory.
pub fn check_provenance(bundle: &Bundle) -> Vec<Finding> {
    let mut findings = Vec::new();
    for concept in &bundle.concepts {
        for label in &concept.citations {
            let resolved = concept
                .sources
                .iter()
                .any(|source| source.id.as_deref() == Some(label.as_str()));
            if !resolved {
                findings.push(Finding {
                    concept_id: concept.id.clone(),
                    code: FindingCode::UnresolvedCitation,
                    detail: format!("body cites [^{label}] but no sources entry carries that id"),
                });
            }
        }
        if let Some(files) = &bundle.files {
            for source in &concept.sources {
                let Some(resource) = &source.resource else {
                    continue;
                };
                if let Some(path) = resolve_resource(&concept.id, resource)
                    && !files.contains(&path)
                {
                    findings.push(Finding {
                        concept_id: concept.id.clone(),
                        code: FindingCode::MissingSource,
                        detail: format!(
                            "source {} points at {resource}, which is not in the bundle",
                            source.id.as_deref().unwrap_or("without an id")
                        ),
                    });
                }
            }
        }
    }
    sort_findings(&mut findings);
    findings
}

/// Call four of the locked interface. Check every supersession
/// chain in a bundle, including the rule that `superseded_by` has
/// meaning only on a deprecated concept.
pub fn check_supersession(bundle: &Bundle) -> Vec<Finding> {
    let mut findings = Vec::new();
    for concept in &bundle.concepts {
        let Some(raw) = &concept.superseded_by else {
            continue;
        };
        if concept.status != "deprecated" {
            findings.push(Finding {
                concept_id: concept.id.clone(),
                code: FindingCode::SupersededWithoutDeprecated,
                detail: format!("superseded_by={raw} but status={}", concept.status),
            });
        }
        let report = walk_supersession(bundle, &concept.id);
        if let Some(finding) = report.finding {
            findings.push(finding);
        }
    }
    sort_findings(&mut findings);
    findings
}
