//! Re-execution of Attested Computations under the deterministic v1 profile.
//!
//! Execution lives in the `perishables` crate rather than `perishables-core`
//! so the core library performs zero process spawning or scratch writes.
//! Every `Attested Computation` concept receives a single [`Verdict`]. Only
//! [`Verdict::RecomputedMatch`] counts as attested. A clean run whose result
//! differs from the claimed value emits [`FindingCode::AttestedResultMismatch`],
//! and any concept that claims an attested result while its computation is
//! missing, outside the profile, non-deterministic, or failing emits
//! [`FindingCode::UnattestedClaim`].

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use perishables_core::{Bundle, Concept, Finding, FindingCode, sha256_hex};

/// The concept type that marks an Attested Computation.
pub const ATTESTED_TYPE: &str = "Attested Computation";
/// The supported deterministic local runtime in the v1 allowlist.
pub const PYTHON_RUNTIME: &str = "python";
/// Default wall-clock limit per computation, in milliseconds.
pub const DEFAULT_TIMEOUT_MS: u64 = 5_000;
/// Default address-space/data limit per computation, in megabytes.
pub const DEFAULT_MEMORY_MB: u64 = 256;

const MAX_CAPTURE_BYTES: usize = 1024 * 1024;
const NON_DETERMINISTIC_EXIT_CODE: i32 = 78;
const NON_DETERMINISTIC_PREFIX: &str = "PERISHABLES_NON_DETERMINISTIC:";

const PYTHON_KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "case", "class",
    "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if",
    "import", "in", "is", "lambda", "match", "nonlocal", "not", "or", "pass", "raise", "return",
    "try", "while", "with", "yield", "_",
];

const BLOCKED_IDENTIFIERS: &[&str] = &[
    "open",
    "eval",
    "exec",
    "compile",
    "__import__",
    "input",
    "breakpoint",
    "result",
];

/// Execution limits and policy for Attested Computations.
///
/// Environment variables `PERISHABLES_EXEC_TIMEOUT_MS` and
/// `PERISHABLES_EXEC_MEMORY_MB` override the default bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecConfig {
    /// Maximum wall-clock duration per computation.
    pub timeout: Duration,
    /// Soft memory cap in megabytes when supported by the host OS.
    pub memory_mb: u64,
    /// Whether subprocess execution is enabled (`false` under `--no-exec`).
    pub enabled: bool,
}

impl Default for ExecConfig {
    fn default() -> Self {
        ExecConfig {
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
            memory_mb: DEFAULT_MEMORY_MB,
            enabled: true,
        }
    }
}

impl ExecConfig {
    /// Load configuration from defaults plus environment overrides.
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

/// Outcome verdict for one `Attested Computation` concept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Computation ran cleanly and agreed with the claimed result.
    RecomputedMatch,
    /// Computation ran cleanly and disagreed with the claimed result.
    Mismatch,
    /// Required contract metadata, code, parameter validity, or claim is missing.
    NotAttestable(String),
    /// Declared runtime is outside the deterministic local allowlist.
    NotExecutableInProfile(String),
    /// Subprocess failed to start, raised an exception, or timed out.
    ExecutionFailed(String),
    /// AST screening detected non-deterministic or sandbox-escaping constructs.
    NonDeterministicProfile(String),
    /// Subprocess execution was explicitly disabled by the caller (`--no-exec`).
    Skipped(String),
}

impl Verdict {
    /// Canonical uppercase verdict label.
    pub fn name(&self) -> &'static str {
        match self {
            Verdict::RecomputedMatch => "RECOMPUTED_MATCH",
            Verdict::Mismatch => "MISMATCH",
            Verdict::NotAttestable(_) => "NOT_ATTESTABLE",
            Verdict::NotExecutableInProfile(_) => "NOT_EXECUTABLE_IN_PROFILE",
            Verdict::ExecutionFailed(_) => "EXECUTION_FAILED",
            Verdict::NonDeterministicProfile(_) => "NON_DETERMINISTIC_PROFILE",
            Verdict::Skipped(_) => "SKIPPED",
        }
    }

    /// Optional explanatory reason carried by non-match/non-mismatch verdicts.
    pub fn reason(&self) -> Option<&str> {
        match self {
            Verdict::NotAttestable(reason)
            | Verdict::NotExecutableInProfile(reason)
            | Verdict::ExecutionFailed(reason)
            | Verdict::NonDeterministicProfile(reason)
            | Verdict::Skipped(reason) => Some(reason),
            Verdict::RecomputedMatch | Verdict::Mismatch => None,
        }
    }
}

