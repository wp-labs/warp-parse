#!/usr/bin/env sh
set -euo pipefail

REPO="wp-labs/warp-parse"
API_URL="https://api.github.com/repos/${REPO}/releases/latest"
INSTALL_DIR="${WARP_PARSE_INSTALL_DIR:-$HOME/.local/bin}"
REQUESTED_TAG="${WARP_PARSE_VERSION:-latest}"

need_cmd() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "[warp-parse] missing required command: $1" >&2
        exit 1
    }
}

need_cmd curl
need_cmd uname
need_cmd mktemp
need_cmd tar
need_cmd install

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$OS" in
    linux|darwin) : ;;
    *)
        echo "[warp-parse] unsupported OS: $OS" >&2
        exit 1
        ;;
esac

ARCH=$(uname -m)
case "$ARCH" in
    x86_64|amd64)
        ARCH="x86_64"
        ;;
    arm64|aarch64)
        ARCH="arm64"
        ;;
    *)
        echo "[warp-parse] unsupported architecture: $ARCH" >&2
        exit 1
        ;;
esac

if [ "$REQUESTED_TAG" = "latest" ]; then
    echo "[warp-parse] resolving latest release tag..."
    TAG=$(curl -fsSL "$API_URL" | awk -F'"' '/"tag_name":/ {print $4; exit}')
    if [ -z "$TAG" ]; then
        echo "[warp-parse] failed to determine latest release" >&2
        exit 1
    fi
else
    TAG="$REQUESTED_TAG"
fi

VERSION_NO_PREFIX=${TAG#v}
ASSET="warp-parse-${VERSION_NO_PREFIX}-${OS}-${ARCH}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"

TMP_DIR=$(mktemp -d)
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

ARCHIVE_PATH="$TMP_DIR/$ASSET"

printf '[warp-parse] downloading %s\n' "$DOWNLOAD_URL"
if ! curl -fL "$DOWNLOAD_URL" -o "$ARCHIVE_PATH"; then
    echo "[warp-parse] download failed" >&2
    exit 1
fi

tar -xzf "$ARCHIVE_PATH" -C "$TMP_DIR"
mkdir -p "$INSTALL_DIR"

BINARIES="wparse wpgen wprescue wproj"
INSTALLED=""
for bin in $BINARIES; do
    if [ -f "$TMP_DIR/$bin" ]; then
        install -m 755 "$TMP_DIR/$bin" "$INSTALL_DIR/$bin"
        INSTALLED="$INSTALLED $bin"
    fi
done

if [ -z "$INSTALLED" ]; then
    echo "[warp-parse] no binaries were found in archive" >&2
    exit 1
fi

cat <<MSG
[warp-parse] installed binaries:$INSTALLED
[warp-parse] location: $INSTALL_DIR

Ensure $INSTALL_DIR is on your PATH, e.g.:
  export PATH="$INSTALL_DIR":\$PATH

Optional env vars:
  WARP_PARSE_VERSION=v0.13.0
  WARP_PARSE_INSTALL_DIR=/usr/local/bin
MSG
