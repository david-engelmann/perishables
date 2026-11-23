//! The perishables command line auditor.
//!
//! Commands:
//!   perishables audit <bundle> --as-of YYYY-MM-DD
//!   perishables corpus [fixtures-dir]
//!   perishables drift <bundle>
//!
//! The audit command prints a tier line for every concept showing
//! the raw and recomputed tiers, one line per computation verdict,
//! one line per information note, and one line per finding, and
//! exits non zero when any finding exists. The as-of argument is
//! required, because a defaulted clock destroys byte for byte
//! reproducibility. The corpus command is the reference runner for
//! the published corpus. It reads the corpus manifest and every
//! fixture label, states the corpus and auditor versions, compares
//! the auditor output against the labels exactly, verifies that
//! each fixture is byte unchanged by the run, and fails when any
//! fixture disagrees. The drift command runs the extension level
//! drift check only.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use perishables_cli::execute::{ExecConfig, findings_from_reports, run_computations};
use perishables_core::{
    AuditClock, Bundle, Finding, check_currency, check_drift, check_freshness, check_provenance,
    check_supersession, info_notes, recompute_tier, sha256_hex, sort_findings,
};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("audit") => audit(&args[1..]),
        Some("corpus") => corpus(&args[1..]),
        Some("drift") => drift(&args[1..]),
        _ => {
            eprintln!("usage: perishables audit <bundle> --as-of YYYY-MM-DD");
            eprintln!("       perishables corpus [fixtures-dir]");
            eprintln!("       perishables drift <bundle>");
            ExitCode::from(2)
        }
    }
}

/// Load a bundle or report the failure and return the exit code.
fn load_or_report(bundle_path: &str) -> Result<Bundle, ExitCode> {
    Bundle::load(Path::new(bundle_path)).map_err(|err| {
        eprintln!("cannot load bundle {bundle_path}: {err}");
        ExitCode::from(2)
    })
}

/// Print findings in the shared line shape.
fn print_findings(findings: &[Finding]) {
    for finding in findings {
        println!(
            "  FINDING {} {} {}",
            finding.concept_id,
            finding.code.label(),
            finding.detail
        );
    }
}

/// The default gate findings for a bundle, meaning the library
/// checks plus the re execution findings. Drift is extension
/// level and is never part of this set.
fn default_findings(bundle: &Bundle, clock: &AuditClock) -> Vec<Finding> {
    let mut findings = check_currency(bundle, clock);
    findings.extend(check_supersession(bundle));
    findings.extend(check_freshness(bundle, clock));
    findings.extend(check_provenance(bundle));
    findings.extend(findings_from_reports(&run_computations(
        bundle,
        &ExecConfig::from_env(),
    )));
    sort_findings(&mut findings);
    findings
}

fn audit(args: &[String]) -> ExitCode {
    let mut as_of: Option<String> = None;
    let mut bundle_path: Option<&String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--as-of" => {
                i += 1;
                if i < args.len() {
                    as_of = Some(args[i].clone());
                }
            }
            other => {
                if bundle_path.is_none() {
                    bundle_path = Some(&args[i]);
                } else {
                    eprintln!("unexpected argument {other}");
                    return ExitCode::from(2);
                }
            }
        }
        i += 1;
    }
    let Some(bundle_path) = bundle_path else {
        eprintln!("audit needs a bundle directory");
        return ExitCode::from(2);
    };
    let Some(as_of) = as_of else {
        eprintln!("audit needs --as-of YYYY-MM-DD, the clock is never defaulted");
        return ExitCode::from(2);
    };
    let Some(clock) = AuditClock::parse(&as_of) else {
        eprintln!("--as-of value {as_of} is not a date");
        return ExitCode::from(2);
    };
    let bundle = match load_or_report(bundle_path) {
        Ok(bundle) => bundle,
        Err(code) => return code,
    };
    println!(
        "BUNDLE {} as-of {} concepts={}",
        bundle_path,
        as_of,
        bundle.concepts.len()
    );
    for concept in &bundle.concepts {
        let report = recompute_tier(concept, &clock);
        println!(
            "  TIER {} raw={} recomputed={}",
            report.concept_id,
            report.raw_tier.name(),
            report.recomputed_tier.name()
        );
    }
    for report in run_computations(&bundle, &ExecConfig::from_env()) {
        let mut line = format!(
            "  COMPUTATION {} {}",
            report.concept_id,
            report.verdict.name()
        );
        if let Some(reason) = report.verdict.reason() {
            line.push_str(&format!(" reason={reason}"));
        }
        if let Some(hash) = &report.artifact_hash {
            line.push_str(&format!(" artifact={hash}"));
        }
        if let Some(elapsed) = report.elapsed_ms {
            line.push_str(&format!(" elapsed_ms={elapsed}"));
        }
        if let Some(timeout) = report.timeout_ms {
            line.push_str(&format!(" timeout_ms={timeout}"));
        }
        println!("{line}");
    }
    for note in info_notes(&bundle, &clock) {
        println!("  NOTE {} {}", note.concept_id, note.text);
    }
    let findings = default_findings(&bundle, &clock);
    print_findings(&findings);
    println!(
        "RESULT {} findings={}",
        if findings.is_empty() { "PASS" } else { "FAIL" },
        findings.len()
    );
    if findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// The extension level drift check, run only when asked for by