/// Full report for re-executing one `Attested Computation` concept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputationReport {
    /// Bundle concept id.
    pub concept_id: String,
    /// Resulting verdict.
    pub verdict: Verdict,
    /// Claimed result parsed from the concept body, when present.
    pub claimed: Option<String>,
    /// Recomputed result printed by the computation, when executed cleanly.
    pub recomputed: Option<String>,
    /// SHA-256 hex digest of the bound parameter assignments plus computation code.
    pub artifact_hash: Option<String>,
    /// Measured wall-clock duration in milliseconds when executed.
    pub elapsed_ms: Option<u128>,
    /// Configured wall-clock timeout in milliseconds when executed.
    pub timeout_ms: Option<u64>,
}

/// Evaluate every `Attested Computation` concept in `bundle`.
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

enum RunError {
    NonDeterministic(String),
    Failed(String),
}

fn judge(
    concept: &Concept,
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
        .find(|(lang, _)| lang.eq_ignore_ascii_case(PYTHON_RUNTIME))
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
    let bound = match bind_parameters(concept, &code) {
        Ok(bound) => bound,
        Err(reason) => {
            return (Verdict::NotAttestable(reason), None, None, None);
        }
    };
    let artifact_hash = Some(sha256_hex(bound.as_bytes()));
    let Some(claimed) = concept.claimed_result.as_deref() else {
        return (
            Verdict::NotAttestable("no attested result recorded to compare against".to_string()),
            None,
            artifact_hash,
            None,
        );
    };
    if !config.enabled {
        return (
            Verdict::Skipped("execution disabled by --no-exec".to_string()),
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
        Err(RunError::NonDeterministic(target)) => (
            Verdict::NonDeterministicProfile(format!(
                "computation reaches outside the deterministic profile via {target}"
            )),
            None,
            artifact_hash,
            None,
        ),
        Err(RunError::Failed(reason)) => {
            (Verdict::ExecutionFailed(reason), None, artifact_hash, None)
        }
    }
}

fn is_valid_python_param_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return false;
    }
    if name.starts_with("_perishables") {
        return false;
    }
    !PYTHON_KEYWORDS.contains(&name) && !BLOCKED_IDENTIFIERS.contains(&name)
}

fn is_valid_float_literal(raw: &str) -> bool {
    let s = raw.strip_prefix(['+', '-']).unwrap_or(raw);
    if s.is_empty() {
        return false;
    }
    let (mantissa, exp) = match s.split_once(['e', 'E']) {
        Some((m, e)) => (m, Some(e)),
        None => (s, None),
    };
    let mantissa_ok = match mantissa.split_once('.') {
        Some((int_part, frac_part)) => {
            (!int_part.is_empty() || !frac_part.is_empty())
                && int_part.bytes().all(|b| b.is_ascii_digit())
                && frac_part.bytes().all(|b| b.is_ascii_digit())
        }
        None => !mantissa.is_empty() && mantissa.bytes().all(|b| b.is_ascii_digit()),
    };
    if !mantissa_ok {
        return false;
    }
    if let Some(e) = exp {
        let e = e.strip_prefix(['+', '-']).unwrap_or(e);
        if e.is_empty() || !e.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
    }
    raw.parse::<f64>().is_ok_and(f64::is_finite)
}

