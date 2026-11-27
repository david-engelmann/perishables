//! End-to-end binary integration tests for the `perishables` CLI.

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use perishables::completions::{Shell, completions_script};

static CLI_TEST_SEQ: AtomicU64 = AtomicU64::new(0);

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_perishables")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn join_posix(mut base: PathBuf, rel: &str) -> PathBuf {
    for part in rel.split('/') {
        if !part.is_empty() {
            base.push(part);
        }
    }
    base
}

fn corpus_fixture(name: &str) -> PathBuf {
    join_posix(repo_root(), &format!("corpus/fixtures/{name}"))
}

fn semantic_fixture(rel: &str) -> PathBuf {
    join_posix(
        repo_root(),
        &format!("crates/perishables-semantic/tests/fixtures/{rel}"),
    )
}

fn run_cli(args: &[&str]) -> Output {
    Command::new(bin())
        .current_dir(repo_root())
        .args(args)
        .output()
        .expect("execute perishables binary")
}

fn temp_path(label: &str) -> PathBuf {
    let seq = CLI_TEST_SEQ.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!(
        "perishables-bin-{label}-{}-{seq}",
        std::process::id()
    ))
}

#[test]
fn help_and_version_exit_zero_across_all_subcommands() {
    for args in [
        &["--help"][..],
        &["-h"],
        &["help"],
        &["help", "audit"],
        &["help", "completions"],
        &["--version"],
        &["-V"],
        &["audit", "--help"],
        &["corpus", "-h"],
        &["completions", "--help"],
        &["drift", "--help"],
        &["grade", "--help"],
        &["grade-dbt", "--help"],
        &["grade-ossie", "--help"],
        &["grade-snowflake", "--help"],
        &["grade-databricks", "--help"],
        &["grade-cube", "--help"],
        &["grade-looker", "--help"],
        &["grade-atscale", "--help"],
    ] {
        let out = run_cli(args);
        assert_eq!(
            out.status.code(),
            Some(0),
            "expected exit 0 for {args:?}, stderr={}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!out.stdout.is_empty());
    }

    let ver = run_cli(&["--version"]);
    assert_eq!(
        String::from_utf8_lossy(&ver.stdout).trim(),
        format!("perishables {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn completions_subcommand_prints_the_library_script_for_each_shell() {
    for shell in Shell::ALL {
        let out = run_cli(&["completions", shell.as_str()]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "expected exit 0 for completions {}, stderr={}",
            shell.as_str(),
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            out.stdout,
            completions_script(shell).as_bytes(),
            "binary output must be the library script for {}",
            shell.as_str()
        );
    }

    let pwsh = run_cli(&["completions", "pwsh"]);
    assert_eq!(pwsh.status.code(), Some(0));
    assert_eq!(
        pwsh.stdout,
        completions_script(Shell::PowerShell).as_bytes()
    );
}

#[test]
fn invalid_flags_and_missing_arguments_exit_two() {
    let clean = corpus_fixture("clean-healthy-chain");
    let clean_s = clean.to_str().unwrap();
    let inside_out = clean.join("inside-report.txt");
    let inside_s = inside_out.to_str().unwrap();

    for args in [
        &[][..],
        &["unknown-subcommand"],
        &["audit"],
        &["audit", clean_s],
        &["audit", clean_s, "--as-of", "2026-02-29"],
        &["audit", clean_s, "--as-of=2026-11-20", "--bogus"],
        &["audit", clean_s, "extra", "--as-of", "2026-11-20"],
        &["audit", clean_s, "--as-of=2026-11-20", "--format=xml"],
        &["audit", clean_s, "--as-of=2026-11-20", "--output", inside_s],
        &["drift"],
        &["completions"],
        &["completions", "tcsh"],
        &["completions", "bash", "zsh"],
        &["grade-dbt"],
        &[
            "grade-snowflake",
            "--changed-at=not-a-date",
            "nonexistent.yaml",
        ],
    ] {
        let out = run_cli(args);
        assert_eq!(
            out.status.code(),
            Some(2),
            "expected exit 2 for {args:?}, got {:?}",
            out.status.code()
        );
    }
    assert!(
        !inside_out.exists(),
        "must never create report inside bundle"
    );
}

#[test]
fn audit_is_byte_identical_and_supports_no_exec_timing_and_output_formats() {
    let clean = corpus_fixture("clean-healthy-chain");
    let clean_s = clean.to_str().unwrap();
    let r1 = run_cli(&["audit", clean_s, "--as-of", "2026-11-20"]);
    let r2 = run_cli(&["audit", "--as-of=2026-11-20", clean_s]);
    assert_eq!(r1.status.code(), Some(0));
    assert_eq!(
        r1.stdout, r2.stdout,
        "audit text output must be byte-for-byte identical"
    );

    let mismatch = corpus_fixture("t5-attested-mismatch");
    let mismatch_s = mismatch.to_str().unwrap();

    // Default audit omits wall-clock elapsed_ms for reproducibility.
    let fail_default = run_cli(&["audit", mismatch_s, "--as-of=2026-11-20"]);
    assert_eq!(fail_default.status.code(), Some(1));
    let stdout_default = String::from_utf8_lossy(&fail_default.stdout);
    assert!(stdout_default.contains("ATTESTED_RESULT_MISMATCH"));
    assert!(!stdout_default.contains("elapsed_ms="));

    // --timing opts into elapsed_ms.
    let fail_timing = run_cli(&["audit", mismatch_s, "--as-of=2026-11-20", "--timing"]);
    assert_eq!(fail_timing.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&fail_timing.stdout).contains("elapsed_ms="));

    // --no-exec skips Python execution and reports SKIPPED.
    let skipped = run_cli(&["audit", mismatch_s, "--as-of=2026-11-20", "--no-exec"]);
    assert_eq!(skipped.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&skipped.stdout).contains("SKIPPED"));

    // JSON, SARIF, GitHub formats + --output file outside bundle.
    let lapsed = corpus_fixture("t1-lapsed");
    let lapsed_s = lapsed.to_str().unwrap();

    let json_out = run_cli(&["audit", lapsed_s, "--as-of=2026-11-20", "--format=json"]);
    assert_eq!(json_out.status.code(), Some(1));
    let json_text = String::from_utf8_lossy(&json_out.stdout);
    assert!(json_text.contains("\"result\": \"FAIL\""));
    assert!(json_text.contains("\"LAPSED_VERIFICATION\""));

    let sarif_file = temp_path("audit.sarif");
    let sarif_out = run_cli(&[
        "audit",
        lapsed_s,
        "--as-of=2026-11-20",
        "--format=sarif",
        "--output",
        sarif_file.to_str().unwrap(),
    ]);
    assert_eq!(sarif_out.status.code(), Some(1));
    assert!(sarif_out.stdout.is_empty());
    let sarif_text = std::fs::read_to_string(&sarif_file).expect("read sarif output");
    let _ = std::fs::remove_file(&sarif_file);
    assert!(sarif_text.contains("\"version\": \"2.1.0\""));
    assert!(sarif_text.contains("\"ruleId\": \"LAPSED_VERIFICATION\""));

    let gh_out = run_cli(&["audit", lapsed_s, "--as-of=2026-11-20", "--format=github"]);
    assert_eq!(gh_out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&gh_out.stdout).starts_with("::error file="));
}

