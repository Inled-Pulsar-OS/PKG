//! Store logic with no GTK dependency: model, ports, catalog, search, ledger,
//! install pipeline, privilege helper, and package sources.

pub mod catalog;
pub mod config;
pub mod error;
pub mod fakes;
pub mod http;
pub mod i18n;
pub mod icons;
pub mod model;
pub mod ports;
pub mod search;
pub mod sources;
