//! Shared design system: tokens, glass surface, motion, router, widgets.
//! Imports `core` only.

pub mod chrome;
pub mod ctx;
pub mod glass;
pub mod motion;
pub mod page;
pub mod router;
pub mod task;
pub mod theme;
pub mod tokens;
pub mod widget;

pub use ctx::{Change, Ctx, Op};
pub use router::{Ecosystem, Route, Router};
