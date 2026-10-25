# Feature tickets inspired by Vicinae

Reference: https://github.com/vicinaehq/vicinae (GPL-3.0). Behavior reference only; re-implement,
do not copy code. Target platform is GNOME (Pulsar OS). Items marked "GNOME check" cannot be
validated on the Omarchy dev machine.

Architecture rule (same as `src/core`): pure logic in `src/core/*.rs` with no gtk imports and unit
tests; GTK wiring stays in `src/ui/`.

## V1. Script commands (M)
Raycast-compatible script commands. Scripts live in `~/.config/pulsaros-spotlight/scripts/` (and an
optional extra dir from config). A script is any executable with header comments:
`@raycast.schemaVersion 1`, `@raycast.title`, `@raycast.mode` (`silent|compact|fullOutput|inline`),
`@raycast.icon`, `@raycast.packageName`, `@raycast.argument1` (JSON: type text|password|dropdown,
placeholder, optional, data).
- Parse headers (`#`, `//` comment styles), validate, skip invalid with a logged reason.
- Run with `std::process::Command`, no shell string. Arguments passed as argv. Timeout and output cap.
- Modes: `silent` shows nothing except failure toast; `compact` shows last line as toast; `fullOutput` opens a pushed view; `inline` refreshes subtitle.
- Appears as `Command` rows with the package name as breadcrumb. Arguments use the argument pill.
Accept: sample scripts for every mode have tests on parse and run results; a bad header never crashes.

## V2. Quicklinks (S)
User-defined `alias -> URL template` with `{query}` placeholder (optionally `{clipboard}`).
Stored in `settings.json`. Typing `gh rust` with quicklink `gh = https://github.com/search?q={query}`
shows a row "Open GitHub search: rust". URL-encode the query. Open via `xdg-open`/`gio open`.
Accept: tests for encoding, missing placeholder, duplicate alias.

## V3. Snippets (M)
Text snippets with keyword and placeholders: `{clipboard}`, `{date}`, `{date:%Y-%m-%d}`, `{time}`,
`{uuid}`, `{cursor}` (cursor position after paste), `{argument name="x"}`.
Stored in `db/snippets.json`. Searchable; Enter pastes through the existing paste path
(extension virtual keyboard, `wl-copy` fallback). Auto-expansion while typing is out of scope
(needs input access; GNOME check).
Accept: placeholder engine unit tests with injected clock; unknown placeholder kept literally.

## V4. Emoji and symbol picker (S-M)
Searchable emoji list with names, keywords, skin tone variants, recently used first (frecency).
Enter pastes (or copies if paste is unavailable). Dataset generated into a Rust source or JSON at
build-time-independent step (script checked in, output checked in), never fetched at runtime.
Accept: search `heart` returns hearts; recent items first; tests on search ranking.

## V5. dmenu mode (S)
`pulsaros-spotlight --dmenu [--prompt TEXT] [--placeholder TEXT]` reads newline-separated items
from stdin, shows them in the launcher list (fuzzy search through the core ranker), prints the
selected item to stdout and exits 0; Esc exits 1. Does not touch the normal single instance.
Accept: `printf 'a\nb\n' | pulsaros-spotlight --dmenu` returns the picked line; exit codes tested at the logic level.

## V6. Window and workspace switcher (M, GNOME check)
GNOME has no generic window-list API for apps. Add D-Bus methods to the Shell extension:
`ListWindows() -> a(ussss)` (id, title, wm_class, app_id, workspace) and `Activate(id)`,
`Close(id)`. Rust side: zbus client in `src/` with the same data types in `src/core`
(`WindowEntry`, ranking by title, wm_class, frecency). Extension exposes the bus name
`es.inled.PulsarosSpotlight.Windows`. Degrades silently when the extension is absent.
Accept: core ranking tests; extension JS passes `gjs` syntax check; D-Bus API documented in the file header.

## Out of scope for now
Browser tab switcher, React extension runtime, store, telemetry, cross-platform abstractions.
