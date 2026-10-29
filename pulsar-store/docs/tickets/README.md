# Pulsar Store tickets

Backlog for the Rust rewrite of Pulsar Store. Each ticket is one Markdown file, grouped by block (epic).

## Target layout

```
pulsar-store/
  Cargo.toml            single crate, bin `pulsar-store`
  src/
    main.rs  app.rs  lib.rs
    bin/                storectl.rs, updated.rs
    core/               logic only, no GTK: model, ports, catalog, search, ledger, install, privilege, config, sources/
    ui/                 shared kit: tokens, glass, motion, router, widget/
    modules/            one folder per feature: discover, categories, search, item, updates, ecosystem
  assets/               css/, icons/, fonts/, catalog/, desktop/, locale/
  cli/  docs/  DEBIAN/  prepare-assets.sh  .github/
```

Dependency rules: `core` imports nothing from `ui`, `modules`, or GTK. `ui` imports only `core`. A module imports only `core` and `ui`, never another module; cross-module navigation uses `Route`. CI enforces this (ticket 0.3).

## Conventions

- Language: English. Every user-facing string goes through the i18n helpers (ticket 6.6) so translations can be added later.
- Sizes: S up to 1 day, M up to 3 days, L up to 1 week.
- Commits follow Conventional Commits. Each ticket meets its acceptance criteria and the definition of done.
- Suggested order: B0 and B1; then B2 to B4 and ticket 9.2 in parallel; then B5 and B6; then B7 and B8; then the rest of B9; B10 to B12 throughout.

## Index

Totals: 1 closed, 46 done, 22 open, 12 partial.
Status: **done** = every acceptance box ticked, **partial** = some ticked (the ticket says what is left), **open**, **closed** = decided not to do.

### B0 Foundation and governance

- [0.1](b0-foundation/0.1-write-initial-adrs.md) Write initial ADRs (M) **done**
- [0.2](b0-foundation/0.2-scaffold-the-crate-and-folder-layout.md) Scaffold the crate and folder layout (S) **done**
- [0.3](b0-foundation/0.3-ci-pipeline-and-import-boundary-check.md) CI pipeline and import-boundary check (M) **partial** — depends on 0.2
- [0.4](b0-foundation/0.4-baseline-benchmark-of-the-python-app-and-performance-budget.md) Baseline benchmark of the Python app and performance budget (M) **done**
- [0.5](b0-foundation/0.5-contribution-guide-templates-labels.md) Contribution guide, templates, labels (S) **done**
- [0.6](b0-foundation/0.6-verified-reference-notes-on-apple-s-app-store-behavior.md) Verified reference notes on Apple's App Store behavior (M) **partial**

### B1 core: model, ports, schema

- [1.1](b1-core-model/1.1-domain-model-and-error-types.md) Domain model and error types (M) **done** — depends on 0.2
- [1.2](b1-core-model/1.2-port-traits.md) Port traits (M) **done** — depends on 1.1
- [1.3](b1-core-model/1.3-catalog-schema-v1.md) Catalog schema v1 (M) **done** — depends on 1.1
- [1.4](b1-core-model/1.4-localized-fields-in-the-catalog-schema.md) Localized fields in the catalog schema (S) **done** — depends on 1.3

### B2 core: catalog and search

- [2.1](b2-core-catalog/2.1-catalog-fetch-with-mirror-fallback-and-etag-cache.md) Catalog fetch with mirror fallback and ETag cache (M) **done** — depends on 1.2, 1.3
- [2.2](b2-core-catalog/2.2-catalog-normalization-and-bundled-fallback.md) Catalog normalization and bundled fallback (S) **done** — depends on 2.1
- [2.3](b2-core-catalog/2.3-search-and-ranking.md) Search and ranking (L) **done** — depends on 2.2
- [2.4](b2-core-catalog/2.4-icon-and-screenshot-cache.md) Icon and screenshot cache (M) **done** — depends on 2.1

### B3 core: package sources

- [3.1](b3-core-sources/3.1-flatpak-source.md) Flatpak source (L) **done** — depends on 1.2
- [3.2](b3-core-sources/3.2-system-package-source.md) System package source (L) **partial** — depends on 1.2
- [3.3](b3-core-sources/3.3-gnome-extension-source.md) GNOME extension source (M) **done** — depends on 1.2
- [3.4](b3-core-sources/3.4-sayri-skill-and-plugin-sources.md) Sayri skill and plugin sources (M) **done** — depends on 1.2
- [3.5](b3-core-sources/3.5-privilege-helper.md) Privilege helper (M) **done** — depends on 1.2
- [3.6](b3-core-sources/3.6-source-contract-test-suite.md) Source contract test suite (M) **done** — depends on 1.2

