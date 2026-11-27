#!/usr/bin/env python3
"""End-to-end Python SDK walkthrough for OKF auditing, SARIF output, and semantic grading."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
if importlib.util.find_spec("perishables") is None:
    sys.path.insert(0, str(ROOT / "python"))

import perishables  # noqa: E402

CORPUS_FIXTURES = ROOT / "corpus" / "fixtures"
SEMANTIC_FIXTURES = ROOT / "crates" / "perishables-semantic" / "tests" / "fixtures"
AS_OF = "2026-11-20"


def main() -> None:
    # 1. Clean OKF bundle audit at a pinned instant
    clean_bundle = CORPUS_FIXTURES / "clean-team-verified"
    clean_audit = perishables.audit(clean_bundle, as_of=AS_OF, no_exec=True)
    assert clean_audit.passed, f"expected clean pass, got {clean_audit}"
    assert clean_audit.exit_code == 0
    assert clean_audit.findings == ()
    clean_audit.raise_for_findings()
    print(
        f"[okf-clean] {clean_bundle.name}: {clean_audit.result} "
        f"({clean_audit.concepts_count} concepts, 0 findings)"
    )

    # 2. Lapsed OKF bundle audit + SHA-256 source drift check
    lapsed_bundle = CORPUS_FIXTURES / "t1-lapsed"
    lapsed_audit = perishables.audit(lapsed_bundle, as_of=AS_OF, no_exec=True)
    assert not lapsed_audit.passed
    assert lapsed_audit.exit_code == 1
    lapsed_codes = [f.code for f in lapsed_audit.findings]
    assert lapsed_codes == ["LAPSED_VERIFICATION"], lapsed_codes
    print(f"[okf-lapsed] {lapsed_bundle.name}: convicted {lapsed_codes}")

    drift_bundle = CORPUS_FIXTURES / "t9-source-drift"
    drift_report = perishables.drift(drift_bundle)
    assert not drift_report.clean
    assert drift_report.exit_code == 1
    drift_codes = [f.code for f in drift_report.findings]
    assert drift_codes == ["SOURCE_DRIFT"], drift_codes
    print(f"[okf-drift] {drift_bundle.name}: convicted {drift_codes}")

    # 3. SARIF 2.1.0 generation into a temporary file
    with tempfile.TemporaryDirectory() as tmp:
        sarif_path = Path(tmp) / "t1-lapsed.sarif"
        sarif_doc = perishables.audit_sarif(
            lapsed_bundle,
            as_of=AS_OF,
            no_exec=True,
            output=sarif_path,
        )
        assert sarif_path.is_file(), "expected SARIF file on disk"
        on_disk = json.loads(sarif_path.read_text(encoding="utf-8"))
        assert sarif_doc == on_disk
        assert sarif_doc["version"] == "2.1.0"
        rule_ids = [r["ruleId"] for r in sarif_doc["runs"][0]["results"]]
        assert rule_ids == ["LAPSED_VERIFICATION"], rule_ids
        print(f"[sarif] wrote SARIF {sarif_doc['version']} with rules {rule_ids}")

    # 4. Semantic-layer grading across dbt, LookML, and Snowflake
    dbt_clean = perishables.grade_dbt(SEMANTIC_FIXTURES / "clean.json")
    dbt_clean.raise_for_findings()
    assert dbt_clean.counts is not None and dbt_clean.counts.metrics == 3

    dbt_bad = perishables.grade_dbt(SEMANTIC_FIXTURES / "dangling.json")
    assert not dbt_bad.passed
    dbt_codes = sorted({f.code for f in dbt_bad.findings})
    assert dbt_codes == ["DANGLING_METRIC_REFERENCE"], dbt_codes
    print(f"[semantic-dbt] clean=PASS, dangling={dbt_codes}")

    lookml_clean = perishables.grade_looker(SEMANTIC_FIXTURES / "lookml-clean.lookml")
    lookml_clean.raise_for_findings()

    lookml_project = SEMANTIC_FIXTURES / "project"
    lookml_proj_rep = perishables.grade_looker(
        [
            lookml_project / "orders.view.lookml",
            lookml_project / "customers.view.lookml",
            lookml_project / "extended.view.lookml",
            lookml_project / "sales.model.lookml",
        ]
    )
    assert not lookml_proj_rep.passed
    lookml_codes = [f.code for f in lookml_proj_rep.findings]
    lookml_items = [f.item for f in lookml_proj_rep.findings]
    assert lookml_codes == ["DANGLING_RELATIONSHIP"], lookml_codes
    assert lookml_items == ["ghost_to_missing_view"], lookml_items
    print(f"[semantic-lookml] clean=PASS, project={lookml_codes} ({lookml_items[0]})")

    sf_clean = perishables.grade_snowflake(
        SEMANTIC_FIXTURES / "snowflake-clean.yaml",
        changed_at="1750000000",
    )
    sf_clean.raise_for_findings()

    sf_lapsed = perishables.grade_snowflake(
        SEMANTIC_FIXTURES / "snowflake-lapsed.yaml",
        changed_at=AS_OF,
    )
    assert not sf_lapsed.passed
    sf_codes = [f.code for f in sf_lapsed.findings]
    assert sf_codes == ["LAPSED_VERIFIED_QUERY"], sf_codes
    print(f"[semantic-snowflake] clean=PASS, lapsed={sf_codes}")

    print("All Python SDK example assertions passed.")


if __name__ == "__main__":
    main()