/// name. It is not part of the default audit gate.
fn drift(args: &[String]) -> ExitCode {
    let Some(bundle_path) = args.first() else {
        eprintln!("drift needs a bundle directory");
        return ExitCode::from(2);
    };
    let bundle = match load_or_report(bundle_path) {
        Ok(bundle) => bundle,
        Err(code) => return code,
    };
    let findings = check_drift(&bundle);
    print_findings(&findings);
    println!(
        "DRIFT {} findings={}",
        if findings.is_empty() { "NONE" } else { "FOUND" },
        findings.len()
    );
    if findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// One parsed fixture label.
struct Label {
    expect: String,
    expected_finding: Option<String>,
    concept: Option<String>,
    set: String,
}

/// Parse a LABEL.toml of flat `key = "value"` pairs.
fn parse_label(text: &str) -> Label {
    let mut label = Label {
        expect: String::new(),
        expected_finding: None,
        concept: None,
        set: "gate".to_string(),
    };
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"').to_string();
        match key.trim() {
            "expect" => label.expect = value,
            "expected_finding" => label.expected_finding = Some(value),
            "concept" => label.concept = Some(value),
            "set" => label.set = value,
            _ => {}
        }
    }
    label
}

/// One flat value from the corpus manifest.
fn manifest_value(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let Some((k, value)) = line.split_once('=') else {
            continue;
        };
        if k.trim() == key {
            return Some(value.trim().trim_matches('"').to_string());
        }
    }
    None
}

/// Hash a fixture tree, paths and bytes together, so the runner
/// can prove the audit left it byte for byte unchanged.
fn hash_tree(root: &Path) -> String {
    let mut files = Vec::new();
    collect_tree(root, root, &mut files);
    files.sort();
    let mut material = String::new();
    for rel in files {
        if let Ok(bytes) = std::fs::read(root.join(&rel)) {
            material.push_str(&rel);
            material.push('\n');
            material.push_str(&sha256_hex(&bytes));
            material.push('\n');
        }
    }
    sha256_hex(material.as_bytes())
}

fn collect_tree(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_tree(root, &path, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn corpus(args: &[String]) -> ExitCode {
    let dir = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("corpus/fixtures"));
    let manifest_path = dir
        .parent()
        .map(|parent| parent.join("manifest.toml"))
        .unwrap_or_else(|| PathBuf::from("corpus/manifest.toml"));
    let manifest = std::fs::read_to_string(&manifest_path).unwrap_or_default();
    let corpus_version = manifest_value(&manifest, "corpus_version").unwrap_or_default();
    let as_of = manifest_value(&manifest, "as_of").unwrap_or_default();
    let Some(clock) = AuditClock::parse(&as_of) else {
        eprintln!("corpus manifest as_of {as_of} is not a date");
        return ExitCode::from(2);
    };
    println!(
        "CORPUS version={} auditor={} as-of={}",
        corpus_version,
        env!("CARGO_PKG_VERSION"),
        as_of
    );
    let mut entries: Vec<PathBuf> = match std::fs::read_dir(&dir) {
        Ok(read) => read
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect(),
        Err(err) => {
            eprintln!("cannot read fixtures dir {}: {err}", dir.display());
            return ExitCode::from(2);
        }
    };
    entries.sort();
    let count = entries.len();
    let mut failures = 0usize;
    for fixture in entries {
        let name = fixture
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let label_text = std::fs::read_to_string(fixture.join("LABEL.toml")).unwrap_or_default();
        let label = parse_label(&label_text);
        let before = hash_tree(&fixture);
        let bundle = match Bundle::load(&fixture) {
            Ok(bundle) => bundle,
            Err(err) => {
                eprintln!("cannot load fixture {name}: {err}");
                return ExitCode::from(2);
            }
        };
        let mut findings = default_findings(&bundle, &clock);
        if label.set == "extension" {
            findings.extend(check_drift(&bundle));
            sort_findings(&mut findings);
        }
        let after = hash_tree(&fixture);
        let codes: BTreeSet<String> = findings
            .iter()
            .map(|f| f.code.label().to_string())
            .collect();
        let expected: BTreeSet<String> = match (&label.expect, &label.expected_finding) {
            (expect, Some(code)) if expect == "fail" => BTreeSet::from([code.clone()]),
            _ => BTreeSet::new(),
        };
        let concept_ok = match (&label.concept, &label.expected_finding) {
            (Some(concept), Some(_)) => findings
                .iter()
                .any(|f| &f.concept_id == concept && codes.contains(f.code.label())),
            _ => true,
        };
        let unchanged = before == after;
        let pass = codes == expected && concept_ok && unchanged;
        if !pass {
            failures += 1;
        }
        println!(
            "FIXTURE {} {} findings={} unchanged={}",
            name,
            if pass { "PASS" } else { "FAIL" },
            findings.len(),
            unchanged
        );
        print_findings(&findings);
        if codes != expected {
            println!("  EXPECTED codes={expected:?} got={codes:?}");
        }
        if !unchanged {
            println!("  MUTATED the audit changed the fixture bytes");
        }
    }
    println!("CORPUS fixtures={count} failures={failures}");
    if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
