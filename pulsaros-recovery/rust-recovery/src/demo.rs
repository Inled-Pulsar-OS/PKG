//! Detección de modo demo / dry-run.
use std::sync::atomic::{AtomicBool, Ordering};
static DEMO_MODE: AtomicBool = AtomicBool::new(false);

pub fn is_demo_mode() -> bool {
    DEMO_MODE.load(Ordering::SeqCst)
}

pub fn set_demo_mode(val: bool) {
    DEMO_MODE.store(val, Ordering::SeqCst);
}
