//! Pure-logic core (no gtk/gdk/glib). Everything here is unit-testable without a display.
//!
//! Public API for the UI:
//! - `store::Store` - JSON files under `$XDG_DATA_HOME/pulsaros-spotlight/db/`;
//!   `Store::default_dir()`, `load::<T>(name)` (tolerant, falls back to `Default`), `save(name,&T)` (atomic).
//! - `settings::Settings::load(&Store)` / `.save(&Store)`: aliases (`set_alias` rejects duplicates with
//!   `Err(String)`), favorites (`toggle_favorite`, `move_favorite`), disabled ids (`disable`; `enable`
//!   is the undo), search history (`push_history`, `history`, `clear_history`), web `fallbacks`.
//! - `frecency::Frecency`: `record_launch(id, query, now)`, `score(id, now)`, `learned(query, id)`.
//!   `now` is unix seconds (injected, so tests use a fake clock).
//! - `ranker::{Item, score, rank}`: text score for one query against one item.
//! - `commands::builtin()` -> `Vec<Command>` (60+ `gnome:*` ids); `Command::argv` is the run argv
//!   (never a shell string), `confirm` marks destructive ones; `commands::probe_states(&cmds, &runner)`
//!   returns `id -> checked` using `commands::system_runner` (or a fake in tests).
//! - `root_search::RootSearch::sections(query, &Ctx) -> Vec<Section>` with `Section{title, rows}` and
//!   `Row{id,title,subtitle,breadcrumb,icon_name,kind,checked,alias,favorite,confirm,actions}`.
//!   Feed apps as `root_search::AppEntry`; File/Clipboard/Calc rows are passed in via `Ctx::extra`.
//!   Empty query: Favorites, Suggestions, Commands, Applications. With a query: Results, then
//!   `Use "<q>" with <engine>` Fallback rows. `ActionDesc{id,title,argv}`: `argv` is `Some` when
//!   the action is a plain process spawn (commands, URLs); `None` means the UI handles it by id
//!   (`open`, `favorite`, `alias`, `disable`).
#![allow(dead_code)]

pub mod commands;
pub mod frecency;
pub mod ranker;
pub mod root_search;
pub mod settings;
pub mod store;
