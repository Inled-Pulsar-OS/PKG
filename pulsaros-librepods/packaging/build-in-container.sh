#!/bin/bash
# Runs inside debian:trixie (started by prepare-assets.sh): builds the daemon against trixie's Qt 6.8
# and installs it into /stage. Building in trixie keeps the binary within trixie's glibc.
set -e
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq --no-install-recommends \
    build-essential cmake ninja-build pkg-config ca-certificates \
    qt6-base-dev qt6-declarative-dev qt6-connectivity-dev qt6-tools-dev qt6-tools-dev-tools qt6-l10n-tools \
    libqt6opengl6-dev libpulse-dev libssl-dev >/dev/null
mkdir -p /work
cp -r /src/. /work/
cd /work
cmake -B /build -G Ninja -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF >/tmp/cmake.log || { tail -40 /tmp/cmake.log; exit 1; }
cmake --build /build
DESTDIR=/stage cmake --install /build --prefix /usr
# the stage is bind-mounted from the host: hand the files back to the invoking user
if [ -n "${HOST_UID:-}" ]; then
    chown -R "$HOST_UID:${HOST_GID:-$HOST_UID}" /stage
fi
