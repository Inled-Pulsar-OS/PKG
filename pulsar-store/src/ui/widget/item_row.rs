//! A compact app row: icon, name, one-line subtitle, install button.
//! This is the form used in the lists on Discover and the category pages.

use crate::core::model::Item;
use crate::ui::ctx::Ctx;
use crate::ui::tokens::SPACE_M;
use crate::ui::widget::{icon::item_icon, install_button::InstallButton, item_tile::make_openable};
use gtk4 as gtk;
use gtk4::prelude::*;

pub fn item_row(ctx: &Ctx, item: &Item) -> gtk::Widget {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_M);
    row.add_css_class("store-row");
    // Keeps grids from squeezing rows into too many columns.
    row.set_size_request(280, -1);
    row.append(&item_icon(ctx, item, 48));

    let text = gtk::Box::new(gtk::Orientation::Vertical, 1);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);
    let name = gtk::Label::new(Some(&ctx.name(item)));
    name.add_css_class("name");
    name.set_xalign(0.0);
    name.set_max_width_chars(26);
    name.set_ellipsize(gtk::pango::EllipsizeMode::End);
    text.append(&name);
    let subtitle = gtk::Label::new(Some(&ctx.summary(item)));
    subtitle.add_css_class("store-caption");
    subtitle.set_xalign(0.0);
    subtitle.set_max_width_chars(32);
    subtitle.set_ellipsize(gtk::pango::EllipsizeMode::End);
    text.append(&subtitle);
    row.append(&text);

    row.append(InstallButton::new(ctx, item).widget());
    make_openable(&row, ctx, &item.id, &ctx.name(item));
    row.upcast()
}
