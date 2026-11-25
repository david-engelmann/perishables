//! The four locked checks and auditor-side checks.
//!
//! Tier recompute and the currency check implement verification
//! currency from SPEC 5.3. A verification counts only when it is
//! current, meaning its timestamp is at or after `generated.at`, or
//! the concept carries no `generated.at`. Tiers are derived from
//! current events only, and only the `human:` scheme reaches the
//! `human-reviewed` tier. Findings are per event, one for each lapsed
//! or malformed event, and sorted by concept id then rule id.
//!
//! The chain walk and the supersession check implement SPEC 5.4 and
//! the path rules of SPEC 6. `superseded_by` carries meaning only on
//! a deprecated concept, a chain must stay inside the bundle, it
//! must terminate, and it must not cycle. Bundles that declare
//! version `0.2` or earlier, or no version, also get the SPEC 12.1
//! fallback, and any target found only through it is recorded both in
//! `ChainReport::fallback_targets` and in `info_notes`.

use std::collections::BTreeSet;

use crate::bundle::{Bundle, Concept, VerifiedEvent};
use crate::{
    AuditClock, ChainReport, ChainTerminal, EventClass, EventReport, Finding, FindingCode,
    InfoNote, Tier, TierReport,
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

pub(crate) fn sort_findings(findings: &mut [Finding]) {
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
/// call does not consult it.
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
/// SPEC 6.
fn resolve_path(from_concept: &str, raw: &str, root_relative: bool) -> Option<String> {
    let raw = raw.trim();
    let rooted;
    let effective = if root_relative && !raw.starts_with('/') {
        rooted = format!("/{raw}");
        rooted.as_str()
    } else {
        raw
    };
    let path = crate::bundle::resolve_file_path(from_concept, effective)?;
    Some(path.strip_suffix(".md").unwrap_or(&path).to_string())
}

/// Resolve one `superseded_by` hop to `(target_id, used_v02_fallback)`.
fn resolve_hop(bundle: &Bundle, from_concept: &str, raw: &str) -> Option<(String, bool)> {
    if let Some(id) =
        resolve_path(from_concept, raw, false).filter(|id| bundle.concept(id).is_some())
    {
        return Some((id, false));
    }
    if bundle.uses_v02_fallback()
        && !raw.trim().starts_with('/')
        && let Some(id) =
            resolve_path(from_concept, raw, true).filter(|id| bundle.concept(id).is_some())
    {
        return Some((id, true));
    }
    None
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
        let Some((next, via_fallback)) = resolve_hop(bundle, &current, &raw) else {
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
                detail: format!("chain exceeds {MAX_HOPS} hops"),
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

/// The freshness check, implementing SPEC 5.5.
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

/// Resolve a source resource to a bundle-relative file path.
fn resolve_resource(from_concept: &str, raw: &str) -> Option<String> {
    crate::bundle::resolve_file_path(from_concept, raw)
}

/// The information notes. Surfaces stale deprecated concepts,
/// concepts with no recorded provenance, SPEC 12.1 root-relative
/// supersession fallbacks, and source modification ordering facts
/// when explicit source timestamps were supplied.
pub fn info_notes(bundle: &Bundle, clock: &AuditClock) -> Vec<InfoNote> {
    let mut notes = Vec::new();
    for concept in &bundle.concepts {
        if concept.status == "deprecated"
            && let (Some(text), Some(stale_after)) =
                (&concept.stale_after_text, concept.stale_after)
            && clock.epoch_seconds >= stale_after
        {
            notes.push(InfoNote {
                concept_id: concept.id.clone(),
                text: format!(
                    "stale since {text} and deprecated, reported as information and not as a finding"
                ),
            });
        }
        if concept.sources.is_empty() {
            notes.push(InfoNote {
                concept_id: concept.id.clone(),
                text: "no recorded provenance, the concept carries no sources entry".to_string(),
            });
        }
        if let Some(raw) = &concept.superseded_by
            && !raw.trim().is_empty()
            && let Some((target, true)) = resolve_hop(bundle, &concept.id, raw)
        {
            notes.push(InfoNote {
                concept_id: concept.id.clone(),
                text: format!(
                    "superseded_by {} resolved to {target} via SPEC 12.1 root-relative fallback",
                    raw.trim()
                ),
            });
        }
        for source in &concept.sources {
            if source.sha256.is_some() {
                continue;
            }
            let (Some(resource), Some(generated)) = (&source.resource, concept.generated_at) else {
                continue;
            };
            let Some(path) = resolve_resource(&concept.id, resource) else {
                continue;
            };
            let Some(modified) = bundle.source_modified.get(&path) else {
                continue;
            };
            let ordering = match modified.cmp(&generated) {
                std::cmp::Ordering::Greater => "was modified after the concept was generated",
                std::cmp::Ordering::Less => "was modified before the concept was generated",
                std::cmp::Ordering::Equal => {
                    "was modified in the same second the concept was generated"
                }
            };
            notes.push(InfoNote {
                concept_id: concept.id.clone(),
                text: format!(
                    "source {} {ordering}, an ordering fact only and not a drift verdict",
                    source.id.as_deref().unwrap_or("without an id")
                ),
            });
        }
    }
    notes.sort_by(|a, b| a.concept_id.cmp(&b.concept_id).then(a.text.cmp(&b.text)));
    notes
}

/// The drift check. Rehashes every in-bundle source that records a
/// `sha256` digest and emits `FindingCode::SourceDrift` on mismatch.
pub fn check_drift(bundle: &Bundle) -> Vec<Finding> {
    let mut findings = Vec::new();
    for concept in &bundle.concepts {
        for source in &concept.sources {
            let (Some(recorded), Some(resource)) = (&source.sha256, &source.resource) else {
                continue;
            };
            let Some(path) = resolve_resource(&concept.id, resource) else {
                continue;
            };
            let Some(bytes) = bundle.source_bytes.get(&path) else {
                continue;
            };
            let actual = crate::hash::sha256_hex(bytes);
            if !actual.eq_ignore_ascii_case(recorded.trim()) {
                findings.push(Finding {
                    concept_id: concept.id.clone(),
                    code: FindingCode::SourceDrift,
                    detail: format!(
                        "source {} records sha256 {} but the bytes in the bundle hash to {actual}",
                        source.id.as_deref().unwrap_or("without an id"),
                        recorded.trim()
                    ),
                });
            }
        }
    }
    sort_findings(&mut findings);
    findings
}

/// The provenance check. Citations resolve against `sources` ids, and
/// in-bundle resources resolve against the bundle file inventory.
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
                let is_missing = match crate::bundle::classify_resource_path(&concept.id, resource)
                {
                    crate::bundle::ResourceTarget::ExternalUri => false,
                    crate::bundle::ResourceTarget::InvalidOrEscapes => true,
                    crate::bundle::ResourceTarget::InBundle(path) => !files.contains(&path),
                };
                if is_missing {
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
/// chain head in a bundle, deduplicating shared tails and cycles, and
/// enforce that `superseded_by` appears only on deprecated concepts.
pub fn check_supersession(bundle: &Bundle) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut referenced_targets: BTreeSet<String> = BTreeSet::new();
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
        if !raw.trim().is_empty()
            && let Some((target, _)) = resolve_hop(bundle, &concept.id, raw)
        {
            referenced_targets.insert(target);
        }
    }

    let mut visited: BTreeSet<String> = BTreeSet::new();
    // First pass: walk every true chain head (zero incoming superseded_by edges).
    for concept in &bundle.concepts {
        if concept.superseded_by.is_none() || referenced_targets.contains(&concept.id) {
            continue;
        }
        let report = walk_supersession(bundle, &concept.id);
        mark_chain_component(bundle, &concept.id, &mut visited);
        if let Some(finding) = report.finding {
            findings.push(finding);
        }
    }
    // Second pass: walk canonical entry points for headless pure cycles.
    for concept in &bundle.concepts {
        if concept.superseded_by.is_none() || visited.contains(&concept.id) {
            continue;
        }
        let report = walk_supersession(bundle, &concept.id);
        mark_chain_component(bundle, &concept.id, &mut visited);
        if let Some(finding) = report.finding {
            findings.push(finding);
        }
    }

    sort_findings(&mut findings);
    findings
}

fn mark_chain_component(bundle: &Bundle, start: &str, visited: &mut BTreeSet<String>) {
    let mut cur = start.to_string();
    while visited.insert(cur.clone()) {
        let Some(concept) = bundle.concept(&cur) else {
            break;
        };
        let Some(raw) = &concept.superseded_by else {
            break;
        };
        if raw.trim().is_empty() {
            break;
        }
        let Some((next, _)) = resolve_hop(bundle, &cur, raw) else {
            break;
        };
        cur = next;
    }
}
