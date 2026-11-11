# Pulsar OS PKG

Package sources for Pulsar OS (Bittenfruit) and Tube OS. Arch + Debian. Deployed to apt.inled.es via GH Actions.

## Commands

| Need | Command |
|---|---|
| Build + deploy local | `./package-and-deploy.sh` |
| Promote unstable → stable | `./promote-package.sh` |
| Check repo versions | `./check-repo-versions.sh` |
| Chroot tests | `./test-in-chroot.sh` |

## Where things live

| Path | Holds |
|---|---|
| `pulsaros-*/`, `Pulsar-*/` | package sources; `prepare-assets.sh` stages assets |
| `arch/pkgbuilds/` | Arch PKGBUILDs |
| `build/` | local staging output; gitignored |
| `BRANCHES-WORKFLOW.md` | stable/unstable channel guide |
| `docs/` | docs |

## Boundaries

- ALWAYS bump package version when changing a package's files.
- ALWAYS keep deprecated packages out of new images (see README).
- NEVER commit build output (`*.deb`, `*.pkg.tar.*`, `build/`).
- NEVER push straight to `main`.
- Ask first: stable channel promotion.

## Commits and PRs

Conventional commits: `type(scope): subject`. Scope = package name.

## Agent skills

### Issue tracker

GitHub Issues (`Victor1890/pulsaros-PKG`) via `gh`. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five-role vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context. See `docs/agents/domain.md`.
