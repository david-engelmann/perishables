//! The T5 corpus gate. Re execution lives in this adapter crate,
//! so its fixture is gated here rather than in the core corpus
//! test. The fixture claims 42 for a computation that gives 3.

use std::path::PathBuf;

use perishables_cli::execute::{ExecConfig, Verdict, mismatch_findings, run_computations};
use perishables_core::Bundle;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/fixtures")
        .join(name)
}

#[test]
fn t5_attested_mismatch() {
    let bundle = Bundle::load(&fixture("t5-attested-mismatch")).expect("fixture loads");
    let reports = run_computations(&bundle, &ExecConfig::default());
    assert_eq!(reports.len(), 1, "one computation concept expected");
    let report = &reports[0];
    assert_eq!(report.concept_id, "computations/row-count");
    assert_eq!(report.claimed.as_deref(), Some("42"));
    assert_eq!(report.recomputed.as_deref(), Some("3"));
    assert_eq!(report.verdict, Verdict::Mismatch);
    assert!(
        report.artifact_hash.is_some(),
        "the bound artifact hash is part of the report"
    );
    assert!(
        report.elapsed_ms.is_some(),
        "the run records its elapsed time"
    );
    assert_eq!(report.timeout_ms, Some(5_000));
    let findings = mismatch_findings(&reports);
    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].code,
        perishables_core::FindingCode::AttestedResultMismatch
    );
}

#[test]
fn matching_claim_is_attested() {
    // A synthetic bundle whose claim agrees with its computation
    // must come back RECOMPUTED_MATCH, the only attested verdict.
    let dir = std::env::temp_dir().join(format!("perishables-t5-match-{}", std::process::id()));
    let computations = dir.join("computations");
    std::fs::create_dir_all(&computations).expect("dirs");
    std::fs::write(
        dir.join("index.md"),
        "---\nokf_version: \"0.3\"\ntitle: Synthetic\n---\n",
    )
    .expect("index");
    std::fs::write(
        computations.join("row-count.md"),
        "---\ntype: Attested Computation\ntitle: Row count\nstatus: stable\nruntime: python\n---\n\n# Computation\n\n```python\nresult = len([1, 2, 3])\n```\n\n# Claimed result\n\nThe attested result claimed for this computation is 3.\n",
    )
    .expect("concept");
    let bundle = Bundle::load(&dir).expect("bundle loads");
    let reports = run_computations(&bundle, &ExecConfig::default());
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].verdict, Verdict::RecomputedMatch);
    assert!(mismatch_findings(&reports).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}
