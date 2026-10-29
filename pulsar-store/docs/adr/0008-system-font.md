# 0008 Use the system font

Status: Accepted

## Context
Apple's design uses San Francisco, a proprietary font we cannot ship. We want a modern, consistent look without bundling large font files or breaking the user's font choices.

## Decision
Use the system interface font (libadwaita's default). Set weights and sizes through the type scale in `docs/design-system.md`. Bundle no font. `assets/fonts/` stays empty unless a later decision adds an open-licensed one.

## Consequences
- No licensing risk, smaller package, and the app follows the user's font settings.
- Appearance varies a little between systems. Layouts must tolerate different metrics, which the pseudo-locale test helps check.

## Rejected
- Bundling Inter or another open font: nice consistency, but extra size and it ignores the user's font.
- Any Apple font: not licensed for redistribution.
