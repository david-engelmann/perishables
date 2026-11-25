//! Attested Computation execution and sandbox gate tests.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use perishables::execute::{
    ExecConfig, Verdict, findings_from_reports, mismatch_findings, run_computations,
};
use perishables_core::{Bundle, FindingCode};

static TEST_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/fixtures")
        .join(name)
}

fn make_temp_bundle(concept_markdown: &str) -> PathBuf {
    let seq = TEST_DIR_SEQ.fetch_add(1, Ordering::SeqCst);
    let dir =
        std::env::temp_dir().join(format!("perishables-t5-test-{}-{seq}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let computations = dir.join("computations");
    std::fs::create_dir_all(&computations).expect("create dirs");
    std::fs::write(
        dir.join("index.md"),
        "---\nokf_version: \"0.3\"\ntitle: Synthetic\n---\n",
    )
    .expect("write index");
    std::fs::write(computations.join("calc.md"), concept_markdown).expect("write concept");
    dir
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
    assert_eq!(findings[0].code, FindingCode::AttestedResultMismatch);
}

#[test]
fn t5_unattested_claim_fixture() {
    let bundle = Bundle::load(&fixture("t5-unattested-claim")).expect("fixture loads");
    let reports = run_computations(&bundle, &ExecConfig::default());
    assert_eq!(reports.len(), 1);
    assert!(matches!(
        reports[0].verdict,
        Verdict::NotExecutableInProfile(_)
    ));
    let findings = findings_from_reports(&reports);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code, FindingCode::UnattestedClaim);
}

#[test]
fn matching_claim_is_attested() {
    let dir = make_temp_bundle(
        "---\ntype: Attested Computation\ntitle: Row count\nstatus: stable\nruntime: python\n---\n\n# Computation\n\n```python\nresult = len([1, 2, 3])\n```\n\n# Claimed result\n\nThe attested result claimed for this computation is 3.\n",
    );
    let bundle = Bundle::load(&dir).expect("bundle loads");
    let reports = run_computations(&bundle, &ExecConfig::default());
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].verdict, Verdict::RecomputedMatch);
    assert!(findings_from_reports(&reports).is_empty());
}

#[test]
fn ast_screen_allows_comments_strings_and_reopen_identifier() {
    let dir = make_temp_bundle(
        "---\ntype: Attested Computation\ntitle: Safe AST\nstatus: stable\nruntime: python\n---\n\n# Computation\n\n```python\n# import os and open('/etc/passwd') in a comment\nbanner = \"import random; eval('1'); open('x')\"\ndef reopen(x):\n    return x * 2\nresult = reopen(len(banner) % 5 + 3)\n```\n\n# Claimed result\n\nThe attested result claimed for this computation is 6.\n",
    );
    let bundle = Bundle::load(&dir).expect("bundle loads");
    let reports = run_computations(&bundle, &ExecConfig::default());
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].verdict, Verdict::RecomputedMatch, "{reports:?}");
}

#[test]
fn ast_screen_blocks_from_imports_spaced_imports_and_builtin_aliases() {
    for snippet in [
        "from os import listdir\nresult = 1",
        "import   random\nresult = 1",
        "import urllib.request\nresult = 1",
        "fn = open\nresult = 1",
        "result = eval('1 + 1')",
    ] {
        let md = format!(
            "---\ntype: Attested Computation\nstatus: stable\nruntime: python\n---\n\n```python\n{snippet}\n```\n\n# Claimed result\n\nThe attested result claimed for this computation is 1.\n"
        );
        let dir = make_temp_bundle(&md);
        let bundle = Bundle::load(&dir).expect("bundle loads");
        let reports = run_computations(&bundle, &ExecConfig::default());
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            matches!(reports[0].verdict, Verdict::NonDeterministicProfile(_)),
            "expected NonDeterministicProfile for {snippet:?}, got {:?}",
            reports[0].verdict
        );
        let findings = findings_from_reports(&reports);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, FindingCode::UnattestedClaim);
    }
}

