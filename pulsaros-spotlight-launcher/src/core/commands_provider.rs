//! Commands as a `Provider`: the `RowKind::Command` rows of the shared `RootRun`.
use std::rc::Rc;

use super::provider::{Provider, Query};
use super::root_run::RootRun;
use super::root_search::{Row, RowKind};

pub struct CommandsProvider {
    run: Rc<RootRun>,
}

impl CommandsProvider {
    pub fn new(run: Rc<RootRun>) -> Self {
        CommandsProvider { run }
    }
}

impl Provider for CommandsProvider {
    fn id(&self) -> &str {
        "commands"
    }

    fn instant(&self, _query: &Query) -> Vec<Row> {
        self.run.rows_of(RowKind::Command)
    }
}

#[cfg(test)]
#[path = "../tests/commands_provider_tests.rs"]
mod tests;
