"""Typed Python interface and bundled CLI wrapper for ``perishables``."""

from __future__ import annotations

from dataclasses import dataclass
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import sysconfig
from typing import Any, Sequence

__all__ = [
    "__version__",
    "AuditReport",
    "ComputationRecord",
    "CorpusFixtureRecord",
    "CorpusReport",
    "DriftReport",
    "FindingRecord",
    "GradeReport",
    "NoteRecord",
    "PerishablesError",
    "SemanticCounts",
    "TierRecord",
    "audit",
    "audit_sarif",
    "completions",
    "corpus",
    "drift",
    "find_binary",
    "grade",
    "grade_atscale",
    "grade_cube",
    "grade_databricks",
    "grade_dbt",
    "grade_looker",
    "grade_ossie",
    "grade_sarif",
    "grade_snowflake",
    "version",
]

__version__ = "0.1.2"

PathInput = str | os.PathLike[str]


class PerishablesError(RuntimeError):
    """Raised on CLI usage errors or when ``raise_for_findings`` trips."""

    def __init__(self, message: str, *, exit_code: int = 2) -> None:
        super().__init__(message)
        self.exit_code = exit_code


@dataclass(frozen=True, slots=True)
class TierRecord:
    """Recomputed trust tier for one OKF concept."""

    concept_id: str
    raw_tier: str
    recomputed_tier: str


@dataclass(frozen=True, slots=True)
class ComputationRecord:
    """Re-execution outcome for an OKF Attested Computation block."""

    concept_id: str
    verdict: str
    reason: str | None
    claimed: str | None
    recomputed: str | None
    artifact_hash: str | None
    timeout_ms: int | None
    elapsed_ms: int | None = None


@dataclass(frozen=True, slots=True)
class FindingRecord:
    """One deterministic trust or semantic integrity finding."""

    code: str
    detail: str
    concept_id: str | None = None
    item: str | None = None


@dataclass(frozen=True, slots=True)
class NoteRecord:
    """Informational note emitted during bundle auditing or semantic grading."""

    text: str
    concept_id: str | None = None
    item: str | None = None


@dataclass(frozen=True, slots=True)
class AuditReport:
    """Structured report produced by ``perishables audit``."""

    schema_version: str
    tool_version: str
    bundle: str
    as_of: str
    result: str
    concepts_count: int
    tiers: tuple[TierRecord, ...]
    computations: tuple[ComputationRecord, ...]
    notes: tuple[NoteRecord, ...]
    findings: tuple[FindingRecord, ...]
    exit_code: int

    @property
    def passed(self) -> bool:
        """True when zero trust findings were emitted."""
        return self.result == "PASS" and self.exit_code == 0

    def raise_for_findings(self) -> None:
        """Raise ``PerishablesError`` when any finding was convicted."""
        if not self.passed:
            codes = ", ".join(f.code for f in self.findings) or "unknown"
            raise PerishablesError(
                f"perishables audit failed on {self.bundle} ({len(self.findings)} findings: {codes})",
                exit_code=self.exit_code,
            )


@dataclass(frozen=True, slots=True)
class DriftReport:
    """Structured report produced by ``perishables drift``."""

    schema_version: str
    tool_version: str
    bundle: str
    result: str
    findings: tuple[FindingRecord, ...]
    exit_code: int

    @property
    def clean(self) -> bool:
        """True when all recorded source SHA-256 hashes match on disk."""
        return self.result == "NONE" and self.exit_code == 0

    def raise_for_findings(self) -> None:
        """Raise ``PerishablesError`` when source drift was detected."""
        if not self.clean:
            raise PerishablesError(
                f"perishables drift detected {len(self.findings)} drifted source(s) in {self.bundle}",
                exit_code=self.exit_code,
            )


@dataclass(frozen=True, slots=True)
class SemanticCounts:
    """Normalized entity counts across a graded semantic definition."""

    models: int
    measures: int
    dimensions: int
    metrics: int


