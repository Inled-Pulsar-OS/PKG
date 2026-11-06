#!/bin/bash
# Prints the container image used to build packages: ci-image.sh <debian|arch> [branch]
# The Debian image follows the same branch -> DEBIAN_VERSION mapping as package-and-deploy.sh.
case "$1:$2" in
    debian:forky) echo debian:forky ;;
    debian:rolling) echo debian:testing ;;
    debian:*) echo debian:trixie ;;
    arch:*) echo archlinux:latest ;;
    *) echo "usage: $0 <debian|arch> [branch]" >&2; exit 1 ;;
esac