fn format_typed_parameter(
    param: &perishables_core::Parameter,
    raw: &str,
) -> Result<String, String> {
    let kind = param
        .param_type
        .as_deref()
        .map(|t| t.trim().to_ascii_lowercase());
    match kind.as_deref() {
        Some("integer" | "int") => {
            let parsed = raw.trim().parse::<i128>().map_err(|_| {
                format!(
                    "parameter {} declares integer type and value {:?} is not an integer",
                    param.name, raw
                )
            })?;
            Ok(parsed.to_string())
        }
        Some("number" | "float") => {
            let trimmed = raw.trim();
            if !is_valid_float_literal(trimmed) {
                return Err(format!(
                    "parameter {} declares number type and value {:?} is not a finite number",
                    param.name, raw
                ));
            }
            Ok(trimmed.to_string())
        }
        Some("boolean" | "bool") => match raw.trim().to_ascii_lowercase().as_str() {
            "true" => Ok("True".to_string()),
            "false" => Ok("False".to_string()),
            _ => Err(format!(
                "parameter {} declares boolean type and value {:?} is not true or false",
                param.name, raw
            )),
        },
        _ => Ok(python_string_literal(raw)),
    }
}

/// Validate declared parameter names and bind parameter values ahead of `code`.
fn bind_parameters(concept: &Concept, code: &str) -> Result<String, String> {
    let mut bound = String::new();
    for parameter in &concept.parameters {
        if !is_valid_python_param_name(&parameter.name) {
            return Err(format!(
                "parameter name {:?} is not a valid identifier in the deterministic python profile",
                parameter.name
            ));
        }
        if let Some(value) = &parameter.value {
            let literal = format_typed_parameter(parameter, value)?;
            bound.push_str(&format!("{} = {literal}\n", parameter.name));
        }
    }
    bound.push_str(code);
    Ok(bound)
}

/// Escape an arbitrary UTF-8 string into a safe single-line Python string literal.
fn python_string_literal(value: &str) -> String {
    let mut literal = String::with_capacity(value.len() + 2);
    literal.push('"');
    for c in value.chars() {
        match c {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            c if c.is_control() => {
                use std::fmt::Write as _;
                let _ = write!(literal, "\\u{:04x}", c as u32);
            }
            c => literal.push(c),
        }
    }
    literal.push('"');
    literal
}

/// Convert [`ComputationReport`] entries into gate [`Finding`] values.
///
/// Emits [`FindingCode::AttestedResultMismatch`] on [`Verdict::Mismatch`] and
/// [`FindingCode::UnattestedClaim`] whenever a concept claims an attested
/// result whose computation is missing, outside the profile, non-deterministic,
/// or failed at execution time.
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
            Verdict::NotAttestable(reason)
            | Verdict::NotExecutableInProfile(reason)
            | Verdict::ExecutionFailed(reason)
            | Verdict::NonDeterministicProfile(reason)
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

/// Filter [`findings_from_reports`] down to [`FindingCode::AttestedResultMismatch`] only.
pub fn mismatch_findings(reports: &[ComputationReport]) -> Vec<Finding> {
    findings_from_reports(reports)
        .into_iter()
        .filter(|finding| finding.code == FindingCode::AttestedResultMismatch)
        .collect()
}

static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

fn create_exclusive_scratch() -> Result<PathBuf, String> {
    let base = std::env::temp_dir();
    let pid = std::process::id();
    for _ in 0..32 {
        let seq = SCRATCH_COUNTER.fetch_add(1, Ordering::SeqCst);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let candidate = base.join(format!("perishables-exec-{pid}-{seq}-{nanos}"));
        #[cfg(unix)]
        let create_result = {
            use std::os::unix::fs::DirBuilderExt as _;
            let mut builder = std::fs::DirBuilder::new();
            builder.mode(0o700);
            builder.create(&candidate)
        };
        #[cfg(not(unix))]
        let create_result = std::fs::DirBuilder::new().create(&candidate);
        match create_result {
            Ok(()) => return Ok(candidate),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(format!("scratch dir failed: {err}")),
        }
    }
    Err("scratch dir failed: could not allocate exclusive directory".to_string())
}