### B4 core: install pipeline

- [4.1](b4-core-install/4.1-ledger-with-migration.md) Ledger with migration (M) **done** — depends on 1.2
- [4.2](b4-core-install/4.2-edition-selection.md) Edition selection (S) **done** — depends on 1.1
- [4.3](b4-core-install/4.3-install-orchestrator.md) Install orchestrator (L) **done** — depends on 3.1, 3.2, 3.3, 3.4, 4.1, 4.2
- [4.4](b4-core-install/4.4-uninstall-and-update-flows.md) Uninstall and update flows (M) **done** — depends on 4.3

### B5 ui: design system

- [5.1](b5-ui-design-system/5.1-design-tokens-and-base-css.md) Design tokens and base CSS (M) **partial**
- [5.2](b5-ui-design-system/5.2-typography-and-font-decision.md) Typography and font decision (S) **done** — depends on 5.1
- [5.3](b5-ui-design-system/5.3-glass-surface-widget.md) Glass surface widget (L) **partial** — depends on 5.1
- [5.4](b5-ui-design-system/5.4-radii-and-elevation-system.md) Radii and elevation system (S) **done** — depends on 5.1
- [5.5](b5-ui-design-system/5.5-shared-widget-kit.md) Shared widget kit (L) **partial** — depends on 5.1
- [5.6](b5-ui-design-system/5.6-install-button.md) Install button (L) **partial** — depends on 4.3, 5.5
- [5.7](b5-ui-design-system/5.7-design-system-documentation.md) Design system documentation (M) **done** — depends on 5.5

### B6 Application shell

- [6.1](b6-shell/6.1-application-window-sidebar.md) Application, window, sidebar (M) **partial** — depends on 5.1
- [6.2](b6-shell/6.2-typed-router.md) Typed router (M) **done** — depends on 6.1
- [6.3](b6-shell/6.3-module-trait-and-registry.md) Module trait and registry (M) **done** — depends on 6.2
- [6.4](b6-shell/6.4-toast-service-and-event-bus.md) Toast service and event bus (S) **done** — depends on 6.1
- [6.5](b6-shell/6.5-single-instance-and-deep-link.md) Single instance and deep link (M) **done** — depends on 6.1
- [6.6](b6-shell/6.6-i18n-scaffold.md) i18n scaffold (M) **done** — depends on 0.2

### B7 Feature modules

- [7.1](b7-modules/7.1-categories-module.md) Categories module (L) **partial** — depends on 2.2, 5.5, 6.3
- [7.2](b7-modules/7.2-item-module.md) Item module (L) **done** — depends on 5.6, 6.3
- [7.3](b7-modules/7.3-search-module.md) Search module (L) **done** — depends on 2.3, 6.3
- [7.4](b7-modules/7.4-discover-module.md) Discover module (L) **done** — depends on 2.2, 5.5, 6.3, 7.7
- [7.5](b7-modules/7.5-updates-module.md) Updates module (M) **partial** — depends on 4.4, 6.3
- [7.6](b7-modules/7.6-ecosystem-module.md) Ecosystem module (M) **done** — depends on 3.3, 3.4, 6.3
- [7.7](b7-modules/7.7-catalog-schema-extension-for-editorial-content.md) Catalog schema extension for editorial content (M) **done** — depends on 1.3

### B8 Motion

- [8.1](b8-motion/8.1-motion-tokens-and-reduced-motion-handling.md) Motion tokens and reduced-motion handling (S) **done** — depends on 5.1
- [8.2](b8-motion/8.2-page-transitions-and-icon-transition.md) Page transitions and icon transition (L) **open** — depends on 6.2, 8.1
- [8.3](b8-motion/8.3-screenshot-carousel-with-spring-snap.md) Screenshot carousel with spring snap (M) **open** — depends on 7.2, 8.1
- [8.4](b8-motion/8.4-sidebar-selection-and-card-interactions.md) Sidebar selection and card interactions (M) **partial** — depends on 8.1

### B9 CLI and background daemon

