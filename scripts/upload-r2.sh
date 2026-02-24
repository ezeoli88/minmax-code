#!/usr/bin/env bash
set -euo pipefail

# Upload built binaries to Cloudflare R2
# Requires: AWS CLI configured with R2 credentials
# Usage: ./scripts/upload-r2.sh [version]

VERSION="${1:-$(node -p "require('./package.json').version")}"
DIST_DIR="dist/binaries"
BUCKET="${R2_BUCKET_NAME:?R2_BUCKET_NAME env var is required}"
R2_ENDPOINT="${R2_ENDPOINT_URL:?R2_ENDPOINT_URL env var is required}"

echo "Uploading minmax-code v${VERSION} binaries to R2..."
echo "  Bucket: ${BUCKET}"
echo "  Endpoint: ${R2_ENDPOINT}"
echo ""

for FILE in "$DIST_DIR"/*; do
  FILENAME=$(basename "$FILE")
  R2_KEY="releases/v${VERSION}/${FILENAME}"

  echo "  Uploading ${FILENAME} -> s3://${BUCKET}/${R2_KEY}"
  aws s3 cp "$FILE" "s3://${BUCKET}/${R2_KEY}" \
    --endpoint-url "$R2_ENDPOINT" \
    --no-progress

  echo "    Done."
done

# Also upload a latest manifest
cat > /tmp/latest.json <<EOF
{
  "version": "${VERSION}",
  "binaries": {
    "linux-x64": "releases/v${VERSION}/minmax-code-v${VERSION}-linux-x64",
    "linux-arm64": "releases/v${VERSION}/minmax-code-v${VERSION}-linux-arm64",
    "darwin-x64": "releases/v${VERSION}/minmax-code-v${VERSION}-darwin-x64",
    "darwin-arm64": "releases/v${VERSION}/minmax-code-v${VERSION}-darwin-arm64",
    "win32-x64": "releases/v${VERSION}/minmax-code-v${VERSION}-windows-x64.exe"
  }
}
EOF

aws s3 cp /tmp/latest.json "s3://${BUCKET}/releases/latest.json" \
  --endpoint-url "$R2_ENDPOINT" \
  --content-type "application/json" \
  --no-progress

echo ""
echo "All binaries uploaded to R2."
