//! One folder per feature. A module imports `core` and `ui`, never another module;
//! it opens other pages by asking the router for a `Route`.

pub mod categories;
pub mod discover;
pub mod ecosystem;
pub mod item;
pub mod search;
pub mod updates;

use crate::ui::{Ctx, Route};
use gtk4 as gtk;

/// A row in the sidebar.
pub struct SidebarEntry {
    pub title: String,
    pub icon: &'static str,
    pub route: Route,
    /// Position in the sidebar, lowest first.
    pub order: u32,
}

pub trait Module {
    fn id(&self) -> &'static str;
    /// Sidebar rows this module contributes; empty for modules reached only by links.
    fn sidebar(&self) -> Vec<SidebarEntry>;
    fn handles(&self, route: &Route) -> bool;
    fn build(&self, route: &Route, ctx: &Ctx) -> gtk::Widget;
}

/// Every module, in no particular order. Adding a feature means adding a line here.
pub fn all() -> Vec<Box<dyn Module>> {
    vec![
        Box::new(discover::Discover),
        Box::new(categories::Categories),
        Box::new(ecosystem::Ecosystem),
        Box::new(updates::Updates),
        Box::new(search::Search),
        Box::new(item::ItemPage),
    ]
}

/// Sidebar rows from all modules, in display order.
pub fn sidebar_entries(modules: &[Box<dyn Module>]) -> Vec<SidebarEntry> {
    let mut rows: Vec<SidebarEntry> = modules.iter().flat_map(|m| m.sidebar()).collect();
    rows.sort_by_key(|r| r.order);
    rows
}

/// The module that builds `route`.
pub fn for_route<'a>(modules: &'a [Box<dyn Module>], route: &Route) -> Option<&'a dyn Module> {
    modules
        .iter()
        .map(|m| m.as_ref())
        .find(|m| m.handles(route))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_ids_are_unique() {
        let mods = all();
        let mut ids: Vec<_> = mods.iter().map(|m| m.id()).collect();
        ids.sort_unstable();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }

    #[test]
    fn every_route_has_exactly_one_module() {
        let mods = all();
        for route in [
            Route::Discover,
            Route::Category("apps".into()),
            Route::Ecosystem(crate::ui::Ecosystem::Sayri),
            Route::Search("x".into()),
            Route::Item("x".into()),
            Route::Updates,
        ] {
            let n = mods.iter().filter(|m| m.handles(&route)).count();
            assert_eq!(n, 1, "{route:?} handled by {n} modules");
        }
    }

    #[test]
    fn sidebar_entries_are_ordered_and_reach_a_module() {
        let mods = all();
        let rows = sidebar_entries(&mods);
        assert!(rows.len() >= 5);
        assert!(rows.windows(2).all(|w| w[0].order <= w[1].order));
        assert!(rows.iter().all(|r| for_route(&mods, &r.route).is_some()));
    }
}
