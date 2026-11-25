from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
if importlib.util.find_spec("perishables") is None:
    sys.path.insert(0, str(ROOT / "python"))

import perishables  # noqa: E402


class PerishablesSdkTests(unittest.TestCase):
    def test_version_matches_package(self) -> None:
        self.assertEqual(perishables.version(), perishables.__version__)

    def test_audit_clean_and_lapsed_fixtures(self) -> None:
        clean_dir = ROOT / "corpus" / "fixtures" / "clean-team-verified"
        clean_report = perishables.audit(clean_dir, as_of="2026-11-20", no_exec=True)
        self.assertTrue(clean_report.passed)
        self.assertEqual(clean_report.exit_code, 0)
        self.assertEqual(len(clean_report.findings), 0)
        clean_report.raise_for_findings()

        lapsed_dir = ROOT / "corpus" / "fixtures" / "t1-lapsed"
        lapsed_report = perishables.audit(lapsed_dir, as_of="2026-11-20", no_exec=True)
        self.assertFalse(lapsed_report.passed)
        self.assertEqual(lapsed_report.exit_code, 1)
        self.assertIn(
            "LAPSED_VERIFICATION",
            [finding.code for finding in lapsed_report.findings],
        )
        with self.assertRaises(perishables.PerishablesError) as ctx:
            lapsed_report.raise_for_findings()
        self.assertEqual(ctx.exception.exit_code, 1)

    def test_drift_detects_modified_source(self) -> None:
        drift_dir = ROOT / "corpus" / "fixtures" / "t9-source-drift"
        report = perishables.drift(drift_dir)
        self.assertFalse(report.clean)
        self.assertEqual(report.exit_code, 1)
        self.assertEqual(report.findings[0].code, "SOURCE_DRIFT")

    def test_grade_lookml_defects(self) -> None:
        lookml = (
            ROOT
            / "crates"
            / "perishables-semantic"
            / "tests"
            / "fixtures"
            / "lookml-defects.lookml"
        )
        report = perishables.grade(lookml, ecosystem="looker")
        self.assertFalse(report.passed)
        self.assertEqual(report.ecosystem, "looker")
        self.assertIsNotNone(report.counts)
        codes = {f.code for f in report.findings}
        self.assertIn("DANGLING_RELATIONSHIP", codes)
        self.assertIn("NAME_COLLISION", codes)

    def test_invalid_as_of_raises_usage_error(self) -> None:
        clean_dir = ROOT / "corpus" / "fixtures" / "clean-team-verified"
        with self.assertRaises(perishables.PerishablesError) as ctx:
            perishables.audit(clean_dir, as_of="not-a-date", no_exec=True)
        self.assertEqual(ctx.exception.exit_code, 2)


if __name__ == "__main__":
    unittest.main()
