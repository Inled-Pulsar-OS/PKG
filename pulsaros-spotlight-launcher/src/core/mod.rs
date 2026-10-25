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
//! - `script_commands`: `parse_header(text)->Result<Header,String>`, `discover(&[dirs])->Discovery{scripts,skipped}`
//!   (`default_dir()` = `~/.config/pulsaros-spotlight/scripts`), `to_command(&Script)->commands::Command`
//!   (id `script:<rel>`), `run(&Script,&args,&dyn Runner)->Outcome{Silent|Compact|FullOutput|Inline|Failed}`
//!   (30s timeout via `run_with`, 1 MiB cap, `SystemRunner`; fake `Runner` in tests).
//! - `quicklinks::Quicklinks`: `load/save(&Store)`, `add(alias,name,url)` (rejects duplicates), `remove`,
//!   `resolve(input)->Option<Resolved{title,url}>` / `resolve_with_clipboard`, `open_argv(url)` = `xdg-open url`.
//! - `snippets::{Snippets, render, Env}`: snippet CRUD/search (`db/snippets.json`) and the placeholder
//!   engine (`{clipboard} {date[:fmt]} {time} {datetime} {uuid} {cursor} {argument name=".."}`); see module doc.
//! - `emoji::{search, record_use, all}`: embedded `data/emoji.tsv` (made by `scripts/gen-emoji.py`),
//!   ranked search, recents first on empty query, skin tones folded as `Emoji::variants`.
//! - `dmenu`: `Args::{wants_dmenu,parse}`, `read_items(reader)`, `filter(query,&items)` (fuzzy via ranker),
//!   `Outcome::{Selected(s),Cancelled}` with `exit_code()` (0/1) and `stdout()`.
//! - `windows`: `WindowEntry{id,title,wm_class,app_id,workspace}`, `rank(q,&wins,&frecency,now)`,
//!   `to_row`/`rows(..)` give `Row`s with id `window:<id>` and actions `window_activate`/`window_close`.
#![allow(dead_code)]

pub mod commands;
pub mod emoji;
pub mod dmenu;
pub mod frecency;
pub mod quicklinks;
pub mod ranker;
pub mod root_search;
pub mod script_commands;
pub mod settings;
pub mod snippets;
pub mod shortcuts;
pub mod store;
pub mod windows;
