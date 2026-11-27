#!/usr/bin/env bash
# Stage the curated public docs/ pages into book/src/docs/ so mdBook builds
# them as in-site chapters, and rewrite relative links that leave the book to
# canonical GitHub URLs so mdbook-linkcheck passes with zero dead links.
#
# Deliberately excluded from the published book:
#   * docs/BUILD-ROUND-*.md (18 maintainer build-round history logs)
#   * DECISIONS.md and docs/README.md (repo-level navigation and maintainer log)
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
src_docs="${repo_root}/book/src/docs"

rm -rf "${src_docs}"
mkdir -p "${src_docs}/media"

curated_pages=(
  "ARCHITECTURE"
  "DESIGN"
  "ATTESTATION"
  "CROSS-ECOSYSTEM"
  "LOOKML"
  "CLAIMS"
  "FAQ"
  "COMPARISON"
)

for page in "${curated_pages[@]}"; do
  cp "${repo_root}/docs/${page}.md" "${src_docs}/${page}.md"
done

cp "${repo_root}"/docs/media/*.svg "${src_docs}/media/"
cp "${repo_root}"/docs/media/*.png "${src_docs}/media/"
cp "${repo_root}"/docs/media/*.gif "${src_docs}/media/"

python3 - "${src_docs}" <<'PY'
import pathlib
import re
import sys

src_docs = pathlib.Path(sys.argv[1])
gh_blob = "https://github.com/david-engelmann/perishables/blob/main"
gh_tree = "https://github.com/david-engelmann/perishables/tree/main"
curated = {
    "ARCHITECTURE.md",
    "DESIGN.md",
    "ATTESTATION.md",
    "CROSS-ECOSYSTEM.md",
    "LOOKML.md",
    "CLAIMS.md",
    "FAQ.md",
    "COMPARISON.md",
}

root_files = {
    "README.md",
    "CHANGELOG.md",
    "CONTRIBUTING.md",
    "CODE_OF_CONDUCT.md",
    "SECURITY.md",
    "AGENTS.md",
    "DECISIONS.md",
    "CITATION.cff",
    "Cargo.toml",
    "Cargo.lock",
    "Makefile",
    "action.yml",
    "deny.toml",
    "install.sh",
    "pyproject.toml",
}

root_dirs = ("crates/", "corpus/", "examples/", "fuzz/", "python/", ".github/")

link_re = re.compile(r"(!?\[[^\]]*\]\()([^)]+)(\))")


def rewrite_target(target: str) -> str:
    raw = target.strip()
    if (
        not raw
        or raw.startswith("#")
        or raw.startswith("http://")
        or raw.startswith("https://")
        or raw.startswith("mailto:")
    ):
        return target

    if "#" in raw:
        path_part, frag = raw.split("#", 1)
        frag_suffix = "#" + frag
    else:
        path_part, frag_suffix = raw, ""

    # Normalize optional ./ prefix
    clean = path_part[2:] if path_part.startswith("./") else path_part

    # Links to curated sibling docs or staged media stay in-book
    if clean in curated or clean.startswith("media/"):
        return clean + frag_suffix
    if clean.startswith("../docs/"):
        sub = clean[len("../docs/") :]
        if sub in curated or sub.startswith("media/"):
            return sub + frag_suffix
        return f"{gh_blob}/docs/{sub}{frag_suffix}"

    # Unpublished docs/ pages (e.g. BUILD-ROUND-*.md or docs/README.md) -> GitHub
    if re.match(r"^BUILD-ROUND-\d+\.md$", clean) or clean == "README.md":
        return f"{gh_blob}/docs/{clean}{frag_suffix}"

    # Links climbing to repository root (../...)
    if clean.startswith("../"):
        rel = clean[3:]
        if rel in root_files:
            return f"{gh_blob}/{rel}{frag_suffix}"
        if any(rel.startswith(d) for d in root_dirs):
            base = gh_tree if rel.endswith("/") else gh_blob
            return f"{base}/{rel.rstrip('/')}{frag_suffix}"
        return f"{gh_blob}/{rel}{frag_suffix}"

    return target


for md_path in sorted(src_docs.glob("*.md")):
    original = md_path.read_text(encoding="utf-8")
    updated = link_re.sub(
        lambda m: f"{m.group(1)}{rewrite_target(m.group(2))}{m.group(3)}",
        original,
    )
    if updated != original:
        md_path.write_text(updated, encoding="utf-8")
PY

echo "staged $(find "${src_docs}" -maxdepth 1 -name '*.md' | wc -l | tr -d ' ') curated docs into book/src/docs/"
