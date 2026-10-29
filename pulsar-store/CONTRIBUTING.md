# Contributing to Pulsar Store

Thanks for helping. Start with `docs/architecture.md` (when available) and `docs/tickets/README.md`.

## Workflow
1. Pick a ticket from `docs/tickets/`. Tickets labeled `good-first-issue` are small and self-contained.
2. Branch from `main`: `feat/<topic>`, `fix/<topic>`, or `docs/<topic>`.
3. Keep PRs small: one or two tickets.
4. Use [Conventional Commits](https://www.conventionalcommits.org/), for example `feat(core): add catalog fetch`.

## Definition of done
- Tests added or updated; `cargo test` passes.
- `cargo clippy -- -D warnings` and `cargo fmt --check` pass.
- No user-facing string bypasses the i18n helpers (ADR 0006).
- Layer rules hold (ADR 0002): `core` never imports `ui`, `modules`, or GTK; modules never import each other.
- Docs updated; screenshots attached for UI changes (light and dark).

## Design rules
- Take inspiration from Apple's design language, but do not copy Apple artwork, icons, fonts, text, or trademarks.
- Glass effects belong on the navigation layer only, with a flat fallback and support for reduced motion and reduced transparency.

## Reporting problems
Open an issue with steps to reproduce, expected and actual behavior, and your distro and GNOME version.

By contributing you agree that your work is licensed under GPL-3.0-or-later (ADR 0004).

## Labels
- `block:<name>` groups a ticket by block, for example `block:core-catalog`.
- `size:S`, `size:M`, `size:L` estimate effort (up to 1 day, 3 days, 1 week).
- `good-first-issue` marks small, self-contained work.
