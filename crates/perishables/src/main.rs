//! The `perishables` command-line auditor.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use perishables::execute::{
    ComputationReport, ExecConfig, findings_from_reports, run_computations,
};
use perishables::render::{
    CorpusFixtureSummary, GradeBatchEntry, OutputFormat, render_audit, render_corpus, render_drift,
    render_grade, render_grade_batch,
};
use perishables_core::{
    AuditClock, Bundle, Finding, TierReport, check_currency, check_drift, check_freshness,
    check_provenance, check_supersession, info_notes, recompute_tier, sha256_hex, sort_findings,
};
use perishables_semantic::Definition;

const ROOT_HELP: &str = "\
perishables - Deterministic trust-claim auditor for OKF bundles and semantic layers

USAGE:
  perishables <COMMAND> [OPTIONS]

COMMANDS:
  audit             Audit an Open Knowledge Format bundle at a pinned as-of date
  corpus            Run the planted-defect reference corpus gate
  drift             Check SHA-256 source drift across an OKF bundle
  grade             Grade semantic-layer definitions with --ecosystem or auto-detection
  grade-dbt         Grade a dbt semantic manifest (manifest.json or semantic_manifest.json)
  grade-ossie       Grade an Apache Ossie model (.json or .yaml)
  grade-snowflake   Grade a Snowflake semantic view (.yaml)
  grade-databricks  Grade a Databricks Genie serialized space (.json)
  grade-cube        Grade a Cube model (meta .json, .yaml, or .js)
  grade-looker      Grade a Looker API model (.json) or native LookML project (.lookml...)
  grade-atscale     Grade an AtScale semantic model export (.json)

GLOBAL OPTIONS:
  -h, --help        Print help information and exit 0
  -V, --version     Print version information and exit 0

EXIT CODES:
  0  Pass (zero findings)
  1  Gate failure (one or more findings or corpus mismatches)
  2  Usage, file I/O, or parse error
";

fn subcommand_help(cmd: &str) -> Option<&'static str> {
    match cmd {
        "audit" => Some(
            "\
perishables audit - Audit an Open Knowledge Format bundle at a pinned instant

USAGE:
  perishables audit <BUNDLE> --as-of <YYYY-MM-DD> [OPTIONS]

OPTIONS:
      --as-of <TIMESTAMP>           Pinned RFC 3339 timestamp or YYYY-MM-DD date (required)
      --format <FORMAT>             Output format: text, json, sarif, github [default: text]
      --output <PATH>               Write report to PATH (must lie outside BUNDLE)
      --no-exec                     Skip Python execution of Attested Computations
      --timing                      Include wall-clock elapsed_ms on executed computations
  -h, --help                        Print help information
",
        ),
        "corpus" => Some(
            "\
perishables corpus - Run the planted-defect reference corpus gate

USAGE:
  perishables corpus [FIXTURES_DIR] [OPTIONS]

OPTIONS:
      --format <FORMAT>             Output format: text, json [default: text]
      --output <PATH>               Write report to PATH
  -h, --help                        Print help information
",
        ),
        "drift" => Some(
            "\
perishables drift - Check SHA-256 source content hashes in an OKF bundle

USAGE:
  perishables drift <BUNDLE> [OPTIONS]

OPTIONS:
      --format <FORMAT>             Output format: text, json, sarif, github [default: text]
      --output <PATH>               Write report to PATH (must lie outside BUNDLE)
  -h, --help                        Print help information
",
        ),
        "grade" => Some(
            "\
perishables grade - Grade semantic-layer files across supported ecosystems

USAGE:
  perishables grade <FILE>... [--ecosystem <NAME>] [OPTIONS]

OPTIONS:
      --ecosystem <NAME>            Ecosystem: dbt, ossie, snowflake, databricks, cube, looker, atscale
      --changed-at <TIMESTAMP>      Definition last-changed date or epoch seconds (for snowflake)
      --format <FORMAT>             Output format: text, json, sarif, github [default: text]
      --output <PATH>               Write report to PATH
  -h, --help                        Print help information
",
        ),
        "grade-dbt" => Some(
            "\
perishables grade-dbt - Grade a dbt semantic manifest

USAGE:
  perishables grade-dbt <MANIFEST.JSON> [--format text|json|sarif|github] [--output <PATH>]
",
        ),
        "grade-ossie" => Some(
            "\
perishables grade-ossie - Grade an Apache Ossie model (.json or .yaml)

USAGE:
  perishables grade-ossie <MODEL.JSON|MODEL.YAML> [--format text|json|sarif|github] [--output <PATH>]
",
        ),
        "grade-snowflake" => Some(
            "\
perishables grade-snowflake - Grade a Snowflake semantic view YAML

USAGE:
  perishables grade-snowflake <VIEW.YAML> [--changed-at <TIMESTAMP>] [--format text|json|sarif|github] [--output <PATH>]
",
        ),
        "grade-databricks" => Some(
            "\
perishables grade-databricks - Grade a Databricks Genie serialized space JSON

USAGE:
  perishables grade-databricks <SPACE.JSON> [--format text|json|sarif|github] [--output <PATH>]
",
        ),
        "grade-cube" => Some(
            "\
perishables grade-cube - Grade a Cube meta JSON, YAML model, or JS model

USAGE:
  perishables grade-cube <META.JSON|MODEL.YAML|MODEL.JS> [--format text|json|sarif|github] [--output <PATH>]
",
        ),
        "grade-looker" => Some(
            "\
perishables grade-looker - Grade a Looker API model JSON or native LookML files

USAGE:
  perishables grade-looker <MODEL.JSON|FILE.LOOKML...> [--format text|json|sarif|github] [--output <PATH>]
",
        ),
        "grade-atscale" => Some(
            "\
perishables grade-atscale - Grade an AtScale semantic model export JSON

USAGE:
  perishables grade-atscale <MODEL.JSON> [--format text|json|sarif|github] [--output <PATH>]
",
        ),
        _ => None,
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("-h" | "--help") => {
            print!("{ROOT_HELP}");
            ExitCode::SUCCESS
        }
        Some("help") => {
            if let Some(sub) = args.get(1).map(String::as_str) {
                if let Some(help) = subcommand_help(sub) {
                    print!("{help}");
                    ExitCode::SUCCESS
                } else {
                    eprintln!("unknown subcommand {sub}");
                    ExitCode::from(2)
                }
            } else {
                print!("{ROOT_HELP}");
                ExitCode::SUCCESS
            }
        }
        Some("-V" | "--version") => {
            println!("perishables {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("audit") => audit(&args[1..]),
        Some("corpus") => corpus(&args[1..]),
        Some("drift") => drift(&args[1..]),
        Some("grade") => grade_unified(&args[1..]),
        Some("grade-dbt") => grade_ecosystem(&args[1..], "dbt", "GRADE-DBT"),
        Some("grade-ossie") => grade_ecosystem(&args[1..], "ossie", "GRADE-OSSIE"),
        Some("grade-snowflake") => grade_ecosystem(&args[1..], "snowflake", "GRADE-SNOWFLAKE"),
        Some("grade-databricks") => grade_ecosystem(&args[1..], "databricks", "GRADE-DATABRICKS"),
        Some("grade-cube") => grade_ecosystem(&args[1..], "cube", "GRADE-CUBE"),
        Some("grade-looker") => grade_ecosystem(&args[1..], "looker", "GRADE-LOOKER"),
        Some("grade-atscale") => grade_ecosystem(&args[1..], "atscale", "GRADE-ATSCALE"),
        Some(other) => {
            eprintln!("unknown command {other}");
            eprint!("{ROOT_HELP}");
            ExitCode::from(2)
        }
        None => {
            eprint!("{ROOT_HELP}");
            ExitCode::from(2)
        }
    }
}

fn has_help_flag(args: &[String]) -> bool {
    for arg in args {
        if arg == "--" {
            break;
        }
        if arg == "-h" || arg == "--help" {
            return true;
        }
    }
    false
}

fn take_flag_value<'a>(
    flag: &str,
    inline: Option<&'a str>,
    args: &'a [String],
    idx: &mut usize,
) -> Result<String, ExitCode> {
    if let Some(val) = inline {
        if val.is_empty() {
            eprintln!("{flag} requires a value");
            return Err(ExitCode::from(2));
        }
        return Ok(val.to_string());
    }
    *idx += 1;
    let Some(next) = args.get(*idx) else {
        eprintln!("{flag} requires a value");
        return Err(ExitCode::from(2));
    };
    Ok(next.clone())
}

