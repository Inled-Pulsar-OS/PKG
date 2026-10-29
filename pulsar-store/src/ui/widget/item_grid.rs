//! A scrolling grid of item tiles that only builds the rows on screen,
//! so a catalog of thousands of items opens without a stall.

use crate::ui::ctx::Ctx;
use crate::ui::widget::item_tile::item_tile;
use gtk4 as gtk;
use gtk4::prelude::*;

/// Shows the items with the given ids, in order. Ids not in the catalog are skipped.
pub fn item_grid(ctx: &Ctx, ids: &[String]) -> gtk::Widget {
    let model = gtk::StringList::new(&ids.iter().map(String::as_str).collect::<Vec<_>>());
    let factory = gtk::SignalListItemFactory::new();

    factory.connect_setup(|_, obj| {
        if let Some(list_item) = obj.downcast_ref::<gtk::ListItem>() {
            list_item.set_activatable(false);
            list_item.set_selectable(false);
            let slot = gtk::Box::new(gtk::Orientation::Vertical, 0);
            slot.set_margin_top(4);
            slot.set_margin_bottom(4);
            slot.set_margin_start(4);
            slot.set_margin_end(4);
            list_item.set_child(Some(&slot));
        }
    });

    let bind_ctx = ctx.clone();
    factory.connect_bind(move |_, obj| {
        let Some(list_item) = obj.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let (Some(slot), Some(id)) = (
            list_item
                .child()
                .and_then(|c| c.downcast::<gtk::Box>().ok()),
            list_item
                .item()
                .and_then(|o| o.downcast::<gtk::StringObject>().ok()),
        ) else {
            return;
        };
        if let Some(item) = bind_ctx.item(&id.string()) {
            slot.append(&item_tile(&bind_ctx, &item));
        }
    });

    factory.connect_unbind(|_, obj| {
        let Some(list_item) = obj.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        if let Some(slot) = list_item
            .child()
            .and_then(|c| c.downcast::<gtk::Box>().ok())
        {
            while let Some(child) = slot.first_child() {
                slot.remove(&child);
            }
        }
    });

    let grid = gtk::GridView::new(None::<gtk::NoSelection>, Some(factory));
    grid.set_model(Some(&gtk::NoSelection::new(Some(model))));
    grid.add_css_class("store-grid");
    grid.set_min_columns(1);
    grid.set_max_columns(2);
    grid.set_enable_rubberband(false);
    grid.set_vexpand(true);
    grid.set_focusable(true);
    grid.upcast()
}
