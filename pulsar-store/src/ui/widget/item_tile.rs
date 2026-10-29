//! A catalog item as a row: icon, name, summary, and the install button.

use crate::core::model::Item;
use crate::ui::ctx::Ctx;
use crate::ui::router::Route;
use crate::ui::tokens::{SPACE_M, SPACE_S};
use crate::ui::widget::{icon::item_icon, install_button::InstallButton};
use gtk4 as gtk;
use gtk4::prelude::*;

/// Wires a widget so clicking it or pressing Enter opens the item page.
pub fn make_openable(
    widget: &(impl IsA<gtk::Widget> + IsA<gtk::Accessible>),
    ctx: &Ctx,
    id: &str,
    label: &str,
) {
    widget.set_focusable(true);
    widget.set_cursor_from_name(Some("pointer"));
    widget.update_property(&[gtk::accessible::Property::Label(label)]);

    let click = gtk::GestureClick::new();
    {
        let (ctx, id) = (ctx.clone(), id.to_string());
        click.connect_released(move |_, _, _, _| ctx.navigate(Route::Item(id.clone())));
    }
    widget.add_controller(click);

    let keys = gtk::EventControllerKey::new();
    {
        let (ctx, id) = (ctx.clone(), id.to_string());
        keys.connect_key_pressed(move |_, key, _, _| {
            if matches!(
                key,
                gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter | gtk::gdk::Key::space
            ) {
                ctx.navigate(Route::Item(id.clone()));
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        });
    }
    widget.add_controller(keys);
}

pub fn item_tile(ctx: &Ctx, item: &Item) -> gtk::Widget {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_M);
    row.add_css_class("store-tile");
    row.append(&item_icon(ctx, item, 48));

    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);

    let name = gtk::Label::new(Some(&ctx.name(item)));
    name.set_halign(gtk::Align::Start);
    name.set_xalign(0.0);
    name.set_ellipsize(gtk::pango::EllipsizeMode::End);
    name.add_css_class("heading");
    text.append(&name);

    let summary = gtk::Label::new(Some(&ctx.summary(item)));
    summary.set_halign(gtk::Align::Start);
    summary.set_xalign(0.0);
    summary.set_lines(2);
    summary.set_wrap(true);
    summary.set_ellipsize(gtk::pango::EllipsizeMode::End);
    summary.add_css_class("store-caption");
    text.append(&summary);
    row.append(&text);

    let button = InstallButton::new(ctx, item);
    button.widget().set_margin_start(SPACE_S);
    row.append(button.widget());

    make_openable(&row, ctx, &item.id, &ctx.name(item));
    row.upcast()
}