fn split_flag(arg: &str) -> (&str, Option<&str>) {
    if let Some((k, v)) = arg.split_once('=') {
        (k, Some(v))
    } else {
        (arg, None)
    }
}

fn ensure_output_outside_bundle(output_path: &Path, bundle_path: &Path) -> Result<(), ExitCode> {
    let Ok(bundle_canon) = std::fs::canonicalize(bundle_path) else {
        return Ok(());
    };
    let abs_out = if output_path.is_absolute() {
        output_path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(output_path)
    };
    // Canonicalize the nearest existing ancestor of output_path so new files are resolved too.
    let mut probe: &Path = &abs_out;
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let resolved = loop {
        if let Ok(canon) = std::fs::canonicalize(probe) {
            let mut full = canon;
            for part in tail.into_iter().rev() {
                full.push(part);
            }
            break Some(full);
        }
        let Some(name) = probe.file_name() else {
            break None;
        };
        tail.push(name.to_os_string());
        let Some(parent) = probe.parent() else {
            break None;
        };
        probe = parent;
    };
    if let Some(resolved_out) = resolved
        && resolved_out.starts_with(&bundle_canon)
    {
        eprintln!(
            "--output path {} lies inside audited bundle {}; reports must be written outside the bundle",
            output_path.display(),
            bundle_path.display()
        );
        return Err(ExitCode::from(2));
    }
    Ok(())
}

fn emit_report(rendered: &str, output: Option<&Path>) -> Result<(), ExitCode> {
    if let Some(path) = output {
        std::fs::write(path, rendered).map_err(|err| {
            eprintln!("cannot write report to {}: {err}", path.display());
            ExitCode::from(2)
        })
    } else {
        print!("{rendered}");
        Ok(())
    }
}

fn load_or_report(bundle_path: &str) -> Result<Bundle, ExitCode> {
    let p = Path::new(bundle_path);
    let meta = std::fs::metadata(p).map_err(|err| {
        eprintln!("cannot load bundle {bundle_path}: {err}");
        ExitCode::from(2)
    })?;
    if !meta.is_dir() {
        eprintln!("bundle path {bundle_path} is not a directory");
        return Err(ExitCode::from(2));
    }
    Bundle::load(p).map_err(|err| {
        eprintln!("cannot load bundle {bundle_path}: {err}");
        ExitCode::from(2)
    })
}

fn default_findings(
    bundle: &Bundle,
    clock: &AuditClock,
    reports: &[ComputationReport],
) -> Vec<Finding> {
    let mut findings = check_currency(bundle, clock);
    findings.extend(check_supersession(bundle));
    findings.extend(check_freshness(bundle, clock));
    findings.extend(check_provenance(bundle));
    findings.extend(check_drift(bundle));
    findings.extend(findings_from_reports(reports));
    sort_findings(&mut findings);
    findings
}

