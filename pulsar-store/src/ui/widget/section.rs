//! A section title with an optional action on the right.

use crate::ui::tokens::SPACE_M;
use gtk4 as gtk;
use gtk4::prelude::*;

/// A label and the function to run when it is clicked.
pub type SectionAction<'a> = (&'a str, Box<dyn Fn()>);

pub fn section_header(title: &str, action: Option<SectionAction>) -> gtk::Widget {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_M);
    let label = gtk::Label::new(Some(title));
    label.add_css_class("store-section-title");
    label.set_halign(gtk::Align::Start);
    label.set_hexpand(true);
    label.set_xalign(0.0);
    label.set_accessible_role(gtk::AccessibleRole::Heading);
    row.append(&label);

    if let Some((text, on_click)) = action {
        let button = gtk::Button::with_label(text);
        button.add_css_class("flat");
        button.connect_clicked(move |_| on_click());
        row.append(&button);
    }
    row.upcast()
}
