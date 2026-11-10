# Debian GNOME 48 baseline: Arch Settings CSS on libadwaita 1.7.6

Evidence for #136 (spec #134). Date: 2026-11-10. Container `debian:trixie`.

| Item | Value |
|---|---|
| libadwaita | 1.7.6-1~deb13u1 |
| GTK | 4.18.6+ds-2 |
| gnome-control-center built | **48.7** (`prepare-assets.sh` picks the newest `48.*` tag from the GNOME GitLab API; Debian itself ships 48.4. The ticket says 48.4, the build is 48.7) |
| Package | `pulsaros-control-center_1.0.43_amd64.deb` (`DEBIAN/control` says 1.0.42, `package-and-deploy.sh` bumps it in the container copy) |

## Commands

```
scripts/render-settings-trixie.sh docs/img/settings-trixie-appearance.png   # build, install, render, PNG
scripts/probe-selectors.py <workdir>                                        # selector table below
```

The render script runs `./package-and-deploy.sh pulsaros-control-center` (default branch, trixie) in
`debian:trixie` on a copy of the repo (mounted read-only), installs the `.deb` in a second clean
`debian:trixie` container, then starts `gnome-control-center` under `xvfb-run` with
`GSK_RENDERER=cairo GDK_BACKEND=x11 XDG_CURRENT_DESKTOP=GNOME` and screenshots with `import`.
Nothing is deployed or uploaded.

Things the headless rig needed (all in the script, none in the package):

- `XDG_CURRENT_DESKTOP=GNOME`: otherwise the app exits "only supported under GNOME".
- `librsvg2-common`: without it every panel icon is the missing-image placeholder.
- Schemas: `mutter-common`, `gnome-shell-common`, and `org.gnome.shell.extensions.pulsaros-global-menu`
  copied from `pulsaros-global-menu/`. Appearance and Multitasking abort (GLib ERROR, core dump) without them.
- `chmod -R u=rwX,go=rX` on the copy: with umask 027 `dpkg-deb` stops with "control directory has bad permissions 750".

## Build blocker fixed

The `.deb` built (exit 0) but `gnome-control-center` did not start in a clean trixie: `libibus-1.0.so.5` not found.
`ldd` listed 8 libraries that `Depends` did not pull in. Added to `pulsaros-control-center/DEBIAN/control`
(Version 1.0.41 to 1.0.42): `libibus-1.0-5`, `libgnome-bg-4-2t64`, `libgnome-rr-4-2t64`, `libgoa-backend-1.0-2`,
`libsmbclient0`, `libgsound0t64`, `libnma-gtk4-0`, `libgnome-bluetooth-ui-3.0-13`. After that the `.deb` installs
with `apt-get install --no-install-recommends` and `gnome-control-center --version` prints `48.7`.

## Method for the selector table

`scripts/probe-selectors.py` gives each selector in `scripts/probe-selectors.txt` its own colour (outline, fill,
`opacity: 1`), loads the result as `~/.config/gtk-4.0/gtk.css` of the **real** app, clicks through 14 views
(Appearance, Multitasking, Sound, Power, Displays, Apps, Notifications, Mouse, Keyboard, Printers, Accessibility,
Privacy, System, and the scrolled sidebar) and counts the exact pixels of each colour (at least 8 = matched). Colours
exclude every colour of an unprobed render. Selectors hitting the same widget are resolved by repeating the run
with only the unmatched ones. "Matched" means: matched at least one visible widget in one of those views.

Two findings come from other runs: the whole Arch CSS (`pulsar.css` + `css/*.css`, 674 lines) loaded as user CSS
(`docs/img/settings-trixie-appearance-arch-css.png`, shows the look it would give), and a small standalone
libadwaita 1.7.6 window (Adw.NavigationSplitView + Gtk.Scale with marks + ComboRow) for selectors the real
window never shows.

## Result per selector family

