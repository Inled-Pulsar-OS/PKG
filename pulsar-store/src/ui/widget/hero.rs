//! The wide banner at the top of Discover: text on a dark fade at the left,
//! artwork filling the rest. Uses the item's `hero_image` when it has one,
//! otherwise an accent gradient with the app icon.

use crate::core::i18n;
use crate::core::model::Item;
use crate::tr;
use crate::ui::ctx::Ctx;
use crate::ui::tokens::{SPACE_L, SPACE_M};
use crate::ui::widget::{icon::item_icon, install_button::InstallButton, item_tile::make_openable};
use gtk4 as gtk;
use gtk4::prelude::*;

const HEIGHT: i32 = 300;
const TEXT_WIDTH: i32 = 340;

/// The banner for `item`. `eyebrow` is the small label above the title.
pub fn hero(ctx: &Ctx, item: &Item, eyebrow: &str) -> gtk::Widget {
    let overlay = gtk::Overlay::new();
    overlay.add_css_class("store-hero");
    overlay.set_overflow(gtk::Overflow::Hidden);
    overlay.set_size_request(-1, HEIGHT);

    // Artwork.
    let art = gtk::Overlay::new();
    let backdrop = gtk::Box::new(gtk::Orientation::Vertical, 0);
    backdrop.add_css_class("store-hero-art");
    art.set_child(Some(&backdrop));
    let icon = item_icon(ctx, item, 128);
    icon.set_halign(gtk::Align::End);
    icon.set_valign(gtk::Align::Center);
    icon.set_margin_end(64);
    art.add_overlay(&icon);
    if let Some(url) = item
        .hero_image()
        .filter(|u| crate::core::remote::is_web_url(u))
    {
        let picture = gtk::Picture::new();
        picture.set_content_fit(gtk::ContentFit::Cover);
        picture.set_visible(false);
        art.add_overlay(&picture);
        let (weak, weak_icon) = (picture.downgrade(), icon.downgrade());
        ctx.load_image(url.to_string(), move |path| {
            if let (Some(p), Some(pic)) = (path, weak.upgrade()) {
                pic.set_filename(Some(p));
                pic.set_visible(true);
                if let Some(i) = weak_icon.upgrade() {
                    i.set_visible(false);
                }
            }
        });
    }
    overlay.set_child(Some(&art));

    // Text on the left.
    let text = gtk::Box::new(gtk::Orientation::Vertical, SPACE_M);
    text.add_css_class("store-hero-text");
    text.set_halign(gtk::Align::Start);
    text.set_valign(gtk::Align::Fill);
    text.set_size_request(TEXT_WIDTH, -1);

    let eyebrow = gtk::Label::new(Some(&eyebrow.to_uppercase()));
    eyebrow.add_css_class("store-eyebrow");
    eyebrow.set_xalign(0.0);
    text.append(&eyebrow);

    let title = gtk::Label::new(Some(&ctx.name(item)));
    title.add_css_class("store-hero-title");
    title.set_xalign(0.0);
    title.set_wrap(true);
    title.set_max_width_chars(24);
    title.set_accessible_role(gtk::AccessibleRole::Heading);
    text.append(&title);

    let loc = ctx.locales();
    let story = item
        .story()
        .map(|s| s.resolve(&loc).to_string())
        .filter(|s| !s.is_empty());
    let summary = gtk::Label::new(Some(&story.unwrap_or_else(|| ctx.summary(item))));
    summary.add_css_class("store-caption");
    summary.set_xalign(0.0);
    summary.set_wrap(true);
    summary.set_lines(3);
    summary.set_max_width_chars(38);
    summary.set_ellipsize(gtk::pango::EllipsizeMode::End);
    text.append(&summary);

    let spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    text.append(&spacer);

    let foot = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_L);
    let caption = gtk::Label::new(Some(&i18n::format(
        &tr!("Version {version}"),
        &[("version", item.version.as_deref().unwrap_or("1.0"))],
    )));
    caption.add_css_class("store-caption");
    caption.set_hexpand(true);
    caption.set_xalign(0.0);
    caption.set_valign(gtk::Align::Center);
    foot.append(&caption);
    foot.append(InstallButton::new(ctx, item).widget());
    text.append(&foot);
    overlay.add_overlay(&text);

    make_openable(&overlay, ctx, &item.id, &ctx.name(item));
    overlay.upcast()
}
