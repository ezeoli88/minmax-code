#!/usr/bin/env bash
set -euo pipefail

# Cross-compile minmax-code for all supported platforms
# Usage: ./scripts/build-all.sh [version]
# Example: ./scripts/build-all.sh 0.2.0

VERSION="${1:-$(node -p "require('./package.json').version")}"
DIST_DIR="dist/binaries"
ENTRY="src/index.ts"
BIN_NAME="minmax-code"

# Supported targets for bun build --compile
TARGETS=(
  "bun-linux-x64"
  "bun-linux-arm64"
  "bun-darwin-x64"
  "bun-darwin-arm64"
  "bun-windows-x64"
)

echo "Building minmax-code v${VERSION} for all platforms..."
echo ""

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

for TARGET in "${TARGETS[@]}"; do
  # Extract platform and arch from target (e.g., bun-linux-x64 -> linux-x64)
  PLATFORM_ARCH="${TARGET#bun-}"

  # Windows gets .exe extension
  if [[ "$TARGET" == *"windows"* ]]; then
    OUTFILE="${DIST_DIR}/${BIN_NAME}-v${VERSION}-${PLATFORM_ARCH}.exe"
  else
    OUTFILE="${DIST_DIR}/${BIN_NAME}-v${VERSION}-${PLATFORM_ARCH}"
  fi

  echo "  Building ${TARGET}..."
  bun build "$ENTRY" --compile --target="$TARGET" --outfile "$OUTFILE"

  if [ -f "$OUTFILE" ]; then
    SIZE=$(du -h "$OUTFILE" | cut -f1)
    echo "    -> ${OUTFILE} (${SIZE})"
  else
    echo "    -> FAILED"
    exit 1
  fi
done

echo ""
echo "All builds complete. Binaries in ${DIST_DIR}/"
ls -lh "$DIST_DIR/"