| Family (Arch) | On 48.7 / libadwaita 1.7.6 | Evidence |
|---|---|---|
| `navigation-split-view > .content-pane` and all rules scoped by it | **works** | matched in every view; libadwaita 1.7.6 `adw-navigation-split-view.c` doc tree: `widget.sidebar-pane`, `widget.content-pane` are direct children |
| `navigation-split-view > .sidebar-pane` / `> widget` | works (the sidebar bin is a `widget`) | matched |
| `.cc-sidebar-pane`, `list.navigation-sidebar.cc-sidebar` (and `> row`) | **dead** | 0 px: the 48 `cc-window.ui` and `cc-panel-list.c` never set these classes. `list.navigation-sidebar > row`, `:selected` match, so the rules work once the classes are added |
| `preferencesgroup > box > box.header` and `label.heading` | works | matched (Appearance, Keyboard, Sound...) |
| `preferencespage > scrolledwindow > viewport > clamp > box` | works | matched in all panels |
| `list.boxed-list`, `> row`, `row.activatable`, `row:last-child`, `row > box.header`, `box.title` | works | matched |
| `list.boxed-list > row > box.header > image` | works | matched (Accessibility, Privacy, System, Displays) |
| `label.subtitle` under `row > box.header > box.title` | works | matched (needed `opacity: 1`: subtitles are translucent) |
| `label.dim-label` in rows | works | matched (Displays) |
| `row.expander row.header`, `row.expander list.nested > row`, `.card` | **unverified** | no visible instance in any view |
| `row.combo`, `row.combo > box.header > box.suffixes` | works | matched (Appearance, Power, Displays) |
| `row.combo ... box.suffixes > box > image.dropdown-arrow` | works | matched (Power) |
| `row.combo ... box.suffixes > box > label` (value label) | **differs / dead** | 0 px with a visible value label ("5 minutes", Power). The label is not a child of that `box`; real parent not determined |
| `scale trough`, `slider`, `marks mark indicator`, `marks mark label` under `.content-pane` | works in a standalone 1.7.6 window, **not seen in the real app** | standalone counts: trough 2060, slider 96, indicator 24, label 602 px. No scale is visible in any headless view (no audio or input devices) |
| `scale trough highlight`, `ccvolumeslider` (+ `> scale`, `> button.flat`) | **unverified** | `highlight` was not in the standalone run; `ccvolumeslider` needs an output device |
| `switch`, `switch slider`, `switch:checked` | works | matched; `:checked` colour follows `--accent-bg-color` |
| `.pulsar-multitasking`, `.pulsar-display`, `.pulsar-power`, `.pulsar-keyboard`, `.pulsar-user-row`, `.pulsar-status-*`, `.pulsar-ua-row`, `.pulsar-pill` | **dead** | 0 px: Arch patches set these classes, the 48 overlay does not |
| `.pulsar-squircle`, `.pulsar-hub-header`, `.pulsar-group-gap`, `.pulsar-page-title`, `.pulsar-nav-capsule`, `button.pulsar-profile-row`, `entry.pulsar-search-pill` | **dead** | 0 px, same reason. The sidebar squircles you see in the PNG come from the SVG icons of the 48 overlay, not from CSS |
| `.background-preview-button` (and `:checked`) | works | matched (Appearance) |
| `.background-thumbnail`, `.background-flowbox > flowboxchild`, `.pulsar-live-wallpaper` | unverified | wallpaper grid is below the fold |
| `:root { --sidebar-bg-color / --card-bg-color / --accent-bg-color }` | **works** | the override recolours the sidebar, cards and checked switches; `var(--x)` custom properties work in GTK 4.18 (probe) |
| `:root { --view-bg-color }` | unverified | no visible effect in these views |
| `:root { --pulsar-accent-* }` | works as variables, nothing reads them on 48 | they feed `.pulsar-*` rules only |
| `@media (prefers-color-scheme: dark) { ... }` | **dead** | GTK 4.18 logs `Theme parser error: gtk.css:22:1-7: Unknown @ rule`; the dark surface tokens never apply. The only parse error in all 674 lines |

Loading the full Arch CSS on 48.7 gives a different Appearance page (larger group titles, grey rounded
cards, no row borders; `docs/img/settings-trixie-appearance-arch-css.png`) next to the current look
(`docs/img/settings-trixie-appearance.png`). So the shared selectors work, and the missing pieces are
the classes (`cc-sidebar*`, `pulsar-*`) and the dark-mode block.

## Not verified

- Dark mode (the harness renders light only).
- Scales, expander rows, cards, wallpaper grid, volume slider in the real app (not visible headless).
- Whether a matched selector's properties give the same pixels as on Arch (only "matches" is tested; the
  full-CSS PNG is a visual check, not a diff against the Arch screenshot).
- The real parent of the combo value label.
- Real hardware, a real session, gnome-shell overrides (blur, dock), NetworkManager and ModemManager panels
  (the log shows their D-Bus errors; the network panel renders "Network Unavailable").
- The panel was probed with root as the user and no input or audio devices.