fn audit(args: &[String]) -> ExitCode {
    if has_help_flag(args) {
        print!("{}", subcommand_help("audit").unwrap_or(ROOT_HELP));
        return ExitCode::SUCCESS;
    }
    let mut as_of: Option<String> = None;
    let mut format = OutputFormat::Text;
    let mut output: Option<PathBuf> = None;
    let mut no_exec = false;
    let mut timing = false;
    let mut positionals: Vec<String> = Vec::new();
    let mut end_of_flags = false;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if !end_of_flags && arg == "--" {
            end_of_flags = true;
            i += 1;
            continue;
        }
        if !end_of_flags && arg.starts_with('-') && arg != "-" {
            let (flag, inline) = split_flag(arg);
            match flag {
                "--as-of" => match take_flag_value(flag, inline, args, &mut i) {
                    Ok(v) => as_of = Some(v),
                    Err(c) => return c,
                },
                "--format" => match take_flag_value(flag, inline, args, &mut i) {
                    Ok(v) => match OutputFormat::parse(&v) {
                        Some(f) => format = f,
                        None => {
                            eprintln!(
                                "unknown --format {v} (expected text, json, sarif, or github)"
                            );
                            return ExitCode::from(2);
                        }
                    },
                    Err(c) => return c,
                },
                "--output" | "-o" => match take_flag_value(flag, inline, args, &mut i) {
                    Ok(v) => output = Some(PathBuf::from(v)),
                    Err(c) => return c,
                },
                "--no-exec" => {
                    if inline.is_some() {
                        eprintln!("--no-exec takes no value");
                        return ExitCode::from(2);
                    }
                    no_exec = true;
                }
                "--timing" => {
                    if inline.is_some() {
                        eprintln!("--timing takes no value");
                        return ExitCode::from(2);
                    }
                    timing = true;
                }
                _ => {
                    eprintln!("unknown flag {flag}");
                    return ExitCode::from(2);
                }
            }
        } else {
            positionals.push(arg.clone());
        }
        i += 1;
    }
    if positionals.is_empty() {
        eprintln!("audit requires a <BUNDLE> directory");
        return ExitCode::from(2);
    }
    if positionals.len() > 1 {
        eprintln!("unexpected positional argument {}", positionals[1]);
        return ExitCode::from(2);
    }
    let bundle_path = &positionals[0];
    let Some(as_of) = as_of else {
        eprintln!("audit requires --as-of YYYY-MM-DD; the clock is never defaulted");
        return ExitCode::from(2);
    };
    let Some(clock) = AuditClock::parse(&as_of) else {
        eprintln!("--as-of value {as_of} is not a valid RFC 3339 timestamp or YYYY-MM-DD date");
        return ExitCode::from(2);
    };
    if let Some(out_path) = output.as_deref()
        && let Err(c) = ensure_output_outside_bundle(out_path, Path::new(bundle_path))
    {
        return c;
    }
    let bundle = match load_or_report(bundle_path) {
        Ok(b) => b,
        Err(c) => return c,
    };
    let tiers: Vec<TierReport> = bundle
        .concepts
        .iter()
        .map(|c| recompute_tier(c, &clock))
        .collect();
    let mut exec_cfg = ExecConfig::from_env();
    exec_cfg.enabled = !no_exec;
    // Execute computations at most once per audit invocation.
    let reports = run_computations(&bundle, &exec_cfg);
    let notes = info_notes(&bundle, &clock);
    let findings = default_findings(&bundle, &clock, &reports);
    let rendered = render_audit(
        format,
        bundle_path,
        &as_of,
        &tiers,
        &reports,
        &notes,
        &findings,
        timing,
    );
    if let Err(c) = emit_report(&rendered, output.as_deref()) {
        return c;
    }
    if findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn drift(args: &[String]) -> ExitCode {
    if has_help_flag(args) {
        print!("{}", subcommand_help("drift").unwrap_or(ROOT_HELP));
        return ExitCode::SUCCESS;
    }
    let mut format = OutputFormat::Text;
    let mut output: Option<PathBuf> = None;
    let mut positionals: Vec<String> = Vec::new();
    let mut end_of_flags = false;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if !end_of_flags && arg == "--" {
            end_of_flags = true;
            i += 1;
            continue;
        }
        if !end_of_flags && arg.starts_with('-') && arg != "-" {
            let (flag, inline) = split_flag(arg);
            match flag {
                "--format" => match take_flag_value(flag, inline, args, &mut i) {
                    Ok(v) => match OutputFormat::parse(&v) {
                        Some(f) => format = f,
                        None => {
                            eprintln!(
                                "unknown --format {v} (expected text, json, sarif, or github)"
                            );
                            return ExitCode::from(2);
                        }
                    },
                    Err(c) => return c,
                },
                "--output" | "-o" => match take_flag_value(flag, inline, args, &mut i) {
                    Ok(v) => output = Some(PathBuf::from(v)),
                    Err(c) => return c,
                },
                _ => {
                    eprintln!("unknown flag {flag}");
                    return ExitCode::from(2);
                }
            }
        } else {
            positionals.push(arg.clone());
        }
        i += 1;
    }
    if positionals.is_empty() {
        eprintln!("drift requires a <BUNDLE> directory");
        return ExitCode::from(2);
    }
    if positionals.len() > 1 {
        eprintln!("unexpected positional argument {}", positionals[1]);
        return ExitCode::from(2);
    }
    let bundle_path = &positionals[0];
    if let Some(out_path) = output.as_deref()
        && let Err(c) = ensure_output_outside_bundle(out_path, Path::new(bundle_path))
    {
        return c;
    }
    let bundle = match load_or_report(bundle_path) {
        Ok(b) => b,
        Err(c) => return c,
    };
    let findings = check_drift(&bundle);
    let rendered = render_drift(format, bundle_path, &findings);
    if let Err(c) = emit_report(&rendered, output.as_deref()) {
        return c;
    }
    if findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn parse_epoch_or_timestamp(raw: &str) -> Option<i64> {
    let trimmed = raw.trim();
    trimmed
        .parse::<i64>()
        .ok()
        .or_else(|| AuditClock::parse(trimmed).map(|c| c.epoch_seconds))
}