fn build_runner_script(bound: &str, memory_bytes: u64) -> String {
    let encoded_src = python_string_literal(bound);
    format!(
        r#"import ast
import sys

_PERISHABLES_BLOCKED_MODULES = frozenset({{
    "asyncio", "atexit", "builtins", "code", "codeop", "ctypes", "datetime",
    "dbm", "fcntl", "ftplib", "gc", "http", "imaplib", "importlib", "inspect",
    "io", "marshal", "multiprocessing", "nt", "os", "pathlib", "pickle",
    "pkgutil", "platform", "poplib", "posix", "pty", "random", "requests",
    "resource", "runpy", "shelve", "shutil", "signal", "site", "smtplib",
    "socket", "sqlite3", "ssl", "subprocess", "sys", "sysconfig", "telnetlib",
    "tempfile", "threading", "time", "urllib", "webbrowser", "winreg",
    "zipimport",
}})

_PERISHABLES_BLOCKED_BUILTINS = frozenset({{
    "__import__", "breakpoint", "compile", "eval", "exec", "input", "open",
}})

_PERISHABLES_BLOCKED_ATTRS = frozenset({{
    "__builtins__", "__globals__", "__import__", "__loader__", "__spec__",
    "__subclasses__",
}})

def _perishables_screen(tree):
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                root = alias.name.split(".")[0]
                if root in _PERISHABLES_BLOCKED_MODULES:
                    return f"import {{alias.name}}"
        elif isinstance(node, ast.ImportFrom):
            if (node.level or 0) > 0:
                return "relative import"
            root = (node.module or "").split(".")[0]
            if root in _PERISHABLES_BLOCKED_MODULES:
                return f"from {{node.module}} import ..."
        elif isinstance(node, ast.Call):
            if isinstance(node.func, ast.Name) and node.func.id in _PERISHABLES_BLOCKED_BUILTINS:
                return f"{{node.func.id}}()"
        elif isinstance(node, ast.Name):
            if isinstance(node.ctx, ast.Load) and node.id in _PERISHABLES_BLOCKED_BUILTINS:
                return node.id
        elif isinstance(node, ast.Attribute):
            if node.attr in _PERISHABLES_BLOCKED_ATTRS:
                return node.attr
    return None

def _perishables_limit_memory(limit_bytes):
    try:
        import resource
    except ImportError:
        return
    for attr in ("RLIMIT_AS", "RLIMIT_DATA"):
        res = getattr(resource, attr, None)
        if res is None:
            continue
        try:
            _soft, hard = resource.getrlimit(res)
            target = limit_bytes if hard < 0 else min(limit_bytes, hard)
            resource.setrlimit(res, (target, hard))
            break
        except (ValueError, OSError):
            continue

_perishables_src = {encoded_src}
try:
    _perishables_tree = ast.parse(_perishables_src, filename="<computation>", mode="exec")
except SyntaxError as _exc:
    sys.stderr.write(f"SyntaxError: {{_exc.msg}}\n")
    sys.exit(1)

_perishables_violation = _perishables_screen(_perishables_tree)
if _perishables_violation is not None:
    sys.stderr.write("{NON_DETERMINISTIC_PREFIX}" + _perishables_violation + "\n")
    sys.exit({NON_DETERMINISTIC_EXIT_CODE})

_perishables_limit_memory({memory_bytes})
_perishables_ns = {{"__name__": "__computation__"}}
exec(compile(_perishables_tree, "<computation>", "exec"), _perishables_ns)
if "result" not in _perishables_ns:
    sys.stderr.write("NameError: computation did not assign result\n")
    sys.exit(1)
sys.stdout.write("\n" + str(_perishables_ns["result"]) + "\n")
sys.stdout.flush()
"#
    )
}

fn execute_python(bound: &str, config: &ExecConfig) -> Result<(String, u128), RunError> {
    let scratch = create_exclusive_scratch().map_err(RunError::Failed)?;
    let script_path = scratch.join("computation.py");
    let script = build_runner_script(bound, config.memory_mb.saturating_mul(1024 * 1024));
    let outcome = std::fs::write(&script_path, &script)
        .map_err(|err| RunError::Failed(format!("script write failed: {err}")))
        .and_then(|()| run_script(&script_path, &scratch, config.timeout));
    let _ = std::fs::remove_dir_all(&scratch);
    outcome
}

