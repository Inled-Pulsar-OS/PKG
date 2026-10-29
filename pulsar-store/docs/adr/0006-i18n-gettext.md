# 0006 Internationalization with gettext

Status: Accepted

## Context
The project is English-first but must be ready for translation from the first screen. Retrofitting is costly.

## Decision
Use gettext with domain `pulsar-store`, via `gettext-rs`. Source strings are English. Every user-facing string goes through helper macros for plain, plural, and context strings; a CI rule rejects string literals that bypass them. Error types carry message keys, not literal text. Catalog text uses the localized fields from ADR 0005. Translations live in `assets/locale/`. Spanish is the first translation.

## Consequences
- Standard toolchain familiar to GNOME translators.
- Slight friction when writing UI text; the CI rule keeps it honest.

## Rejected
- Fluent: good format, but weaker tooling in the GNOME ecosystem.
