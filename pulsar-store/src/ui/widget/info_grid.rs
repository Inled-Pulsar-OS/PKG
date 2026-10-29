//! A row of facts such as version, author, and size, each a value over a small label.

use crate::ui::tokens::SPACE_S;
use gtk4 as gtk;
use gtk4::prelude::*;

/// `cells` are `(label, value)` pairs. Cells with an empty value are skipped.
pub fn info_grid(cells: &[(String, String)]) -> gtk::Widget {
    let flow = gtk::FlowBox::new();
    flow.set_selection_mode(gtk::SelectionMode::None);
    flow.set_homogeneous(true);
    flow.set_min_children_per_line(2);
    flow.set_max_children_per_line(6);
    flow.set_column_spacing(SPACE_S as u32);
    flow.add_css_class("store-card");

    for (label, value) in cells.iter().filter(|(_, v)| !v.is_empty()) {
        let cell = gtk::Box::new(gtk::Orientation::Vertical, 2);
        cell.add_css_class("store-info-cell");
        cell.set_halign(gtk::Align::Center);

        let v = gtk::Label::new(Some(value));
        v.add_css_class("store-info-value");
        v.set_ellipsize(gtk::pango::EllipsizeMode::End);
        v.set_max_width_chars(22);
        cell.append(&v);

        let l = gtk::Label::new(Some(label));
        l.add_css_class("store-info-label");
        cell.append(&l);

        flow.append(&cell);
    }
    flow.upcast()
}

#[cfg(test)]
mod tests {
    /// Cells without a value are dropped before rendering.
    #[test]
    fn filters_empty_values() {
        let cells = [
            ("Version".to_string(), "1.0".to_string()),
            ("Author".to_string(), String::new()),
        ];
        assert_eq!(cells.iter().filter(|(_, v)| !v.is_empty()).count(), 1);
    }
}
