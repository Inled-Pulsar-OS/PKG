# releases.d — Pulsar OS changelog

This folder is the **single source of truth** for the Pulsar OS changelog.
Every relevant update is documented in **one Markdown file**, and that file is
bundled into the `pulsaros-whatsnew` package, which shows a window with the
release notes right after the system is updated.

## Why here and not on a website?

- **Offline and signed**: the changelog travels inside a package from the very
  repository it describes (`apt.inled.es`), so it works even without network
  access or if the repo is blocked, and it is guaranteed to match what is
  actually installed.
- **Real history**: because it lives in the PKG repo it is versioned in git.
- The `pulsaros-whatsnew` package bundles **all** the notes, so the user can
  browse the full history offline.

## Creating an entry

```bash
./releases.d/new-release.sh                 # uses today's date as the id
./releases.d/new-release.sh 2026.11.02      # explicit id
```

Then publish the package that bundles the notes:

```bash
./package-and-deploy.sh pulsaros-whatsnew --deploy --branch unstable
```

> Note: `pulsaros-whatsnew` is rebuilt automatically whenever anything inside
> `releases.d/` changes (see `EXTRA_PATHS`), so nothing else needs to be
> touched.

## File format

`releases.d/<id>.md`. The `<id>` is the file name (usually the date, e.g.
`2026.10.17`) and is used for ordering.

Optional header (front matter) + Markdown body:

```markdown
---
id: 2026.10.17
date: 2026-10-17
title: Desktop and updater improvements
channel: unstable
important: false
packages:
  - pulsaros-theme
  - pulsaros-update
---

## What's new

- ...

## Fixes

- ...
```

### Header fields

| Field       | Required | Description                                                             |
|-------------|----------|-------------------------------------------------------------------------|
| `id`        | No       | Identifier. Defaults to the file name without `.md`.                     |
| `date`      | No       | `YYYY-MM-DD` date used for ordering. Defaults to `id`.                   |
| `title`     | No       | Title shown in the window. Defaults to "What's New in Pulsar OS".        |
| `channel`   | No       | `stable`, `unstable`, `rolling`… Informational only.                     |
| `important` | No       | `true` to mark the note as highlighted.                                  |
| `packages`  | No       | List of affected packages (YAML list or comma-separated).                |

The body accepts normal Markdown: headings, lists, bold, code, links,
blockquotes, tables and horizontal rules.

## How it is shown to the user

- A `dpkg` hook (Debian) / `libalpm` hook (Arch) marks "there are news" when any
  transaction touching `pulsaros-*` / `pulsar-*` packages finishes.
- A session watcher (`pulsaros-whatsnew-watch`, started by autostart) detects
  the flag and opens the window **once** with the new notes.
- On a freshly created installation all notes are marked as seen so the first
  boot is not disturbed.
- The user can always open the history from the "What's New in Pulsar OS" app.
