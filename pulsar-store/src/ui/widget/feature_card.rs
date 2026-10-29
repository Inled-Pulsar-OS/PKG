//! An editorial card: a small label, a title, a description, and round artwork on the right.

use crate::core::model::Item;
use crate::ui::ctx::Ctx;
use crate::ui::tokens::SPACE_L;
use crate::ui::widget::{icon::item_icon, item_tile::make_openable};
use gtk4 as gtk;
use gtk4::prelude::*;

const ART: i32 = 72;

pub fn feature_card(ctx: &Ctx, item: &Item, eyebrow: &str) -> gtk::Widget {
    let card = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_L);
    card.add_css_class("store-card");
    card.add_css_class("store-feature");

    let text = gtk::Box::new(gtk::Orientation::Vertical, 4);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);
    let label = gtk::Label::new(Some(&eyebrow.to_uppercase()));
    label.add_css_class("store-eyebrow");
    label.set_xalign(0.0);
    text.append(&label);
    let title = gtk::Label::new(Some(&ctx.name(item)));
    title.add_css_class("heading");
    title.set_xalign(0.0);
    title.set_wrap(true);
    title.set_lines(2);
    title.set_max_width_chars(20);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    text.append(&title);
    let summary = gtk::Label::new(Some(&ctx.summary(item)));
    summary.add_css_class("store-caption");
    summary.set_xalign(0.0);
    summary.set_wrap(true);
    summary.set_lines(3);
    summary.set_max_width_chars(24);
    summary.set_ellipsize(gtk::pango::EllipsizeMode::End);
    text.append(&summary);
    card.append(&text);

    let art = gtk::Box::new(gtk::Orientation::Vertical, 0);
    art.add_css_class("store-circle-art");
    art.set_size_request(ART, ART);
    art.set_valign(gtk::Align::Center);
    art.set_overflow(gtk::Overflow::Hidden);
    let icon = item_icon(ctx, item, 48);
    icon.set_halign(gtk::Align::Center);
    icon.set_valign(gtk::Align::Center);
    icon.set_vexpand(true);
    art.append(&icon);
    card.append(&art);

    make_openable(&card, ctx, &item.id, &ctx.name(item));
    card.upcast()
}

/// The small label for an item's kind, shared by editorial cards and banners.
pub fn kind_eyebrow(kind: crate::core::model::SourceKind) -> String {
    use crate::core::model::SourceKind;
    use crate::tr;
    match kind {
        SourceKind::Flatpak => tr!("Application"),
        SourceKind::System => tr!("System package"),
        SourceKind::GnomeExtension => tr!("Shell extension"),
        SourceKind::SayriSkill => tr!("Sayri skill"),
        SourceKind::SayriPlugin => tr!("Sayri plugin"),
        SourceKind::Unknown => tr!("Other"),
    }
}

/// The label above a card or banner: the catalog's editorial `tag`, else the item's kind.
pub fn eyebrow_for(ctx: &Ctx, item: &Item) -> String {
    item.tag()
        .map(|t| t.resolve(&ctx.locales()).to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| kind_eyebrow(item.kind))
}

/// Editorial cards side by side, wrapping onto more lines in a narrow window.
pub fn feature_row(ctx: &Ctx, items: &[&Item]) -> gtk::Widget {
    let flow = gtk::FlowBox::new();
    flow.set_selection_mode(gtk::SelectionMode::None);
    flow.set_homogeneous(true);
    flow.set_min_children_per_line(1);
    flow.set_max_children_per_line(3);
    flow.set_column_spacing(SPACE_L as u32);
    flow.set_row_spacing(SPACE_L as u32);
    for item in items {
        flow.append(&feature_card(ctx, item, &eyebrow_for(ctx, item)));
    }
    flow.upcast()
}
