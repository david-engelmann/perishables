from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
if importlib.util.find_spec("perishables") is None:
    sys.path.insert(0, str(ROOT / "python"))

import perishables  # noqa: E402

SEMANTIC_FIXTURES = ROOT / "crates" / "perishables-semantic" / "tests" / "fixtures"
CORPUS_FIXTURES = ROOT / "corpus" / "fixtures"


class PerishablesSdkTests(unittest.TestCase):
    def test_version_and_exports(self) -> None:
        self.assertEqual(perishables.__version__, "0.1.1")
        self.assertEqual(perishables.version(), perishables.__version__)
        expected_exports = {
            "audit_sarif",
            "grade_atscale",
            "grade_cube",
            "grade_databricks",
            "grade_dbt",
            "grade_looker",
            "grade_ossie",
            "grade_sarif",
            "grade_snowflake",
        }
        self.assertTrue(expected_exports.issubset(set(perishables.__all__)))

    def test_audit_clean_and_lapsed_fixtures(self) -> None:
        clean_dir = CORPUS_FIXTURES / "clean-team-verified"
        clean_report = perishables.audit(clean_dir, as_of="2026-11-20", no_exec=True)
        self.assertTrue(clean_report.passed)
        self.assertEqual(clean_report.exit_code, 0)
        self.assertEqual(len(clean_report.findings), 0)
        clean_report.raise_for_findings()

        lapsed_dir = CORPUS_FIXTURES / "t1-lapsed"
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
        drift_dir = CORPUS_FIXTURES / "t9-source-drift"
        report = perishables.drift(drift_dir)
        self.assertFalse(report.clean)
        self.assertEqual(report.exit_code, 1)
        self.assertEqual(report.findings[0].code, "SOURCE_DRIFT")

    def test_grade_dbt_clean_and_dangling(self) -> None:
        clean = perishables.grade_dbt(SEMANTIC_FIXTURES / "clean.json")
        self.assertTrue(clean.passed)
        self.assertEqual(clean.command, "grade-dbt")
        self.assertEqual(clean.ecosystem, "dbt")
        self.assertIsNotNone(clean.counts)
        assert clean.counts is not None
        self.assertEqual(clean.counts.metrics, 3)

        dangling = perishables.grade_dbt(SEMANTIC_FIXTURES / "dangling.json")
        self.assertFalse(dangling.passed)
        self.assertEqual(dangling.exit_code, 1)
        self.assertIn(
            "DANGLING_METRIC_REFERENCE",
            {f.code for f in dangling.findings},
        )
        with self.assertRaises(perishables.PerishablesError) as ctx:
            dangling.raise_for_findings()
        self.assertEqual(ctx.exception.exit_code, 1)

    def test_grade_ossie_clean_and_collision(self) -> None:
        clean = perishables.grade_ossie(SEMANTIC_FIXTURES / "ossie-clean.yaml")
        self.assertTrue(clean.passed)
        self.assertEqual(clean.command, "grade-ossie")
        self.assertEqual(clean.ecosystem, "ossie")

        collision = perishables.grade_ossie(SEMANTIC_FIXTURES / "ossie-collision.json")
        self.assertFalse(collision.passed)
        self.assertEqual([f.code for f in collision.findings], ["NAME_COLLISION"])

    def test_grade_snowflake_currency_and_clean(self) -> None:
        clean = perishables.grade_snowflake(
            SEMANTIC_FIXTURES / "snowflake-clean.yaml",
            changed_at="1750000000",
        )
        self.assertTrue(clean.passed)
        self.assertEqual(clean.command, "grade-snowflake")
        self.assertEqual(clean.ecosystem, "snowflake")

        unstamped_change = perishables.grade_snowflake(
            SEMANTIC_FIXTURES / "snowflake-lapsed.yaml"
        )
        self.assertTrue(unstamped_change.passed)

        lapsed = perishables.grade_snowflake(
            SEMANTIC_FIXTURES / "snowflake-lapsed.yaml",
            changed_at="2026-11-20",
        )
        self.assertFalse(lapsed.passed)
        self.assertEqual([f.code for f in lapsed.findings], ["LAPSED_VERIFIED_QUERY"])

    def test_grade_databricks_clean_and_benchmark(self) -> None:
        clean = perishables.grade_databricks(SEMANTIC_FIXTURES / "databricks-clean.json")
        self.assertTrue(clean.passed)
        self.assertEqual(clean.command, "grade-databricks")
        self.assertEqual(clean.ecosystem, "databricks")

        defect = perishables.grade_databricks(
            SEMANTIC_FIXTURES / "databricks-benchmark.json"
        )
        self.assertFalse(defect.passed)
        self.assertEqual(
            [f.code for f in defect.findings], ["BENCHMARK_WITHOUT_ANSWER"]
        )

    def test_grade_cube_single_and_multi_file(self) -> None:
        clean = perishables.grade_cube(SEMANTIC_FIXTURES / "cube-clean.js")
        self.assertTrue(clean.passed)
        self.assertEqual(clean.command, "grade-cube")
        self.assertEqual(clean.ecosystem, "cube")

        dangling = perishables.grade_cube(SEMANTIC_FIXTURES / "cube-dangling.yaml")
        self.assertFalse(dangling.passed)
        self.assertEqual([f.code for f in dangling.findings], ["DANGLING_RELATIONSHIP"])

        batch = perishables.grade_cube(
            [
                SEMANTIC_FIXTURES / "cube-clean.json",
                SEMANTIC_FIXTURES / "cube-dangling.js",
            ]
        )
        self.assertFalse(batch.passed)
        self.assertEqual(batch.command, "grade")
        self.assertEqual(len(batch.reports), 2)
        self.assertTrue(batch.reports[0].passed)
        self.assertFalse(batch.reports[1].passed)

    def test_grade_looker_single_and_project_scope(self) -> None:
        clean = perishables.grade_looker(SEMANTIC_FIXTURES / "lookml-clean.lookml")
        self.assertTrue(clean.passed)
        self.assertEqual(clean.command, "grade-looker")
        self.assertEqual(clean.ecosystem, "looker")

        defects = perishables.grade_looker(SEMANTIC_FIXTURES / "looker-defects.json")
        self.assertFalse(defects.passed)
        self.assertEqual(
            {f.code for f in defects.findings},
            {"DANGLING_RELATIONSHIP", "NAME_COLLISION"},
        )

        proj_dir = SEMANTIC_FIXTURES / "project"
        proj = perishables.grade_looker(
            [
                proj_dir / "orders.view.lookml",
                proj_dir / "customers.view.lookml",
                proj_dir / "extended.view.lookml",
                proj_dir / "sales.model.lookml",
            ]
        )
        self.assertFalse(proj.passed)
        self.assertEqual(proj.command, "grade-looker")
        self.assertEqual(proj.ecosystem, "looker")
        self.assertEqual([f.item for f in proj.findings], ["ghost_to_missing_view"])

    def test_grade_atscale_clean_and_dangling(self) -> None:
        clean = perishables.grade_atscale(SEMANTIC_FIXTURES / "atscale-clean.json")
        self.assertTrue(clean.passed)
        self.assertEqual(clean.command, "grade-atscale")
        self.assertEqual(clean.ecosystem, "atscale")

        dangling = perishables.grade_atscale(
            SEMANTIC_FIXTURES / "atscale-dangling.json"
        )
        self.assertFalse(dangling.passed)
        self.assertEqual(
            [f.code for f in dangling.findings], ["DANGLING_METRIC_REFERENCE"]
        )

    def test_audit_sarif_clean_findings_and_output_file(self) -> None:
        clean_dir = CORPUS_FIXTURES / "clean-team-verified"
        clean_sarif = perishables.audit_sarif(
            clean_dir,
            as_of="2026-11-20",
            no_exec=True,
        )
        self.assertEqual(clean_sarif["version"], "2.1.0")
        self.assertEqual(
            clean_sarif["runs"][0]["tool"]["driver"]["name"],
            "perishables",
        )
        self.assertEqual(clean_sarif["runs"][0]["results"], [])

        lapsed_dir = CORPUS_FIXTURES / "t1-lapsed"
        with tempfile.TemporaryDirectory() as tmp:
            out_file = Path(tmp) / "audit-report.sarif"
            lapsed_sarif = perishables.audit_sarif(
                lapsed_dir,
                as_of="2026-11-20",
                no_exec=True,
                output=out_file,
            )
            self.assertTrue(out_file.is_file())
            disk_sarif = json.loads(out_file.read_text(encoding="utf-8"))
            self.assertEqual(lapsed_sarif, disk_sarif)

        results = lapsed_sarif["runs"][0]["results"]
        self.assertEqual(len(results), 1)
        self.assertEqual(results[0]["ruleId"], "LAPSED_VERIFICATION")

        with self.assertRaises(perishables.PerishablesError) as ctx:
            perishables.audit_sarif(
                clean_dir,
                as_of="not-a-date",
                no_exec=True,
            )
        self.assertEqual(ctx.exception.exit_code, 2)

    def test_grade_sarif_clean_findings_and_output_file(self) -> None:
        clean_sarif = perishables.grade_sarif(
            SEMANTIC_FIXTURES / "clean.json",
            ecosystem="dbt",
        )
        self.assertEqual(clean_sarif["version"], "2.1.0")
        self.assertEqual(clean_sarif["runs"][0]["results"], [])

        with tempfile.TemporaryDirectory() as tmp:
            out_file = Path(tmp) / "semantic.sarif"
            sf_sarif = perishables.grade_sarif(
                SEMANTIC_FIXTURES / "snowflake-lapsed.yaml",
                ecosystem="snowflake",
                changed_at="2026-11-20",
                output=out_file,
            )
            self.assertTrue(out_file.is_file())
            self.assertEqual(
                sf_sarif,
                json.loads(out_file.read_text(encoding="utf-8")),
            )

        sf_results = sf_sarif["runs"][0]["results"]
        self.assertEqual(len(sf_results), 1)
        self.assertEqual(sf_results[0]["ruleId"], "LAPSED_VERIFIED_QUERY")

        multi_sarif = perishables.grade_sarif(
            [
                SEMANTIC_FIXTURES / "dangling.json",
                SEMANTIC_FIXTURES / "lookml-defects.lookml",
            ]
        )
        rule_ids = {r["ruleId"] for r in multi_sarif["runs"][0]["results"]}
        self.assertEqual(
            rule_ids,
            {"DANGLING_METRIC_REFERENCE", "DANGLING_RELATIONSHIP", "NAME_COLLISION"},
        )

        with self.assertRaises(perishables.PerishablesError) as ctx:
            perishables.grade_sarif([], ecosystem="dbt")
        self.assertEqual(ctx.exception.exit_code, 2)

    def test_invalid_as_of_raises_usage_error(self) -> None:
        clean_dir = CORPUS_FIXTURES / "clean-team-verified"
        with self.assertRaises(perishables.PerishablesError) as ctx:
            perishables.audit(clean_dir, as_of="not-a-date", no_exec=True)
        self.assertEqual(ctx.exception.exit_code, 2)


if __name__ == "__main__":
    unittest.main()
