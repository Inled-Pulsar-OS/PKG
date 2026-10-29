//! A circular progress indicator: a ring that fills clockwise, or spins when
//! the amount is unknown. A small square in the middle marks "tap to stop".

use crate::ui::motion;
use gtk4 as gtk;
use gtk4::prelude::*;
use std::cell::Cell;
use std::f64::consts::PI;
use std::rc::Rc;

#[derive(Clone)]
pub struct ProgressRing {
    area: gtk::DrawingArea,
    /// Fraction being drawn (animated toward the target).
    shown: Rc<Cell<f64>>,
    target: Rc<Cell<Option<f32>>>,
    spin: Rc<Cell<f64>>,
}

impl ProgressRing {
    pub fn new(size: i32) -> Self {
        let area = gtk::DrawingArea::new();
        area.set_content_width(size);
        area.set_content_height(size);
        area.add_css_class("store-ring");
        let ring = Self {
            area,
            shown: Rc::new(Cell::new(0.0)),
            target: Rc::new(Cell::new(None)),
            spin: Rc::new(Cell::new(0.0)),
        };

        let (shown, target, spin) = (ring.shown.clone(), ring.target.clone(), ring.spin.clone());
        ring.area.set_draw_func(move |area, cr, w, h| {
            let color = area.color();
            let (w, h) = (f64::from(w), f64::from(h));
            let (cx, cy) = (w / 2.0, h / 2.0);
            let radius = (w.min(h) / 2.0) - 2.0;
            let rgba = |a: f64| {
                cr.set_source_rgba(
                    f64::from(color.red()),
                    f64::from(color.green()),
                    f64::from(color.blue()),
                    a,
                )
            };

            cr.set_line_width(3.0);
            rgba(0.25);
            cr.arc(cx, cy, radius, 0.0, 2.0 * PI);
            let _ = cr.stroke();

            rgba(1.0);
            cr.set_line_cap(gtk::cairo::LineCap::Round);
            let start = -PI / 2.0;
            if target.get().is_some() {
                cr.arc(cx, cy, radius, start, start + shown.get() * 2.0 * PI);
            } else {
                let from = start + spin.get();
                cr.arc(cx, cy, radius, from, from + PI / 2.0);
            }
            let _ = cr.stroke();

            let side = radius * 0.55;
            cr.rectangle(cx - side / 2.0, cy - side / 2.0, side, side);
            let _ = cr.fill();
        });

        let (target, spin) = (ring.target.clone(), ring.spin.clone());
        ring.area.add_tick_callback(move |area, clock| {
            if target.get().is_none() && motion::animations_enabled() {
                let t = clock.frame_time() as f64 / 1_000_000.0;
                spin.set((t * 2.0 * PI * 0.9) % (2.0 * PI));
                area.queue_draw();
            }
            gtk::glib::ControlFlow::Continue
        });
        ring
    }

    pub fn widget(&self) -> &gtk::DrawingArea {
        &self.area
    }

    /// `None` shows a spinning ring.
    pub fn set_fraction(&self, fraction: Option<f32>) {
        self.target.set(fraction);
        match fraction {
            Some(f) => {
                let (shown, area) = (self.shown.clone(), self.area.clone());
                motion::animate(
                    &self.area,
                    self.shown.get(),
                    f64::from(f.clamp(0.0, 1.0)),
                    motion::FAST_MS,
                    move |v| {
                        shown.set(v);
                        area.queue_draw();
                    },
                );
            }
            None => self.area.queue_draw(),
        }
    }
}
