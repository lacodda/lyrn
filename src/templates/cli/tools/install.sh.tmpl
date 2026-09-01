#!/bin/sh
# {{ name }} installer for macOS and Linux:
#   curl -fsSL https://raw.githubusercontent.com/{{ repo }}/main/tools/install.sh | sh
set -eu

REPO="{{ repo }}"

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
    Darwin-arm64) TARGET="aarch64-apple-darwin" ;;
    # Git Bash, MSYS2 and Cygwin run this script happily on Windows, where it
    # has no business being: the Windows build is installed by install.ps1.
    MINGW*|MSYS*|CYGWIN*)
        echo "This is the macOS/Linux installer, but you are on Windows ($(uname -s))." >&2
        echo "Install with PowerShell instead:" >&2
        echo "  irm https://raw.githubusercontent.com/$REPO/main/tools/install.ps1 | iex" >&2
        exit 1
        ;;
    *)
        echo "No prebuilt binary for $(uname -s)/$(uname -m); install with: cargo install {{ name }}" >&2
        exit 1
        ;;
esac

# The tag comes from the /releases/latest redirect rather than the REST API:
# unauthenticated API calls are capped at 60 per hour per IP, and an installer
# that fails because someone else on the same address ran it is no installer.
TAG="${{{ env_prefix }}_VERSION:-}"
if [ -z "$TAG" ]; then
    LOCATION=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest" || true)
    TAG="${LOCATION##*/}"
fi
case "$TAG" in
    v[0-9]*) ;;
    *)
        echo "Cannot resolve the latest release of $REPO - set {{ env_prefix }}_VERSION to a tag like v0.1.0" >&2
        exit 1
        ;;
esac

NAME="{{ name }}-$TAG-$TARGET"
URL="https://github.com/$REPO/releases/download/$TAG/$NAME.tar.gz"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

echo "Downloading $URL"
curl -fsSL "$URL" | tar xz -C "$TMP"

BIN_DIR="${{{ env_prefix }}_INSTALL_DIR:-$HOME/.local/bin}"
mkdir -p "$BIN_DIR"
install -m 755 "$TMP/$NAME/{{ name }}" "$BIN_DIR/{{ name }}"
echo "Installed {{ name }} $TAG to $BIN_DIR/{{ name }}"

case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "Note: add $BIN_DIR to your PATH." ;;
esac