@dataclass(frozen=True, slots=True)
class GradeReport:
    """Structured report produced by ``perishables grade`` or ``grade-*``."""

    schema_version: str
    tool_version: str
    command: str
    ecosystem: str | None
    result: str
    counts: SemanticCounts | None
    findings: tuple[FindingRecord, ...]
    notes: tuple[NoteRecord, ...]
    reports: tuple["GradeReport", ...]
    exit_code: int

    @property
    def passed(self) -> bool:
        """True when zero semantic grading findings were emitted."""
        return self.result == "PASS" and self.exit_code == 0

    def raise_for_findings(self) -> None:
        """Raise ``PerishablesError`` when semantic grading convicted findings."""
        if not self.passed:
            all_findings = self.findings or tuple(
                f for sub in self.reports for f in sub.findings
            )
            codes = ", ".join(f.code for f in all_findings) or "unknown"
            raise PerishablesError(
                f"perishables {self.command} failed ({len(all_findings)} findings: {codes})",
                exit_code=self.exit_code,
            )


@dataclass(frozen=True, slots=True)
class CorpusFixtureRecord:
    """Outcome for one fixture inside ``perishables corpus``."""

    name: str
    set: str
    status: str
    unchanged: bool
    findings: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class CorpusReport:
    """Structured report produced by ``perishables corpus``."""

    schema_version: str
    tool_version: str
    corpus_version: str
    as_of: str
    result: str
    fixtures_count: int
    failures_count: int
    fixtures: tuple[CorpusFixtureRecord, ...]
    exit_code: int

    @property
    def passed(self) -> bool:
        """True when every reference fixture matched its planted labels."""
        return self.result == "PASS" and self.failures_count == 0 and self.exit_code == 0


def find_binary(binary: PathInput | None = None) -> Path:
    """Locate the bundled or installed ``perishables`` executable."""
    if binary is not None:
        candidate = Path(binary)
        if candidate.is_file():
            return candidate
        raise FileNotFoundError(f"perishables binary not found at {candidate}")

    env_override = os.environ.get("PERISHABLES_BIN", "").strip()
    if env_override:
        candidate = Path(env_override)
        if candidate.is_file():
            return candidate
        raise FileNotFoundError(
            f"PERISHABLES_BIN points to missing file {candidate}"
        )

    exe_names = (
        ("perishables.exe", "perishables")
        if os.name == "nt"
        else ("perishables",)
    )
    search_dirs: list[Path] = []
    scripts_dir = sysconfig.get_path("scripts")
    if scripts_dir:
        search_dirs.append(Path(scripts_dir))
    py_dir = Path(sys.executable).resolve().parent
    search_dirs.extend([py_dir, py_dir / "Scripts", py_dir / "bin"])

    repo_root = Path(__file__).resolve().parents[2]
    search_dirs.extend(
        [
            repo_root / "target" / "release",
            repo_root / "target" / "debug",
        ]
    )

    for directory in search_dirs:
        for name in exe_names:
            candidate = directory / name
            if candidate.is_file():
                return candidate

    found = shutil.which("perishables")
    if found:
        return Path(found)

    raise FileNotFoundError(
        "could not locate the perishables executable; install the wheel or set PERISHABLES_BIN"
    )


def _normalize_paths(paths: PathInput | Sequence[PathInput], caller: str) -> list[str]:
    if isinstance(paths, (str, os.PathLike)):
        file_args = [os.fspath(paths)]
    else:
        file_args = [os.fspath(p) for p in paths]
    if not file_args:
        raise PerishablesError(f"{caller} requires at least one path", exit_code=2)
    return file_args