fn detect_ecosystem_single(first: &str) -> Option<(&'static str, &'static str)> {
    if first.ends_with(".lookml") {
        return Some(("looker", "GRADE-LOOKER"));
    }
    if first.ends_with(".js") {
        return Some(("cube", "GRADE-CUBE"));
    }
    let text = std::fs::read_to_string(first).ok()?;
    if first.ends_with(".yaml") || first.ends_with(".yml") {
        if text.contains("cubes:") {
            return Some(("cube", "GRADE-CUBE"));
        }
        if text.contains("datasets:") {
            return Some(("ossie", "GRADE-OSSIE"));
        }
        if text.contains("tables:") || text.contains("verified_queries:") {
            return Some(("snowflake", "GRADE-SNOWFLAKE"));
        }
        return None;
    }
    if text.contains("\"semantic_models\"") {
        return Some(("dbt", "GRADE-DBT"));
    }
    if text.contains("\"datasets\"") {
        return Some(("ossie", "GRADE-OSSIE"));
    }
    if text.contains("\"data_sources\"") || text.contains("\"benchmarks\"") {
        return Some(("databricks", "GRADE-DATABRICKS"));
    }
    if text.contains("\"cubes\"") {
        return Some(("cube", "GRADE-CUBE"));
    }
    if text.contains("\"views\"") || text.contains("\"explores\"") {
        return Some(("looker", "GRADE-LOOKER"));
    }
    if text.contains("\"hierarchies\"") || text.contains("\"calculation_groups\"") {
        return Some(("atscale", "GRADE-ATSCALE"));
    }
    None
}

fn ecosystem_tag(name: &str) -> Option<(&'static str, &'static str)> {
    match name.trim().to_ascii_lowercase().as_str() {
        "dbt" => Some(("dbt", "GRADE-DBT")),
        "ossie" => Some(("ossie", "GRADE-OSSIE")),
        "snowflake" => Some(("snowflake", "GRADE-SNOWFLAKE")),
        "databricks" => Some(("databricks", "GRADE-DATABRICKS")),
        "cube" => Some(("cube", "GRADE-CUBE")),
        "looker" | "lookml" => Some(("looker", "GRADE-LOOKER")),
        "atscale" => Some(("atscale", "GRADE-ATSCALE")),
        _ => None,
    }
}

fn load_semantic_definition(
    paths: &[String],
    ecosystem: &str,
    changed_at: Option<i64>,
) -> Result<Definition, ExitCode> {
    let mut def: Definition = if ecosystem == "looker" && paths[0].ends_with(".lookml") {
        if paths.len() > 1 {
            let mut files = Vec::with_capacity(paths.len());
            for p in paths {
                match std::fs::read_to_string(p) {
                    Ok(text) => files.push((p.clone(), text)),
                    Err(err) => {
                        eprintln!("cannot read {p}: {err}");
                        return Err(ExitCode::from(2));
                    }
                }
            }
            perishables_semantic::read_lookml_project(&files).map_err(|err| {
                eprintln!("cannot parse LookML project: {err}");
                ExitCode::from(2)
            })?
        } else {
            read_single(&paths[0], perishables_semantic::read_lookml_definition)?
        }
    } else {
        let p = &paths[0];
        let reader: fn(&str) -> Result<Definition, String> = match ecosystem {
            "dbt" => perishables_semantic::read_definition,
            "ossie" => {
                if p.ends_with(".yaml") || p.ends_with(".yml") {
                    perishables_semantic::read_ossie_yaml
                } else {
                    perishables_semantic::read_ossie_definition
                }
            }
            "snowflake" => perishables_semantic::read_snowflake_definition,
            "databricks" => perishables_semantic::read_databricks_definition,
            "cube" => {
                if p.ends_with(".yaml") || p.ends_with(".yml") {
                    perishables_semantic::read_cube_yaml
                } else if p.ends_with(".js") {
                    perishables_semantic::read_cube_js
                } else {
                    perishables_semantic::read_cube_definition
                }
            }
            "looker" => perishables_semantic::read_looker_definition,
            "atscale" => perishables_semantic::read_atscale_definition,
            _ => unreachable!(),
        };
        read_single(p, reader)?
    };
    if ecosystem == "snowflake"
        && let Some(ts) = changed_at
    {
        def.changed_at = Some(ts);
    }
    Ok(def)
}

