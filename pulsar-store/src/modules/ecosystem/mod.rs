//! Pages for GNOME Shell extensions and the Sayri AI ecosystem (skills and plugins).

use crate::core::model::SourceKind;
use crate::modules::{Module, SidebarEntry};
use crate::tr;
use crate::ui::page::{self, page};
use crate::ui::widget::{feature_card::feature_row, item_grid::item_grid};
use crate::ui::{Ctx, Ecosystem as Which, Route};
use gtk4 as gtk;
use gtk4::prelude::*;

pub struct Ecosystem;

fn includes(which: Which, kind: SourceKind) -> bool {
    match which {
        Which::Extensions => kind == SourceKind::GnomeExtension,
        Which::Sayri => matches!(kind, SourceKind::SayriSkill | SourceKind::SayriPlugin),
    }
}

impl Module for Ecosystem {
    fn id(&self) -> &'static str {
        "ecosystem"
    }

    fn sidebar(&self) -> Vec<SidebarEntry> {
        vec![
            SidebarEntry {
                title: tr!("Extensions"),
                icon: "application-x-addon-symbolic",
                route: Route::Ecosystem(Which::Extensions),
                order: 2,
            },
            SidebarEntry {
                title: tr!("Sayri AI"),
                icon: "emblem-system-symbolic",
                route: Route::Ecosystem(Which::Sayri),
                order: 3,
            },
        ]
    }

    fn handles(&self, route: &Route) -> bool {
        matches!(route, Route::Ecosystem(_))
    }

    fn build(&self, route: &Route, ctx: &Ctx) -> gtk::Widget {
        let Route::Ecosystem(which) = *route else {
            unreachable!("handles() only accepts Ecosystem routes")
        };
        page(ctx, 960, move |ctx, column| {
            let heading = match which {
                Which::Extensions => tr!("GNOME Shell Extensions"),
                Which::Sayri => tr!("Sayri AI Ecosystem"),
            };
            column.append(&page::title(&heading));
            let items = ctx.items();
            let shown: Vec<&crate::core::model::Item> =
                items.iter().filter(|i| includes(which, i.kind)).collect();
            let ids: Vec<String> = shown.iter().map(|i| i.id.clone()).collect();
            if shown.len() >= 4 {
                column.append(&feature_row(ctx, &shown[..2]));
            }
            if ids.is_empty() {
                column.append(&page::empty(
                    "emblem-default-symbolic",
                    &tr!("No packages available"),
                    &tr!("No items found in this section."),
                ));
            } else {
                let grid = item_grid(ctx, &ids);
                grid.set_size_request(-1, 560);
                column.append(&grid);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_map_to_sections() {
        assert!(includes(Which::Extensions, SourceKind::GnomeExtension));
        assert!(!includes(Which::Extensions, SourceKind::SayriSkill));
        assert!(includes(Which::Sayri, SourceKind::SayriPlugin));
        assert!(includes(Which::Sayri, SourceKind::SayriSkill));
        assert!(!includes(Which::Sayri, SourceKind::Flatpak));
    }
}