#[test]
fn large_stdout_does_not_deadlock_pipe() {
    let dir = make_temp_bundle(
        "---\ntype: Attested Computation\nstatus: stable\nruntime: python\n---\n\n```python\nfor _ in range(2000):\n    print('x' * 100)\nresult = 77\n```\n\n# Claimed result\n\nThe attested result claimed for this computation is 77.\n",
    );
    let bundle = Bundle::load(&dir).expect("bundle loads");
    let reports = run_computations(&bundle, &ExecConfig::default());
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(reports[0].verdict, Verdict::RecomputedMatch);
}

#[test]
fn timeout_and_runtime_failure_gate_as_unattested_claim() {
    let dir = make_temp_bundle(
        "---\ntype: Attested Computation\nstatus: stable\nruntime: python\n---\n\n```python\nwhile True:\n    pass\n```\n\n# Claimed result\n\nThe attested result claimed for this computation is 5.\n",
    );
    let bundle = Bundle::load(&dir).expect("bundle loads");
    let cfg = ExecConfig {
        timeout: Duration::from_millis(120),
        memory_mb: 256,
        enabled: true,
    };
    let reports = run_computations(&bundle, &cfg);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(matches!(reports[0].verdict, Verdict::ExecutionFailed(_)));
    let findings = findings_from_reports(&reports);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code, FindingCode::UnattestedClaim);
}

#[test]
fn parameter_validation_escaping_and_typed_binding() {
    // Typed parameters + control-char escaping in string parameter.
    let dir = make_temp_bundle(
        "---\ntype: Attested Computation\nstatus: stable\nruntime: python\nparameters:\n  - { name: count, type: integer, value: \"6\" }\n  - { name: factor, type: number, value: \"2.5\" }\n  - { name: active, type: boolean, value: \"true\" }\n  - { name: note, type: string, value: \"a\\rb\\tc\\\"d\" }\n---\n\n```python\nresult = int(count * factor) if active and len(note) > 0 else 0\n```\n\n# Claimed result\n\nThe attested result claimed for this computation is 15.\n",
    );
    let bundle = Bundle::load(&dir).expect("bundle loads");
    let reports = run_computations(&bundle, &ExecConfig::default());
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(reports[0].verdict, Verdict::RecomputedMatch, "{reports:?}");

    // Invalid parameter identifier must be rejected before execution.
    let bad_dir = make_temp_bundle(
        "---\ntype: Attested Computation\nstatus: stable\nruntime: python\nparameters:\n  - { name: \"x = 1; import os #\", value: \"abc\" }\n---\n\n```python\nresult = 1\n```\n\n# Claimed result\n\nThe attested result claimed for this computation is 1.\n",
    );
    let bad_bundle = Bundle::load(&bad_dir).expect("bundle loads");
    let bad_reports = run_computations(&bad_bundle, &ExecConfig::default());
    let _ = std::fs::remove_dir_all(&bad_dir);
    assert!(matches!(bad_reports[0].verdict, Verdict::NotAttestable(_)));
    assert_eq!(
        findings_from_reports(&bad_reports)[0].code,
        FindingCode::UnattestedClaim
    );
}

#[test]
fn no_exec_marks_computations_skipped() {
    let bundle = Bundle::load(&fixture("t5-attested-mismatch")).expect("fixture loads");
    let cfg = ExecConfig {
        enabled: false,
        ..ExecConfig::default()
    };
    let reports = run_computations(&bundle, &cfg);
    assert_eq!(reports.len(), 1);
    assert!(matches!(reports[0].verdict, Verdict::Skipped(_)));
    assert!(reports[0].artifact_hash.is_some());
    assert!(findings_from_reports(&reports).is_empty());
}

#[test]
fn indented_fenced_python_block_inside_list_executes_without_indentation_error() {
    let dir = make_temp_bundle(
        "---\ntype: Attested Computation\nstatus: stable\nruntime: python\n---\n\n- Step 1\n  ```python\n  values = [2, 4, 6]\n  result = sum(values)\n  ```\n\n# Claimed result\n\nThe attested result claimed for this computation is 12.\n",
    );
    let bundle = Bundle::load(&dir).expect("bundle loads");
    let reports = run_computations(&bundle, &ExecConfig::default());
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(reports[0].verdict, Verdict::RecomputedMatch, "{reports:?}");
}
