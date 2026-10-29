//! The strip of facts under a product header: each cell is a small label, a
//! large value (text or an icon), and a line of detail, with hairlines between cells.

use gtk4 as gtk;
use gtk4::prelude::*;

pub enum InfoValue {
    Text(String),
    /// A symbolic icon name, for values that read better as a picture (the developer).
    Icon(&'static str),
}

pub struct InfoCell {
    /// Shown small and uppercase above the value.
    pub label: String,
    pub value: InfoValue,
    /// A line under the value: its unit, or extra detail.
    pub sub: String,
}

impl InfoCell {
    pub fn text(label: String, value: String, sub: String) -> Self {
        Self {
            label,
            value: InfoValue::Text(value),
            sub,
        }
    }

    pub fn icon(label: String, icon: &'static str, sub: String) -> Self {
        Self {
            label,
            value: InfoValue::Icon(icon),
            sub,
        }
    }
}

/// Cells worth showing: a text value must not be blank, and an icon cell needs its detail line.
pub fn visible_cells(cells: &[InfoCell]) -> Vec<&InfoCell> {
    cells
        .iter()
        .filter(|c| match &c.value {
            InfoValue::Text(t) => !t.trim().is_empty(),
            InfoValue::Icon(_) => !c.sub.trim().is_empty(),
        })
        .collect()
}

pub fn info_strip(cells: &[InfoCell]) -> gtk::Widget {
    let strip = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    strip.add_css_class("store-info-strip");

    for (i, cell) in visible_cells(cells).into_iter().enumerate() {
        if i > 0 {
            let line = gtk::Separator::new(gtk::Orientation::Vertical);
            line.add_css_class("store-info-divider");
            strip.append(&line);
        }
        let col = gtk::Box::new(gtk::Orientation::Vertical, 2);
        col.add_css_class("store-info-cell");
        col.set_hexpand(true);
        col.set_halign(gtk::Align::Fill);

        let label = gtk::Label::new(Some(&cell.label.to_uppercase()));
        label.add_css_class("store-info-label");
        col.append(&label);

        match &cell.value {
            InfoValue::Text(t) => {
                let v = gtk::Label::new(Some(t));
                v.add_css_class("store-info-value");
                v.set_ellipsize(gtk::pango::EllipsizeMode::End);
                v.set_max_width_chars(14);
                col.append(&v);
            }
            InfoValue::Icon(name) => {
                let img = gtk::Image::from_icon_name(name);
                img.set_pixel_size(28);
                img.add_css_class("store-info-icon");
                col.append(&img);
            }
        }

        let sub = gtk::Label::new(Some(&cell.sub));
        sub.add_css_class("store-info-sub");
        sub.set_ellipsize(gtk::pango::EllipsizeMode::End);
        sub.set_max_width_chars(16);
        col.append(&sub);
        strip.append(&col);
    }
    strip.upcast()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_values_are_hidden_but_icons_need_a_detail_line() {
        let cells = [
            InfoCell::text("Version".into(), "1.0".into(), "Installed".into()),
            InfoCell::text("Size".into(), "  ".into(), "GB".into()),
            InfoCell::icon(
                "Developer".into(),
                "avatar-default-symbolic",
                "Apple".into(),
            ),
            InfoCell::icon("Developer".into(), "avatar-default-symbolic", String::new()),
        ];
        let shown: Vec<&str> = visible_cells(&cells)
            .iter()
            .map(|c| c.label.as_str())
            .collect();
        assert_eq!(shown, ["Version", "Developer"]);
    }
}
