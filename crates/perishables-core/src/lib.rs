//! Trust layer checks for Open Knowledge Format bundles.
//!
//! This crate recomputes trust tiers from current verifications
//! only, checks verification currency, and walks `superseded_by`
//! chains. It performs no writes. Every instant comes from a pinned
//! [`AuditClock`], so the same bundle and the same clock always give
//! the same findings.
//!
//! The public surface is the locked interface the upkeep engine
//! consumes. It has exactly four calls: [`recompute_tier`],
//! [`check_currency`], [`walk_supersession`], and
//! [`check_supersession`]. Finding codes live in the closed
//! [`FindingCode`] enum and never change meaning once published.

mod bundle;
mod checks;
mod frontmatter;
mod time;

pub use bundle::{Bundle, Concept, VerifiedEvent};

/// A pinned instant. The library never reads the system clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AuditClock {
    /// Seconds since the Unix epoch, in UTC.
    pub epoch_seconds: i64,
}

impl AuditClock {
    /// Build a clock from an OKF timestamp or bare date. Returns
    /// `None` when the text is not a timestamp.
    pub fn parse(text: &str) -> Option<AuditClock> {
        time::parse_timestamp(text).map(|epoch_seconds| AuditClock { epoch_seconds })
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
    /// The tier name as the spec writes it.
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
        }
    }
}

/// One finding, tied to one concept and one rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The concept id, which is the bundle relative path without
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
    /// Well formed and at or after the last generation. It counts.
    Current,
    /// Well formed but earlier than the last generation. It does
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
    /// as the spec requires. Empty for current version bundles.
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
