#!/bin/bash
# Pulsar OS package hook: runs on the staging dir. Drops development-only files so they are
# not shipped as /tests, /tools or /README.md, and makes payload files world-readable
# (a restrictive umask on the build machine must not leak into the package).
set -e
STAGING="$1"
rm -r -f "$STAGING/tests" "$STAGING/tools" "$STAGING/README.md"
find "$STAGING" -name '__pycache__' -type d -prune -exec rm -r -f {} +
chmod -R a+rX "$STAGING"
