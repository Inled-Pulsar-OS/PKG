# oma-cast parity: diagnostic and tickets

Goal: give pulsaros-spotlight-launcher (GNOME) the same user experience as
[oma-cast](https://github.com/typecraft-dev/oma-cast) (Omarchy / Hyprland / Quickshell).
Only the experience is ported. oma-cast is a clean-room Raycast clone and its code is
Quickshell QML + JS, so nothing is copied. Logic is re-implemented in the language this repo ships.

## 1. Diagnostic: this repo

### What it is
A macOS-Spotlight-style launcher for Pulsar OS (Ubuntu/GNOME based, `.deb` packaging).

| Piece | Path | Role |
|---|---|---|
| Rust app (primary) | `src/` (about 3.9k lines) | GTK4 + libadwaita UI, zbus, rusqlite. Installed as `/usr/bin/pulsaros-spotlight` by `prepare-assets.sh`. |
| Python app (legacy / mirror) | `src_python/pulsaros_spotlight/` (about 2.7k lines) | PyGObject version of the same app. Same module split as the Rust one. |
| GNOME Shell extension | `gnome-shell-extension/` | Top-bar icon, Super-key handling, Wayland paste via Clutter virtual keyboard. Shell 45 to 51. |
| Helper scripts | `cli/pulsaros-toggle-launcher`, `cli/pulsaros-toggle-remap` | Switch Spotlight vs native Overview, edit dock favorites, disable `overlay-key`. |
| Styling | `data/*.css` | base, dark, light, liquid-glass. Switched on libadwaita color scheme. |
| Packaging | `DEBIAN/`, `prepare-assets.sh` | Depends on gnome-shell, GTK4, libadwaita, localsearch/tracker, wl-clipboard. Recommends ydotool. |

### How it works
1. Window opens (extension or `.desktop`). A `Gtk.Entry` drives search with a debounce.
2. Instant in-memory providers run first: apps (parsed `.desktop` files, localized names, `Gio.FileMonitor` reload), calculator (AST-whitelisted eval), clipboard history (JSON, `Gdk.Clipboard` listener), web fallback.
3. File search runs async against Tracker/TinySPARQL (`localsearch`) over D-Bus with category filters (documents, images, audio, video).
4. Results show as list or grid. Tab cycles categories (all, apps, files, clipboard, web). Right-click menu: open, open folder, pin to dock, uninstall, copy name/path.
5. Paste of a clipboard item goes through the extension (virtual keyboard), with `wl-copy` / `ydotool` as fallback.

### Dependencies
- Rust: gtk4 0.9, libadwaita 0.7, gdk-pixbuf, zbus 4.4, rusqlite 0.32, serde, regex, dirs, mime_guess, urlencoding.
- Python: pygobject >= 3.48, Python >= 3.11.
- System: gnome-shell, libgtk-4, libadwaita, localsearch or tracker-miner-fs, wl-clipboard, optional ydotool.

### Features today
App launch, file search by category, directory browsing (`/path`), basic calculator (arithmetic + sqrt/trig/log), clipboard history with auto-paste, web search result, pinned favorites (dock), uninstall, list/grid view, four themes, indexing-progress banner, i18n `.desktop` strings.

### Problems found (fix before or during the port)
- **Two parallel implementations.** `src/` (Rust) and `src_python/` duplicate every module. Packaging ships Rust only. Drift risk.
- **Committed 5 MB ELF** at `cli/pulsaros-spotlight` (tracked in git). Build output should not live in the repo.
- **Version drift:** `Cargo.toml` / `pyproject.toml` say 2.0.0, `DEBIAN/control` says 2.0.34.
- **`pyproject.toml` broken:** `packages.find where = ["src"]` but Python code is in `src_python/`; `readme = "README.md"` does not exist.
- **No tests, no CI, no README** in this repo.
- **No ranking.** Order is app-name match then Tracker order. No frecency, no learning.
- **Key handling is hard-coded** in one `_on_key_pressed` method. No action panel, no shortcut table.
- **Config is tiny:** only `is_grid_view`, `clipboard_max_items`, `clipboard_auto_paste`.
- **Pins are dock-only.** No in-launcher favorites, aliases, or hidden commands.

## 2. Diagnostic: oma-cast

### What it is
A Raycast-like launcher, built as an `omarchy-shell` plugin (Quickshell). No Electron, no Node, no daemon. Opens in about 40 to 60 ms. About 417 stars, no license file.

### Architecture
- `plugin/Launcher.qml` entry, `Backend.qml` in-process backend, `ui/` QML views, `services/` I/O (JSON store, Omarchy menu source, calculator data, agent launcher).
- `plugin/core/*.js` is pure logic, no Qt imports, one file per module, unit-tested with `node --test`: `Ranker`, `Frecency`, `RootSearch`, `Calculator`, `OmarchyMenu`, `Rpc`, `Schema`, `Migrate`, `Paths`, `Agent`, `Log`.
- `bin/oma-cast` is a bash CLI: `install`, `uninstall`, `reload`, `dev`, `toggle|show|hide`, `run <id>`, `call <method> [json]`, `doctor`, `setup hyprland`.
- IPC via `omarchy-shell oma-cast <fn>`; hotkey via Hyprland global shortcut `oma-cast:toggle` (Alt+Space).
- Storage is JSON under XDG: `db/frecency.json`, `db/settings.json`, `db/calculator.json`, `config.json`.
- Theme is the host's live colors. There is no theme config.

### Dependencies
Omarchy 4, Hyprland 0.56+, Quickshell 0.3.x, `socat`, bash. Dev: node (tests), inotifywait. Nothing else. This is a design rule ("keep external dependencies near zero").

### Experience to match (the actual product)
1. **Ranked search** with a ported Raycast root-search ranker, frecency (+100 per launch, 10-day half-life) and learned search terms (`ff` then Firefox keeps meaning Firefox).
2. **Sections:** empty root shows Favorites, Suggestions, Commands, Applications. A query shows Results then `Use "<q>" with…` fallbacks (Google, DuckDuckGo).
3. **One search for system actions** (about 400 Omarchy menu items) with breadcrumbs, live ✓ state, and confirm alerts for Shut Down / Restart / Log Out.
4. **Personalization:** aliases (exact match goes top), favorites (reorder), disabled commands, all undoable by toast.
5. **Keyboard first:** Ctrl+K action panel; ↵ / Ctrl+↵ / Ctrl+Shift+↵ run actions 1 to 3; Ctrl+1 to 9 quick pick; Ctrl+↑↓ section jump; ↑ on empty = history; Esc back, Shift+Esc to root; emacs / vim navigation modes; zoom.
6. **Calculator card** with units, currencies, dates, time zones, bases, percentages, history.
7. **Feedback:** footer toasts, HUD on close, 2 px loading bar, argument pills (Tab into a query field).
8. **Window behavior:** pop-to-root timeout, compact mode, escape behavior setting, hover-select only after real pointer move.
9. **Ops UX:** one-command install, `doctor`, `reload`/`dev`, `run <id>` CLI, smoke test with latency budget.
10. **Ask default agent** hand-off (Omarchy-specific).

oma-cast roadmap items (not done there either): settings window, script commands, clipboard history, snippets, quicklinks, extensions, window management, `.rayconfig` import.

## 3. Gap summary

| oma-cast UX | pulsaros-spotlight today | Gap |
|---|---|---|
| Ranker + frecency + learned terms | None | Large |
| Sections and fallbacks | Flat list + category tabs | Large |
| System-action provider | None (apps and files only) | Large |
| Aliases / favorites / disable | Dock pin only | Medium |
| Ctrl+K action panel, numbered picks | Right-click menu only | Large |
| Search history, nav stack, pop-to-root | Directory browse only | Medium |
| Rich calculator + history | Basic AST calculator | Medium |
| Toasts, HUD, alerts, loading bar | Indexing banner only | Medium |
| Live host theme | CSS themes by color scheme | Small |
| CLI (`toggle/run/call/doctor/setup`) | `toggle-launcher`, `toggle-remap` only | Medium |
| Unit tests, smoke test, CI | None | Large |
| Clipboard history | Present | Keep (oma-cast has none yet) |
| File search (Tracker) | Present | Keep (oma-cast has none yet) |

Keep clipboard history and file search: they are ahead of oma-cast.

## 4. Decisions to take first

- **D1. Rust or Python.** Recommendation: Rust (`src/`) is what ships. Port into Rust, delete `src_python/` (ticket 0.2).
- **D2. Where ranking logic lives.** Put ranker / frecency / aliases in one pure module with no GTK imports, so it is unit-testable like `plugin/core/*.js`.
- **D3. Storage.** JSON files under XDG like oma-cast (simple, inspectable). SQLite already in `Cargo.toml` only if clipboard needs it.
- **D4. Hotkey.** Default Alt+Space via GNOME custom keybinding or extension keybinding. Do not take Super+Space by force (oma-cast rule). `toggle-remap` already offers Super opt-in.
- **D5. Licensing.** Do not copy oma-cast code (no license). Re-implement from the documented behavior. Do not use Raycast assets.

## 5. Tickets

Format: ID, title, size (S under 1 day, M 1 to 3 days, L over 3 days), depends on, then description and acceptance criteria.

### Epic 0: Foundation

**0.1 Add tests and CI skeleton** (M)
Add `cargo test` unit tests and a GitHub Actions workflow (fmt, clippy, test). Add a README with install and dev steps.
Accept: CI green on a PR; README documents build, install, `.deb` staging.

**0.2 Consolidate on Rust and clean the repo** (S) (D1)
Delete `src_python/`, drop the tracked `cli/pulsaros-spotlight` binary (add to `.gitignore`, build in `prepare-assets.sh`), align version in `Cargo.toml` and `DEBIAN/control`, remove or fix `pyproject.toml`.
Accept: `git ls-files` has no ELF; one version number; `prepare-assets.sh` builds from clean checkout.

**0.3 Split pure logic from GTK** (M) (D2)
Create `src/core/` (no gtk imports): `ranker`, `frecency`, `root_search`, `settings_store`, `calculator`. UI calls it through plain structs.
Accept: `cargo test` covers core without a display.

**0.4 JSON store with schema and migration** (S) (D3)
`~/.local/share/pulsaros-spotlight/db/{frecency,settings,calculator}.json` with a `version` field, atomic write, tolerant read. Move `config.json` keys into the new layout with a one-time migration.
Accept: corrupt file does not crash the app; old config is migrated; tests for both.

### Epic 1: Search quality

**1.1 Root-search ranker** (M) depends 0.3
Implement a documented ranker: exact / prefix / word-boundary / initials / subsequence scoring over title, keywords, alias, with stable tie-breaks. Write golden tests from a table of query to expected order.
Accept: `ff` ranks Firefox over "Fast Fonts"; golden test file in repo.

**1.2 Frecency and learned search terms** (M) depends 0.4, 1.1
Record launches: +100 per launch, 10-day half-life decay. Store `query -> chosen id`; typing the same prefix boosts that item.
Accept: pick Firefox after `ff` twice, then `ff` returns Firefox first even if another app scores higher on text; decay unit-tested with a fake clock.

**1.3 Sectioned root results** (M) depends 1.1, 1.2
Empty query: Favorites, Suggestions (frecency above threshold, top 6), Commands, Applications. With a query: Results, then web-search fallbacks. Section headers in UI, section index available to key handlers. Keep the existing category tabs as a filter dropdown.
Accept: screenshot parity with oma-cast layout; Suggestions hidden on a fresh profile.

**1.4 Web-search fallbacks with argument pill** (S) depends 1.3
Replace the single web result with `Use "<q>" with…` rows (Google, DuckDuckGo, configurable list). Tab moves into a pill that holds the query, Shift+Tab out.
Accept: fallbacks appear only when query is non-empty; list editable in settings JSON.

**1.5 GNOME commands provider** (L) depends 1.3
New provider that indexes system actions with breadcrumbs, as oma-cast does for the Omarchy menu. Sources: GNOME Settings panels (`gnome-control-center <panel>`), common `gsettings` toggles (dark mode, night light, do-not-disturb, Wi-Fi / Bluetooth via `nmcli` / `bluetoothctl`), power (lock, suspend, restart, shut down, log out), screenshot, empty trash. Each command has `id`, `title`, `breadcrumb`, `keywords`, optional `state()` for the ✓ mark.
Accept: at least 60 commands; typing `dark` shows Appearance > Dark Style with ✓ when on; each id runs through the CLI (`run <id>`).

**1.6 Confirm alerts for destructive commands** (S) depends 1.5
Shut Down, Restart, Log Out, Empty Trash show an alert (↵ confirms, Esc cancels). `run <id>` skips it.
Accept: no destructive command runs on a single ↵ without the alert.

### Epic 2: Personalization

**2.1 Aliases** (S) depends 0.4, 1.1
Set an alias per item; exact alias match goes to top; duplicates rejected with a clear message.
Accept: alias `ff` set on Firefox always ranks first; duplicate shows error toast.

**2.2 In-launcher favorites with reorder** (M) depends 1.3
Favorites section on empty root. Ctrl+Shift+F toggles, Ctrl+Alt+↑↓ reorders. Keep "Pin to dock" as a separate action.
Accept: order persists across restarts.

**2.3 Disable and re-enable commands** (S) depends 0.4
Alt+Shift+D hides an item, toast offers Undo. Add a "Disabled commands" built-in view to re-enable (oma-cast lacks this until its settings window).
Accept: hidden item absent from search; re-enable works without editing JSON.

### Epic 3: Keyboard and navigation

**3.1 Central shortcut table** (M) depends 0.3
Replace the `_on_key_pressed` if-chain with a table `(context, key) -> action`, shown in a help view. Needed by every ticket below.
Accept: all existing keys keep working; table has unit tests for conflicts.

**3.2 Action panel (Ctrl+K)** (L) depends 3.1
Filterable panel of actions for the selected result. ↵ / Ctrl+↵ / Ctrl+Shift+↵ run the first three actions without opening it. Move the right-click menu content in (open, open folder, copy name, copy path, pin, uninstall, add alias, favorite, disable) and make right-click open the same panel.
Accept: every old context-menu entry reachable by keyboard; per-row shortcut hints shown.

**3.3 Quick picks and section jump** (S) depends 1.3, 3.1
Ctrl+1 to 9 runs the Nth visible result (number badges on rows). Ctrl+↑ / Ctrl+↓ jump between sections. PageUp / PageDown.
Accept: badges match visible order after filtering.

**3.4 Search history** (S) depends 0.4, 3.1
↑ on an empty query lists recent searches. Do not store agent / secret-like queries (see 6.2).
Accept: history capped, deduped, clearable.

**3.5 Navigation stack and pop-to-root** (M) depends 3.1
Pushed views (submenus, alias editor, calculator history, clipboard) with back on Esc / Backspace-on-empty / header arrow; Shift+Esc to root. Settings: `escapeKeyBehavior`, `popToRootTimeout`, compact mode (header only until typing). Reuse the existing directory-browse as a pushed view.
Accept: matches oma-cast behavior list; timeout setting honored.

**3.6 Navigation binding modes** (S) depends 3.1
`default`, `emacs` (Ctrl+N / P), `vim-motions` (Ctrl+J / K, action panel on Alt+K) via setting.
Accept: no conflict with text-entry editing keys; tests.

**3.7 Zoom** (S) depends 3.1
Ctrl + / Ctrl − / Ctrl 0 scale UI via CSS font-size.
Accept: persists; resets with Ctrl 0.

### Epic 4: Feedback and polish

**4.1 Toasts, HUD, loading bar** (M)
Footer toast with Ctrl+T for toast actions (Copy Error, Undo). Brief HUD after the window closes on a launch or copy. Thin loading bar while providers refresh.
Accept: replaces the current indexing banner for non-Tracker work; Tracker indexing keeps its banner.

**4.2 Hover selects only after real pointer move** (S)
Ignore synthetic motion when the window maps under a stationary pointer.
Accept: opening the launcher with the mouse resting on a row does not change selection.

**4.3 Live theme follow** (S)
No theme setting. Follow `org.gnome.desktop.interface` color-scheme and accent color live (Adwaita `StyleManager`), keep liquid-glass as an optional style.
Accept: switching dark mode or accent in GNOME Settings updates the open window.

### Epic 5: Calculator

**5.1 Calculator engine rewrite** (L) depends 0.3
Own expression engine in Rust: percentages (`$30 + 20%`, `20% off 50`), ~250 units, bases (`255 to hex`), dates (`3 days from now`), time zones, mixed units, Unix time. No external `qalc` dependency.
Accept: table test of at least 150 query/answer pairs.

**5.2 Currency rates** (S) depends 5.1
Daily ECB rates cached in `~/.cache/pulsaros-spotlight/currencies.json`; setting `currencyRates:false` disables network.
Accept: works offline from cache; no request when disabled.

**5.3 Calculator card and history** (M) depends 5.1, 3.5
Two-pane card (question -> answer). ↵ copies, Ctrl+↵ pastes (reuse extension virtual keyboard), Ctrl+Shift+C copies "question = answer". History view (4096 rows / 90 days), Ctrl+D deletes one, Ctrl+Shift+D all.
Accept: history persists, searchable.

### Epic 6: Platform integration and ops

**6.1 `pulsaros-spotlight` CLI and D-Bus API** (M) depends 0.3
Single-instance app with D-Bus actions: `toggle | show | hide | run <id> | call <method> [json] | doctor`. The extension and `.desktop` use the same API (zbus is already a dependency).
Accept: `run gnome:appearance.dark` works with launcher closed; toggle latency measured.

**6.2 Default-agent hand-off (optional)** (M) depends 1.3
GNOME analog of "Ask default agent": fallback row and Ctrl+Alt+↵ that opens the user's default terminal with a configured agent command, query passed as a single argv element, never auto-submitted, not stored in history. Skip if the product owner does not want it.
Accept: preflight checks command and terminal; closing the launcher cancels a pending hand-off.

**6.3 Hotkey setup command** (S) depends 6.1
`pulsaros-spotlight setup hotkey` registers Alt+Space (custom GNOME keybinding or the extension keybinding), idempotent, reversible. Merge with `pulsaros-toggle-remap` rather than duplicating it.
Accept: second run changes nothing; `setup --remove` restores previous state.

**6.4 Doctor** (S) depends 6.1
Checks: localsearch running and indexed, wl-clipboard present, extension enabled for current Shell version, hotkey bound, ydotool / uinput permission, D-Bus name owned. Prints fix hint per failure.
Accept: exit code non-zero on failure; each check unit-testable.

**6.5 Install / dev loop** (S)
`make install`, `make dev` (rebuild and restart on save with `cargo watch` or `inotifywait`), uninstall target. Mirrors `oma-cast install / dev / uninstall`.
Accept: one command from clone to working launcher on a dev machine.

**6.6 Smoke test with latency budget** (S) depends 6.1
Script that toggles show/hide on the live session and fails if show-to-mapped exceeds 100 ms. Never types keys (oma-cast lesson: keys sent within about 40 ms of show go to the window below).
Accept: runs in dev; documented budget. Keep the window resident to meet it.

## 6. Suggested order

1. Phase A (foundation): 0.1, 0.2, 0.3, 0.4, 3.1
2. Phase B (the "feel"): 1.1, 1.2, 1.3, 3.2, 3.3, 4.1
3. Phase C (personalization + commands): 2.1, 2.2, 2.3, 1.4, 1.5, 1.6, 3.4, 3.5
4. Phase D (calculator + polish): 5.1, 5.2, 5.3, 4.2, 4.3, 3.6, 3.7
5. Phase E (ops): 6.1, 6.3, 6.4, 6.5, 6.6, 6.2

Highest value per effort: 1.2 (frecency), 1.3 (sections), 3.2 (action panel). These three create most of the oma-cast feel.

## 7. Risks

- GNOME Shell on Wayland blocks global hotkeys and synthetic keys from normal apps. Hotkey and paste must stay in the extension (already true today).
- Shell extension API changes each GNOME release (45 to 51 listed). Keep a version matrix and test on the oldest and newest.
- Tracker / localsearch availability varies; file search must degrade quietly.
- Rust and Python code both exist; any ticket done before 0.2 risks being done twice.
- oma-cast has no license; treat it as behavior reference only.
