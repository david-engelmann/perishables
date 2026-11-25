//! Trust-layer checks for Open Knowledge Format bundles.
//!
//! This crate recomputes trust tiers from current verifications
//! only, checks verification currency, and walks `superseded_by`
//! chains. It performs no writes. Every instant comes from a pinned
//! [`AuditClock`], so the same bundle and the same clock always give
//! the same findings.
//!
//! The public surface includes the four locked calls consumed by
//! downstream upkeep engines: [`recompute_tier`], [`check_currency`],
//! [`walk_supersession`], and [`check_supersession`]. Finding codes
//! live in the closed [`FindingCode`] enum and never change meaning
//! once published.
//!
//! # Example
//!
//! ```
//! use perishables_core::{
//!     AuditClock, Bundle, Concept, Tier, VerifiedEvent, check_currency, recompute_tier,
//! };
//!
//! let clock = AuditClock::parse("2026-11-20").expect("valid date");
//! let concept = Concept {
//!     id: "metrics/revenue".to_string(),
//!     concept_type: "Metric".to_string(),
//!     status: "stable".to_string(),
//!     superseded_by: None,
//!     generated_at: AuditClock::parse("2026-06-20T14:00:00Z").map(|c| c.epoch_seconds),
//!     generated_text: Some("2026-06-20T14:00:00Z".to_string()),
//!     stale_after_text: Some("2026-12-31T00:00:00Z".to_string()),
//!     stale_after: AuditClock::parse("2026-12-31T00:00:00Z").map(|c| c.epoch_seconds),
//!     verified: vec![VerifiedEvent {
//!         by: Some("human:reviewer".to_string()),
//!         at_text: Some("2026-06-25T09:00:00Z".to_string()),
//!         at: AuditClock::parse("2026-06-25T09:00:00Z").map(|c| c.epoch_seconds),
//!     }],
//!     sources: Vec::new(),
//!     citations: Vec::new(),
//!     runtime: None,
//!     parameters: Vec::new(),
//!     code_blocks: Vec::new(),
//!     file_code: None,
//!     claimed_result: None,
//! };
//!
//! let report = recompute_tier(&concept, &clock);
//! assert_eq!(report.recomputed_tier, Tier::HumanReviewed);
//!
//! let bundle = Bundle::from_concepts(vec![concept]);
//! assert!(check_currency(&bundle, &clock).is_empty());
//! ```

mod bundle;
mod checks;
mod frontmatter;
mod hash;
mod time;

pub use bundle::{Bundle, Concept, Parameter, SourceRef, VerifiedEvent};
pub use hash::sha256_hex;

/// A pinned instant. The library never reads the system clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AuditClock {
    /// Seconds since the Unix epoch, in UTC.
    pub epoch_seconds: i64,
}

impl AuditClock {
    /// Build a clock from an OKF RFC 3339 timestamp or bare `YYYY-MM-DD` date.
    /// Returns `None` when the text is not a valid calendar date or timestamp.
    ///
    /// ```
    /// use perishables_core::AuditClock;
    ///
    /// let clock = AuditClock::parse("2026-11-20").unwrap();
    /// assert_eq!(clock.to_rfc3339(), "2026-11-20T00:00:00Z");
    /// assert!(AuditClock::parse("2026-02-29").is_none());
    /// ```
    pub fn parse(text: &str) -> Option<AuditClock> {
        time::parse_timestamp(text).map(|epoch_seconds| AuditClock { epoch_seconds })
    }

    /// Format this instant as a canonical UTC RFC 3339 timestamp (`YYYY-MM-DDTHH:MM:SSZ`).
    pub fn to_rfc3339(&self) -> String {
        time::format_rfc3339(self.epoch_seconds)
    }
}

impl std::fmt::Display for AuditClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_rfc3339())
    }
}

/// A trust tier, derived and never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// No current verification event.
    Unverified,
    /// Current events exist and none is from a human actor.
    MachineConfirmed,
    /// At least one current event is from a human actor.
    HumanReviewed,
}

impl Tier {
    /// The tier name as the OKF specification writes it.
    pub fn name(&self) -> &'static str {
        match self {
            Tier::Unverified => "unverified",
            Tier::MachineConfirmed => "machine-confirmed",
            Tier::HumanReviewed => "human-reviewed",
        }
    }
}

/// A closed set of finding codes, shared with the corpus labels.
///
/// A code once published never changes meaning, and a retired code
/// is never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingCode {
    /// A verification predates the last meaningful change, so the
    /// raw tier overstates the recomputed tier.
    LapsedVerification,
    /// A verification event cannot be judged, because it lacks an
    /// actor or a parseable timestamp. It counts for nothing.
    MalformedVerification,
    /// A supersession chain reaches a concept that does not exist.
    DanglingSupersededBy,
    /// A supersession chain revisits a concept.
    SupersessionCycle,
    /// `superseded_by` appears on a concept that is not deprecated.
    SupersededWithoutDeprecated,
    /// The pinned instant is at or past `stale_after` and the
    /// concept still presents as stable. Stale means check again,
    /// it does not mean false.
    StaleButStable,
    /// A `stale_after` value does not parse as a timestamp. The
    /// spec says such a value is never treated as fresh.
    UnparseableStaleAfter,
    /// The body cites a footnote label that no `sources` entry
    /// carries as its id, so the attribution resolves to nothing.
    UnresolvedCitation,
    /// A `sources` entry points at an in-bundle resource that is
    /// not in the bundle.
    MissingSource,
    /// An Attested Computation ran cleanly under re-execution and
    /// its result differs from the attested result the bundle
    /// claims.
    AttestedResultMismatch,
    /// A source carries a recorded content hash and the bytes in
    /// the bundle no longer hash to it.
    SourceDrift,
    /// A concept presents a value as attested, while the computation
    /// behind it is missing, non-deterministic, failed, or outside
    /// the executable profile.
    UnattestedClaim,
}

