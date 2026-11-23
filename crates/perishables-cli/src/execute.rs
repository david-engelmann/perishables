//! Re execution of Attested Computations.
//!
//! Execution lives in this adapter crate, not in the library,
//! because the library performs no IO beyond reading the bundle it
//! is handed. The verdicts follow the design record table. Only a
//! clean run that agrees with the claimed result is reported as
//! attested. A clean run that disagrees is a mismatch and a gate
//! finding. A concept that presents a value as attested while its
//! computation is missing or outside the profile is also a gate
//! finding, because the bundle claims a state the audit cannot
//! support. Everything else is reported as what it is, never as a
//! silent skip.
//!
//! The v1 profile executes deterministic local python computations
//! over declared parameters. Declared parameter values bind as
//! variables before the computation runs, exactly as the contract
//! requires, and the bound artifact, meaning the bindings plus the
//! declared computation, is hashed before execution and the hash
//! is part of the report. The computation text itself is never
//! edited. It runs with a scrubbed environment, a scratch working
//! directory that is discarded afterwards, a wall clock limit, and
//! a memory limit enforced through the resource limits the runner
//! sets on its own process image before the computation starts.
//! The stated v1 limits are these. Computations receive no bundle
//! access at all, which is stronger than the read only view the
//! contract describes, so a computation over bundle files is
//! outside the profile. Network reach is screened statically
//! rather than blocked by the operating system. The interpreter
//! invocation is pinned as `python3 -I`.

use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use perishables_core::{Bundle, Finding, FindingCode, sha256_hex};

/// The concept type that marks a computation.
pub const ATTESTED_TYPE: &str = "Attested Computation";
/// The one runtime in the v1 allowlist.
pub const PYTHON_RUNTIME: &str = "python";
/// The default wall clock limit for one computation.
pub const DEFAULT_TIMEOUT_MS: u64 = 5_000;
/// The default memory limit for one computation, in megabytes.
pub const DEFAULT_MEMORY_MB: u64 = 256;

/// Execution limits, set in configuration with conservative
/// defaults. The environment variables
/// `PERISHABLES_EXEC_TIMEOUT_MS` and `PERISHABLES_EXEC_MEMORY_MB`
/// override the defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecConfig {
    /// The wall clock limit.
    pub timeout: Duration,
    /// The memory limit in megabytes.
    pub memory_mb: u64,
}

impl Default for ExecConfig {
    fn default() -> Self {
        ExecConfig {
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
            memory_mb: DEFAULT_MEMORY_MB,
        }
    }
}

impl ExecConfig {
    /// The configured limits, defaults first and environment
    /// overrides where they parse.
    pub fn from_env() -> ExecConfig {
        let mut config = ExecConfig::default();
        if let Ok(value) = std::env::var("PERISHABLES_EXEC_TIMEOUT_MS")
            && let Ok(ms) = value.parse::<u64>()
        {
            config.timeout = Duration::from_millis(ms);
        }
        if let Ok(value) = std::env::var("PERISHABLES_EXEC_MEMORY_MB")
            && let Ok(mb) = value.parse::<u64>()
        {
            config.memory_mb = mb;
        }
        config
    }
}

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
    /// The SHA-256 of the bound artifact, bindings plus the
    /// declared computation, recorded before execution whenever a
    /// computation was present to bind.
    pub artifact_hash: Option<String>,
    /// The elapsed wall clock time of the run, in milliseconds,
    /// when the computation executed.
    pub elapsed_ms: Option<u128>,
    /// The wall clock bound the run executed under, in
    /// milliseconds, when the computation executed.
    pub timeout_ms: Option<u64>,
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
/// profile under the configured limits and report a verdict for
/// each. Every computation concept receives exactly one verdict.
pub fn run_computations(bundle: &Bundle, config: &ExecConfig) -> Vec<ComputationReport> {
    let mut reports = Vec::new();
    for concept in &bundle.concepts {
        if concept.concept_type != ATTESTED_TYPE {
            continue;
        }
        let (verdict, recomputed, artifact_hash, elapsed_ms) = judge(concept, config);
        reports.push(ComputationReport {
            concept_id: concept.id.clone(),
            verdict,
            claimed: concept.claimed_result.clone(),
            recomputed,
            artifact_hash,
            elapsed_ms,
            timeout_ms: elapsed_ms.map(|_| config.timeout.as_millis() as u64),
        });
    }
    reports
}

/// Judge one computation concept.
fn judge(
    concept: &perishables_core::Concept,
    config: &ExecConfig,
) -> (Verdict, Option<String>, Option<String>, Option<u128>) {
    let Some(runtime) = concept.runtime.as_deref() else {
        return (
            Verdict::NotAttestable(
                "no runtime declared, so the concept is not attestable".to_string(),
            ),
            None,
            None,
            None,
        );
    };
    if runtime != PYTHON_RUNTIME {
        return (
            Verdict::NotExecutableInProfile(format!(
                "runtime {runtime} is outside the v1 allowlist of deterministic local runtimes"
            )),
            None,
            None,
            None,
        );
    }
    let code = concept
        .code_blocks
        .iter()
        .find(|(lang, _)| lang == PYTHON_RUNTIME)
        .map(|(_, code)| code.clone())
        .or_else(|| concept.file_code.clone());
    let Some(code) = code else {
        return (
            Verdict::NotAttestable("no computation recorded in the concept".to_string()),
            None,
            None,
            None,
        );
    };
    let bound = bind_parameters(concept, &code);
    let artifact_hash = Some(sha256_hex(bound.as_bytes()));
    let Some(claimed) = concept.claimed_result.as_deref() else {
        return (
            Verdict::NotAttestable("no attested result recorded to compare against".to_string()),
            None,
            artifact_hash,
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
            artifact_hash,
            None,
        );
    }
    match execute_python(&bound, config) {
        Ok((recomputed, elapsed_ms)) => {
            let verdict = if recomputed == claimed {
                Verdict::RecomputedMatch
            } else {
                Verdict::Mismatch
            };
            (verdict, Some(recomputed), artifact_hash, Some(elapsed_ms))
        }
        Err(reason) => (Verdict::ExecutionFailed(reason), None, artifact_hash, None),
    }
}

