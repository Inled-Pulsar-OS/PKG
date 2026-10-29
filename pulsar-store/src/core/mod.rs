//! Store logic with no GTK dependency: model, ports, catalog, search, ledger,
//! install pipeline, privilege helper, and package sources.

pub mod archive;
pub mod cache;
pub mod catalog;
pub mod config;
pub mod contract;
pub mod deeplink;
pub mod editions;
pub mod error;
pub mod fakes;
pub mod host;
pub mod http;
pub mod i18n;
pub mod icons;
pub mod install;
pub mod ledger;
pub mod model;
pub mod ports;
pub mod prefs;
pub mod privilege;
pub mod process;
pub mod remote;
pub mod search;
pub mod sources;
pub mod store;
pub mod version;
