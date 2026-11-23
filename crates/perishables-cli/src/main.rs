//! The perishables command line auditor.
//!
//! Commands:
//!   perishables audit <bundle> [--as-of YYYY-MM-DD]
//!   perishables corpus [fixtures-dir]
//!
//! The audit command prints a tier line for every concept and one
//! line per finding, sorted, and exits non zero when any finding
//! exists. The corpus command runs the same checks over each
//! fixture bundle in a directory and prints a verdict per fixture.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use perishables_core::{
    AuditClock, Bundle, check_currency, check_freshness, check_provenance, check_supersession,
    recompute_tier,
};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("audit") => audit(&args[1..]),
        Some("corpus") => corpus(&args[1..]),
        _ => {
            eprintln!("usage: perishables audit <bundle> [--as-of YYYY-MM-DD]");
            eprintln!("       perishables corpus [fixtures-dir]");
            ExitCode::from(2)
        }
    }
}

fn audit(args: &[String]) -> ExitCode {
    let mut as_of = "2026-11-20".to_string();
    let mut bundle_path: Option<&String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--as-of" => {
                i += 1;
                if i < args.len() {
                    as_of = args[i].clone();
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
    let Some(clock) = AuditClock::parse(&as_of) else {
        eprintln!("--as-of value {as_of} is not a date");
        return ExitCode::from(2);
    };
    let bundle = match Bundle::load(Path::new(bundle_path)) {
        Ok(bundle) => bundle,
        Err(err) => {
            eprintln!("cannot load bundle {bundle_path}: {err}");
            return ExitCode::from(2);
        }
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
            "  TIER {} {}",
            report.concept_id,
            report.recomputed_tier.name()
        );
    }
    let mut findings = check_currency(&bundle, &clock);
    findings.extend(check_supersession(&bundle));
    findings.extend(check_freshness(&bundle, &clock));
    findings.extend(check_provenance(&bundle));
    findings.sort_by(|a, b| {
        a.concept_id
            .cmp(&b.concept_id)
            .then(a.code.label().cmp(b.code.label()))
    });
    for finding in &findings {
        println!(
            "  FINDING {} {} {}",
            finding.concept_id,
            finding.code.label(),
            finding.detail
        );
    }
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

fn corpus(args: &[String]) -> ExitCode {
    let dir = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("corpus/fixtures"));
    let clock = AuditClock::parse("2026-11-20").expect("pinned date parses");
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
    let mut total = 0usize;
    for fixture in entries {
        let name = fixture
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        match Bundle::load(&fixture) {
            Ok(bundle) => {
                let mut findings = check_currency(&bundle, &clock);
                findings.extend(check_supersession(&bundle));
                findings.extend(check_freshness(&bundle, &clock));
                findings.extend(check_provenance(&bundle));
                total += findings.len();
                println!(
                    "FIXTURE {} concepts={} findings={}",
                    name,
                    bundle.concepts.len(),
                    findings.len()
                );
                for finding in &findings {
                    println!(
                        "  FINDING {} {} {}",
                        finding.concept_id,
                        finding.code.label(),
                        finding.detail
                    );
                }
            }
            Err(err) => {
                eprintln!("cannot load fixture {name}: {err}");
                return ExitCode::from(2);
            }
        }
    }
    println!("CORPUS fixtures={count} findings={total}");
    ExitCode::SUCCESS
}
