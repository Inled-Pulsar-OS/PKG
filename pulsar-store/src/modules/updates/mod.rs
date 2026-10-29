//! Updates: items whose catalog version is newer than the installed one.

use crate::core::i18n;
use crate::core::install::Action;
use crate::core::model::{InstallState, Item};
use crate::modules::{Module, SidebarEntry};
use crate::tr;
use crate::ui::page::{self, live_page};
use crate::ui::tokens::{SPACE_M, SPACE_S};
use crate::ui::widget::{icon::item_icon, install_button::InstallButton};
use crate::ui::{Ctx, Route};
use adw::prelude::*;
use gtk4 as gtk;

pub struct Updates;

/// `(item, installed, available)` for every item with an update waiting.
fn pending(ctx: &Ctx) -> Vec<(Item, String, String)> {
    ctx.items()
        .iter()
        .filter_map(|i| match ctx.state(&i.id) {
            InstallState::UpdateAvailable {
                installed,
                available,
            } => Some((i.clone(), installed, available)),
            _ => None,
        })
        .collect()
}

impl Module for Updates {
    fn id(&self) -> &'static str {
        "updates"
    }

    fn sidebar(&self) -> Vec<SidebarEntry> {
        vec![SidebarEntry {
            title: tr!("Updates"),
            icon: "software-update-available-symbolic",
            route: Route::Updates,
            order: 5,
        }]
    }

    fn handles(&self, route: &Route) -> bool {
        *route == Route::Updates
    }

    fn build(&self, _route: &Route, ctx: &Ctx) -> gtk::Widget {
        live_page(ctx, 860, |ctx, column| {
            let updates = pending(ctx);

            let head = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_M);
            let title = page::title(&tr!("Updates"));
            title.set_hexpand(true);
            head.append(&title);
            let all = gtk::Button::with_label(&tr!("Update All"));
            all.add_css_class("suggested-action");
            all.add_css_class("store-pill");
            all.set_valign(gtk::Align::Center);
            all.set_sensitive(!updates.is_empty() && !ctx.has_active_ops());
            {
                let (ctx, items): (Ctx, Vec<Item>) =
                    (ctx.clone(), updates.iter().map(|u| u.0.clone()).collect());
                all.connect_clicked(move |b| {
                    b.set_sensitive(false);
                    for item in &items {
                        ctx.act(Action::Update, item.clone());
                    }
                });
            }
            head.append(&all);
            column.append(&head);

            if updates.is_empty() {
                column.append(&page::empty(
                    "emblem-ok-symbolic",
                    &tr!("Up to Date"),
                    &tr!("All packages, extensions, and AI plugins are up to date."),
                ));
                return;
            }

            let group = adw::PreferencesGroup::new();
            for (item, installed, available) in updates {
                let row = adw::ActionRow::new();
                row.set_title(&ctx.name(&item));
                row.set_subtitle(&i18n::format(
                    &tr!("{installed} → {available}"),
                    &[("installed", &installed), ("available", &available)],
                ));
                row.add_prefix(&item_icon(ctx, &item, 32));
                let button = InstallButton::new(ctx, &item);
                button.widget().set_margin_start(SPACE_S);
                row.add_suffix(button.widget());
                group.add(&row);
            }
            column.append(&group);
        })
    }
}
