//! Which list the Panel shows, and how the Panel was opened. No GTK here.
//!
//! One `Browse` value replaces the old flags `history_mode`, `calc_hist_mode` and
//! `current_dir`, so two lists can never be active at once. `Nav::apply` is the only
//! way to change it. The window feeds it events and reads the result.

use std::path::Path;

/// The list under the search entry. Pushed views (windows, emoji ...) are separate.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Browse {
    /// Search results for the current mode.
    #[default]
    Root,
    /// Plain folder listing.
    Dir(String),
    /// Recent searches (Up on an empty query).
    RecentSearches,
    /// Calculator history.
    CalcHistory,
}

/// How the Panel was opened from the bar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Opened {
    #[default]
    None,
    /// Down arrow: home view.
    Home,
    /// Up arrow: history.
    History,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavEvent {
    OpenDir(String),
    /// Backspace on an empty query: go up one folder. Does nothing at `/`.
    ParentDir,
    ShowRecent,
    ShowCalcHistory,
    /// The query text changed: recent searches no longer apply.
    QueryChanged,
    /// Esc: leave the current list.
    Back,
    /// Mode switch or reset: back to search results.
    ToRoot,
    /// The user typed (`true`) or something else set the query (`false`).
    Typed(bool),
    PanelOpened(Opened),
    /// The window shrank to the bar: forget how it was opened.
    Collapsed,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Nav {
    pub browse: Browse,
    pub opened: Opened,
    /// The query text came from the user's typing (not from a mode or a view).
    pub typed: bool,
}

impl Nav {
    pub fn dir(&self) -> Option<&str> {
        match &self.browse {
            Browse::Dir(d) => Some(d),
            _ => None,
        }
    }

    pub fn is_root(&self) -> bool {
        self.browse == Browse::Root
    }

    pub fn apply(&mut self, event: NavEvent) {
        match event {
            NavEvent::OpenDir(d) => self.browse = Browse::Dir(d),
            NavEvent::ParentDir => {
                if let Some(parent) = self.dir().and_then(parent_of) {
                    self.browse = Browse::Dir(parent);
                }
            }
            NavEvent::ShowRecent => self.browse = Browse::RecentSearches,
            NavEvent::ShowCalcHistory => self.browse = Browse::CalcHistory,
            NavEvent::QueryChanged => {
                if self.browse == Browse::RecentSearches {
                    self.browse = Browse::Root;
                }
            }
            NavEvent::Back | NavEvent::ToRoot => self.browse = Browse::Root,
            NavEvent::Typed(t) => self.typed = t,
            NavEvent::PanelOpened(o) => self.opened = o,
            NavEvent::Collapsed => {
                self.opened = Opened::None;
                self.typed = false;
            }
        }
    }
}

/// Parent folder of `dir`, or `None` at the top.
fn parent_of(dir: &str) -> Option<String> {
    Path::new(dir).parent().map(|p| p.display().to_string()).filter(|p| p != dir)
}

#[cfg(test)]
#[path = "../tests/nav_tests.rs"]
mod tests;