impl FindingCode {
    /// The code as the corpus labels write it.
    pub fn label(&self) -> &'static str {
        match self {
            FindingCode::LapsedVerification => "LAPSED_VERIFICATION",
            FindingCode::MalformedVerification => "MALFORMED_VERIFICATION",
            FindingCode::DanglingSupersededBy => "DANGLING_SUPERSEDED_BY",
            FindingCode::SupersessionCycle => "SUPERSESSION_CYCLE",
            FindingCode::SupersededWithoutDeprecated => "SUPERSEDED_WITHOUT_DEPRECATED",
            FindingCode::StaleButStable => "STALE_BUT_STABLE",
            FindingCode::UnparseableStaleAfter => "UNPARSEABLE_STALE_AFTER",
            FindingCode::UnresolvedCitation => "UNRESOLVED_CITATION",
            FindingCode::MissingSource => "MISSING_SOURCE",
            FindingCode::AttestedResultMismatch => "ATTESTED_RESULT_MISMATCH",
            FindingCode::SourceDrift => "SOURCE_DRIFT",
            FindingCode::UnattestedClaim => "UNATTESTED_CLAIM",
        }
    }
}

/// One finding, tied to one concept and one rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The concept id, which is the bundle-relative path without
    /// the `.md` suffix.
    pub concept_id: String,
    /// The rule that produced this finding.
    pub code: FindingCode,
    /// A plain statement of the fact behind the finding.
    pub detail: String,
}

/// How one verification event classifies against its concept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventClass {
    /// Well-formed and at or after the last generation. It counts.
    Current,
    /// Well-formed and earlier than the last generation. It does
    /// not count.
    Lapsed,
    /// Missing its actor or a parseable timestamp. It counts for
    /// nothing and is reported, never silently dropped.
    Malformed,
}

/// The classification of one verification event, with its reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventReport {
    /// The actor string, when the event carries one.
    pub by: Option<String>,
    /// The raw timestamp text, when the event carries one.
    pub at_text: Option<String>,
    /// The class this event falls in.
    pub class: EventClass,
    /// A plain statement of why the event falls in that class.
    pub reason: String,
}

/// The result of recomputing one concept tier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierReport {
    /// The concept id.
    pub concept_id: String,
    /// The tier the raw verification list implies.
    pub raw_tier: Tier,
    /// The tier from current verifications only.
    pub recomputed_tier: Tier,
    /// The classification of every recorded event, in order, so a
    /// reader can see which events were excluded and why.
    pub events: Vec<EventReport>,
}

/// How a supersession chain ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainTerminal {
    /// The chain ends at a concept with no `superseded_by`.
    Terminated,
    /// The chain points at a target that is not in the bundle.
    Dangling,
    /// The chain revisits a concept or exceeds the hop cap.
    Cycle,
    /// The chain carries a `superseded_by` value that is not a
    /// usable path at all.
    Malformed,
}

/// The result of walking one supersession chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainReport {
    /// The concept the walk started from.
    pub start: String,
    /// The concepts visited, in walk order.
    pub hops: Vec<String>,
    /// How the chain ends.
    pub terminal: ChainTerminal,
    /// Targets reached only through the SPEC 12.1 fallback, marked
    /// as the spec requires. Empty for current-version bundles.
    pub fallback_targets: Vec<String>,
    /// The finding when the chain is defective.
    pub finding: Option<Finding>,
}

/// Recompute the trust tier of one concept from its current
/// verifications only. Call one of the locked interface.
pub fn recompute_tier(concept: &Concept, clock: &AuditClock) -> TierReport {
    checks::recompute_tier(concept, clock)
}

/// Check verification currency across a whole bundle. Call two of
/// the locked interface.
pub fn check_currency(bundle: &Bundle, clock: &AuditClock) -> Vec<Finding> {
    checks::check_currency(bundle, clock)
}

/// Walk one supersession chain to its end. Call three of the
/// locked interface.
pub fn walk_supersession(bundle: &Bundle, start: &str) -> ChainReport {
    checks::walk_supersession(bundle, start)
}

/// Check every supersession chain in a bundle. Call four of the
/// locked interface.
pub fn check_supersession(bundle: &Bundle) -> Vec<Finding> {
    checks::check_supersession(bundle)
}

/// Check freshness against the pinned clock.
pub fn check_freshness(bundle: &Bundle, clock: &AuditClock) -> Vec<Finding> {
    checks::check_freshness(bundle, clock)
}

/// Check provenance completeness.
pub fn check_provenance(bundle: &Bundle) -> Vec<Finding> {
    checks::check_provenance(bundle)
}

/// Check source drift against recorded SHA-256 content hashes.
pub fn check_drift(bundle: &Bundle) -> Vec<Finding> {
    checks::check_drift(bundle)
}

/// One piece of information reported without tripping the audit gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoNote {
    /// The concept the note concerns.
    pub concept_id: String,
    /// The note, in plain words.
    pub text: String,
}

/// Information notes for a bundle, covering stale deprecated concepts,
/// concepts with no recorded provenance, SPEC 12.1 fallback targets,
/// and explicit source modification ordering facts.
pub fn info_notes(bundle: &Bundle, clock: &AuditClock) -> Vec<InfoNote> {
    checks::info_notes(bundle, clock)
}

/// Sort findings into deterministic report order by concept id, then
/// rule id, then detail.
pub fn sort_findings(findings: &mut [Finding]) {
    checks::sort_findings(findings)
}
