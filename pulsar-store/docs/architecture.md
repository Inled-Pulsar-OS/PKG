# Architecture

One Rust crate with three layers, the same shape as `pulsaros-spotlight-launcher`. The reasons are in `docs/adr/`.

```
src/
  core/      logic, no GTK: model, ports, catalog, search, ledger, install, sources/
  ui/        design system and shared widgets (imports core)
  modules/   one folder per feature (imports core and ui)
  bin/       storectl, updated (import core only)
  app.rs     window, sidebar, glass toolbar, router wiring
```

## Rules

1. `core` imports nothing from `ui`, `modules`, or GTK.
2. `ui` imports only `core`.
3. A module imports only `core` and `ui`, never another module. It opens another page by asking for a `Route`.

`scripts/check-layers.sh` enforces this in CI.

## Data flow

1. `Ctx::load_catalog` reads the catalog (cache, then installed copy, then the copy compiled in) on a worker thread.
2. `Ctx` keeps the items and each item's install state. Widgets subscribe to `Change` events and refresh only what they show.
3. A click on Get calls `Ctx::act`, which queues the action. Operations run one at a time because package managers lock their databases.
4. `Installer::start` runs the action on a thread and streams `Progress` events. The button shows them as a ring.

## Add a module

A module is a folder in `src/modules/` with a `mod.rs` that implements `Module`.

1. Create `src/modules/<name>/mod.rs`:

```rust
pub struct Library;

impl Module for Library {
    fn id(&self) -> &'static str { "library" }
    fn sidebar(&self) -> Vec<SidebarEntry> {
        vec![SidebarEntry { title: tr!("Library"), icon: "folder-symbolic", route: Route::Category("library".into()), order: 6 }]
    }
    fn handles(&self, route: &Route) -> bool { matches!(route, Route::Category(k) if k == "library") }
    fn build(&self, route: &Route, ctx: &Ctx) -> gtk::Widget { /* build with ui::page */ }
}
```

2. Declare it in `src/modules/mod.rs` (`pub mod library;`) and add `Box::new(library::Library)` to `all()`.
3. If it needs a new destination, add a `Route` variant in `src/ui/router.rs`.
4. Wrap every user-facing string in `tr!`. Run `scripts/update-pot.sh`.
5. Run `scripts/check-layers.sh`, `scripts/check-i18n.sh`, `cargo clippy`, and add a screenshot line to `scripts/screenshot-all.sh`.

The tests in `src/modules/mod.rs` check that ids are unique and every route has exactly one module.