fn grade_unified(args: &[String]) -> ExitCode {
    if has_help_flag(args) {
        print!("{}", subcommand_help("grade").unwrap_or(ROOT_HELP));
        return ExitCode::SUCCESS;
    }
    let mut ecosystem_opt: Option<String> = None;
    let mut format = OutputFormat::Text;
    let mut output: Option<PathBuf> = None;
    let mut changed_at: Option<i64> = None;
    let mut paths: Vec<String> = Vec::new();
    let mut end_of_flags = false;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if !end_of_flags && arg == "--" {
            end_of_flags = true;
            i += 1;
            continue;
        }
        if !end_of_flags && arg.starts_with('-') && arg != "-" {
            let (flag, inline) = split_flag(arg);
            match flag {
                "--ecosystem" => match take_flag_value(flag, inline, args, &mut i) {
                    Ok(v) => ecosystem_opt = Some(v),
                    Err(c) => return c,
                },
                "--format" => match take_flag_value(flag, inline, args, &mut i) {
                    Ok(v) => match OutputFormat::parse(&v) {
                        Some(f) => format = f,
                        None => {
                            eprintln!(
                                "unknown --format {v} (expected text, json, sarif, or github)"
                            );
                            return ExitCode::from(2);
                        }
                    },
                    Err(c) => return c,
                },
                "--output" | "-o" => match take_flag_value(flag, inline, args, &mut i) {
                    Ok(v) => output = Some(PathBuf::from(v)),
                    Err(c) => return c,
                },
                "--changed-at" => match take_flag_value(flag, inline, args, &mut i) {
                    Ok(when) => match parse_epoch_or_timestamp(&when) {
                        Some(epoch) => changed_at = Some(epoch),
                        None => {
                            eprintln!("unparseable --changed-at {when}");
                            return ExitCode::from(2);
                        }
                    },
                    Err(c) => return c,
                },
                _ => {
                    eprintln!("unknown flag {flag}");
                    return ExitCode::from(2);
                }
            }
        } else {
            paths.push(arg.clone());
        }
        i += 1;
    }

    if paths.is_empty() {
        eprintln!("grade requires at least one definition file");
        return ExitCode::from(2);
    }

    let explicit_eco = match &ecosystem_opt {
        Some(raw_eco) => {
            let Some(pair) = ecosystem_tag(raw_eco) else {
                eprintln!(
                    "unknown --ecosystem {raw_eco} (expected dbt, ossie, snowflake, databricks, cube, looker, or atscale)"
                );
                return ExitCode::from(2);
            };
            Some(pair)
        }
        None => None,
    };

    if let Some((eco, _)) = explicit_eco
        && eco != "snowflake"
        && changed_at.is_some()
    {
        eprintln!("--changed-at is only supported for snowflake definitions");
        return ExitCode::from(2);
    }

    // Group LookML files together for project-scoped resolution while grading
    // each non-LookML definition file individually in deterministic argument order.
    let mut lookml_files: Vec<String> = Vec::new();
    let mut other_files: Vec<String> = Vec::new();
    for p in &paths {
        if p.ends_with(".lookml") && explicit_eco.is_none_or(|(e, _)| e == "looker") {
            lookml_files.push(p.clone());
        } else {
            other_files.push(p.clone());
        }
    }

    let mut batch: Vec<GradeBatchEntry> = Vec::new();
    if !lookml_files.is_empty() {
        let def = match load_semantic_definition(&lookml_files, "looker", None) {
            Ok(d) => d,
            Err(c) => return c,
        };
        let findings = perishables_semantic::grade(&def);
        let found_notes = perishables_semantic::notes(&def);
        batch.push(GradeBatchEntry {
            tag: "GRADE-LOOKER".to_string(),
            ecosystem: "looker".to_string(),
            primary_file: lookml_files[0].clone(),
            def,
            findings,
            notes: found_notes,
        });
    }

    for p in &other_files {
        let (eco, tag) = match explicit_eco {
            Some(pair) => pair,
            None => {
                let Some(pair) = detect_ecosystem_single(p) else {
                    eprintln!(
                        "could not infer semantic ecosystem for {p}; pass --ecosystem <name>"
                    );
                    return ExitCode::from(2);
                };
                pair
            }
        };
        let single = std::slice::from_ref(p);
        let def = match load_semantic_definition(single, eco, changed_at) {
            Ok(d) => d,
            Err(c) => return c,
        };
        let findings = perishables_semantic::grade(&def);
        let found_notes = perishables_semantic::notes(&def);
        batch.push(GradeBatchEntry {
            tag: tag.to_string(),
            ecosystem: eco.to_string(),
            primary_file: p.clone(),
            def,
            findings,
            notes: found_notes,
        });
    }

    let any_findings = batch.iter().any(|b| !b.findings.is_empty());
    let rendered = render_grade_batch(format, &batch);
    if let Err(c) = emit_report(&rendered, output.as_deref()) {
        return c;
    }
    if any_findings {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn grade_ecosystem(args: &[String], ecosystem: &'static str, tag: &'static str) -> ExitCode {
    let cmd_name = match ecosystem {
        "dbt" => "grade-dbt",
        "ossie" => "grade-ossie",
        "snowflake" => "grade-snowflake",
        "databricks" => "grade-databricks",
        "cube" => "grade-cube",
        "looker" => "grade-looker",
        "atscale" => "grade-atscale",
        _ => "grade",
    };
    if has_help_flag(args) {
        print!("{}", subcommand_help(cmd_name).unwrap_or(ROOT_HELP));
        return ExitCode::SUCCESS;
    }
    grade_ecosystem_inner(args, ecosystem, tag)
}

fn grade_ecosystem_inner(args: &[String], ecosystem: &str, tag: &str) -> ExitCode {
    let mut format = OutputFormat::Text;
    let mut output: Option<PathBuf> = None;
    let mut changed_at: Option<i64> = None;
    let mut paths: Vec<String> = Vec::new();
    let mut end_of_flags = false;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if !end_of_flags && arg == "--" {
            end_of_flags = true;
            i += 1;
            continue;
        }
        if !end_of_flags && arg.starts_with('-') && arg != "-" {
            let (flag, inline) = split_flag(arg);
            match flag {
                "--format" => match take_flag_value(flag, inline, args, &mut i) {
                    Ok(v) => match OutputFormat::parse(&v) {
                        Some(f) => format = f,
                        None => {
                            eprintln!(
                                "unknown --format {v} (expected text, json, sarif, or github)"
                            );
                            return ExitCode::from(2);
                        }
                    },
                    Err(c) => return c,
                },
                "--output" | "-o" => match take_flag_value(flag, inline, args, &mut i) {
                    Ok(v) => output = Some(PathBuf::from(v)),
                    Err(c) => return c,
                },
                "--changed-at" if ecosystem == "snowflake" => {
                    match take_flag_value(flag, inline, args, &mut i) {
                        Ok(when) => match parse_epoch_or_timestamp(&when) {
                            Some(epoch) => changed_at = Some(epoch),
                            None => {
                                eprintln!("unparseable --changed-at {when}");
                                return ExitCode::from(2);
                            }
                        },
                        Err(c) => return c,
                    }
                }
                _ => {
                    eprintln!("unknown flag {flag}");
                    return ExitCode::from(2);
                }
            }
        } else {
            paths.push(arg.clone());
        }
        i += 1;
    }

    if paths.is_empty() {
        eprintln!("{tag} requires a definition file");
        return ExitCode::from(2);
    }

    let allow_multi = ecosystem == "looker" && paths.iter().all(|p| p.ends_with(".lookml"));
    if paths.len() > 1 && !allow_multi {
        eprintln!("unexpected positional argument {}", paths[1]);
        return ExitCode::from(2);
    }

    let def = match load_semantic_definition(&paths, ecosystem, changed_at) {
        Ok(d) => d,
        Err(c) => return c,
    };

    let findings = perishables_semantic::grade(&def);
    let found_notes = perishables_semantic::notes(&def);
    let rendered = render_grade(
        format,
        tag,
        ecosystem,
        &paths[0],
        &def,
        &findings,
        &found_notes,
    );
    if let Err(c) = emit_report(&rendered, output.as_deref()) {
        return c;
    }
    if findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn read_single(
    path: &str,
    reader: fn(&str) -> Result<Definition, String>,
) -> Result<Definition, ExitCode> {
    let text = std::fs::read_to_string(path).map_err(|err| {
        eprintln!("cannot read {path}: {err}");
        ExitCode::from(2)
    })?;
    reader(&text).map_err(|err| {
        eprintln!("cannot parse {path}: {err}");
        ExitCode::from(2)
    })
}

#[derive(Default)]
struct ManifestEntry {
    name: String,
    taxonomy: String,
    expect: String,
    expected_finding: Option<String>,
    set: String,
}

struct ParsedManifest {
    corpus_version: String,
    as_of: String,
    fixtures: BTreeMap<String, ManifestEntry>,
}

fn parse_manifest(text: &str) -> Result<ParsedManifest, String> {
    let mut corpus_version = String::new();
    let mut as_of = String::new();
    let mut fixtures: BTreeMap<String, ManifestEntry> = BTreeMap::new();
    let mut current: Option<ManifestEntry> = None;

    for (line_no, raw_line) in text.lines().enumerate() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed == "[[fixture]]" {
            if let Some(entry) = current.take() {
                validate_manifest_entry(&entry)?;
                if fixtures.insert(entry.name.clone(), entry).is_some() {
                    return Err("duplicate [[fixture]] name in manifest.toml".to_string());
                }
            }
            current = Some(ManifestEntry {
                set: "gate".to_string(),
                ..ManifestEntry::default()
            });
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            return Err(format!(
                "malformed line {} in manifest.toml: {trimmed}",
                line_no + 1
            ));
        };
        let key = key.trim();
        let value = parse_toml_string(value.trim())
            .ok_or_else(|| format!("malformed string on line {} in manifest.toml", line_no + 1))?;
        if let Some(entry) = &mut current {
            match key {
                "name" => entry.name = value,
                "taxonomy" => entry.taxonomy = value,
                "expect" => entry.expect = value,
                "expected_finding" => entry.expected_finding = Some(value),
                "set" => entry.set = value,
                _ => {}
            }
        } else {
            match key {
                "corpus_version" => corpus_version = value,
                "as_of" => as_of = value,
                _ => {}
            }
        }
    }
    if let Some(entry) = current.take() {
        validate_manifest_entry(&entry)?;
        if fixtures.insert(entry.name.clone(), entry).is_some() {
            return Err("duplicate [[fixture]] name in manifest.toml".to_string());
        }
    }
    if corpus_version.is_empty() {
        return Err("manifest.toml missing corpus_version".to_string());
    }
    if as_of.is_empty() {
        return Err("manifest.toml missing as_of".to_string());
    }
    if fixtures.is_empty() {
        return Err("manifest.toml lists zero [[fixture]] entries".to_string());
    }
    Ok(ParsedManifest {
        corpus_version,
        as_of,
        fixtures,
    })
}

