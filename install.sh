#!/usr/bin/env bash
# minmax-code — Install Script (macOS / Linux)
# Usage: curl -fsSL https://raw.githubusercontent.com/ezeoli88/minmax-code/main/install.sh | bash

set -e

REPO="ezeoli88/minmax-code"
INSTALL_DIR="$HOME/.minmax-code/bin"

echo ""
echo "  minmax-code Installer"
echo "  ====================="
echo ""

# Detect OS
case "$(uname -s)" in
  Linux*)  OS="linux" ;;
  Darwin*) OS="darwin" ;;
  *)
    echo "  Unsupported OS: $(uname -s)"
    exit 1
    ;;
esac

# Detect architecture
case "$(uname -m)" in
  x86_64)  ARCH="x64" ;;
  aarch64) ARCH="arm64" ;;
  arm64)   ARCH="arm64" ;;
  *)
    echo "  Unsupported architecture: $(uname -m)"
    exit 1
    ;;
esac

ARCHIVE="minmax-code-${OS}-${ARCH}.tar.gz"
URL="https://github.com/${REPO}/releases/latest/download/${ARCHIVE}"

echo "  Platform: ${OS}-${ARCH}"
echo "  Downloading ${ARCHIVE}..."

# Download
TMPDIR=$(mktemp -d)
curl -fSL -o "${TMPDIR}/${ARCHIVE}" "$URL"

# Extract
mkdir -p "$INSTALL_DIR"
tar xzf "${TMPDIR}/${ARCHIVE}" -C "$INSTALL_DIR"
chmod +x "$INSTALL_DIR/minmax-code"
chmod +x "$INSTALL_DIR/rg" 2>/dev/null || true
rm -rf "$TMPDIR"

echo "  Installed to ${INSTALL_DIR}"

# Add to PATH
add_to_path() {
  local rcfile="$1"
  if [ -f "$rcfile" ]; then
    if ! grep -q '.minmax-code/bin' "$rcfile" 2>/dev/null; then
      echo '' >> "$rcfile"
      echo '# minmax-code' >> "$rcfile"
      echo 'export PATH="$HOME/.minmax-code/bin:$PATH"' >> "$rcfile"
      echo "  Added to PATH in $(basename "$rcfile")"
    fi
  fi
}

if ! echo "$PATH" | grep -q '.minmax-code/bin'; then
  add_to_path "$HOME/.bashrc"
  add_to_path "$HOME/.zshrc"
  export PATH="$INSTALL_DIR:$PATH"
fi

echo ""
echo "  Done! Run 'minmax-code' to start."
echo "  (You may need to restart your shell or run: export PATH=\"\$HOME/.minmax-code/bin:\$PATH\")"
echo ""