#[test]
fn drift_and_corpus_commands_enforce_gates_and_manifest_integrity() {
    let clean_s = corpus_fixture("clean-healthy-chain");
    let drift_ok = run_cli(&["drift", clean_s.to_str().unwrap()]);
    assert_eq!(drift_ok.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&drift_ok.stdout).contains("DRIFT NONE"));

    let drift_bad = corpus_fixture("t9-source-drift");
    let drift_fail = run_cli(&["drift", drift_bad.to_str().unwrap(), "--format=json"]);
    assert_eq!(drift_fail.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&drift_fail.stdout).contains("\"SOURCE_DRIFT\""));

    // Real shipped corpus passes 100% in both text and JSON modes.
    let corpus_ok = run_cli(&["corpus"]);
    assert_eq!(
        corpus_ok.status.code(),
        Some(0),
        "corpus failed: {}",
        String::from_utf8_lossy(&corpus_ok.stdout)
    );
    assert!(String::from_utf8_lossy(&corpus_ok.stdout).contains("failures=0"));

    let corpus_json = run_cli(&["corpus", "--format=json"]);
    assert_eq!(corpus_json.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&corpus_json.stdout).contains("\"failures_count\": 0"));

    // Synthetic broken corpus with unlisted fixture directory must exit 2.
    let broken_root = temp_path("broken-corpus");
    let broken_fixtures = broken_root.join("fixtures");
    std::fs::create_dir_all(broken_fixtures.join("unlisted")).expect("mkdir");
    std::fs::write(
        broken_root.join("manifest.toml"),
        "corpus_version = \"0.1.0\"\nas_of = \"2026-11-20\"\n\n[[fixture]]\nname = \"other\"\ntaxonomy = \"clean\"\nexpect = \"pass\"\n",
    )
    .expect("manifest");
    let broken_run = run_cli(&["corpus", broken_fixtures.to_str().unwrap()]);
    let _ = std::fs::remove_dir_all(&broken_root);
    assert_eq!(broken_run.status.code(), Some(2));
}

