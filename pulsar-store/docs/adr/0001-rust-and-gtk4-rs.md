# 0001 Rewrite in Rust with gtk4-rs and libadwaita

Status: Accepted

## Context
Pulsar Store is about 3,000 lines of Python on GTK4 and libadwaita. The UI builds every tile eagerly, and install logic shells out heavily. We want faster startup and search, a typed core, and code that shares conventions with `pulsaros-spotlight-launcher`, which already uses gtk4-rs 0.9 and libadwaita 0.7.

## Decision
Rewrite in Rust using gtk4-rs and libadwaita-rs.

## Consequences
- Lower startup time and memory use; virtualized lists (`GridView`) become practical.
- Shared knowledge and crates with the launcher.
- Contributors need Rust and GTK build dependencies; packaging must build from source.

## Rejected
- Keep Python and optimize: does not address the structural costs.
- Qt/QML or a web UI: diverges from the GNOME toolchain used across Pulsar OS.
