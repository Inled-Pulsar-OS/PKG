//! A catalog cell as on the App Store's Discover page: the icon at the left, then the
//! name, a one-line subtitle, and the Get capsule underneath, all left-aligned.
//! Cells sit in a grid that gains columns as the window gets wider.

use crate::core::model::Item;
use crate::ui::ctx::Ctx;
use crate::ui::tokens::SPACE_M;
use crate::ui::widget::{icon::item_icon, install_button::InstallButton, item_tile::make_openable};
use gtk4 as gtk;
use gtk4::prelude::*;

/// Narrowest a cell gets; the grid fits as many columns as this allows.
const MIN_WIDTH: i32 = 200;

pub fn app_cell(ctx: &Ctx, item: &Item) -> gtk::Widget {
    let cell = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_M);
    cell.add_css_class("store-row");
    cell.set_size_request(MIN_WIDTH, -1);
    let icon = item_icon(ctx, item, 48);
    icon.set_valign(gtk::Align::Start);
    cell.append(&icon);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 1);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);

    let name = gtk::Label::new(Some(&ctx.name(item)));
    name.add_css_class("name");
    name.set_xalign(0.0);
    name.set_max_width_chars(20);
    name.set_ellipsize(gtk::pango::EllipsizeMode::End);
    text.append(&name);

    let subtitle = gtk::Label::new(Some(&ctx.summary(item)));
    subtitle.add_css_class("store-caption");
    subtitle.set_xalign(0.0);
    subtitle.set_max_width_chars(22);
    subtitle.set_ellipsize(gtk::pango::EllipsizeMode::End);
    text.append(&subtitle);

    let button = InstallButton::new(ctx, item);
    button.widget().set_halign(gtk::Align::Start);
    button.widget().set_margin_top(4);
    text.append(button.widget());
    cell.append(&text);

    make_openable(&cell, ctx, &item.id, &ctx.name(item));
    cell.upcast()
}
