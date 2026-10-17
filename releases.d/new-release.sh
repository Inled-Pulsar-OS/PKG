#!/bin/bash
# ==============================================================================
# Pulsar OS - Create a new changelog entry (releases.d)
# ==============================================================================
# Creates a new release note from the template and opens it in $EDITOR.
#
# Usage:
#   ./releases.d/new-release.sh [id]
#
# Examples:
#   ./releases.d/new-release.sh              # id = today's date (YYYY.MM.DD)
#   ./releases.d/new-release.sh 2026.11.02
# ==============================================================================

set -e

RELEASES_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

ID="${1:-$(date +%Y.%m.%d)}"
DATE="$(date +%Y-%m-%d)"
TARGET="$RELEASES_DIR/${ID}.md"

if [ -e "$TARGET" ]; then
    echo "❌ Already exists: $TARGET"
    exit 1
fi

cat > "$TARGET" <<EOF
---
id: ${ID}
date: ${DATE}
title: What's New in Pulsar OS
channel: unstable
important: false
packages:
  - pulsaros-whatsnew
---

## What's new

- 

## Fixes

- 
EOF

echo "✅ Created release note: $TARGET"

if [ -n "$EDITOR" ] && command -v "$EDITOR" >/dev/null 2>&1; then
    "$EDITOR" "$TARGET"
fi

cat <<'EOF'

To publish it:
  ./package-and-deploy.sh pulsaros-whatsnew --deploy --branch unstable

(The pulsaros-whatsnew package bundles every note from releases.d/.)
EOF
