# Quickstart

## 1. Install `perishables`

Choose whichever channel matches your environment. None of the prebuilt channels require a Rust compiler.

```sh
# PyPI wheel (bundles native Rust CLI + typed Python SDK)
pip install perishables

# Run directly with uv without a persistent install
uvx perishables --version

# Prebuilt Linux / macOS tarball verified against SHA256SUMS
curl -fsSL https://raw.githubusercontent.com/david-engelmann/perishables/main/install.sh | sh

# Build from source via crates.io (or download prebuilt via cargo-binstall)
cargo install perishables
```

Verify the installed binary:

```sh
perishables --version
```

## 2. Audit an Open Knowledge Format bundle

Every `audit` invocation takes an explicit `--as-of` date (`YYYY-MM-DD`) or UTC RFC 3339 timestamp (`YYYY-MM-DDTHH:MM:SSZ`) so results never depend on the host wall clock:

```sh
# Clean control bundle (exits 0)
perishables audit corpus/fixtures/clean-team-verified --as-of 2026-11-20

# Planted lapsed-verification bundle (exits 1 with LAPSED_VERIFICATION)
perishables audit corpus/fixtures/t1-lapsed --as-of 2026-11-20

# Audit an untrusted bundle without spawning Python subprocesses
perishables audit ./untrusted-bundle --as-of 2026-11-20 --no-exec

# Check SHA-256 source hashes independently
perishables drift ./bundle
```

## 3. Grade analytics semantic layers

Grade individual vendor files with `grade-<ecosystem>` or pass multiple files across ecosystems to `perishables grade`:

```sh
perishables grade-dbt manifest.json
perishables grade-ossie model.yaml
perishables grade-snowflake view.yaml --changed-at 2026-11-20
perishables grade-databricks space.json
perishables grade-cube model.js
perishables grade-looker orders.view.lookml customers.view.lookml sales.model.lookml
perishables grade-atscale model.json

# Unified multi-file SARIF 2.1.0 report
perishables grade model.yaml orders.view.lookml --format sarif --output semantic.sarif
```

## 4. Drive from Python

The `perishables` package provides frozen, typed standard-library dataclasses over the bundled binary:

```python
from pathlib import Path
import tempfile
import perishables

# 1. Audit a clean OKF bundle
clean = perishables.audit("corpus/fixtures/clean-team-verified", as_of="2026-11-20")
clean.raise_for_findings()

# 2. Emit SARIF 2.1.0 for a lapsed bundle
with tempfile.TemporaryDirectory() as tmp:
    sarif_path = Path(tmp) / "audit.sarif"
    sarif = perishables.audit_sarif(
        "corpus/fixtures/t1-lapsed",
        as_of="2026-11-20",
        no_exec=True,
        output=sarif_path,
    )
    assert sarif["runs"][0]["results"][0]["ruleId"] == "LAPSED_VERIFICATION"

# 3. Grade semantic definitions across dbt, LookML, and Snowflake
lookml = perishables.grade_looker("crates/perishables-semantic/tests/fixtures/lookml-clean.lookml")
lookml.raise_for_findings()
```

## 5. Wire into GitHub Actions

Use the composite action at `david-engelmann/perishables@v0.1.3` to annotate pull requests directly:

```yaml
name: Trust Gate
on: [pull_request]

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7.0.1
      - uses: david-engelmann/perishables@v0.1.3
        with:
          command: audit
          path: ./knowledge-bundle
          as-of: "2026-11-20"
          format: github
      - uses: david-engelmann/perishables@v0.1.3
        with:
          command: grade-looker
          path: "views/orders.view.lookml models/sales.model.lookml"
          format: github
```

## 6. Enable shell tab completions

Generate shell completions for `bash`, `zsh`, `fish`, or `powershell`:

```sh
python3 -m perishables completions bash > ~/.local/share/bash-completion/completions/perishables
python3 -m perishables completions zsh > ~/.zfunc/_perishables
python3 -m perishables completions fish > ~/.config/fish/completions/perishables.fish
```
