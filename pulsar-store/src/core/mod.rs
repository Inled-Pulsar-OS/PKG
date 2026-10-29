//! Store logic with no GTK dependency: model, ports, catalog, search, ledger,
//! install pipeline, privilege helper, and package sources.

pub mod archive;
pub mod catalog;
pub mod config;
pub mod contract;
pub mod error;
pub mod fakes;
pub mod host;
pub mod http;
pub mod i18n;
pub mod icons;
pub mod model;
pub mod ports;
pub mod privilege;
pub mod process;
pub mod search;
pub mod sources;
