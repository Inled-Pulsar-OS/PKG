//! Discover: a hero card, a few rows per ecosystem, and the catalog announcement.

mod view;

use crate::modules::{Module, SidebarEntry};
use crate::tr;
use crate::ui::{Ctx, Route};
use gtk4 as gtk;

pub struct Discover;

impl Module for Discover {
    fn id(&self) -> &'static str {
        "discover"
    }

    fn sidebar(&self) -> Vec<SidebarEntry> {
        vec![SidebarEntry {
            title: tr!("Discover"),
            icon: "starred-symbolic",
            route: Route::Discover,
            order: 0,
        }]
    }

    fn handles(&self, route: &Route) -> bool {
        *route == Route::Discover
    }

    fn build(&self, _route: &Route, ctx: &Ctx) -> gtk::Widget {
        view::build(ctx)
    }
}
