# 0002 Single crate with `core`, `ui`, and `modules`

Status: Accepted

## Context
We want a modular architecture where each feature (categories, item page, search, updates) is isolated. The launcher uses one crate with `src/core` and `src/ui`; we want a similar structure that contributors already know.

## Decision
One crate, `pulsar-store`, laid out as:

- `src/core/`: logic with no GTK imports (model, ports, catalog, search, ledger, install, privilege, config, `sources/`).
- `src/ui/`: shared design system and widgets (tokens, glass, motion, router, `widget/`).
- `src/modules/<name>/`: one folder per feature, registered through a `Module` trait.
- `src/bin/`: `storectl` and `updated`, which use only `core`.
- A Cargo feature `ui` (default on) gates `ui/` and `modules/`, so the binaries in `src/bin/` build without GTK.

Dependency rules, enforced in CI:
1. `core` imports nothing from `ui`, `modules`, or GTK.
2. `ui` imports only `core`.
3. A module imports only `core` and `ui`, never another module. Cross-module navigation uses a typed `Route`.

## Consequences
- Simple build and release; one version number.
- A single crate has no compiler-enforced boundaries, so a CI check is required (ticket 0.3).
- Splitting into a workspace later is possible because the boundaries are already clean.

## Rejected
- Cargo workspace with one crate per module: more ceremony than a ~15k-line app needs, and diverges from the launcher.
