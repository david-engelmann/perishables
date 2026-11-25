//! Example demonstrating how to embed `perishables-core` directly in Rust.

use std::path::PathBuf;

use perishables_core::{
    AuditClock, Bundle, ChainTerminal, Tier, check_currency, check_drift, check_freshness,
    check_provenance, check_supersession, info_notes, recompute_tier, walk_supersession,
};

fn main() {
    let fixture_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/fixtures/clean-healthy-chain");
    let bundle = Bundle::load(&fixture_dir).expect("load clean-healthy-chain fixture");
    let clock = AuditClock::parse("2026-11-20").expect("parse pinned audit date");

    println!(
        "Loaded bundle with {} concepts at {}",
        bundle.concepts.len(),
        clock
    );

    for concept in &bundle.concepts {
        let tier = recompute_tier(concept, &clock);
        println!(
            "  concept={} raw={} recomputed={}",
            tier.concept_id,
            tier.raw_tier.name(),
            tier.recomputed_tier.name()
        );
    }

    let chain = walk_supersession(&bundle, "metrics/old-revenue");
    assert_eq!(chain.terminal, ChainTerminal::Terminated);
    println!(
        "  chain start={} hops={:?} terminal={:?}",
        chain.start, chain.hops, chain.terminal
    );

    let revenue = bundle
        .concept("metrics/revenue")
        .expect("metrics/revenue exists");
    assert_eq!(
        recompute_tier(revenue, &clock).recomputed_tier,
        Tier::HumanReviewed
    );

    let mut findings = check_currency(&bundle, &clock);
    findings.extend(check_supersession(&bundle));
    findings.extend(check_freshness(&bundle, &clock));
    findings.extend(check_provenance(&bundle));
    findings.extend(check_drift(&bundle));
    assert!(
        findings.is_empty(),
        "expected zero findings, got {findings:?}"
    );

    let notes = info_notes(&bundle, &clock);
    println!(
        "Audit passed with {} findings and {} info notes.",
        findings.len(),
        notes.len()
    );
}