def _invoke_json(
    args: Sequence[str],
    *,
    binary: PathInput | None = None,
) -> tuple[dict[str, Any], int]:
    exe = find_binary(binary)
    completed = subprocess.run(
        [str(exe), *args],
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    if completed.returncode not in (0, 1):
        detail = completed.stderr.strip() or completed.stdout.strip() or "command failed"
        raise PerishablesError(detail, exit_code=completed.returncode)
    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise PerishablesError(
            f"failed to decode JSON output from perishables: {exc}",
            exit_code=completed.returncode,
        ) from exc
    return payload, completed.returncode


def _invoke_sarif(
    args: Sequence[str],
    *,
    output: PathInput | None = None,
    binary: PathInput | None = None,
) -> dict[str, Any]:
    exe = find_binary(binary)
    cmd = list(args)
    out_path: Path | None = None
    if output is not None:
        out_path = Path(os.fspath(output))
        cmd.extend(["--output", os.fspath(out_path)])
    completed = subprocess.run(
        [str(exe), *cmd],
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    if completed.returncode not in (0, 1):
        detail = completed.stderr.strip() or completed.stdout.strip() or "command failed"
        raise PerishablesError(detail, exit_code=completed.returncode)
    try:
        raw = (
            out_path.read_text(encoding="utf-8")
            if out_path is not None
            else completed.stdout
        )
        payload = json.loads(raw)
    except (OSError, json.JSONDecodeError) as exc:
        raise PerishablesError(
            f"failed to decode SARIF output from perishables: {exc}",
            exit_code=completed.returncode,
        ) from exc
    if not isinstance(payload, dict):
        raise PerishablesError(
            "expected SARIF JSON object from perishables",
            exit_code=completed.returncode,
        )
    return payload


def version(*, binary: PathInput | None = None) -> str:
    """Return the version string reported by ``perishables --version``."""
    exe = find_binary(binary)
    completed = subprocess.run(
        [str(exe), "--version"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    if completed.returncode != 0:
        raise PerishablesError(
            completed.stderr.strip() or "failed to query perishables version",
            exit_code=completed.returncode,
        )
    parts = completed.stdout.strip().split()
    return parts[-1] if parts else completed.stdout.strip()


def audit(
    bundle_path: PathInput,
    *,
    as_of: str,
    no_exec: bool = False,
    timing: bool = False,
    binary: PathInput | None = None,
) -> AuditReport:
    """Audit an Open Knowledge Format bundle at a pinned ``as_of`` instant."""
    cmd = ["audit", os.fspath(bundle_path), "--as-of", as_of, "--format", "json"]
    if no_exec:
        cmd.append("--no-exec")
    if timing:
        cmd.append("--timing")
    data, code = _invoke_json(cmd, binary=binary)
    tiers = tuple(
        TierRecord(
            concept_id=row["concept_id"],
            raw_tier=row["raw_tier"],
            recomputed_tier=row["recomputed_tier"],
        )
        for row in data.get("tiers", ())
    )
    computations = tuple(
        ComputationRecord(
            concept_id=row["concept_id"],
            verdict=row["verdict"],
            reason=row.get("reason"),
            claimed=row.get("claimed"),
            recomputed=row.get("recomputed"),
            artifact_hash=row.get("artifact_hash"),
            timeout_ms=row.get("timeout_ms"),
            elapsed_ms=row.get("elapsed_ms"),
        )
        for row in data.get("computations", ())
    )
    notes = tuple(
        NoteRecord(text=row["text"], concept_id=row.get("concept_id"))
        for row in data.get("notes", ())
    )
    findings = tuple(
        FindingRecord(
            code=row["code"],
            detail=row["detail"],
            concept_id=row.get("concept_id"),
        )
        for row in data.get("findings", ())
    )
    return AuditReport(
        schema_version=data["schema_version"],
        tool_version=data["tool_version"],
        bundle=data["bundle"],
        as_of=data["as_of"],
        result=data["result"],
        concepts_count=int(data["concepts_count"]),
        tiers=tiers,
        computations=computations,
        notes=notes,
        findings=findings,
        exit_code=code,
    )


def audit_sarif(
    bundle: PathInput,
    *,
    as_of: str,
    no_exec: bool = False,
    output: PathInput | None = None,
    binary: PathInput | None = None,
) -> dict[str, Any]:
    """Audit an OKF bundle and return the parsed SARIF 2.1.0 report dictionary."""
    cmd = ["audit", os.fspath(bundle), "--as-of", as_of, "--format", "sarif"]
    if no_exec:
        cmd.append("--no-exec")
    return _invoke_sarif(cmd, output=output, binary=binary)


def drift(
    bundle_path: PathInput,
    *,
    binary: PathInput | None = None,
) -> DriftReport:
    """Verify SHA-256 content hashes of in-bundle OKF sources."""
    data, code = _invoke_json(
        ["drift", os.fspath(bundle_path), "--format", "json"],
        binary=binary,
    )
    findings = tuple(
        FindingRecord(
            code=row["code"],
            detail=row["detail"],
            concept_id=row.get("concept_id"),
        )
        for row in data.get("findings", ())
    )
    return DriftReport(
        schema_version=data["schema_version"],
        tool_version=data["tool_version"],
        bundle=data["bundle"],
        result=data["result"],
        findings=findings,
        exit_code=code,
    )


def _parse_grade_payload(data: dict[str, Any], exit_code: int) -> GradeReport:
    raw_counts = data.get("counts")
    counts = (
        SemanticCounts(
            models=int(raw_counts["models"]),
            measures=int(raw_counts["measures"]),
            dimensions=int(raw_counts["dimensions"]),
            metrics=int(raw_counts["metrics"]),
        )
        if isinstance(raw_counts, dict)
        else None
    )
    findings = tuple(
        FindingRecord(
            code=row["code"],
            detail=row["detail"],
            item=row.get("item"),
        )
        for row in data.get("findings", ())
    )
    notes = tuple(
        NoteRecord(text=row["text"], item=row.get("item"))
        for row in data.get("notes", ())
    )
    sub_reports = tuple(
        _parse_grade_payload(sub, 1 if sub.get("result") == "FAIL" else 0)
        for sub in data.get("reports", ())
    )
    return GradeReport(
        schema_version=data["schema_version"],
        tool_version=data["tool_version"],
        command=data["command"],
        ecosystem=data.get("ecosystem"),
        result=data["result"],
        counts=counts,
        findings=findings,
        notes=notes,
        reports=sub_reports,
        exit_code=exit_code,
    )


def grade(
    paths: PathInput | Sequence[PathInput],
    *,
    ecosystem: str | None = None,
    changed_at: str | None = None,
    binary: PathInput | None = None,
) -> GradeReport:
    """Grade one or more semantic-layer definitions across supported ecosystems."""
    file_args = _normalize_paths(paths, "grade")
    cmd = ["grade", *file_args, "--format", "json"]
    if ecosystem is not None:
        cmd.extend(["--ecosystem", ecosystem])
    if changed_at is not None:
        cmd.extend(["--changed-at", changed_at])
    data, code = _invoke_json(cmd, binary=binary)
    return _parse_grade_payload(data, code)


def grade_dbt(
    manifest: PathInput,
    *,
    binary: PathInput | None = None,
) -> GradeReport:
    """Grade a dbt semantic manifest (``manifest.json`` or ``semantic_manifest.json``)."""
    data, code = _invoke_json(
        ["grade-dbt", os.fspath(manifest), "--format", "json"],
        binary=binary,
    )
    return _parse_grade_payload(data, code)


def grade_ossie(
    model: PathInput,
    *,
    binary: PathInput | None = None,
) -> GradeReport:
    """Grade an Apache Ossie model (``.json`` or ``.yaml``)."""
    data, code = _invoke_json(
        ["grade-ossie", os.fspath(model), "--format", "json"],
        binary=binary,
    )
    return _parse_grade_payload(data, code)


def grade_snowflake(
    view: PathInput,
    *,
    changed_at: str | None = None,
    binary: PathInput | None = None,
) -> GradeReport:
    """Grade a Snowflake semantic view YAML definition."""
    cmd = ["grade-snowflake", os.fspath(view), "--format", "json"]
    if changed_at is not None:
        cmd.extend(["--changed-at", changed_at])
    data, code = _invoke_json(cmd, binary=binary)
    return _parse_grade_payload(data, code)


def grade_databricks(
    space: PathInput,
    *,
    binary: PathInput | None = None,
) -> GradeReport:
    """Grade a Databricks Genie serialized space JSON definition."""
    data, code = _invoke_json(
        ["grade-databricks", os.fspath(space), "--format", "json"],
        binary=binary,
    )
    return _parse_grade_payload(data, code)


def grade_cube(
    paths: PathInput | Sequence[PathInput],
    *,
    binary: PathInput | None = None,
) -> GradeReport:
    """Grade one or more Cube meta JSON, YAML, or JS model files."""
    file_args = _normalize_paths(paths, "grade_cube")
    if len(file_args) == 1:
        cmd = ["grade-cube", file_args[0], "--format", "json"]
    else:
        cmd = ["grade", *file_args, "--ecosystem", "cube", "--format", "json"]
    data, code = _invoke_json(cmd, binary=binary)
    return _parse_grade_payload(data, code)


def grade_looker(
    paths: PathInput | Sequence[PathInput],
    *,
    binary: PathInput | None = None,
) -> GradeReport:
    """Grade a Looker API model JSON or one or more native LookML files."""
    file_args = _normalize_paths(paths, "grade_looker")
    if len(file_args) == 1 or all(p.endswith(".lookml") for p in file_args):
        cmd = ["grade-looker", *file_args, "--format", "json"]
    else:
        cmd = ["grade", *file_args, "--ecosystem", "looker", "--format", "json"]
    data, code = _invoke_json(cmd, binary=binary)
    return _parse_grade_payload(data, code)


def grade_atscale(
    model: PathInput,
    *,
    binary: PathInput | None = None,
) -> GradeReport:
    """Grade an AtScale semantic model export JSON definition."""
    data, code = _invoke_json(
        ["grade-atscale", os.fspath(model), "--format", "json"],
        binary=binary,
    )
    return _parse_grade_payload(data, code)


def grade_sarif(
    paths: PathInput | Sequence[PathInput],
    *,
    ecosystem: str | None = None,
    changed_at: str | None = None,
    output: PathInput | None = None,
    binary: PathInput | None = None,
) -> dict[str, Any]:
    """Grade semantic definitions and return the parsed SARIF 2.1.0 report dictionary."""
    file_args = _normalize_paths(paths, "grade_sarif")
    cmd = ["grade", *file_args, "--format", "sarif"]
    if ecosystem is not None:
        cmd.extend(["--ecosystem", ecosystem])
    if changed_at is not None:
        cmd.extend(["--changed-at", changed_at])
    return _invoke_sarif(cmd, output=output, binary=binary)


def corpus(
    fixtures_dir: PathInput | None = None,
    *,
    binary: PathInput | None = None,
) -> CorpusReport:
    """Run the planted-defect reference corpus gate."""
    cmd = ["corpus"]
    if fixtures_dir is not None:
        cmd.append(os.fspath(fixtures_dir))
    cmd.extend(["--format", "json"])
    data, code = _invoke_json(cmd, binary=binary)
    fixtures = tuple(
        CorpusFixtureRecord(
            name=row["name"],
            set=row["set"],
            status=row["status"],
            unchanged=bool(row["unchanged"]),
            findings=tuple(row.get("findings", ())),
        )
        for row in data.get("fixtures", ())
    )
    return CorpusReport(
        schema_version=data["schema_version"],
        tool_version=data["tool_version"],
        corpus_version=data["corpus_version"],
        as_of=data["as_of"],
        result=data["result"],
        fixtures_count=int(data["fixtures_count"]),
        failures_count=int(data["failures_count"]),
        fixtures=fixtures,
        exit_code=code,
    )


_BASH_COMPLETIONS = """_perishables() {
  local cur prev cmd i word
  COMPREPLY=()
  cur="${COMP_WORDS[COMP_CWORD]}"
  prev=""
  if [ "${COMP_CWORD}" -gt 0 ]; then
    prev="${COMP_WORDS[COMP_CWORD-1]}"
  fi

  local commands="audit corpus drift grade grade-dbt grade-ossie grade-snowflake grade-databricks grade-cube grade-looker grade-atscale help"
  local ecosystems="dbt ossie snowflake databricks cube looker atscale"
  local formats="text json sarif github"

  cmd=""
  i=1
  while [ "${i}" -lt "${COMP_CWORD}" ]; do
    word="${COMP_WORDS[i]}"
    case "${word}" in
      --)
        break
        ;;
      -*)
        ;;
      *)
        cmd="${word}"
        break
        ;;
    esac
    i=$((i + 1))
  done

  if [ -z "${cmd}" ]; then
    case "${cur}" in
      -*)
        COMPREPLY=( $(compgen -W "-h --help -V --version" -- "${cur}") )
        ;;
      *)
        COMPREPLY=( $(compgen -W "${commands}" -- "${cur}") )
        ;;
    esac
    return 0
  fi

  case "${prev}" in
    --format)
      if [ "${cmd}" = "corpus" ]; then
        COMPREPLY=( $(compgen -W "text json" -- "${cur}") )
      else
        COMPREPLY=( $(compgen -W "${formats}" -- "${cur}") )
      fi
      return 0
      ;;
    --ecosystem)
      COMPREPLY=( $(compgen -W "${ecosystems}" -- "${cur}") )
      return 0
      ;;
    --output|-o)
      COMPREPLY=( $(compgen -f -- "${cur}") )
      return 0
      ;;
    --as-of|--changed-at)
      return 0
      ;;
  esac

  case "${cmd}" in
    audit)
      if [[ "${cur}" == -* ]]; then
        COMPREPLY=( $(compgen -W "--as-of --format --output -o --no-exec --timing -h --help" -- "${cur}") )
      else
        COMPREPLY=( $(compgen -d -- "${cur}") )
      fi
      ;;
    corpus|drift)
      if [[ "${cur}" == -* ]]; then
        COMPREPLY=( $(compgen -W "--format --output -o -h --help" -- "${cur}") )
      else
        COMPREPLY=( $(compgen -d -- "${cur}") )
      fi
      ;;
    grade)
      if [[ "${cur}" == -* ]]; then
        COMPREPLY=( $(compgen -W "--ecosystem --changed-at --format --output -o -h --help" -- "${cur}") )
      else
        COMPREPLY=( $(compgen -f -- "${cur}") )
      fi
      ;;
    grade-snowflake)
      if [[ "${cur}" == -* ]]; then
        COMPREPLY=( $(compgen -W "--changed-at --format --output -o -h --help" -- "${cur}") )
      else
        COMPREPLY=( $(compgen -f -- "${cur}") )
      fi
      ;;
    grade-dbt|grade-ossie|grade-databricks|grade-cube|grade-looker|grade-atscale)
      if [[ "${cur}" == -* ]]; then
        COMPREPLY=( $(compgen -W "--format --output -o -h --help" -- "${cur}") )
      else
        COMPREPLY=( $(compgen -f -- "${cur}") )
      fi
      ;;
    help)
      COMPREPLY=( $(compgen -W "audit corpus drift grade grade-dbt grade-ossie grade-snowflake grade-databricks grade-cube grade-looker grade-atscale" -- "${cur}") )
      ;;
  esac
  return 0
}

complete -F _perishables perishables
"""

_ZSH_COMPLETIONS = r"""#compdef perishables

_perishables() {
  local context state state_descr line
  typeset -A opt_args

  local -a commands
  commands=(
    'audit:Audit an Open Knowledge Format bundle at a pinned as-of date'
    'corpus:Run the planted-defect reference corpus gate'
    'drift:Check SHA-256 source drift across an OKF bundle'
    'grade:Grade semantic-layer definitions with --ecosystem or auto-detection'
    'grade-dbt:Grade a dbt semantic manifest (manifest.json or semantic_manifest.json)'
    'grade-ossie:Grade an Apache Ossie model (.json or .yaml)'
    'grade-snowflake:Grade a Snowflake semantic view (.yaml)'
    'grade-databricks:Grade a Databricks Genie serialized space (.json)'
    'grade-cube:Grade a Cube model (meta .json, .yaml, or .js)'
    'grade-looker:Grade a Looker API model (.json) or native LookML project (.lookml)'
    'grade-atscale:Grade an AtScale semantic model export (.json)'
    'help:Print help for a subcommand'
  )

  _arguments -C \
    '(-h --help)'{-h,--help}'[Print help information]' \
    '(-V --version)'{-V,--version}'[Print version information]' \
    '1:command:->cmd' \
    '*::arg:->args' && return 0

  case "${state}" in
    cmd)
      _describe -t commands 'perishables command' commands
      ;;
    args)
      case "${line[1]}" in
        audit)
          _arguments \
            '--as-of=[Pinned RFC 3339 timestamp or YYYY-MM-DD date]:timestamp:' \
            '--format=[Output format]:format:(text json sarif github)' \
            '(-o --output)'{-o,--output=}'[Write report to path outside bundle]:output file:_files' \
            '--no-exec[Skip Python execution of Attested Computations]' \
            '--timing[Include wall-clock elapsed_ms on executed computations]' \
            '(-h --help)'{-h,--help}'[Print help information]' \
            '1:bundle directory:_files -/'
          ;;
        corpus)
          _arguments \
            '--format=[Output format]:format:(text json)' \
            '(-o --output)'{-o,--output=}'[Write report to path]:output file:_files' \
            '(-h --help)'{-h,--help}'[Print help information]' \
            '1:corpus or fixtures directory:_files -/'
          ;;
        drift)
          _arguments \
            '--format=[Output format]:format:(text json sarif github)' \
            '(-o --output)'{-o,--output=}'[Write report to path outside bundle]:output file:_files' \
            '(-h --help)'{-h,--help}'[Print help information]' \
            '1:bundle directory:_files -/'
          ;;
        grade)
          _arguments \
            '--ecosystem=[Semantic ecosystem]:ecosystem:(dbt ossie snowflake databricks cube looker atscale)' \
            '--changed-at=[Definition last-changed date or epoch seconds]:timestamp:' \
            '--format=[Output format]:format:(text json sarif github)' \
            '(-o --output)'{-o,--output=}'[Write report to path]:output file:_files' \
            '(-h --help)'{-h,--help}'[Print help information]' \
            '*:definition file:_files'
          ;;
        grade-snowflake)
          _arguments \
            '--changed-at=[Definition last-changed date or epoch seconds]:timestamp:' \
            '--format=[Output format]:format:(text json sarif github)' \
            '(-o --output)'{-o,--output=}'[Write report to path]:output file:_files' \
            '(-h --help)'{-h,--help}'[Print help information]' \
            '1:Snowflake semantic view YAML:_files'
          ;;
        grade-looker)
          _arguments \
            '--format=[Output format]:format:(text json sarif github)' \
            '(-o --output)'{-o,--output=}'[Write report to path]:output file:_files' \
            '(-h --help)'{-h,--help}'[Print help information]' \
            '*:Looker JSON or LookML file:_files'
          ;;
        grade-dbt|grade-ossie|grade-databricks|grade-cube|grade-atscale)
          _arguments \
            '--format=[Output format]:format:(text json sarif github)' \
            '(-o --output)'{-o,--output=}'[Write report to path]:output file:_files' \
            '(-h --help)'{-h,--help}'[Print help information]' \
            '1:definition file:_files'
          ;;
        help)
          _describe -t commands 'perishables subcommand' commands
          ;;
      esac
      ;;
  esac
}

_perishables "$@"
"""

_FISH_COMPLETIONS = """# Fish shell completions for perishables
# Supports --as-of, --format, --output, --no-exec, --timing, --ecosystem, --changed-at, --help, and --version.

complete -c perishables -n "__fish_use_subcommand" -s h -l help -d "Print help information (--help)"
complete -c perishables -n "__fish_use_subcommand" -s V -l version -d "Print version information (--version)"

complete -c perishables -f -n "__fish_use_subcommand" -a audit -d "Audit an Open Knowledge Format bundle at a pinned as-of date"
complete -c perishables -f -n "__fish_use_subcommand" -a corpus -d "Run the planted-defect reference corpus gate"
complete -c perishables -f -n "__fish_use_subcommand" -a drift -d "Check SHA-256 source drift across an OKF bundle"
complete -c perishables -f -n "__fish_use_subcommand" -a grade -d "Grade semantic-layer definitions with --ecosystem or auto-detection"
complete -c perishables -f -n "__fish_use_subcommand" -a grade-dbt -d "Grade a dbt semantic manifest"
complete -c perishables -f -n "__fish_use_subcommand" -a grade-ossie -d "Grade an Apache Ossie model (.json or .yaml)"
complete -c perishables -f -n "__fish_use_subcommand" -a grade-snowflake -d "Grade a Snowflake semantic view (.yaml)"
complete -c perishables -f -n "__fish_use_subcommand" -a grade-databricks -d "Grade a Databricks Genie serialized space (.json)"
complete -c perishables -f -n "__fish_use_subcommand" -a grade-cube -d "Grade a Cube model (.json, .yaml, or .js)"
complete -c perishables -f -n "__fish_use_subcommand" -a grade-looker -d "Grade a Looker API model (.json) or native LookML files (.lookml)"
complete -c perishables -f -n "__fish_use_subcommand" -a grade-atscale -d "Grade an AtScale semantic model export (.json)"
complete -c perishables -f -n "__fish_use_subcommand" -a help -d "Print help for a subcommand"

complete -c perishables -n "__fish_seen_subcommand_from audit" -r -l as-of -d "Pinned RFC 3339 timestamp or YYYY-MM-DD date (--as-of)"
complete -c perishables -n "__fish_seen_subcommand_from audit" -l no-exec -d "Skip Python execution of Attested Computations (--no-exec)"
complete -c perishables -n "__fish_seen_subcommand_from audit" -l timing -d "Include wall-clock elapsed_ms on executed computations (--timing)"

complete -c perishables -n "__fish_seen_subcommand_from grade" -x -l ecosystem -a "dbt ossie snowflake databricks cube looker atscale" -d "Target semantic ecosystem (--ecosystem)"
complete -c perishables -n "__fish_seen_subcommand_from grade grade-snowflake" -r -l changed-at -d "Definition last-changed date or epoch seconds (--changed-at)"

complete -c perishables -n "__fish_seen_subcommand_from corpus" -x -l format -a "text json" -d "Output format (--format)"
complete -c perishables -n "__fish_seen_subcommand_from audit drift grade grade-dbt grade-ossie grade-snowflake grade-databricks grade-cube grade-looker grade-atscale" -x -l format -a "text json sarif github" -d "Output format (--format)"

complete -c perishables -n "__fish_seen_subcommand_from audit corpus drift grade grade-dbt grade-ossie grade-snowflake grade-databricks grade-cube grade-looker grade-atscale" -r -s o -l output -d "Write report to path (--output)"
complete -c perishables -n "__fish_seen_subcommand_from audit corpus drift grade grade-dbt grade-ossie grade-snowflake grade-databricks grade-cube grade-looker grade-atscale" -s h -l help -d "Print subcommand help (--help)"
complete -c perishables -f -n "__fish_seen_subcommand_from help" -a "audit corpus drift grade grade-dbt grade-ossie grade-snowflake grade-databricks grade-cube grade-looker grade-atscale"
"""

_POWERSHELL_COMPLETIONS = """Register-ArgumentCompleter -Native -CommandName perishables -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commands = @(
        'audit',
        'corpus',
        'drift',
        'grade',
        'grade-dbt',
        'grade-ossie',
        'grade-snowflake',
        'grade-databricks',
        'grade-cube',
        'grade-looker',
        'grade-atscale',
        'help'
    )
    $ecosystems = @('dbt', 'ossie', 'snowflake', 'databricks', 'cube', 'looker', 'atscale')
    $formats = @('text', 'json', 'sarif', 'github')

    $elements = $commandAst.CommandElements | ForEach-Object { $_.Extent.Text }
    $subcommand = $null
    for ($i = 1; $i -lt $elements.Count; $i++) {
        $token = $elements[$i]
        if ($token -eq $wordToComplete -and $i -eq ($elements.Count - 1)) {
            break
        }
        if ($token -eq '--') {
            break
        }
        if (-not $token.StartsWith('-')) {
            $subcommand = $token
            break
        }
    }

    $prev = if ($elements.Count -ge 2) {
        if ($wordToComplete -eq '') { $elements[-1] } elseif ($elements.Count -ge 3) { $elements[-2] } else { '' }
    } else { '' }

    if (-not $subcommand) {
        $candidates = if ($wordToComplete.StartsWith('-')) {
            @('--help', '-h', '--version', '-V')
        } else {
            $commands
        }
        $candidates | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
        return
    }

    if ($prev -eq '--format') {
        $choices = if ($subcommand -eq 'corpus') { @('text', 'json') } else { $formats }
        $choices | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
        return
    }

    if ($prev -eq '--ecosystem') {
        $ecosystems | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
        return
    }

    $flags = switch ($subcommand) {
        'audit' { @('--as-of', '--format', '--output', '-o', '--no-exec', '--timing', '--help', '-h') }
        'corpus' { @('--format', '--output', '-o', '--help', '-h') }
        'drift' { @('--format', '--output', '-o', '--help', '-h') }
        'grade' { @('--ecosystem', '--changed-at', '--format', '--output', '-o', '--help', '-h') }
        'grade-snowflake' { @('--changed-at', '--format', '--output', '-o', '--help', '-h') }
        'help' { $commands | Where-Object { $_ -ne 'help' } }
        default { @('--format', '--output', '-o', '--help', '-h') }
    }

    if ($subcommand -eq 'help' -or $wordToComplete.StartsWith('-')) {
        $flags | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterName', $_)
        }
    }
}
"""


def completions(shell: str = "bash") -> str:
    """Return the CLI shell completion script for ``bash``, ``zsh``, ``fish``, or ``powershell``."""
    normalized = shell.strip().lower()
    if normalized == "bash":
        return _BASH_COMPLETIONS
    if normalized == "zsh":
        return _ZSH_COMPLETIONS
    if normalized == "fish":
        return _FISH_COMPLETIONS
    if normalized in ("powershell", "pwsh"):
        return _POWERSHELL_COMPLETIONS
    raise PerishablesError(
        f"unsupported shell {shell!r} (expected bash, zsh, fish, or powershell)",
        exit_code=2,
    )
