# Changelog

## 2.0.0

Pulsar Store is rewritten in Rust. The Python version is gone.

### Migrating from 1.x

- Your installed-apps record (`~/.cache/pulsar-store/installed.json`) is read as is. Nothing to do.
- The `pulsar-store` command and `pulsar://install/<id>` links still work. Links now ask for confirmation before installing.
- Old Python files and caches under `/usr/share/pulsar-store` are removed on upgrade.

### New

- A faster app: it opens its window in about 0.6 s instead of 1 s, and 5,000-item search takes 0.1 ms.
- A new look inspired by Liquid Glass: floating glass toolbar, cards, a Get / progress ring / Open button, light and dark themes.
- A Tahoe-style layout: floating glass sidebar and toolbar, a hero banner with editorial cards, and compact app rows.
- Pages: Discover, Applications, Extensions, Sayri AI, Installed, Updates, Search, and item pages with screenshots, security audit, documentation, and package files.
- `pulsar-storectl`: search, install, update, and more from the command line, with JSON output.
- `pulsar-store-updated`: checks for updates in the background and notifies you.
- Spanish translation.
- Updates are offered only when the catalog version is newer than the installed one.

### Fixed

- A `.flatpakref` link was treated as a bundle and failed to install.
- Installing a "system" item called a function with arguments it did not accept.
- Summaries derived from a description no longer end in "..".

### Security

- Downloads over 4 GiB, archives that expand to over 2 GiB, and HTTPS-to-HTTP redirects are refused.
- Archive entries that try to leave the install folder are refused.
- Identifiers from the catalog must be plain names, never paths.

### Known gaps

- Package signatures and catalog signing are not implemented (`docs/threat-model.md`).
- Native package installs on Debian have not been run end to end in this release.
