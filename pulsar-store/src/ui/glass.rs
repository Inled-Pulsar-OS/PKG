//! A Liquid Glass-style surface for the navigation layer (toolbars, sidebars).
//!
//! It blurs the content layer behind it, adds a tint and a lit edge (from
//! `glass.css`), and rounds its corners. GTK cannot do true lensing, so this is
//! an approximation. When blur is unavailable or transparency is reduced it
//! falls back to a flat surface (`flat` CSS class).
//!
//! Use it only for navigation controls floating over content, never for
//! content itself, and never on top of another glass surface.

use adw::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, graphene, gsk};
use gtk4 as gtk;
use std::cell::{Cell, RefCell};

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct GlassSurface {
        pub child: RefCell<Option<gtk::Widget>>,
        pub backdrop: RefCell<Option<(gtk::Widget, gtk::WidgetPaintable)>>,
        pub flat: Cell<bool>,
        pub radius: Cell<f32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GlassSurface {
        const NAME: &'static str = "PulsarGlassSurface";
        type Type = super::GlassSurface;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("glass");
            klass.set_layout_manager_type::<gtk::BinLayout>();
        }
    }

    impl ObjectImpl for GlassSurface {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.add_css_class("store-glass");
            self.radius.set(crate::ui::tokens::RADIUS_CARD);
            obj.set_overflow(gtk::Overflow::Hidden);

            let manager = adw::StyleManager::default();
            obj.set_flat(needs_flat(&manager));
            let weak = obj.downgrade();
            manager.connect_high_contrast_notify(move |m| {
                if let Some(o) = weak.upgrade() {
                    o.set_flat(needs_flat(m));
                }
            });
        }

        fn dispose(&self) {
            if let Some(child) = self.child.take() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for GlassSurface {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();
            if !self.flat.get() {
                if let Some((widget, paintable)) = self.backdrop.borrow().as_ref() {
                    draw_blurred_backdrop(snapshot, &obj, widget, paintable, self.radius.get());
                }
            }
            self.parent_snapshot(snapshot);
        }
    }

    /// Reduced transparency is approximated by the high-contrast setting.
    fn needs_flat(manager: &adw::StyleManager) -> bool {
        std::env::var_os("PULSAR_STORE_FLAT_GLASS").is_some_and(|v| v != "0")
            || manager.is_high_contrast()
    }

    fn draw_blurred_backdrop(
        snapshot: &gtk::Snapshot,
        glass: &super::GlassSurface,
        backdrop: &gtk::Widget,
        paintable: &gtk::WidgetPaintable,
        radius: f32,
    ) {
        let (w, h) = (glass.width() as f32, glass.height() as f32);
        let Some(origin) = glass.compute_point(backdrop, &graphene::Point::zero()) else {
            return;
        };
        let corner = graphene::Size::new(radius, radius);
        let bounds = gsk::RoundedRect::new(
            graphene::Rect::new(0.0, 0.0, w, h),
            corner,
            corner,
            corner,
            corner,
        );
        snapshot.push_rounded_clip(&bounds);
        snapshot.push_blur(crate::ui::tokens::GLASS_BLUR);
        snapshot.translate(&graphene::Point::new(-origin.x(), -origin.y()));
        paintable.snapshot(
            snapshot.upcast_ref::<gtk::gdk::Snapshot>(),
            backdrop.width() as f64,
            backdrop.height() as f64,
        );
        snapshot.pop();
        snapshot.pop();
    }
}

glib::wrapper! {
    pub struct GlassSurface(ObjectSubclass<imp::GlassSurface>)
        @extends gtk::Widget,
        @implements gtk::Buildable;
}

impl Default for GlassSurface {
    fn default() -> Self {
        Self::new()
    }
}

impl GlassSurface {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_child(&self, child: Option<&impl IsA<gtk::Widget>>) {
        let imp = self.imp();
        if let Some(old) = imp.child.take() {
            old.unparent();
        }
        if let Some(c) = child {
            c.set_parent(self);
            imp.child.replace(Some(c.clone().upcast()));
        }
    }

    /// The content widget to blur. It must be a sibling under the glass (for
    /// example, the other child of a `gtk::Overlay`), never an ancestor or descendant.
    pub fn set_backdrop(&self, backdrop: Option<&impl IsA<gtk::Widget>>) {
        let imp = self.imp();
        imp.backdrop.replace(backdrop.map(|b| {
            let widget: gtk::Widget = b.clone().upcast();
            let paintable = gtk::WidgetPaintable::new(Some(&widget));
            let weak = self.downgrade();
            paintable.connect_invalidate_contents(move |_| {
                if let Some(g) = weak.upgrade() {
                    g.queue_draw();
                }
            });
            (widget, paintable)
        }));
        self.queue_draw();
    }

    pub fn set_corner_radius(&self, radius: f32) {
        self.imp().radius.set(radius);
        self.queue_draw();
    }

    /// Switches between the blurred glass look and the flat fallback.
    pub fn set_flat(&self, flat: bool) {
        self.imp().flat.set(flat);
        if flat {
            self.add_css_class("flat");
        } else {
            self.remove_css_class("flat");
        }
        self.queue_draw();
    }

    pub fn is_flat(&self) -> bool {
        self.imp().flat.get()
    }
}
