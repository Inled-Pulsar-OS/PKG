//! Row widgets for the results list: the calculator card and the standard result row.
use super::*;
use crate::core::root_search::Row;

impl ResultView {
    pub(super) fn build_calc_card(&self, c: &crate::calculator::CalcResult, n: usize) -> gtk4::Box {
        let b = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        b.add_css_class("result-row");
        b.add_css_class("calc-card");
        b.set_homogeneous(false);
        let pane = |text: &str, caption: &str| {
            let v = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
            v.set_hexpand(true);
            v.set_halign(gtk4::Align::Center);
            v.set_valign(gtk4::Align::Center);
            let big = gtk4::Label::builder().label(text).ellipsize(gtk4::pango::EllipsizeMode::End).build();
            big.add_css_class("calc-big");
            v.append(&big);
            if !caption.is_empty() {
                let cap = gtk4::Label::new(Some(caption));
                cap.add_css_class("calc-cap");
                v.append(&cap);
            }
            v
        };
        b.append(&pane(&c.question, &c.caption_question));
        let arrow = gtk4::Label::new(Some("\u{2192}"));
        arrow.add_css_class("calc-cap");
        b.append(&arrow);
        b.append(&pane(&c.answer, &c.caption_answer));
        if n < 9 {
            let acc = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
            acc.set_valign(gtk4::Align::Center);
            for k in crate::core::shortcuts::label(crate::core::shortcuts::Act::Quick(n)) {
                let l = gtk4::Label::new(Some(&k));
                l.add_css_class("keycap");
                l.add_css_class("badge");
                acc.append(&l);
            }
            b.append(&acc);
        }
        b
    }

    pub(super) fn build_row(&self, r: &RowData, n: usize) -> gtk4::Box {
        if let Some(c) = &r.calc {
            return self.build_calc_card(c, n);
        }
        let row = &r.row;
        let b = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        b.add_css_class("result-row");

        b.append(&self.row_icon_tile(r));
        b.append(&Self::row_text(row));
        b.append(&Self::row_accessories(r, n));
        b
    }

    /// The 44px icon tile: app icon, thumbnail, file icon or a tinted symbolic glyph.
    fn row_icon_tile(&self, r: &RowData) -> gtk4::Box {
        let row = &r.row;
        let tile = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        tile.add_css_class("icon-tile");
        tile.set_valign(gtk4::Align::Center);
        tile.set_halign(gtk4::Align::Start);
        tile.set_hexpand(false);
        tile.set_size_request(44, 44);
        let theme = gtk4::IconTheme::for_display(&gdk::Display::default().expect("display"));
        let themed = |name: &str, fallback: &str| {
            gtk4::Image::from_icon_name(if theme.has_icon(name) { name } else { fallback })
        };
        match row.kind {
            RowKind::App => {
                let desktop = row.id.strip_prefix("app:").unwrap_or(&row.id);
                let img = app_icon(&row.icon_name, desktop);
                img.set_pixel_size(40);
                img.add_css_class("result-icon");
                img.set_halign(gtk4::Align::Center);
                img.set_hexpand(true);
                tile.append(&img);
            }
            RowKind::File if r.payload.as_ref().map_or(false, |p| has_thumbnail_support(&p.url)) => {
                let url = &r.payload.as_ref().unwrap().url;
                tile.append(&self.thumbnail_icon(url, 44));
            }
            RowKind::File => {
                let img = match &r.payload {
                    Some(p) => result_icon(p),
                    None => themed(&row.icon_name, "text-x-generic"),
                };
                img.set_pixel_size(36);
                img.set_halign(gtk4::Align::Center);
                img.set_hexpand(true);
                tile.append(&img);
            }
            kind => {
                tile.add_css_class("tinted");
                let (name, fb) = match kind {
                    RowKind::Calc => ("accessories-calculator-symbolic", "accessories-calculator-symbolic"),
                    RowKind::Clipboard => ("edit-paste-symbolic", "edit-copy-symbolic"),
                    RowKind::Fallback if row.id.starts_with("history:") => ("document-open-recent-symbolic", "view-refresh-symbolic"),
                    RowKind::Fallback => ("web-browser-symbolic", "web-browser-symbolic"),
                    _ => (row.icon_name.as_str(), "system-run-symbolic"),
                };
                let img = themed(name, fb);
                img.set_pixel_size(22);
                img.set_halign(gtk4::Align::Center);
                img.set_hexpand(true);
                tile.append(&img);
            }
        }
        tile
    }

    /// Title and optional subtitle.
    fn row_text(row: &Row) -> gtk4::Box {
        let text = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        text.set_hexpand(true);
        text.set_valign(gtk4::Align::Center);
        let title = gtk4::Label::builder()
            .label(&row.title)
            .xalign(0.0)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();
        title.add_css_class("result-title");
        text.append(&title);
        let sub = if row.kind == RowKind::Fallback { String::new() } else { row.subtitle.clone() };
        if !sub.is_empty() && sub != row.title {
            let l = gtk4::Label::builder()
                .label(&sub)
                .xalign(0.0)
                .ellipsize(if row.kind == RowKind::File {
                    gtk4::pango::EllipsizeMode::Middle
                } else {
                    gtk4::pango::EllipsizeMode::End
                })
                .build();
            l.add_css_class("result-subtitle");
            text.append(&l);
        }
        text
    }

    /// Star, alias chip, check, kind label, primary-action hint and quick-key badges.
    fn row_accessories(r: &RowData, n: usize) -> gtk4::Box {
        let row = &r.row;
        let acc = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        acc.set_valign(gtk4::Align::Center);
        if row.favorite {
            let l = gtk4::Label::new(Some("\u{2605}"));
            l.add_css_class("result-star");
            acc.append(&l);
        }
        if let Some(a) = &row.alias {
            let l = gtk4::Label::new(Some(a));
            l.add_css_class("alias-chip");
            acc.append(&l);
        }
        if row.checked == Some(true) {
            let l = gtk4::Label::new(Some("\u{2713}"));
            l.add_css_class("result-check");
            acc.append(&l);
        }
        let kl = kind_label(r);
        if !kl.is_empty() && !matches!(row.kind, RowKind::App | RowKind::Command) {
            let l = gtk4::Label::new(Some(kl));
            l.add_css_class("result-kind");
            acc.append(&l);
        }
        // Primary-action hint, only visible on the selected row (see CSS).
        let hint = gtk4::Label::new(Some(
            crate::ui::window::primary_title(r).split_whitespace().next().unwrap_or("Open"),
        ));
        hint.add_css_class("row-hint");
        acc.append(&hint);
        if n < 9 {
            let keys = crate::core::shortcuts::label(crate::core::shortcuts::Act::Quick(n));
            for k in keys {
                let l = gtk4::Label::new(Some(&k));
                l.add_css_class("keycap");
                l.add_css_class("badge");
                acc.append(&l);
            }
        }
        acc
    }
}
