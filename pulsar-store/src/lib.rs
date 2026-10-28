//! Pulsar Store.
//!
//! Layering (see docs/adr/0002-single-crate-layout.md):
//! `core` has no GTK, `ui` depends on `core`, `modules` depend on `core` and `ui`.

pub mod core;
#[cfg(feature = "ui")]
pub mod modules;
#[cfg(feature = "ui")]
pub mod ui;
