# pulsaros-spotlight-launcher

Raycast-style launcher for Pulsar OS (GNOME). Rust, GTK4, libadwaita.
Behavior reference: [oma-cast](https://github.com/typecraft-dev/oma-cast). Plan: `docs/oma-cast-parity.md`.

## Build

    sudo apt install libgtk-4-dev libadwaita-1-dev libsqlite3-dev pkg-config
    cargo build --release
    cargo test

## Stage a .deb tree

    ./prepare-assets.sh <stage-dir>

Runtime depends: gnome-shell, GTK4, libadwaita, localsearch (or tracker-miner-fs), wl-clipboard.
