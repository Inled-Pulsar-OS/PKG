#!/bin/bash
# Prints the container image used to build packages: ci-image.sh <debian|arch>
# Debian is always trixie: the shared apt list in deploy-package.yml does not resolve on
# forky/testing (e.g. libgnome-rr-4-dev is gone), so every branch builds on the same image.
set -euo pipefail
case "${1:-}" in
    debian) echo debian:trixie ;;
    arch) echo archlinux:latest ;;
    *) echo "usage: $0 <debian|arch>" >&2; exit 1 ;;
esac
