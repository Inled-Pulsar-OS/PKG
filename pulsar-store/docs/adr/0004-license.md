# 0004 License: GPL-3.0-or-later

Status: Accepted

## Context
The store is open source and shares code with `pulsaros-spotlight-launcher`, which is GPL-3.0-or-later.

## Decision
License all new code as GPL-3.0-or-later. Third-party dependencies must be compatible; `cargo-deny` checks licenses in CI. Assets (icons, fonts) carry their own licenses, recorded next to the files. No Apple artwork, icons, fonts, or trademarks are included.

## Consequences
- Code can move between the launcher and the store.
- Permissive-only dependencies are not required, but GPL-incompatible ones are rejected.
