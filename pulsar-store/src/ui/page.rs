//! The standard page shell: a scrolling, width-limited column with room at the
//! top for the floating toolbar, rebuilt when the catalog changes.

use crate::ui::ctx::{Change, Ctx};
use crate::ui::motion;
use crate::ui::tokens::{SIDEBAR_CLEARANCE, SPACE_L, SPACE_XL, TOOLBAR_HEIGHT};
use adw::prelude::*;
use gtk4 as gtk;

/// Height reserved at the top for the floating glass toolbar.
pub const TOOLBAR_CLEARANCE: i32 = TOOLBAR_HEIGHT + 20;

/// Builds a page. `fill` adds content to the column; it runs again whenever the catalog changes.
pub fn page(ctx: &Ctx, max_width: i32, fill: impl Fn(&Ctx, &gtk::Box) + 'static) -> gtk::Widget {
    build_page(ctx, max_width, false, fill)
}

/// Like `page`, but also rebuilds when install states change (for lists that depend on them).
pub fn live_page(
    ctx: &Ctx,
    max_width: i32,
    fill: impl Fn(&Ctx, &gtk::Box) + 'static,
) -> gtk::Widget {
    build_page(ctx, max_width, true, fill)
}

fn build_page(
    ctx: &Ctx,
    max_width: i32,
    on_state: bool,
    fill: impl Fn(&Ctx, &gtk::Box) + 'static,
) -> gtk::Widget {
    let column = gtk::Box::new(gtk::Orientation::Vertical, SPACE_XL);
    column.set_margin_top(TOOLBAR_CLEARANCE + SPACE_L);
    column.set_margin_bottom(SPACE_XL);
    column.set_margin_start(SIDEBAR_CLEARANCE);
    column.set_margin_end(SPACE_XL);
    column.set_valign(gtk::Align::Start);

    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(max_width);
    clamp.set_tightening_threshold(max_width * 3 / 4);
    clamp.set_child(Some(&column));

    let scroll = gtk::ScrolledWindow::new();
    scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroll.set_child(Some(&clamp));

    fill(ctx, &column);
    // New pages ease in; the navigation view adds the slide.
    motion::fade_in(&scroll, motion::BASE_MS);
    let (rebuild_ctx, rebuild_column, rebuild_scroll) =
        (ctx.clone(), column.clone(), scroll.clone());
    let fill = std::rc::Rc::new(fill);
    ctx.subscribe(&scroll, move |c| {
        let rebuild = match c {
            Change::Catalog => true,
            Change::States | Change::State(_) => on_state,
            _ => false,
        };
        if rebuild {
            // Removing the focused widget makes GTK scroll to wherever focus lands next,
            // so remember the position and put it back.
            let adjustment = rebuild_scroll.vadjustment();
            let at = adjustment.value();
            while let Some(child) = rebuild_column.first_child() {
                rebuild_column.remove(&child);
            }
            fill(&rebuild_ctx, &rebuild_column);
            gtk::glib::idle_add_local_once(move || adjustment.set_value(at));
        }
    });
    scroll.upcast()
}

/// A page title.
pub fn title(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("store-title");
    label.set_halign(gtk::Align::Start);
    label.set_xalign(0.0);
    label.set_accessible_role(gtk::AccessibleRole::Heading);
    label
}

/// An empty-state message.
pub fn empty(icon: &str, heading: &str, detail: &str) -> gtk::Widget {
    let status = adw::StatusPage::new();
    status.set_icon_name(Some(icon));
    status.set_title(heading);
    status.set_description(Some(detail));
    status.set_vexpand(true);
    status.upcast()
}
