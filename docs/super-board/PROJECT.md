# Project: Pulsar OS PKG

Package sources for Pulsar OS (Bittenfruit) and Tube OS (Arch + Debian). Each `pulsaros-*` / `Pulsar-*` dir is one package with `prepare-assets.sh` staging assets. Arch PKGBUILDs live in `arch/pkgbuilds/`. Packages are built and deployed from GH Actions to apt.inled.es (unstable → stable channels, see `BRANCHES-WORKFLOW.md`).

No app runtime or test suite at repo root; verification = build/package scripts (`package-and-deploy.sh`, `test-in-chroot.sh`). Several packages are git submodules.

Rules: see `AGENTS.md`. Ticket format: `docs/agents/issue-tracker.md`.