fn validate_manifest_entry(entry: &ManifestEntry) -> Result<(), String> {
    if entry.name.is_empty() {
        return Err("manifest [[fixture]] missing name".to_string());
    }
    match entry.expect.as_str() {
        "pass" => Ok(()),
        "fail" => {
            if entry.expected_finding.as_deref().is_none_or(str::is_empty) {
                Err(format!(
                    "manifest fixture {} has expect=\"fail\" without expected_finding",
                    entry.name
                ))
            } else {
                Ok(())
            }
        }
        other => Err(format!(
            "manifest fixture {} has invalid expect={other:?}",
            entry.name
        )),
    }
}

fn parse_toml_string(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
        Some(raw[1..raw.len() - 1].replace("\\\"", "\""))
    } else {
        None
    }
}

struct Label {
    fixture: String,
    taxonomy: String,
    expect: String,
    expected_finding: Option<String>,
    concept: Option<String>,
    set: String,
    expected_raw_tier: Option<String>,
    expected_recomputed_tier: Option<String>,
    expected_recomputed_result: Option<String>,
    claimed_result: Option<String>,
    slug_tiers: Vec<(String, String)>,
}

fn parse_label(text: &str, dir_name: &str) -> Result<Label, String> {
    let mut label = Label {
        fixture: String::new(),
        taxonomy: String::new(),
        expect: String::new(),
        expected_finding: None,
        concept: None,
        set: "gate".to_string(),
        expected_raw_tier: None,
        expected_recomputed_tier: None,
        expected_recomputed_result: None,
        claimed_result: None,
        slug_tiers: Vec::new(),
    };
    for (line_no, raw_line) in text.lines().enumerate() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            return Err(format!(
                "malformed line {} in {dir_name}/LABEL.toml",
                line_no + 1
            ));
        };
        let key = key.trim();
        let value = parse_toml_string(value.trim()).ok_or_else(|| {
            format!(
                "malformed string value on line {} in {dir_name}/LABEL.toml",
                line_no + 1
            )
        })?;
        match key {
            "fixture" => label.fixture = value,
            "taxonomy" => label.taxonomy = value,
            "expect" => label.expect = value,
            "expected_finding" => label.expected_finding = Some(value),
            "concept" => label.concept = Some(value),
            "set" => label.set = value,
            "expected_raw_tier" => label.expected_raw_tier = Some(value),
            "expected_recomputed_tier" => label.expected_recomputed_tier = Some(value),
            "expected_recomputed_result" => label.expected_recomputed_result = Some(value),
            "claimed_result" => label.claimed_result = Some(value),
            other => {
                if let Some(slug) = other.strip_prefix("expected_tier_") {
                    label.slug_tiers.push((slug.to_string(), value));
                }
            }
        }
    }
    if label.fixture != dir_name {
        return Err(format!(
            "LABEL.toml fixture={:?} does not match directory {dir_name:?}",
            label.fixture
        ));
    }
    match label.expect.as_str() {
        "pass" => {}
        "fail" => {
            if label.expected_finding.as_deref().is_none_or(str::is_empty) {
                return Err(format!(
                    "{dir_name}/LABEL.toml has expect=\"fail\" without expected_finding"
                ));
            }
        }
        other => {
            return Err(format!(
                "{dir_name}/LABEL.toml has invalid expect={other:?}"
            ));
        }
    }
    Ok(label)
}

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
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_symlink() {
            continue;
        }
        let path = entry.path();
        if ft.is_dir() {
            collect_tree(root, &path, out);
        } else if ft.is_file()
            && let Ok(rel) = path.strip_prefix(root)
        {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn corpus(args: &[String]) -> ExitCode {
    if has_help_flag(args) {
        print!("{}", subcommand_help("corpus").unwrap_or(ROOT_HELP));
        return ExitCode::SUCCESS;
    }
    let mut format = OutputFormat::Text;
    let mut output: Option<PathBuf> = None;
    let mut positionals: Vec<String> = Vec::new();
    let mut end_of_flags = false;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if !end_of_flags && arg == "--" {
            end_of_flags = true;
            i += 1;
            continue;
        }
        if !end_of_flags && arg.starts_with('-') && arg != "-" {
            let (flag, inline) = split_flag(arg);
            match flag {
                "--format" => match take_flag_value(flag, inline, args, &mut i) {
                    Ok(v) => match OutputFormat::parse(&v) {
                        Some(f @ (OutputFormat::Text | OutputFormat::Json)) => format = f,
                        _ => {
                            eprintln!("unknown --format {v} for corpus (expected text or json)");
                            return ExitCode::from(2);
                        }
                    },
                    Err(c) => return c,
                },
                "--output" | "-o" => match take_flag_value(flag, inline, args, &mut i) {
                    Ok(v) => output = Some(PathBuf::from(v)),
                    Err(c) => return c,
                },
                _ => {
                    eprintln!("unknown flag {flag}");
                    return ExitCode::from(2);
                }
            }
        } else {
            positionals.push(arg.clone());
        }
        i += 1;
    }
    if positionals.len() > 1 {
        eprintln!("unexpected positional argument {}", positionals[1]);
        return ExitCode::from(2);
    }

    let dir = positionals
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("corpus/fixtures"));
    let manifest_path = dir
        .parent()
        .map(|parent| parent.join("manifest.toml"))
        .unwrap_or_else(|| PathBuf::from("corpus/manifest.toml"));
    let manifest_text = match std::fs::read_to_string(&manifest_path) {
        Ok(t) => t,
        Err(err) => {
            eprintln!(
                "cannot read corpus manifest {}: {err}",
                manifest_path.display()
            );
            return ExitCode::from(2);
        }
    };
    let manifest = match parse_manifest(&manifest_text) {
        Ok(m) => m,
        Err(err) => {
            eprintln!("invalid corpus manifest {}: {err}", manifest_path.display());
            return ExitCode::from(2);
        }
    };
    let Some(clock) = AuditClock::parse(&manifest.as_of) else {
        eprintln!("corpus manifest as_of {} is not a date", manifest.as_of);
        return ExitCode::from(2);
    };

    let mut entries: Vec<PathBuf> = match std::fs::read_dir(&dir) {
        Ok(read) => read
            .flatten()
            .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
            .map(|e| e.path())
            .collect(),
        Err(err) => {
            eprintln!("cannot read fixtures dir {}: {err}", dir.display());
            return ExitCode::from(2);
        }
    };
    entries.sort();

    let disk_names: BTreeSet<String> = entries
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .collect();
    let manifest_names: BTreeSet<String> = manifest.fixtures.keys().cloned().collect();
    if disk_names != manifest_names {
        let missing_on_disk: Vec<&String> = manifest_names.difference(&disk_names).collect();
        let unlisted_on_disk: Vec<&String> = disk_names.difference(&manifest_names).collect();
        if !missing_on_disk.is_empty() {
            eprintln!("manifest fixtures missing on disk: {missing_on_disk:?}");
        }
        if !unlisted_on_disk.is_empty() {
            eprintln!("fixture directories unlisted in manifest.toml: {unlisted_on_disk:?}");
        }
        return ExitCode::from(2);
    }

    let exec_cfg = ExecConfig::from_env();
    let mut summaries: Vec<CorpusFixtureSummary> = Vec::with_capacity(entries.len());
    let mut failures = 0usize;

    for fixture in entries {
        let name = fixture
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let label_path = fixture.join("LABEL.toml");
        let label_text = match std::fs::read_to_string(&label_path) {
            Ok(t) => t,
            Err(err) => {
                eprintln!("cannot read {}: {err}", label_path.display());
                return ExitCode::from(2);
            }
        };
        let label = match parse_label(&label_text, &name) {
            Ok(l) => l,
            Err(err) => {
                eprintln!("{err}");
                return ExitCode::from(2);
            }
        };
        let manifest_entry = &manifest.fixtures[&name];
        if label.expect != manifest_entry.expect
            || label.expected_finding != manifest_entry.expected_finding
            || (!manifest_entry.taxonomy.is_empty() && label.taxonomy != manifest_entry.taxonomy)
        {
            eprintln!("fixture {name} LABEL.toml disagrees with manifest.toml");
            return ExitCode::from(2);
        }

        let before = hash_tree(&fixture);
        let bundle = match Bundle::load(&fixture) {
            Ok(bundle) => bundle,
            Err(err) => {
                eprintln!("cannot load fixture {name}: {err}");
                return ExitCode::from(2);
            }
        };
        let reports = run_computations(&bundle, &exec_cfg);
        let findings = default_findings(&bundle, &clock, &reports);
        let after = hash_tree(&fixture);

        let codes: BTreeSet<String> = findings
            .iter()
            .map(|f| f.code.label().to_string())
            .collect();
        let expected: BTreeSet<String> = match (&label.expect, &label.expected_finding) {
            (expect, Some(code)) if expect == "fail" => BTreeSet::from([code.clone()]),
            _ => BTreeSet::new(),
        };

        let mut diagnostics = Vec::new();
        if codes != expected {
            diagnostics.push(format!("EXPECTED codes={expected:?} got={codes:?}"));
        }

        if let (Some(concept_id), Some(expected_code)) = (&label.concept, &label.expected_finding)
            && !findings
                .iter()
                .any(|f| &f.concept_id == concept_id && f.code.label() == expected_code)
        {
            diagnostics.push(format!(
                "EXPECTED finding {expected_code} on concept {concept_id}"
            ));
        }

        if let Some(concept_id) = &label.concept {
            if label.expected_raw_tier.is_some() || label.expected_recomputed_tier.is_some() {
                match bundle.concept(concept_id) {
                    Some(concept) => {
                        let tr = recompute_tier(concept, &clock);
                        if let Some(exp_raw) = &label.expected_raw_tier
                            && tr.raw_tier.name() != exp_raw
                        {
                            diagnostics.push(format!(
                                "EXPECTED raw_tier={exp_raw} got={}",
                                tr.raw_tier.name()
                            ));
                        }
                        if let Some(exp_rec) = &label.expected_recomputed_tier
                            && tr.recomputed_tier.name() != exp_rec
                        {
                            diagnostics.push(format!(
                                "EXPECTED recomputed_tier={exp_rec} got={}",
                                tr.recomputed_tier.name()
                            ));
                        }
                    }
                    None => diagnostics.push(format!("MISSING labelled concept {concept_id}")),
                }
            }
            if let Some(exp_res) = &label.expected_recomputed_result
                && !reports.iter().any(|r| {
                    &r.concept_id == concept_id && r.recomputed.as_deref() == Some(exp_res)
                })
            {
                diagnostics.push(format!("EXPECTED recomputed_result={exp_res}"));
            }
            if let Some(exp_claim) = &label.claimed_result
                && !reports
                    .iter()
                    .any(|r| &r.concept_id == concept_id && r.claimed.as_deref() == Some(exp_claim))
            {
                diagnostics.push(format!("EXPECTED claimed_result={exp_claim}"));
            }
        }

        for (slug, exp_tier) in &label.slug_tiers {
            let matched = bundle
                .concepts
                .iter()
                .find(|c| c.id.replace(['/', '-'], "_") == *slug);
            match matched {
                Some(c) => {
                    let tr = recompute_tier(c, &clock);
                    if tr.recomputed_tier.name() != exp_tier {
                        diagnostics.push(format!(
                            "EXPECTED tier for {}={exp_tier} got={}",
                            c.id,
                            tr.recomputed_tier.name()
                        ));
                    }
                }
                None => diagnostics.push(format!("MISSING concept matching slug {slug}")),
            }
        }

        let unchanged = before == after;
        if !unchanged {
            diagnostics.push("MUTATED the audit changed the fixture bytes".to_string());
        }

        let pass = diagnostics.is_empty();
        if !pass {
            failures += 1;
        }
        summaries.push(CorpusFixtureSummary {
            name,
            set: label.set,
            pass,
            unchanged,
            findings,
            diagnostics,
        });
    }

    let rendered = render_corpus(
        format,
        &manifest.corpus_version,
        &manifest.as_of,
        &summaries,
        failures,
    );
    if let Err(c) = emit_report(&rendered, output.as_deref()) {
        return c;
    }
    if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
