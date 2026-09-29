#!/usr/bin/env bash
set -eu
(set -o pipefail 2>/dev/null) && set -o pipefail

REPO="njbaulia-wq/linwatch"
BIN="linwatch"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

# Detect platform
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
  aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
  *)
    echo "Unsupported architecture: $ARCH"
    exit 1
    ;;
esac

# Get latest release tag
TAG="${TAG:-${VERSION:-}}"
if [ -z "$TAG" ]; then
  echo "Fetching latest release..."
  TAG=$(basename "$(curl -sIL -o /dev/null -w "%{url_effective}" "https://github.com/$REPO/releases/latest")" 2>/dev/null || true)
  if [ -z "$TAG" ] || [ "$TAG" = "latest" ]; then
    TAG=$(curl -sSfL -H "User-Agent: linwatch-installer" "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null | grep '"tag_name"' | cut -d'"' -f4 || true)
  fi
fi

if [ -z "$TAG" ] || [ "$TAG" = "latest" ]; then
  echo "Failed to fetch latest release tag"
  exit 1
fi
echo "Latest release: $TAG"

# Download archive
ARCHIVE="$BIN-$TAG-$TARGET.tar.gz"
URL="https://github.com/$REPO/releases/download/$TAG/$ARCHIVE"
echo "Downloading $URL ..."
curl -sSfL "$URL" -o "$TMP_DIR/$ARCHIVE"

echo "Downloading checksums..."
curl -sSfL "https://github.com/$REPO/releases/download/$TAG/sha256sums.txt" \
  -o "$TMP_DIR/sha256sums.txt"

echo "Verifying checksum..."
(
  cd "$TMP_DIR"
  grep "  $ARCHIVE$" sha256sums.txt | sha256sum -c -
)

# Extract
echo "Extracting..."
tar -xzf "$TMP_DIR/$ARCHIVE" -C "$TMP_DIR"

# Install
mkdir -p "$INSTALL_DIR"
install -m 755 "$TMP_DIR/$BIN" "$INSTALL_DIR/$BIN"

echo "Installed $BIN $TAG to $INSTALL_DIR/$BIN"
echo "Make sure $INSTALL_DIR is in your PATH."
