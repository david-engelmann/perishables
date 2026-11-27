#!/bin/sh
# Install prebuilt perishables release binaries with mandatory SHA256SUMS verification.
set -eu

DEFAULT_VERSION="v0.1.2"
DEFAULT_BASE_URL="https://github.com/david-engelmann/perishables/releases"

usage() {
  cat <<'EOF'
install.sh - Install prebuilt perishables binary with SHA-256 verification

USAGE:
  install.sh [OPTIONS]

OPTIONS:
  --version <vX.Y.Z>     Release tag to install (defaults to latest release or v0.1.2)
  --install-dir <DIR>    Destination directory (default: $INSTALL_DIR or $HOME/.local/bin)
  --base-url <URL>       Releases base URL (default: https://github.com/david-engelmann/perishables/releases)
  --target <TARGET>      Override target triple (auto-detected from uname by default)
  -h, --help             Print help information and exit 0
EOF
}

err() {
  printf 'install.sh: error: %s\n' "$1" >&2
  exit 1
}

VERSION_OPT="${PERISHABLES_VERSION:-}"
DEST_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"
BASE_URL="${PERISHABLES_BASE_URL:-${DEFAULT_BASE_URL}}"
TARGET_OPT="${PERISHABLES_TARGET:-}"

while [ "$#" -gt 0 ]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --version)
      [ "$#" -ge 2 ] || err "--version requires a value"
      VERSION_OPT="$2"
      shift 2
      ;;
    --version=*)
      VERSION_OPT="${1#--version=}"
      [ -n "${VERSION_OPT}" ] || err "--version requires a value"
      shift
      ;;
    --install-dir)
      [ "$#" -ge 2 ] || err "--install-dir requires a value"
      DEST_DIR="$2"
      shift 2
      ;;
    --install-dir=*)
      DEST_DIR="${1#--install-dir=}"
      [ -n "${DEST_DIR}" ] || err "--install-dir requires a value"
      shift
      ;;
    --base-url)
      [ "$#" -ge 2 ] || err "--base-url requires a value"
      BASE_URL="$2"
      shift 2
      ;;
    --base-url=*)
      BASE_URL="${1#--base-url=}"
      [ -n "${BASE_URL}" ] || err "--base-url requires a value"
      shift
      ;;
    --target)
      [ "$#" -ge 2 ] || err "--target requires a value"
      TARGET_OPT="$2"
      shift 2
      ;;
    --target=*)
      TARGET_OPT="${1#--target=}"
      [ -n "${TARGET_OPT}" ] || err "--target requires a value"
      shift
      ;;
    *)
      err "unknown argument: $1 (pass --help for usage)"
      ;;
  esac
done

BASE_URL="${BASE_URL%/}"

detect_target() {
  os_name="$(uname -s)"
  arch_name="$(uname -m)"
  case "${os_name}" in
    Linux)
      case "${arch_name}" in
        x86_64|amd64) printf '%s\n' "x86_64-unknown-linux-gnu" ;;
        aarch64|arm64) printf '%s\n' "aarch64-unknown-linux-gnu" ;;
        *) err "unsupported Linux architecture: ${arch_name}" ;;
      esac
      ;;
    Darwin)
      case "${arch_name}" in
        x86_64|amd64) printf '%s\n' "x86_64-apple-darwin" ;;
        arm64|aarch64) printf '%s\n' "aarch64-apple-darwin" ;;
        *) err "unsupported macOS architecture: ${arch_name}" ;;
      esac
      ;;
    *)
      err "unsupported operating system: ${os_name} (supported: Linux and Darwin)"
      ;;
  esac
}

resolve_tag() {
  if [ -n "${VERSION_OPT}" ]; then
    printf 'v%s\n' "${VERSION_OPT#v}"
    return 0
  fi
  case "${BASE_URL}" in
    http://*|https://*)
      if command -v curl >/dev/null 2>&1; then
        effective="$(curl -fsSLI -o /dev/null -w '%{url_effective}' "${BASE_URL}/latest" 2>/dev/null || true)"
        case "${effective}" in
          */tag/v*)
            resolved="${effective##*/tag/}"
            resolved="${resolved%%\?*}"
            resolved="${resolved%/}"
            if [ -n "${resolved}" ]; then
              printf '%s\n' "${resolved}"
              return 0
            fi
            ;;
        esac
      fi
      ;;
  esac
  printf '%s\n' "${DEFAULT_VERSION}"
}