#[test]
fn all_seven_semantic_ecosystems_and_unified_grade_work_end_to_end() {
    let dbt_clean = semantic_fixture("clean.json");
    let dbt_dangling = semantic_fixture("dangling.json");
    let ossie_clean = semantic_fixture("ossie-clean.yaml");
    let ossie_col = semantic_fixture("ossie-collision.json");
    let sf_lapsed = semantic_fixture("snowflake-lapsed.yaml");
    let sf_clean = semantic_fixture("snowflake-clean.yaml");
    let db_bench = semantic_fixture("databricks-benchmark.json");
    let cube_js = semantic_fixture("cube-clean.js");
    let lookml_clean = semantic_fixture("lookml-clean.lookml");
    let looker_defects = semantic_fixture("looker-defects.json");
    let atscale_dangling = semantic_fixture("atscale-dangling.json");

    let owned_cases: Vec<(Vec<&str>, i32, &str)> = vec![
        (
            vec!["grade-dbt", dbt_clean.to_str().unwrap()],
            0,
            "GRADE-DBT",
        ),
        (
            vec!["grade-dbt", dbt_dangling.to_str().unwrap()],
            1,
            "DANGLING_METRIC_REFERENCE",
        ),
        (
            vec!["grade-ossie", ossie_clean.to_str().unwrap()],
            0,
            "GRADE-OSSIE",
        ),
        (
            vec!["grade-ossie", ossie_col.to_str().unwrap()],
            1,
            "NAME_COLLISION",
        ),
        (
            vec![
                "grade-snowflake",
                "--changed-at",
                "2026-11-20",
                sf_lapsed.to_str().unwrap(),
            ],
            1,
            "LAPSED_VERIFIED_QUERY",
        ),
        (
            vec![
                "grade-snowflake",
                sf_clean.to_str().unwrap(),
                "--changed-at=1750000000",
            ],
            0,
            "GRADE-SNOWFLAKE",
        ),
        (
            vec!["grade-databricks", db_bench.to_str().unwrap()],
            1,
            "BENCHMARK_WITHOUT_ANSWER",
        ),
        (
            vec!["grade-cube", cube_js.to_str().unwrap()],
            0,
            "GRADE-CUBE",
        ),
        (
            vec!["grade-looker", lookml_clean.to_str().unwrap()],
            0,
            "GRADE-LOOKER",
        ),
        (
            vec!["grade-looker", looker_defects.to_str().unwrap()],
            1,
            "NAME_COLLISION",
        ),
        (
            vec!["grade-atscale", atscale_dangling.to_str().unwrap()],
            1,
            "DANGLING_METRIC_REFERENCE",
        ),
        // Unified grade subcommand with auto-detection and --format=json / sarif / github.
        (
            vec!["grade", cube_js.to_str().unwrap(), "--format=json"],
            0,
            "\"result\": \"PASS\"",
        ),
        (
            vec![
                "grade",
                "--ecosystem=dbt",
                dbt_dangling.to_str().unwrap(),
                "--format=sarif",
            ],
            1,
            "\"DANGLING_METRIC_REFERENCE\"",
        ),
        (
            vec![
                "grade",
                "--ecosystem",
                "atscale",
                atscale_dangling.to_str().unwrap(),
                "--format=github",
            ],
            1,
            "::error file=",
        ),
    ];

    for (args, expected_code, needle) in owned_cases {
        let out = run_cli(&args);
        assert_eq!(
            out.status.code(),
            Some(expected_code),
            "unexpected exit for {args:?}: stderr={}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains(needle),
            "expected {needle:?} in stdout for {args:?}, got:\n{stdout}"
        );
    }

    // Multi-file LookML project scope via CLI.
    let p_orders = semantic_fixture("project/orders.view.lookml");
    let p_customers = semantic_fixture("project/customers.view.lookml");
    let p_extended = semantic_fixture("project/extended.view.lookml");
    let p_sales = semantic_fixture("project/sales.model.lookml");
    let proj_out = run_cli(&[
        "grade-looker",
        p_orders.to_str().unwrap(),
        p_customers.to_str().unwrap(),
        p_extended.to_str().unwrap(),
        p_sales.to_str().unwrap(),
    ]);
    assert_eq!(proj_out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&proj_out.stdout).contains("ghost_to_missing_view"));

    // Multi-file non-LookML batch via unified `perishables grade <FILE>...`.
    let multi_clean = run_cli(&[
        "grade",
        dbt_clean.to_str().unwrap(),
        ossie_clean.to_str().unwrap(),
        "--format=json",
    ]);
    assert_eq!(multi_clean.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&multi_clean.stdout).contains("\"reports\""));

    let multi_fail = run_cli(&[
        "grade",
        dbt_clean.to_str().unwrap(),
        dbt_dangling.to_str().unwrap(),
        "--format=sarif",
    ]);
    assert_eq!(multi_fail.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&multi_fail.stdout).contains("\"DANGLING_METRIC_REFERENCE\""));
}