- [9.1](b9-bins/9.1-storectl-cli.md) storectl CLI (M) **done** — depends on 2.2, 4.3
- [9.2](b9-bins/9.2-interim-bridge-for-the-python-ui.md) Interim bridge for the Python UI (won't do) (M) **closed** — depends on 9.1
- [9.3](b9-bins/9.3-update-daemon.md) Update daemon (L) **done** — depends on 4.4
- [9.4](b9-bins/9.4-update-notifications.md) Update notifications (S) **open** — depends on 9.3

### B10 Packaging and release

- [10.1](b10-release/10.1-debian-and-asset-staging-rewrite.md) DEBIAN and asset staging rewrite (M) **partial**
- [10.2](b10-release/10.2-arch-pkgbuild.md) Arch PKGBUILD (M) **open**
- [10.3](b10-release/10.3-flatpak-manifest-optional.md) Flatpak manifest (optional) (M) **done**
- [10.4](b10-release/10.4-remove-the-python-tree.md) Remove the Python tree (S) **done** — depends on 7.1, 7.2, 7.3, 7.4, 7.5, 7.6
- [10.5](b10-release/10.5-release-automation.md) Release automation (M) **open**

### B11 Quality

- [11.1](b11-quality/11.1-accessibility-pass.md) Accessibility pass (L) **open**
- [11.2](b11-quality/11.2-spanish-translation-and-translation-workflow.md) Spanish translation and translation workflow (M) **done** — depends on 6.6
- [11.3](b11-quality/11.3-screenshot-regression-tests.md) Screenshot regression tests (M) **open** — depends on 5.5
- [11.4](b11-quality/11.4-performance-job-in-ci.md) Performance job in CI (M) **done** — depends on 0.4
- [11.5](b11-quality/11.5-threat-model-for-privileged-paths-and-urls.md) Threat model for privileged paths and URLs (M) **done**
- [11.6](b11-quality/11.6-require-checksums-for-native-package-installs.md) Require checksums for native package installs (M) **open**
- [11.7](b11-quality/11.7-sign-and-verify-the-catalog.md) Sign and verify the catalog (L) **open**
- [11.8](b11-quality/11.8-stop-using-pacman-u-overwrite-for-downloaded-packages.md) Stop using pacman -U --overwrite for downloaded packages (S) **open**
- [11.9](b11-quality/11.9-refuse-plain-http-package-downloads.md) Refuse plain-HTTP package downloads (S) **open**
- [11.10](b11-quality/11.10-load-screenshots-only-on-request.md) Load screenshots only on request (S) **open**

### B12 Docs and community

- [12.1](b12-docs/12.1-architecture-guide-and-add-a-module-tutorial.md) Architecture guide and add-a-module tutorial (M) **done** — depends on 6.3
- [12.2](b12-docs/12.2-add-a-source-tutorial.md) Add-a-source tutorial (M) **done** — depends on 3.6
- [12.3](b12-docs/12.3-catalog-submission-guide.md) Catalog submission guide (S) **done** — depends on 1.3
- [12.4](b12-docs/12.4-good-first-issue-curation.md) Good-first-issue curation (S) **done**
- [12.5](b12-docs/12.5-add-a-version-flag-to-the-app-and-storectl.md) Add a --version flag to the app and storectl (S) **open**
- [12.6](b12-docs/12.6-shell-completions-for-pulsar-storectl.md) Shell completions for pulsar-storectl (S) **open**
- [12.7](b12-docs/12.7-show-the-installed-edition-on-the-item-page.md) Show the installed edition on the item page (S) **open**
- [12.8](b12-docs/12.8-copy-install-link-button-on-the-item-page.md) Copy install link button on the item page (S) **open**
- [12.9](b12-docs/12.9-german-translation.md) German translation (M) **open**
- [12.10](b12-docs/12.10-french-translation.md) French translation (M) **open**
- [12.11](b12-docs/12.11-sort-options-for-item-lists.md) Sort options for item lists (M) **open**
- [12.12](b12-docs/12.12-keyboard-shortcuts-window.md) Keyboard shortcuts window (S) **open**
- [12.13](b12-docs/12.13-screenshot-script-for-non-hyprland-desktops.md) Screenshot script for non-Hyprland desktops (S) **open**
- [12.14](b12-docs/12.14-unit-tests-for-the-info-grid.md) Unit tests for the info grid (S) **open**
