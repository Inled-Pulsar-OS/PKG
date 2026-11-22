//! One `RootSearch` run per query, shared by the apps and commands providers (GTK-free).
//!
//! `RootSearch` owns section layout and the joint apps and commands ranking, so providers do
//! not rank on their own: they read the rows this run produced and keep one `RowKind`.
use std::cell::OnceCell;

use super::root_search::{Row, RowKind, Section};

pub struct RootRun {
    compute: Box<dyn Fn() -> Vec<Section>>,
    sections: OnceCell<Vec<Section>>,
}

impl RootRun {
    /// `compute` runs at most once, on first use.
    pub fn new(compute: impl Fn() -> Vec<Section> + 'static) -> Self {
        RootRun { compute: Box::new(compute), sections: OnceCell::new() }
    }

    pub fn sections(&self) -> &[Section] {
        self.sections.get_or_init(|| (self.compute)())
    }

    /// Rows of one kind in section order (a row may repeat across sections).
    pub fn rows_of(&self, kind: RowKind) -> Vec<Row> {
        self.sections().iter().flat_map(|s| s.rows.iter()).filter(|r| r.kind == kind).cloned().collect()
    }
}
