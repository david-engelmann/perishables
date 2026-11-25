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
    "corpus",
    "drift",
    "find_binary",
    "grade",
    "version",
]

__version__ = "0.1.0"

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
    if isinstance(paths, (str, os.PathLike)):
        file_args = [os.fspath(paths)]
    else:
        file_args = [os.fspath(p) for p in paths]
    if not file_args:
        raise PerishablesError("grade requires at least one path", exit_code=2)

    cmd = ["grade", *file_args, "--format", "json"]
    if ecosystem is not None:
        cmd.extend(["--ecosystem", ecosystem])
    if changed_at is not None:
        cmd.extend(["--changed-at", changed_at])
    data, code = _invoke_json(cmd, binary=binary)
    return _parse_grade_payload(data, code)


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
