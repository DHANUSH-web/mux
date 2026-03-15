#!/usr/bin/env sh
set -eu

REPO="DHANUSH-web/mux"
BINARY_NAME="mux"

print_usage() {
  cat <<USAGE
Usage: install.sh [options]

Options:
  --version <tag>      Install specific version (e.g. v0.1.0). Default: latest
  --install-dir <dir>  Install directory override
  --repo <owner/repo>  Override GitHub repo. Default: ${REPO}
  --system             Install system-wide to /usr/local/bin
  -h, --help           Show help

Examples:
  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh
  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh -s -- --version v0.1.0
  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh -s -- --system
USAGE
}

VERSION="latest"
INSTALL_DIR=""
SYSTEM_INSTALL=0

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
    --system)
      SYSTEM_INSTALL=1
      shift
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
  DOWNLOAD_TOOL="curl"
elif command -v wget >/dev/null 2>&1; then
  DOWNLOAD_TOOL="wget"
else
  echo "Error: curl or wget is required." >&2
  exit 1
fi

download_to() {
  url="$1"
  out="$2"
  if [ "$DOWNLOAD_TOOL" = "curl" ]; then
    curl -fsSL "$url" -o "$out"
  else
    wget -q "$url" -O "$out"
  fi
}

download_text() {
  url="$1"
  if [ "$DOWNLOAD_TOOL" = "curl" ]; then
    curl -fsSL "$url"
  else
    wget -qO- "$url"
  fi
}

uname_s="$(uname -s)"
uname_m="$(uname -m)"

case "$uname_s" in
  Linux) platform="linux" ;;
  Darwin) platform="macos" ;;
  *)
    echo "Unsupported OS for install.sh: $uname_s" >&2
    exit 1
    ;;
esac

case "$uname_m" in
  x86_64|amd64) arch="x86_64" ;;
  aarch64|arm64) arch="aarch64" ;;
  *)
    echo "Unsupported architecture: $uname_m" >&2
    exit 1
    ;;
esac

if [ "$SYSTEM_INSTALL" -eq 1 ] && [ -z "$INSTALL_DIR" ]; then
  INSTALL_DIR="/usr/local/bin"
elif [ -z "$INSTALL_DIR" ]; then
  INSTALL_DIR="$HOME/.local/bin"
fi

if [ "$VERSION" = "latest" ]; then
  api_url="https://api.github.com/repos/${REPO}/releases/latest"
  tag="$(download_text "$api_url" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"
  if [ -z "$tag" ]; then
    echo "Failed to resolve latest release tag from $api_url" >&2
    exit 1
  fi
else
  tag="$VERSION"
fi

case "$platform/$arch" in
  linux/x86_64) target="x86_64-unknown-linux-gnu" ;;
  macos/x86_64) target="x86_64-apple-darwin" ;;
  linux/aarch64) target="aarch64-unknown-linux-gnu" ;;
  macos/aarch64) target="aarch64-apple-darwin" ;;
  *)
    echo "No supported release target for ${platform}/${arch}" >&2
    exit 1
    ;;
esac

asset="${BINARY_NAME}-${target}"
archive_asset="${asset}.tar.gz"
base_url="https://github.com/${REPO}/releases/download/${tag}"
asset_url="${base_url}/${asset}"
archive_url="${base_url}/${archive_asset}"
checksums_url="${base_url}/SHA256SUMS"

tmp_dir="$(mktemp -d 2>/dev/null || mktemp -d -t mux-install)"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

asset_path="${tmp_dir}/${asset}"
archive_path="${tmp_dir}/${archive_asset}"
install_source=""
source_name=""
checksum_path=""

echo "Installing ${BINARY_NAME} ${tag} (${target})"
if download_to "$asset_url" "$asset_path"; then
  install_source="$asset_path"
  source_name="$asset"
  checksum_path="$asset_path"
  chmod +x "$install_source" || true
else
  echo "Direct binary not found, trying archive ${archive_asset}"
  download_to "$archive_url" "$archive_path"
  extract_dir="${tmp_dir}/extract"
  mkdir -p "$extract_dir"
  tar -xzf "$archive_path" -C "$extract_dir"
  install_source="${extract_dir}/${asset}"
  source_name="$archive_asset"
  checksum_path="$archive_path"
  if [ ! -f "$install_source" ]; then
    echo "Could not find ${asset} in archive" >&2
    exit 1
  fi
fi

# Optional checksum verification if SHA256SUMS is present.
if command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1; then
  checksums_path="${tmp_dir}/SHA256SUMS"
  if download_to "$checksums_url" "$checksums_path" 2>/dev/null; then
    expected="$(grep "  ${source_name}$" "$checksums_path" | awk '{print $1}' | head -n1 || true)"
    if [ -n "$expected" ]; then
      if command -v sha256sum >/dev/null 2>&1; then
        actual="$(sha256sum "$checksum_path" | awk '{print $1}')"
      else
        actual="$(shasum -a 256 "$checksum_path" | awk '{print $1}')"
      fi
      if [ "$actual" != "$expected" ]; then
        echo "Checksum verification failed for ${source_name}" >&2
        exit 1
      fi
      echo "Checksum verified"
    fi
  fi
fi

mkdir -p "$INSTALL_DIR"
target_path="${INSTALL_DIR}/${BINARY_NAME}"
cp "$install_source" "$target_path"
chmod +x "$target_path" || true

echo "Installed ${BINARY_NAME} to ${target_path}"

case ":$PATH:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    echo "Note: ${INSTALL_DIR} is not in PATH. Add this to your shell profile:"
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    ;;
esac
