#!/bin/sh
# End-to-end CLI gate walkthrough exercising corpus, audit, drift, and SARIF grading.
set -eu

REPO_ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"

resolve_bin() {
  if [ -n "${PERISHABLES_BIN:-}" ] && [ -f "${PERISHABLES_BIN}" ]; then
    printf '%s\n' "${PERISHABLES_BIN}"
    return 0
  fi
  for candidate in \
    "${REPO_ROOT}/target/debug/perishables" \
    "${REPO_ROOT}/target/debug/perishables.exe" \
    "${REPO_ROOT}/target/release/perishables" \
    "${REPO_ROOT}/target/release/perishables.exe"
  do
    if [ -f "${candidate}" ]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done
  if command -v perishables >/dev/null 2>&1; then
    command -v perishables
    return 0
  fi
  printf 'ci_gate_walkthrough.sh: perishables binary not found; build with cargo build -p perishables\n' >&2
  exit 2
}

BIN="$(resolve_bin)"
OUT_DIR=""
cleanup() {
  if [ -n "${OUT_DIR}" ] && [ -d "${OUT_DIR}" ]; then
    rm -rf "${OUT_DIR}"
  fi
}
trap cleanup EXIT INT TERM
OUT_DIR="$(mktemp -d)"

# 1. Reference corpus gate (19 fixtures, zero directory mutations)
"${BIN}" corpus "${REPO_ROOT}/corpus" --format json --output "${OUT_DIR}/corpus.json"
grep -q '"result": "PASS"' "${OUT_DIR}/corpus.json"
printf '[cli-corpus] verified reference corpus -> PASS\n'

# 2. Clean OKF bundle audit (exits 0)
"${BIN}" audit "${REPO_ROOT}/corpus/fixtures/clean-team-verified" \
  --as-of 2026-11-20 \
  --no-exec \
  --format json \
  --output "${OUT_DIR}/audit-clean.json"
grep -q '"result": "PASS"' "${OUT_DIR}/audit-clean.json"
printf '[cli-audit] clean-team-verified -> exit 0 (PASS)\n'

# 3. Planted lapsed-verification bundle audit (exits 1)
audit_rc=0
"${BIN}" audit "${REPO_ROOT}/corpus/fixtures/t1-lapsed" \
  --as-of 2026-11-20 \
  --no-exec \
  --format json \
  --output "${OUT_DIR}/audit-lapsed.json" || audit_rc=$?
if [ "${audit_rc}" -ne 1 ]; then
  printf 'expected exit code 1 on t1-lapsed, got %s\n' "${audit_rc}" >&2
  exit 1
fi
grep -q '"LAPSED_VERIFICATION"' "${OUT_DIR}/audit-lapsed.json"
printf '[cli-audit] t1-lapsed -> exit 1 (LAPSED_VERIFICATION)\n'

# 4. SHA-256 source drift gate (clean exits 0, drifted exits 1)
"${BIN}" drift "${REPO_ROOT}/corpus/fixtures/clean-team-verified" \
  --format json \
  --output "${OUT_DIR}/drift-clean.json"
grep -q '"result": "NONE"' "${OUT_DIR}/drift-clean.json"

drift_rc=0
"${BIN}" drift "${REPO_ROOT}/corpus/fixtures/t9-source-drift" \
  --format json \
  --output "${OUT_DIR}/drift-bad.json" || drift_rc=$?
if [ "${drift_rc}" -ne 1 ]; then
  printf 'expected exit code 1 on t9-source-drift, got %s\n' "${drift_rc}" >&2
  exit 1
fi
grep -q '"SOURCE_DRIFT"' "${OUT_DIR}/drift-bad.json"
printf '[cli-drift] clean=NONE, t9-source-drift -> exit 1 (SOURCE_DRIFT)\n'

# 5. Unified multi-ecosystem semantic grading emitting SARIF 2.1.0
"${BIN}" grade \
  "${REPO_ROOT}/crates/perishables-semantic/tests/fixtures/clean.json" \
  "${REPO_ROOT}/crates/perishables-semantic/tests/fixtures/lookml-clean.lookml" \
  --format sarif \
  --output "${OUT_DIR}/semantic-clean.sarif"
grep -q '"version": "2.1.0"' "${OUT_DIR}/semantic-clean.sarif"

grade_rc=0
"${BIN}" grade \
  "${REPO_ROOT}/crates/perishables-semantic/tests/fixtures/dangling.json" \
  "${REPO_ROOT}/crates/perishables-semantic/tests/fixtures/lookml-defects.lookml" \
  --format sarif \
  --output "${OUT_DIR}/semantic-defects.sarif" || grade_rc=$?
if [ "${grade_rc}" -ne 1 ]; then
  printf 'expected exit code 1 on defective semantic fixtures, got %s\n' "${grade_rc}" >&2
  exit 1
fi
grep -q '"DANGLING_METRIC_REFERENCE"' "${OUT_DIR}/semantic-defects.sarif"
grep -q '"NAME_COLLISION"' "${OUT_DIR}/semantic-defects.sarif"
printf '[cli-grade] SARIF 2.1.0 verified across clean (exit 0) and defective (exit 1) fixtures\n'
printf 'All CLI walkthrough checks passed.\n'
