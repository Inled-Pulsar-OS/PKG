use crate::core::model::{Item, SourceKind};
use crate::core::remote::is_web_url;
use crate::tr;
use crate::ui::page::{self, page};
use crate::ui::tokens::{SPACE_L, SPACE_M};
use crate::ui::widget::{
    app_cell::app_cell,
    feature_card::{eyebrow_for, feature_row},
    hero::hero,
    section::section_header,
};
use crate::ui::{Ctx, Ecosystem, Route};
use gtk4 as gtk;
use gtk4::prelude::*;

/// Two rows of up to five cells.
const ROW_ITEMS: usize = 10;
const FEATURE_CARDS: usize = 3;

type Filter<'a> = &'a dyn Fn(&&Item) -> bool;

/// The banner item and the editorial cards under it. Editors pick the banner by
/// marking an item `featured`; otherwise the first item leads. The cards prefer
/// one item of each kind, so Discover shows the whole catalog at a glance.
pub fn pick_features(items: &[Item]) -> (Option<&Item>, Vec<&Item>) {
    let hero = items
        .iter()
        .find(|i| i.is_featured())
        .or_else(|| items.first());
    let Some(hero) = hero else {
        return (None, Vec::new());
    };
    let mut cards: Vec<&Item> = Vec::new();
    let mut kinds = vec![hero.kind];
    for item in items.iter().filter(|i| i.id != hero.id) {
        if cards.len() == FEATURE_CARDS {
            break;
        }
        if !kinds.contains(&item.kind) {
            kinds.push(item.kind);
            cards.push(item);
        }
    }
    for item in items.iter().filter(|i| i.id != hero.id) {
        if cards.len() == FEATURE_CARDS {
            break;
        }
        if !cards.iter().any(|c| c.id == item.id) {
            cards.push(item);
        }
    }
    (Some(hero), cards)
}

pub fn build(ctx: &Ctx) -> gtk::Widget {
    page(ctx, 1480, |ctx, column| {
        let items = ctx.items();
        if items.is_empty() {
            column.append(&page::title(&tr!("Discover")));
            column.append(&page::empty(
                "emblem-default-symbolic",
                &tr!("No packages available"),
                &tr!("No items found in the catalog yet."),
            ));
            return;
        }

        let (banner, cards) = pick_features(&items);
        if let Some(item) = banner {
            column.append(&hero(ctx, item, &eyebrow_for(ctx, item)));
        }
        if !cards.is_empty() {
            column.append(&feature_row(ctx, &cards));
        }

        let apps = |i: &&Item| matches!(i.kind, SourceKind::Flatpak | SourceKind::System);
        let exts = |i: &&Item| i.kind == SourceKind::GnomeExtension;
        let sayri = |i: &&Item| matches!(i.kind, SourceKind::SayriSkill | SourceKind::SayriPlugin);
        let sections: [(String, Route, Filter); 3] = [
            (
                tr!("Great Apps and Updates"),
                Route::Category("apps".into()),
                &apps,
            ),
            (
                tr!("Extensions"),
                Route::Ecosystem(Ecosystem::Extensions),
                &exts,
            ),
            (tr!("Sayri AI"), Route::Ecosystem(Ecosystem::Sayri), &sayri),
        ];
        for (title, route, filter) in sections {
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
            column.append(&row_flow(ctx, &picks));
        }

        if let Some(banner) = announcement(ctx) {
            column.append(&banner);
        }
    })
}

/// Cells in up to five columns; narrower windows get fewer.
fn row_flow(ctx: &Ctx, items: &[&Item]) -> gtk::Widget {
    let flow = gtk::FlowBox::new();
    flow.set_selection_mode(gtk::SelectionMode::None);
    flow.set_homogeneous(true);
    flow.set_min_children_per_line(1);
    flow.set_max_children_per_line(5);
    flow.set_row_spacing(SPACE_M as u32 / 2);
    flow.set_column_spacing(SPACE_L as u32);
    for item in items {
        flow.append(&app_cell(ctx, item));
    }
    flow.upcast()
}

/// The announcement banner from the catalog, with its link restricted to web URLs.
fn announcement(ctx: &Ctx) -> Option<gtk::Widget> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn items(json: &str) -> Vec<Item> {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn featured_item_leads_and_cards_vary_by_kind() {
        let it = items(
            r#"[{"id":"a","type":"flatpak"},{"id":"b","type":"flatpak","featured":true},
                {"id":"c","type":"flatpak"},{"id":"d","type":"gnome_extension"},{"id":"e","type":"sayri_skill"},
                {"id":"f","type":"sayri_plugin"}]"#,
        );
        let (hero, cards) = pick_features(&it);
        assert_eq!(hero.unwrap().id, "b");
        let ids: Vec<_> = cards.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, ["d", "e", "f"]);
    }

    #[test]
    fn falls_back_to_first_item_and_fills_with_any_kind() {
        let it = items(
            r#"[{"id":"a","type":"flatpak"},{"id":"b","type":"flatpak"},{"id":"c","type":"flatpak"}]"#,
        );
        let (hero, cards) = pick_features(&it);
        assert_eq!(hero.unwrap().id, "a");
        assert_eq!(cards.len(), 2);
        assert!(pick_features(&[]).0.is_none());
    }
}
