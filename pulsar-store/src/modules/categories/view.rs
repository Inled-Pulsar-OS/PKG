use crate::core::model::{InstallState, Item};
use crate::core::search::by_category;
use crate::tr;
use crate::ui::page::{self, live_page};
use crate::ui::tokens::SPACE_S;
use crate::ui::widget::item_grid::item_grid;
use crate::ui::{Ctx, Route};
use gtk4 as gtk;
use gtk4::prelude::*;

/// Heading for a category key.
pub fn heading(key: &str) -> String {
    match key {
        "apps" => tr!("Applications"),
        "installed" => tr!("Installed"),
        "all" => tr!("All"),
        other => {
            let mut chars = other.chars();
            chars
                .next()
                .map_or_else(String::new, |c| c.to_uppercase().chain(chars).collect())
        }
    }
}

/// Distinct category names the catalog uses, sorted.
pub fn catalog_categories(items: &[Item]) -> Vec<String> {
    let mut names: Vec<String> = items
        .iter()
        .map(|i| i.category.clone())
        .filter(|c| !c.is_empty())
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names.dedup_by_key(|n| n.to_lowercase());
    names
}

pub fn build(ctx: &Ctx, key: String) -> gtk::Widget {
    live_page(ctx, 960, move |ctx, column| {
        column.append(&page::title(&heading(&key)));
        let items = ctx.items();

        if key == "apps" {
            let names = catalog_categories(&items);
            if !names.is_empty() {
                column.append(&chips(ctx, &names));
            }
        }

        let installed = |i: &Item| !matches!(ctx.state(&i.id), InstallState::NotInstalled);
        let ids: Vec<String> = by_category(&items, &key, &installed)
            .iter()
            .map(|i| i.id.clone())
            .collect();
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

fn chips(ctx: &Ctx, names: &[String]) -> gtk::Widget {
    let flow = gtk::FlowBox::new();
    flow.set_selection_mode(gtk::SelectionMode::None);
    flow.set_max_children_per_line(12);
    flow.set_column_spacing(SPACE_S as u32);
    flow.set_row_spacing(SPACE_S as u32);
    for name in names {
        let button = gtk::Button::with_label(name);
        button.add_css_class("pill");
        let (ctx, route) = (ctx.clone(), Route::Category(name.to_lowercase()));
        button.connect_clicked(move |_| ctx.navigate(route.clone()));
        flow.append(&button);
    }
    flow.upcast()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_names_are_distinct_and_sorted() {
        let items: Vec<Item> = serde_json::from_str(
            r#"[{"id":"a","type":"flatpak","category":"Graphics"},{"id":"b","type":"flatpak","category":"audio"},
                {"id":"c","type":"flatpak","category":"graphics"},{"id":"d","type":"flatpak"}]"#,
        )
        .unwrap();
        assert_eq!(catalog_categories(&items), ["audio", "Graphics"]);
    }

    #[test]
    fn headings() {
        assert_eq!(heading("graphics"), "Graphics");
        assert_eq!(heading(""), "");
    }
}