download_to() {
  src_url="$1"
  dst_file="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "${src_url}" -o "${dst_file}" || err "failed to download ${src_url}"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "${dst_file}" "${src_url}" || err "failed to download ${src_url}"
  else
    err "neither curl nor wget is installed"
  fi
}

compute_sha256() {
  target_file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "${target_file}" | awk '{print tolower($1)}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "${target_file}" | awk '{print tolower($1)}'
  else
    err "neither sha256sum nor shasum is installed; refusing unverified installation"
  fi
}

if [ -n "${TARGET_OPT}" ]; then
  TARGET="${TARGET_OPT}"
else
  TARGET="$(detect_target)"
fi

TAG="$(resolve_tag)"
STAGE="perishables-${TAG}-${TARGET}"
ARCHIVE="${STAGE}.tar.gz"

ASSET_PREFIX="${BASE_URL}/download/${TAG}"
case "${BASE_URL}" in
  file://*)
    local_base="${BASE_URL#file://}"
    if [ -f "${local_base}/download/${TAG}/SHA256SUMS" ]; then
      ASSET_PREFIX="${BASE_URL}/download/${TAG}"
    elif [ -f "${local_base}/${TAG}/SHA256SUMS" ]; then
      ASSET_PREFIX="${BASE_URL}/${TAG}"
    elif [ -f "${local_base}/SHA256SUMS" ]; then
      ASSET_PREFIX="${BASE_URL}"
    fi
    ;;
esac

TMP_DIR=""
cleanup() {
  if [ -n "${TMP_DIR}" ] && [ -d "${TMP_DIR}" ]; then
    rm -rf "${TMP_DIR}"
  fi
}
trap cleanup EXIT INT TERM
TMP_DIR="$(mktemp -d)"

ARCHIVE_PATH="${TMP_DIR}/${ARCHIVE}"
SUMS_PATH="${TMP_DIR}/SHA256SUMS"

download_to "${ASSET_PREFIX}/${ARCHIVE}" "${ARCHIVE_PATH}"
download_to "${ASSET_PREFIX}/SHA256SUMS" "${SUMS_PATH}"

EXPECTED_SHA="$(awk -v want="${ARCHIVE}" '
  NF >= 2 {
    name = $2
    sub(/^\*/, "", name)
    sub(/^\.\//, "", name)
    if (name == want) {
      print tolower($1)
      exit
    }
  }
' "${SUMS_PATH}")"

if [ -z "${EXPECTED_SHA}" ]; then
  err "archive ${ARCHIVE} not listed in SHA256SUMS"
fi

if ! printf '%s\n' "${EXPECTED_SHA}" | grep -Eq '^[0-9a-f]{64}$'; then
  err "malformed SHA-256 entry for ${ARCHIVE} in SHA256SUMS: ${EXPECTED_SHA}"
fi

ACTUAL_SHA="$(compute_sha256 "${ARCHIVE_PATH}")"
if [ "${ACTUAL_SHA}" != "${EXPECTED_SHA}" ]; then
  printf 'install.sh: error: SHA-256 checksum mismatch for %s\n' "${ARCHIVE}" >&2
  printf '  expected: %s\n' "${EXPECTED_SHA}" >&2
  printf '  actual:   %s\n' "${ACTUAL_SHA}" >&2
  exit 1
fi

EXTRACT_DIR="${TMP_DIR}/extract"
mkdir -p "${EXTRACT_DIR}"
tar -xzf "${ARCHIVE_PATH}" -C "${EXTRACT_DIR}"

EXTRACTED_BIN="${EXTRACT_DIR}/${STAGE}/perishables"
if [ ! -f "${EXTRACTED_BIN}" ]; then
  err "expected binary ${STAGE}/perishables missing inside verified archive ${ARCHIVE}"
fi

mkdir -p "${DEST_DIR}"
STAGED_DEST="${DEST_DIR}/.perishables.tmp.$$"
cp "${EXTRACTED_BIN}" "${STAGED_DEST}"
chmod 0755 "${STAGED_DEST}"
mv -f "${STAGED_DEST}" "${DEST_DIR}/perishables"

printf 'Installed perishables %s (%s) to %s/perishables\n' "${TAG}" "${TARGET}" "${DEST_DIR}"
