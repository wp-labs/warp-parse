#!/usr/bin/env sh
set -euo pipefail

REPO="wp-labs/warp-parse"
MANIFEST_URL="${WARP_PARSE_MANIFEST_URL:-https://raw.githubusercontent.com/wp-labs/warp-parse/main/dist/install-manifest.json}"
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
need_cmd find
need_cmd python3
need_cmd sed

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
    x86_64|amd64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="arm64" ;;
    *)
        echo "[warp-parse] unsupported architecture: $ARCH" >&2
        exit 1
        ;;
esac

TMP_DIR=$(mktemp -d)
MANIFEST_FILE=$(mktemp)
cleanup() {
    rm -rf "$TMP_DIR"
    rm -f "$MANIFEST_FILE"
}
trap cleanup EXIT

printf '[warp-parse] fetching manifest %s\n' "$MANIFEST_URL"
if ! curl -fsSL "$MANIFEST_URL" -o "$MANIFEST_FILE"; then
    echo "[warp-parse] failed to download manifest" >&2
    exit 1
fi

PY_OUT=$(python3 - "$REQUESTED_TAG" "$OS" "$ARCH" "$MANIFEST_FILE" <<'PY'
import json
import sys

requested = sys.argv[1]
os_key = sys.argv[2]
arch_key = sys.argv[3]
manifest_path = sys.argv[4]

with open(manifest_path, "r", encoding="utf-8") as fh:
    data = json.load(fh)

releases = data.get("releases", [])
if not releases:
    sys.exit("manifest contains no releases")

def normalize(ver: str) -> str:
    return ver if ver.startswith("v") else f"v{ver}"

selected = None
if requested == "latest":
    selected = releases[0]
else:
    needle = normalize(requested)
    for rel in releases:
        ver = rel.get("version", "")
        if ver == needle or ver.lstrip("v") == requested.lstrip("v"):
            selected = rel
            break

if selected is None:
    sys.exit(f"version '{requested}' not found in manifest")

key = f"{os_key}-{arch_key}"
asset = selected.get("artifacts", {}).get(key)
if not asset:
    sys.exit(f"no artifact entry for {key}")

print(selected.get("version", ""))
print(asset)
PY
)

TAG=$(printf '%s' "$PY_OUT" | sed -n '1p')
ASSET=$(printf '%s' "$PY_OUT" | sed -n '2p')

if [ -z "$TAG" ] || [ -z "$ASSET" ]; then
    echo "[warp-parse] failed to resolve download artifact" >&2
    exit 1
fi

DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"
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
    BIN_PATH=$(find "$TMP_DIR" -maxdepth 3 -type f -name "$bin" | head -n 1)
    if [ -n "$BIN_PATH" ]; then
        install -m 755 "$BIN_PATH" "$INSTALL_DIR/$bin"
        INSTALLED="$INSTALLED $bin"
    fi
done

if [ -z "$INSTALLED" ]; then
    echo "[warp-parse] no binaries were installed (archive layout unexpected)" >&2
    exit 1
fi

printf '[warp-parse] installed binaries:%s\n' "$INSTALLED"
printf '[warp-parse] location: %s\n' "$INSTALL_DIR"
printf '\nEnsure %s is on your PATH, e.g.:\n  export PATH="%s":\\$PATH\n\n' "$INSTALL_DIR" "$INSTALL_DIR"
printf 'Optional env vars:\n  WARP_PARSE_VERSION=v0.13.0\n  WARP_PARSE_INSTALL_DIR=/usr/local/bin\n  WARP_PARSE_MANIFEST_URL=https://example.com/custom-manifest.json\n'
