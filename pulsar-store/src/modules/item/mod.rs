//! The item page: header with install button, screenshots, facts, editions, and description.

mod view;

use crate::modules::{Module, SidebarEntry};
use crate::ui::{Ctx, Route};
use gtk4 as gtk;

pub struct ItemPage;

impl Module for ItemPage {
    fn id(&self) -> &'static str {
        "item"
    }

    fn sidebar(&self) -> Vec<SidebarEntry> {
        Vec::new()
    }

    fn handles(&self, route: &Route) -> bool {
        matches!(route, Route::Item(_))
    }

    fn build(&self, route: &Route, ctx: &Ctx) -> gtk::Widget {
        match route {
            Route::Item(id) => view::build(ctx, id.clone()),
            _ => unreachable!("handles() only accepts Item routes"),
        }
    }
}