/// Bind the declared parameter values in front of the declared
/// computation. The result is the bound artifact whose hash the
/// report carries. Parameters without a declared value stay
/// unbound, and a computation that references one fails at run
/// time and is reported as an execution failure.
fn bind_parameters(concept: &perishables_core::Concept, code: &str) -> String {
    let mut bound = String::new();
    for parameter in &concept.parameters {
        if let Some(value) = &parameter.value {
            bound.push_str(&format!(
                "{} = {}\n",
                parameter.name,
                python_string_literal(value)
            ));
        }
    }
    bound.push_str(code);
    bound
}

/// A Python string literal for a declared value.
fn python_string_literal(value: &str) -> String {
    let mut literal = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '\n' => literal.push_str("\\n"),
            _ => literal.push(c),
        }
    }
    literal.push('"');
    literal
}

/// The gate findings in a set of reports. A mismatch is one. A
/// claimed result whose computation is missing or outside the
/// profile is the other, because the bundle presents a value as
/// attested that the audit cannot support.
pub fn findings_from_reports(reports: &[ComputationReport]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for report in reports {
        match &report.verdict {
            Verdict::Mismatch => findings.push(Finding {
                concept_id: report.concept_id.clone(),
                code: FindingCode::AttestedResultMismatch,
                detail: format!(
                    "claimed result {} does not recompute, the computation gives {}",
                    report.claimed.as_deref().unwrap_or(""),
                    report.recomputed.as_deref().unwrap_or("")
                ),
            }),
            Verdict::NotAttestable(reason) | Verdict::NotExecutableInProfile(reason)
                if report.claimed.is_some() =>
            {
                findings.push(Finding {
                    concept_id: report.concept_id.clone(),
                    code: FindingCode::UnattestedClaim,
                    detail: format!(
                        "the bundle presents a value as attested but the computation cannot support it, {reason}"
                    ),
                });
            }
            _ => {}
        }
    }
    findings
}

/// The mismatch findings only, kept for callers that gate on
/// disagreement alone.
pub fn mismatch_findings(reports: &[ComputationReport]) -> Vec<Finding> {
    findings_from_reports(reports)
        .into_iter()
        .filter(|finding| finding.code == FindingCode::AttestedResultMismatch)
        .collect()
}

static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Execute one bound artifact under the profile. The runner wraps
/// the artifact without editing it. A bootstrap sets the memory
/// limit, the artifact runs exactly as bound, and a trailer prints
/// the `result` variable the computation assigns. Returns the
/// printed token and the elapsed milliseconds.
fn execute_python(bound: &str, config: &ExecConfig) -> Result<(String, u128), String> {
    let unique = SCRATCH_COUNTER.fetch_add(1, Ordering::SeqCst);
    let scratch =
        std::env::temp_dir().join(format!("perishables-exec-{}-{unique}", std::process::id()));
    std::fs::create_dir_all(&scratch).map_err(|err| format!("scratch dir failed: {err}"))?;
    let script_path = scratch.join("computation.py");
    let script = format!(
        "import resource\nresource.setrlimit(resource.RLIMIT_AS, ({0}, {0}))\n{1}\nprint(result)\n",
        config.memory_mb * 1024 * 1024,
        bound
    );
    let outcome = std::fs::write(&script_path, &script)
        .map_err(|err| format!("script write failed: {err}"))
        .and_then(|()| run_script(&script_path, &scratch, config.timeout));
    let _ = std::fs::remove_dir_all(&scratch);
    outcome
}

/// Spawn the pinned interpreter invocation and wait for it under
/// the wall clock limit.
fn run_script(
    script_path: &std::path::Path,
    scratch: &std::path::Path,
    timeout: Duration,
) -> Result<(String, u128), String> {
    let start = Instant::now();
    let mut child = Command::new("python3")
        .arg("-I")
        .arg(script_path)
        .current_dir(scratch)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("python3 did not start: {err}"))?;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child
                    .wait_with_output()
                    .map_err(|err| format!("output read failed: {err}"))?;
                let elapsed = start.elapsed().as_millis();
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout);
                    let token = text.lines().next_back().unwrap_or("").trim().to_string();
                    if token.is_empty() {
                        return Err("computation printed no result".to_string());
                    }
                    return Ok((token, elapsed));
                }
                let stderr = String::from_utf8_lossy(&output.stderr);
                let last = stderr.lines().next_back().unwrap_or("").trim();
                return Err(format!("computation failed: {last}"));
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "computation exceeded the {} millisecond limit",
                        timeout.as_millis()
                    ));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(err) => return Err(format!("wait failed: {err}")),
        }
    }
}
