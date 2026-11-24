//! Trailing-edge coalescing for bursts of file-monitor events.
//!
//! `trigger()` is called per event. It returns `true` only for the first event of a burst;
//! the caller arms one timer then and calls `fire()` when it expires.

pub const RELOAD_DELAY_MS: u64 = 500;

#[derive(Debug, Default)]
pub struct Debounce {
    pending: bool,
}

impl Debounce {
    /// `true` when the caller must arm the timer (no timer is pending yet).
    pub fn trigger(&mut self) -> bool {
        !std::mem::replace(&mut self.pending, true)
    }

    /// Timer expired: clear the pending flag so the next event arms a new timer.
    pub fn fire(&mut self) {
        self.pending = false;
    }
}

#[cfg(test)]
#[path = "../tests/debounce_tests.rs"]
mod tests;