fn spawn_python(script_path: &Path, scratch: &Path) -> Result<std::process::Child, String> {
    let candidates: &[&str] = if cfg!(windows) {
        &["python", "python3"]
    } else {
        &["python3", "python"]
    };
    let mut last_err = String::new();
    for bin in candidates {
        let mut cmd = Command::new(bin);
        cmd.arg("-I")
            .arg("-B")
            .arg(script_path)
            .current_dir(scratch)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(path) = std::env::var_os("PATH") {
            cmd.env("PATH", path);
        }
        #[cfg(windows)]
        {
            if let Some(sys_root) = std::env::var_os("SystemRoot") {
                cmd.env("SystemRoot", sys_root);
            }
            cmd.env("TEMP", scratch);
            cmd.env("TMP", scratch);
        }
        match cmd.spawn() {
            Ok(child) => return Ok(child),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                last_err = format!("{bin} did not start: {err}");
                continue;
            }
            Err(err) => return Err(format!("{bin} did not start: {err}")),
        }
    }
    Err(last_err)
}

fn read_bounded<R: Read>(mut reader: R, limit: usize) -> (Vec<u8>, bool) {
    let mut captured = Vec::new();
    let mut overflow = false;
    let mut buf = [0u8; 8192];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if captured.len() < limit {
                    let take = n.min(limit - captured.len());
                    captured.extend_from_slice(&buf[..take]);
                    if take < n {
                        overflow = true;
                    }
                } else {
                    overflow = true;
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    (captured, overflow)
}

fn run_script(
    script_path: &Path,
    scratch: &Path,
    timeout: Duration,
) -> Result<(String, u128), RunError> {
    let start = Instant::now();
    let mut child = spawn_python(script_path, scratch).map_err(RunError::Failed)?;
    let stdout_pipe = child
        .stdout
        .take()
        .ok_or_else(|| RunError::Failed("stdout pipe unavailable".to_string()))?;
    let stderr_pipe = child
        .stderr
        .take()
        .ok_or_else(|| RunError::Failed("stderr pipe unavailable".to_string()))?;

    std::thread::scope(|scope| {
        let out_handle = scope.spawn(move || read_bounded(stdout_pipe, MAX_CAPTURE_BYTES));
        let err_handle = scope.spawn(move || read_bounded(stderr_pipe, MAX_CAPTURE_BYTES));

        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if start.elapsed() > timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        let _ = out_handle.join();
                        let _ = err_handle.join();
                        return Err(RunError::Failed(format!(
                            "computation exceeded the {} millisecond limit",
                            timeout.as_millis()
                        )));
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(err) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(RunError::Failed(format!("wait failed: {err}")));
                }
            }
        };

        let elapsed = start.elapsed().as_millis();
        let (stdout_bytes, stdout_overflow) = out_handle
            .join()
            .map_err(|_| RunError::Failed("stdout reader panicked".to_string()))?;
        let (stderr_bytes, _) = err_handle
            .join()
            .map_err(|_| RunError::Failed("stderr reader panicked".to_string()))?;

        if status.code() == Some(NON_DETERMINISTIC_EXIT_CODE) {
            let stderr = String::from_utf8_lossy(&stderr_bytes);
            for line in stderr.lines().rev() {
                if let Some(target) = line.trim().strip_prefix(NON_DETERMINISTIC_PREFIX) {
                    return Err(RunError::NonDeterministic(target.to_string()));
                }
            }
            return Err(RunError::NonDeterministic(
                "restricted construct".to_string(),
            ));
        }

        if status.success() {
            if stdout_overflow {
                return Err(RunError::Failed(format!(
                    "computation stdout exceeded the {MAX_CAPTURE_BYTES} byte limit"
                )));
            }
            let text = String::from_utf8_lossy(&stdout_bytes);
            let token = text.lines().next_back().unwrap_or("").trim().to_string();
            if token.is_empty() {
                return Err(RunError::Failed(
                    "computation printed no result".to_string(),
                ));
            }
            return Ok((token, elapsed));
        }

        let stderr = String::from_utf8_lossy(&stderr_bytes);
        let last = stderr.lines().next_back().unwrap_or("").trim();
        Err(RunError::Failed(format!("computation failed: {last}")))
    })
}
