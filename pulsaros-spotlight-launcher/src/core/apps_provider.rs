//! Applications as a `Provider`: the `RowKind::App` rows of the shared `RootRun`.
use std::rc::Rc;

use super::provider::{Provider, Query};
use super::root_run::RootRun;
use super::root_search::{Row, RowKind};

pub struct AppsProvider {
    run: Rc<RootRun>,
}

impl AppsProvider {
    pub fn new(run: Rc<RootRun>) -> Self {
        AppsProvider { run }
    }
}

impl Provider for AppsProvider {
    fn instant(&self, _query: &Query) -> Vec<Row> {
        self.run.rows_of(RowKind::App)
    }
}

#[cfg(test)]
#[path = "../tests/apps_provider_tests.rs"]
mod tests;
