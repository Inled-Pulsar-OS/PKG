//! An item's icon: the cached image, or a placeholder until it downloads.

use crate::core::icons::cached_icon;
use crate::core::model::{Item, SourceKind};
use crate::ui::ctx::{Change, Ctx};
use gtk4 as gtk;
use gtk4::prelude::*;

fn fallback_icon(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::GnomeExtension => "application-x-addon-symbolic",
        SourceKind::SayriSkill | SourceKind::SayriPlugin => "emblem-system-symbolic",
        _ => "application-x-executable-symbolic",
    }
}

/// A square icon of `size` pixels that updates itself when icons finish downloading.
pub fn item_icon(ctx: &Ctx, item: &Item, size: i32) -> gtk::Widget {
    let image = gtk::Image::new();
    image.set_pixel_size(size);
    let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
    frame.set_size_request(size, size);
    frame.set_overflow(gtk::Overflow::Hidden);
    frame.add_css_class(&format!("store-icon-{size}"));
    frame.set_valign(gtk::Align::Center);
    frame.set_halign(gtk::Align::Center);
    frame.append(&image);
    // The icon is decoration; the name next to it carries the meaning.
    frame.set_accessible_role(gtk::AccessibleRole::Presentation);

    let (id, kind) = (item.id.clone(), item.kind);
    let paths = ctx.store().paths.clone();
    let apply = {
        let (image, frame) = (image.clone(), frame.clone());
        move || match cached_icon(&paths, &id) {
            Some(path) => {
                image.set_from_file(Some(path));
                frame.remove_css_class("store-icon-fallback");
                frame.add_css_class("store-icon");
            }
            None => {
                image.set_icon_name(Some(fallback_icon(kind)));
                frame.remove_css_class("store-icon");
                frame.add_css_class("store-icon-fallback");
            }
        }
    };
    apply();
    ctx.subscribe(&frame, move |c| {
        if matches!(c, Change::Icons | Change::Catalog) {
            apply();
        }
    });
    frame.upcast()
}
