//! Search: results as you type (the shell debounces the entry), and recent searches when empty.

use crate::core::i18n;
use crate::core::search::search;
use crate::modules::Module;
use crate::modules::SidebarEntry;
use crate::tr;
use crate::ui::page::{self, live_page};
use crate::ui::tokens::SPACE_S;
use crate::ui::widget::item_grid::item_grid;
use crate::ui::{Ctx, Route};
use gtk4 as gtk;
use gtk4::prelude::*;

pub struct Search;

impl Module for Search {
    fn id(&self) -> &'static str {
        "search"
    }

    fn sidebar(&self) -> Vec<SidebarEntry> {
        Vec::new()
    }

    fn handles(&self, route: &Route) -> bool {
        matches!(route, Route::Search(_))
    }

    fn build(&self, route: &Route, ctx: &Ctx) -> gtk::Widget {
        let Route::Search(query) = route.clone() else {
            unreachable!("handles() only accepts Search routes")
        };
        live_page(ctx, 960, move |ctx, column| {
            if query.trim().is_empty() {
                column.append(&page::title(&tr!("Search")));
                column.append(&recents(ctx));
                return;
            }
            let items = ctx.items();
            let found = search(&items, &query);
            let heading = i18n::format(&tr!("Results for “{query}”"), &[("query", query.trim())]);
            column.append(&page::title(&heading));
            if found.is_empty() {
                column.append(&page::empty(
                    "system-search-symbolic",
                    &tr!("No results"),
                    &tr!("Try a different name, or search by category or tag."),
                ));
            } else {
                let ids: Vec<String> = found.iter().map(|i| i.id.clone()).collect();
                let grid = item_grid(ctx, &ids);
                grid.set_size_request(-1, 560);
                column.append(&grid);
            }
        })
    }
}

fn recents(ctx: &Ctx) -> gtk::Widget {
    let list = gtk::Box::new(gtk::Orientation::Vertical, SPACE_S);
    let recent = ctx.prefs().recent_searches();
    if recent.is_empty() {
        let hint = gtk::Label::new(Some(&tr!("Search apps, extensions, and Sayri AI.")));
        hint.add_css_class("store-subtitle");
        hint.set_xalign(0.0);
        list.append(&hint);
        return list.upcast();
    }

    let head = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_S);
    let label = gtk::Label::new(Some(&tr!("Recent searches")));
    label.add_css_class("heading");
    label.set_hexpand(true);
    label.set_xalign(0.0);
    head.append(&label);
    let clear = gtk::Button::with_label(&tr!("Clear"));
    clear.add_css_class("flat");
    {
        let (ctx, list) = (ctx.clone(), list.clone());
        clear.connect_clicked(move |_| {
            ctx.prefs().clear_recent_searches();
            ctx.navigate(Route::Search(String::new()));
            list.set_visible(false);
        });
    }
    head.append(&clear);
    list.append(&head);

    for q in recent {
        let button = gtk::Button::with_label(&q);
        button.add_css_class("flat");
        button.set_halign(gtk::Align::Start);
        let (ctx, route) = (ctx.clone(), Route::Search(q.clone()));
        button.connect_clicked(move |_| ctx.navigate(route.clone()));
        list.append(&button);
    }
    list.upcast()
}
