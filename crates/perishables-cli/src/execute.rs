//! Re execution of Attested Computations.
//!
//! Execution lives in this adapter crate, not in the library,
//! because the library performs no IO beyond reading the bundle it
//! is handed. The verdicts follow the design record table. Only a
//! clean run that agrees with the claimed result is reported as
//! attested. A clean run that disagrees is a mismatch and a gate
//! finding. Everything else is reported as what it is, never as a
//! silent skip.
//!
//! The v1 profile executes deterministic local python computations
//! over declared parameters only. The computation runs with a
//! scrubbed environment, no inherited variables, a scratch working
//! directory that is discarded afterwards, and a wall clock limit.
//! Code that reaches for the clock, randomness, the network, the
//! filesystem, or process control is outside the profile and is
//! reported as non deterministic rather than compared.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use perishables_core::{Bundle, Finding, FindingCode};

const TIMEOUT: Duration = Duration::from_secs(5);

/// One verdict from the design record table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The computation ran and its result matches the claim. This
    /// is the only verdict reported as attested.
    RecomputedMatch,
    /// The computation ran and its result differs from the claim.
    Mismatch,
    /// The concept cannot be attested, because something the
    /// contract requires is missing.
    NotAttestable(String),
    /// The declared runtime is outside the v1 profile.
    NotExecutableInProfile(String),
    /// The computation did not run to completion.
    ExecutionFailed(String),
    /// The computation reaches outside the deterministic profile.
    NonDeterministicProfile(String),
}

impl Verdict {
    /// The verdict name as the design record writes it.
    pub fn name(&self) -> &'static str {
        match self {
            Verdict::RecomputedMatch => "RECOMPUTED_MATCH",
            Verdict::Mismatch => "MISMATCH",
            Verdict::NotAttestable(_) => "NOT_ATTESTABLE",
            Verdict::NotExecutableInProfile(_) => "NOT_EXECUTABLE_IN_PROFILE",
            Verdict::ExecutionFailed(_) => "EXECUTION_FAILED",
            Verdict::NonDeterministicProfile(_) => "NON_DETERMINISTIC_PROFILE",
        }
    }

    /// The stated reason, for verdicts that carry one.
    pub fn reason(&self) -> Option<&str> {
        match self {
            Verdict::NotAttestable(reason)
            | Verdict::NotExecutableInProfile(reason)
            | Verdict::ExecutionFailed(reason)
            | Verdict::NonDeterministicProfile(reason) => Some(reason),
            _ => None,
        }
    }
}

/// The outcome of re executing one computation concept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputationReport {
    /// The computation concept id.
    pub concept_id: String,
    /// The verdict.
    pub verdict: Verdict,
    /// The result the bundle claims, when it states one.
    pub claimed: Option<String>,
    /// The result the re execution produced, when it ran.
    pub recomputed: Option<String>,
}

const NON_DETERMINISTIC_MARKERS: &[&str] = &[
    "import os",
    "import sys",
    "import random",
    "import time",
    "import datetime",
    "import socket",
    "import urllib",
    "import requests",
    "import subprocess",
    "__import__",
    "open(",
    "input(",
    "eval(",
    "exec(",
];

/// Run every Attested Computation in a bundle through the v1
/// profile and report a verdict for each.
pub fn run_computations(bundle: &Bundle) -> Vec<ComputationReport> {
    let mut reports = Vec::new();
    for concept in &bundle.concepts {
        if concept.concept_type != "Attested Computation" {
            continue;
        }
        let (verdict, recomputed) = judge(concept);
        reports.push(ComputationReport {
            concept_id: concept.id.clone(),
            verdict,
            claimed: concept.claimed_result.clone(),
            recomputed,
        });
    }
    reports
}

/// Judge one computation concept, returning its verdict and the
/// recomputed result when it ran.
fn judge(concept: &perishables_core::Concept) -> (Verdict, Option<String>) {
    let Some(runtime) = concept.runtime.as_deref() else {
        return (
            Verdict::NotAttestable(
                "no runtime declared, so the concept is not attestable".to_string(),
            ),
            None,
        );
    };
    if runtime != "python" {
        return (
            Verdict::NotExecutableInProfile(format!(
                "runtime {runtime} is outside the v1 allowlist of deterministic local runtimes"
            )),
            None,
        );
    }
    let Some((_, code)) = concept
        .code_blocks
        .iter()
        .find(|(lang, _)| lang == "python")
    else {
        return (
            Verdict::NotAttestable("no computation recorded in the concept".to_string()),
            None,
        );
    };
    let Some(claimed) = concept.claimed_result.as_deref() else {
        return (
            Verdict::NotAttestable("no attested result recorded to compare against".to_string()),
            None,
        );
    };
    if let Some(marker) = NON_DETERMINISTIC_MARKERS
        .iter()
        .find(|marker| code.contains(**marker))
    {
        return (
            Verdict::NonDeterministicProfile(format!(
                "computation reaches outside the deterministic profile via {marker}"
            )),
            None,
        );
    }
    match execute_python(code) {
        Ok(recomputed) => {
            let verdict = if recomputed == claimed {
                Verdict::RecomputedMatch
            } else {
                Verdict::Mismatch
            };
            (verdict, Some(recomputed))
        }
        Err(reason) => (Verdict::ExecutionFailed(reason), None),
    }
}

/// The mismatch findings in a set of reports. A mismatch is the
/// only re execution outcome that is a gate finding.
pub fn mismatch_findings(reports: &[ComputationReport]) -> Vec<Finding> {
    reports
        .iter()
        .filter(|report| report.verdict == Verdict::Mismatch)
        .map(|report| Finding {
            concept_id: report.concept_id.clone(),
            code: FindingCode::AttestedResultMismatch,
            detail: format!(
                "claimed result {} does not recompute, the computation gives {}",
                report.claimed.as_deref().unwrap_or(""),
                report.recomputed.as_deref().unwrap_or("")
            ),
        })
        .collect()
}

/// Execute one python computation under the profile. The
/// computation assigns its answer to `result`, and the runner
/// prints it. Returns the printed token.
fn execute_python(code: &str) -> Result<String, String> {
    let scratch = std::env::temp_dir().join(format!("perishables-exec-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).map_err(|err| format!("scratch dir failed: {err}"))?;
    let script_path = scratch.join("computation.py");
    let mut script = code.to_string();
    script.push_str("\nprint(result)\n");
    let outcome = std::fs::write(&script_path, &script)
        .map_err(|err| format!("script write failed: {err}"))
        .and_then(|()| {
            let mut child = Command::new("python3")
                .arg("-I")
                .arg(&script_path)
                .current_dir(&scratch)
                .env_clear()
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|err| format!("python3 did not start: {err}"))?;
            let start = Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        let output = child
                            .wait_with_output()
                            .map_err(|err| format!("output read failed: {err}"))?;
                        if output.status.success() {
                            let text = String::from_utf8_lossy(&output.stdout);
                            let token = text.lines().next_back().unwrap_or("").trim().to_string();
                            if token.is_empty() {
                                return Err("computation printed no result".to_string());
                            }
                            return Ok(token);
                        }
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let last = stderr.lines().next_back().unwrap_or("").trim();
                        return Err(format!("computation failed: {last}"));
                    }
                    Ok(None) => {
                        if start.elapsed() > TIMEOUT {
                            let _ = child.kill();
                            let _ = child.wait();
                            return Err("computation exceeded the 5 second limit".to_string());
                        }
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    Err(err) => return Err(format!("wait failed: {err}")),
                }
            }
        });
    let _ = std::fs::remove_dir_all(&scratch);
    outcome
}
