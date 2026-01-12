#!/usr/bin/env sh
set -euo pipefail

REPO="wp-labs/warp-parse"
API_URL="https://api.github.com/repos/${REPO}/releases"
USER_AGENT="warp-parse-installer"
curl_api() {
    if [ -n "${WARP_PARSE_GITHUB_TOKEN:-}" ]; then
        AUTH_HEADER="-H Authorization: Bearer ${WARP_PARSE_GITHUB_TOKEN}"
    else
        AUTH_HEADER=""
    fi
    if ! curl -fsSL \
        -H "Accept: application/vnd.github+json" \
        -H "User-Agent: ${USER_AGENT}" \
        ${AUTH_HEADER} \
        "$1" -o "$2"; then
        echo "[warp-parse] github api request failed" >&2
        exit 1
    fi
}
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
RELEASE_DATA=$(mktemp)
cleanup() {
    rm -rf "$TMP_DIR"
    rm -f "$RELEASE_DATA"
}
trap cleanup EXIT

if [ "$REQUESTED_TAG" = "latest" ]; then
    echo "[warp-parse] resolving latest release tag..."
    curl_api "$API_URL/latest" "$RELEASE_DATA"
else
    TAG_QUERY="$REQUESTED_TAG"
    case "$TAG_QUERY" in
        v*) : ;;
        *) TAG_QUERY="v$TAG_QUERY" ;;
    esac
    echo "[warp-parse] resolving release for $TAG_QUERY ..."
    curl_api "$API_URL/tags/$TAG_QUERY" "$RELEASE_DATA"
fi

TARGET_SUFFIX="${OS}-${ARCH}"
PYTHON_OUT=$(python3 - "$OS" "$ARCH" "$RELEASE_DATA" <<'PY'
import json
import sys

OS_TOKEN = sys.argv[1]
ARCH_TOKEN = sys.argv[2]
PATH = sys.argv[3]

OS_ALIASES = {
    "darwin": ["darwin", "apple-darwin", "macos", "osx"],
    "linux": ["linux", "gnu", "unknown-linux-gnu"],
}

ARCH_ALIASES = {
    "x86_64": ["x86_64", "amd64"],
    "arm64": ["arm64", "aarch64"],
}

def normalize(key, mapping):
    return [token.lower() for token in mapping.get(key, [key])]

os_tokens = normalize(OS_TOKEN, OS_ALIASES)
arch_tokens = normalize(ARCH_TOKEN, ARCH_ALIASES)

with open(PATH, "r", encoding="utf-8") as fh:
    data = json.load(fh)

tag = data.get("tag_name")
if not tag:
    sys.exit("missing tag_name in release metadata")

def matches(name):
    lower = name.lower()
    if not lower.endswith(".tar.gz"):
        return False
    return any(tok in lower for tok in os_tokens) and any(tok in lower for tok in arch_tokens)

for asset in data.get("assets", []):
    name = asset.get("name", "")
    url = asset.get("browser_download_url", "")
    if matches(name) and url:
        print(tag)
        print(name)
        print(url)
        sys.exit(0)

sys.exit(f"no asset matching tokens {os_tokens} + {arch_tokens}")
PY
)

TAG=$(printf '%s' "$PYTHON_OUT" | sed -n '1p')
ASSET=$(printf '%s' "$PYTHON_OUT" | sed -n '2p')
DOWNLOAD_URL=$(printf '%s' "$PYTHON_OUT" | sed -n '3p')

if [ -z "$ASSET" ] || [ -z "$DOWNLOAD_URL" ]; then
    echo "[warp-parse] failed to locate release artifact for $TARGET_SUFFIX" >&2
    exit 1
fi

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
printf 'Optional env vars:\n  WARP_PARSE_VERSION=v0.13.0\n  WARP_PARSE_INSTALL_DIR=/usr/local/bin\n  WARP_PARSE_GITHUB_TOKEN=<token>  # to avoid API rate limit\n'
