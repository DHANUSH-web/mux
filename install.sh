#!/usr/bin/env sh
set -eu

REPO="DHANUSH-web/mux"
BINARY_NAME="mux"

print_usage() {
  cat <<USAGE
Usage: install.sh [options]

Options:
  --version <tag>      Install specific version (e.g. v0.1.0). Default: latest
  --install-dir <dir>  Install directory. Default: \$HOME/.local/bin
  --repo <owner/repo>  Override GitHub repo. Default: ${REPO}
  -h, --help           Show help

Examples:
  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh
  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh -s -- --version v0.1.0
USAGE
}

VERSION="latest"
INSTALL_DIR="${HOME}/.local/bin"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      [ "$#" -ge 2 ] || { echo "Missing value for --version" >&2; exit 1; }
      VERSION="$2"
      shift 2
      ;;
    --install-dir)
      [ "$#" -ge 2 ] || { echo "Missing value for --install-dir" >&2; exit 1; }
      INSTALL_DIR="$2"
      shift 2
      ;;
    --repo)
      [ "$#" -ge 2 ] || { echo "Missing value for --repo" >&2; exit 1; }
      REPO="$2"
      shift 2
      ;;
    -h|--help)
      print_usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      print_usage >&2
      exit 1
      ;;
  esac
done

if command -v curl >/dev/null 2>&1; then
  FETCH="curl -fsSL"
elif command -v wget >/dev/null 2>&1; then
  FETCH="wget -qO-"
else
  echo "Error: curl or wget is required." >&2
  exit 1
fi

uname_s="$(uname -s)"
uname_m="$(uname -m)"

case "${uname_s}" in
  Linux) os="linux" ;;
  Darwin) os="macos" ;;
  MINGW*|MSYS*|CYGWIN*|Windows_NT) os="windows" ;;
  *)
    echo "Unsupported OS: ${uname_s}" >&2
    exit 1
    ;;
esac

case "${uname_m}" in
  x86_64|amd64) arch="x86_64" ;;
  aarch64|arm64) arch="aarch64" ;;
  *)
    echo "Unsupported architecture: ${uname_m}" >&2
    exit 1
    ;;
esac

if [ "${VERSION}" = "latest" ]; then
  api_url="https://api.github.com/repos/${REPO}/releases/latest"
  tag="$(
    sh -c "${FETCH} \"${api_url}\"" \
      | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
      | head -n 1
  )"
  if [ -z "${tag}" ]; then
    echo "Failed to resolve latest release tag from ${api_url}" >&2
    exit 1
  fi
else
  tag="${VERSION}"
fi

ext="tar.gz"
if [ "${os}" = "windows" ]; then
  ext="zip"
fi

asset="${BINARY_NAME}-${os}-${arch}.${ext}"
base_url="https://github.com/${REPO}/releases/download/${tag}"
asset_url="${base_url}/${asset}"
checksums_url="${base_url}/checksums.txt"

tmp_dir="$(mktemp -d 2>/dev/null || mktemp -d -t mux-install)"
trap 'rm -rf "${tmp_dir}"' EXIT INT TERM
archive_path="${tmp_dir}/${asset}"

printf "Installing %s %s (%s/%s)\n" "${BINARY_NAME}" "${tag}" "${os}" "${arch}"
printf "Downloading %s\n" "${asset_url}"
sh -c "${FETCH} \"${asset_url}\" > \"${archive_path}\""

# Optional checksum verification if checksums.txt exists and sha256 tool is available.
has_sha_tool=0
if command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1; then
  has_sha_tool=1
fi

if [ "${has_sha_tool}" -eq 1 ]; then
  checksums_path="${tmp_dir}/checksums.txt"
  if sh -c "${FETCH} \"${checksums_url}\" > \"${checksums_path}\"" 2>/dev/null; then
    expected="$(grep "  ${asset}$" "${checksums_path}" | awk '{print $1}' | head -n1 || true)"
    if [ -n "${expected}" ]; then
      if command -v sha256sum >/dev/null 2>&1; then
        actual="$(sha256sum "${archive_path}" | awk '{print $1}')"
      else
        actual="$(shasum -a 256 "${archive_path}" | awk '{print $1}')"
      fi
      if [ "${actual}" != "${expected}" ]; then
        echo "Checksum verification failed for ${asset}" >&2
        exit 1
      fi
      echo "Checksum verified"
    fi
  fi
fi

extract_dir="${tmp_dir}/extract"
mkdir -p "${extract_dir}"

if [ "${ext}" = "tar.gz" ]; then
  tar -xzf "${archive_path}" -C "${extract_dir}"
else
  if command -v unzip >/dev/null 2>&1; then
    unzip -q "${archive_path}" -d "${extract_dir}"
  else
    echo "unzip is required to install Windows zip assets" >&2
    exit 1
  fi
fi

bin_path="$(find "${extract_dir}" -type f \( -name "${BINARY_NAME}" -o -name "${BINARY_NAME}.exe" \) | head -n1)"
if [ -z "${bin_path}" ]; then
  echo "Could not find ${BINARY_NAME} binary in downloaded archive" >&2
  exit 1
fi

mkdir -p "${INSTALL_DIR}"
target_path="${INSTALL_DIR}/${BINARY_NAME}"
cp "${bin_path}" "${target_path}"
chmod +x "${target_path}" || true

echo "Installed ${BINARY_NAME} to ${target_path}"

case ":$PATH:" in
  *":${INSTALL_DIR}:"*)
    ;;
  *)
    echo "Note: ${INSTALL_DIR} is not in PATH. Add this to your shell profile:"
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    ;;
esac
