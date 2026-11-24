//! Builders for the fixed widgets of the main window: search header, bar buttons,
//! category tabs and the toast pill. Each returns the widgets; wiring stays in `new`.
use super::*;

/// The header row: mode glyph, search entry, argument box and the `...` disc.
pub(super) struct SearchHeader {
    pub(super) container: gtk4::Box,
    pub(super) mode_icon: gtk4::Image,
    pub(super) entry: gtk4::Entry,
    pub(super) more_button: gtk4::Button,
}

impl SearchHeader {
    pub(super) fn build(x: &Extras) -> SearchHeader {
        let search_container = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        search_container.add_css_class("search-header");

        let mode_icon = gtk4::Image::new();
        mode_icon.set_pixel_size(28);
        mode_icon.add_css_class("search-icon");
        set_mode_icon(&mode_icon, "spotlight");

        let search_entry = gtk4::Entry::builder()
            .placeholder_text(Category::All.label())
            .hexpand(true)
            .has_frame(false)
            .valign(gtk4::Align::Center)
            .build();
        search_entry.add_css_class("search-input");

        let more_button = gtk4::Button::new();
        let more_img = gtk4::Image::new();
        more_img.set_pixel_size(22);
        set_mode_icon(&more_img, "more");
        more_button.set_child(Some(&more_img));
        more_button.add_css_class("disc-button");
        more_button.set_valign(gtk4::Align::Center);
        more_button.set_focusable(false);
        more_button.set_tooltip_text(Some("Actions (Ctrl+K)"));

        search_container.append(&mode_icon);
        search_container.append(&search_entry);
        search_container.append(&x.arg_box);
        search_container.append(&more_button);
        SearchHeader { container: search_container, mode_icon, entry: search_entry, more_button }
    }
}

/// The four round mode buttons and the revealer that slides them in beside the pill.
pub(super) fn build_mode_buttons() -> (gtk4::Box, gtk4::Revealer) {
    let mode_buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, MODE_GAP as i32);
    mode_buttons.add_css_class("mode-buttons");
    mode_buttons.set_margin_start(MODE_GAP as i32);
    let buttons_revealer = gtk4::Revealer::builder()
        .transition_type(gtk4::RevealerTransitionType::SlideLeft)
        .transition_duration(200)
        .reveal_child(false)
        .child(&mode_buttons)
        .build();
    for (n, tip) in BAR_BUTTONS {
        let img = gtk4::Image::new();
        img.set_pixel_size(26);
        let glyph = shortcuts::mode_id(n).and_then(Category::from_id).map_or("spotlight", Category::glyph);
        set_mode_icon(&img, glyph);
        let b = gtk4::Button::new();
        b.set_child(Some(&img));
        b.add_css_class("mode-button");
        b.set_focusable(false);
        b.set_valign(gtk4::Align::Center);
        b.set_tooltip_text(Some(tip));
        mode_buttons.append(&b);
    }
    (mode_buttons, buttons_revealer)
}

/// The category tabs inside a horizontal scroller, and each tab's toggle button.
pub(super) fn build_category_bar() -> (gtk4::ScrolledWindow, HashMap<Category, gtk4::ToggleButton>) {
    let category_bar = gtk4::Box::new(gtk4::Orientation::Horizontal, 5);
    category_bar.add_css_class("category-bar");
    let mut cat_buttons = HashMap::new();

    for cat in Category::ALL {
        let btn = gtk4::ToggleButton::builder()
            .label(cat.label())
            .active(cat == Category::All)
            .focusable(false)
            .build();
        btn.add_css_class("category-btn");
        category_bar.append(&btn);
        cat_buttons.insert(cat, btn);
    }

    let category_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::External)
        .vscrollbar_policy(gtk4::PolicyType::Never)
        .propagate_natural_width(false)
        .propagate_natural_height(false)
        .hexpand(true)
        .build();
    category_scroll.add_css_class("category-scroll");
    category_scroll.set_child(Some(&category_bar));
    (category_scroll, cat_buttons)
}

/// The floating toast pill: status label plus the Undo button.
pub(super) fn build_toast() -> (gtk4::Box, gtk4::Label, gtk4::Button) {
    let toast_pill = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    toast_pill.add_css_class("toast-pill");
    toast_pill.set_halign(gtk4::Align::Center);
    toast_pill.set_valign(gtk4::Align::End);
    toast_pill.set_margin_bottom(18);
    toast_pill.set_visible(false);
    let footer_status = gtk4::Label::builder().xalign(0.0).build();
    footer_status.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    footer_status.set_max_width_chars(48);
    let toast_undo = gtk4::Button::with_label("Undo");
    toast_undo.add_css_class("toast-undo");
    toast_undo.set_visible(false);
    toast_undo.set_focusable(false);
    toast_pill.append(&footer_status);
    toast_pill.append(&toast_undo);
    (toast_pill, footer_status, toast_undo)
}
