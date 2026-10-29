//! Browse by category: Applications, Installed, and any category named in the catalog.

mod view;

use crate::modules::{Module, SidebarEntry};
use crate::tr;
use crate::ui::{Ctx, Route};
use gtk4 as gtk;

pub struct Categories;

impl Module for Categories {
    fn id(&self) -> &'static str {
        "categories"
    }

    fn sidebar(&self) -> Vec<SidebarEntry> {
        vec![
            SidebarEntry {
                title: tr!("Applications"),
                icon: "application-x-executable-symbolic",
                route: Route::Category("apps".into()),
                order: 1,
            },
            SidebarEntry {
                title: tr!("Installed"),
                icon: "emblem-default-symbolic",
                route: Route::Category("installed".into()),
                order: 4,
            },
        ]
    }

    fn handles(&self, route: &Route) -> bool {
        matches!(route, Route::Category(_))
    }

    fn build(&self, route: &Route, ctx: &Ctx) -> gtk::Widget {
        match route {
            Route::Category(key) => view::build(ctx, key.clone()),
            _ => unreachable!("handles() only accepts Category routes"),
        }
    }
}
