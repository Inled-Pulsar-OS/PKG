//! Editorial cards: a large tappable tile that opens an item, used on Discover.

use crate::core::model::Item;
use crate::ui::ctx::Ctx;
use crate::ui::tokens::{SPACE_L, SPACE_M, SPACE_XL};
use crate::ui::widget::{icon::item_icon, install_button::InstallButton, item_tile::make_openable};
use gtk4 as gtk;
use gtk4::prelude::*;

/// A card for `item`. A `hero` card is larger and uses the accent gradient.
pub fn item_card(ctx: &Ctx, item: &Item, hero: bool) -> gtk::Widget {
    let card = gtk::Box::new(gtk::Orientation::Vertical, SPACE_M);
    card.add_css_class(if hero {
        "store-card-hero"
    } else {
        "store-card"
    });
    let pad = if hero { SPACE_XL } else { SPACE_L };

    let inner = gtk::Box::new(gtk::Orientation::Vertical, SPACE_M);
    inner.set_margin_top(pad);
    inner.set_margin_bottom(pad);
    inner.set_margin_start(pad);
    inner.set_margin_end(pad);
    card.append(&inner);

    let head = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_M);
    head.append(&item_icon(ctx, item, if hero { 72 } else { 48 }));
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    head.append(&spacer);
    head.append(InstallButton::new(ctx, item).widget());
    inner.append(&head);

    let title = gtk::Label::new(Some(&ctx.name(item)));
    title.set_xalign(0.0);
    title.set_halign(gtk::Align::Start);
    title.set_wrap(true);
    title.add_css_class(if hero { "title-1" } else { "heading" });
    inner.append(&title);

    let summary = gtk::Label::new(Some(&ctx.summary(item)));
    summary.set_xalign(0.0);
    summary.set_halign(gtk::Align::Start);
    summary.set_wrap(true);
    summary.set_lines(if hero { 3 } else { 2 });
    summary.set_ellipsize(gtk::pango::EllipsizeMode::End);
    summary.add_css_class("store-caption");
    inner.append(&summary);

    make_openable(&card, ctx, &item.id, &ctx.name(item));
    card.upcast()
}
