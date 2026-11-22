//! Trust layer checks for Open Knowledge Format bundles.
//!
//! This crate recomputes trust tiers from current verifications only,
//! checks verification currency, and walks `superseded_by` chains.
//! It performs no writes. Every instant comes from a pinned
//! [`AuditClock`], so the same bundle and the same clock always give
//! the same findings.
//!
//! The public surface is the locked interface the upkeep engine
//! consumes. It has exactly four calls: [`recompute_tier`],
//! [`check_currency`], [`walk_supersession`], and
//! [`check_supersession`]. Finding codes live in the closed
//! [`FindingCode`] enum and never change meaning once published.

/// A pinned instant. The library never reads the system clock.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AuditClock {
    /// Seconds since the Unix epoch, in UTC.
    pub epoch_seconds: i64,
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

/// A closed set of finding codes, shared with the corpus labels.
///
/// A code once published never changes meaning, and a retired code
/// is never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingCode {
    /// A verification predates the last meaningful change.
    LapsedVerification,
    /// A supersession chain reaches a concept that does not exist.
    DanglingSupersededBy,
    /// A supersession chain revisits a concept.
    SupersessionCycle,
    /// `superseded_by` appears on a concept that is not deprecated.
    SupersededWithoutDeprecated,
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

/// The result of recomputing one concept tier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierReport {
    /// The concept id.
    pub concept_id: String,
    /// The tier the raw verification list implies.
    pub raw_tier: Tier,
    /// The tier from current verifications only.
    pub recomputed_tier: Tier,
}

/// The result of walking one supersession chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainReport {
    /// The concept the walk started from.
    pub start: String,
    /// The concepts visited, in walk order.
    pub hops: Vec<String>,
    /// The finding when the chain is defective.
    pub finding: Option<Finding>,
}

/// A loaded bundle. Construction from a directory lands with the
/// first build round. The scaffold keeps the type so the interface
/// is complete and reviewable.
#[derive(Debug, Clone, Default)]
pub struct Bundle {
    /// Concept ids known to the bundle, sorted.
    pub concept_ids: Vec<String>,
}

/// Recompute the trust tier of one concept from its current
/// verifications only.
///
/// This is call one of the locked interface.
pub fn recompute_tier(bundle: &Bundle, concept_id: &str, clock: &AuditClock) -> TierReport {
    let _ = (bundle, clock);
    TierReport {
        concept_id: concept_id.to_string(),
        raw_tier: Tier::Unverified,
        recomputed_tier: Tier::Unverified,
    }
}

/// Check verification currency across a whole bundle.
///
/// This is call two of the locked interface.
pub fn check_currency(bundle: &Bundle, clock: &AuditClock) -> Vec<Finding> {
    let _ = (bundle, clock);
    Vec::new()
}

/// Walk one supersession chain to its end.
///
/// This is call three of the locked interface.
pub fn walk_supersession(bundle: &Bundle, start: &str) -> ChainReport {
    let _ = bundle;
    ChainReport {
        start: start.to_string(),
        hops: Vec::new(),
        finding: None,
    }
}

/// Check every supersession chain in a bundle.
///
/// This is call four of the locked interface.
pub fn check_supersession(bundle: &Bundle) -> Vec<Finding> {
    let _ = bundle;
    Vec::new()
}
