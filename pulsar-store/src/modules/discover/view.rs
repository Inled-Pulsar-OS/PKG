use crate::core::model::{Item, SourceKind};
use crate::core::remote::is_web_url;
use crate::tr;
use crate::ui::page::{self, page};
use crate::ui::tokens::{SPACE_L, SPACE_M};
use crate::ui::widget::{card::item_card, item_tile::item_tile, section::section_header};
use crate::ui::{Ctx, Ecosystem, Route};
use gtk4 as gtk;
use gtk4::prelude::*;

const ROW_ITEMS: usize = 6;

type Filter<'a> = &'a dyn Fn(&&Item) -> bool;

pub fn build(ctx: &Ctx) -> gtk::Widget {
    page(ctx, 960, |ctx, column| {
        column.append(&page::title(&tr!("Discover")));
        let items = ctx.items();
        if items.is_empty() {
            column.append(&page::empty(
                "emblem-default-symbolic",
                &tr!("No packages available"),
                &tr!("No items found in the catalog yet."),
            ));
            return;
        }

        // Editors choose the hero by marking items featured; otherwise the first item leads.
        let hero = items
            .iter()
            .find(|i| i.is_featured())
            .or_else(|| items.first());
        if let Some(hero) = hero {
            column.append(&item_card(ctx, hero, true));
        }

        let apps = |i: &&Item| matches!(i.kind, SourceKind::Flatpak | SourceKind::System);
        let exts = |i: &&Item| i.kind == SourceKind::GnomeExtension;
        let sayri = |i: &&Item| matches!(i.kind, SourceKind::SayriSkill | SourceKind::SayriPlugin);
        let rows: [(String, Route, Filter); 3] = [
            (tr!("Applications"), Route::Category("apps".into()), &apps),
            (
                tr!("Extensions"),
                Route::Ecosystem(Ecosystem::Extensions),
                &exts,
            ),
            (tr!("Sayri AI"), Route::Ecosystem(Ecosystem::Sayri), &sayri),
        ];
        for (title, route, filter) in rows {
            let picks: Vec<&Item> = items.iter().filter(|i| filter(i)).take(ROW_ITEMS).collect();
            if picks.is_empty() {
                continue;
            }
            let nav = ctx.clone();
            column.append(&section_header(
                &title,
                Some((
                    &tr!("See All"),
                    Box::new(move || nav.navigate(route.clone())),
                )),
            ));
            column.append(&tile_flow(ctx, &picks));
        }

        if let Some(banner) = banner(ctx) {
            column.append(&banner);
        }
    })
}

fn tile_flow(ctx: &Ctx, items: &[&Item]) -> gtk::Widget {
    let flow = gtk::FlowBox::new();
    flow.set_selection_mode(gtk::SelectionMode::None);
    flow.set_homogeneous(true);
    flow.set_min_children_per_line(1);
    flow.set_max_children_per_line(2);
    flow.set_row_spacing(SPACE_M as u32);
    flow.set_column_spacing(SPACE_L as u32);
    for item in items {
        flow.append(&item_tile(ctx, item));
    }
    flow.upcast()
}

/// The announcement banner from the catalog, with its link restricted to web URLs.
fn banner(ctx: &Ctx) -> Option<gtk::Widget> {
    let ann = ctx.announcement()?;
    let text = |key: &str| {
        ann.get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let (title, description, label, url) = (
        text("title"),
        text("description"),
        text("action_label"),
        text("action_url"),
    );
    if title.is_empty() {
        return None;
    }
    let row = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_L);
    row.add_css_class("store-banner");

    let col = gtk::Box::new(gtk::Orientation::Vertical, 4);
    col.set_hexpand(true);
    col.set_valign(gtk::Align::Center);
    let t = gtk::Label::new(Some(&title));
    t.add_css_class("heading");
    t.set_xalign(0.0);
    t.set_wrap(true);
    col.append(&t);
    if !description.is_empty() {
        let d = gtk::Label::new(Some(&description));
        d.add_css_class("store-caption");
        d.set_xalign(0.0);
        d.set_wrap(true);
        col.append(&d);
    }
    row.append(&col);

    if is_web_url(&url) && !label.is_empty() {
        let button = gtk::Button::with_label(&label);
        button.add_css_class("suggested-action");
        button.add_css_class("store-pill");
        button.set_valign(gtk::Align::Center);
        button.connect_clicked(move |_| {
            let _ = gtk::gio::AppInfo::launch_default_for_uri(
                &url,
                None::<&gtk::gio::AppLaunchContext>,
            );
        });
        row.append(&button);
    }
    Some(row.upcast())
}
