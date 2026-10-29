//! A value that is reloaded after a short time or when told to.

use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct TtlCache<T> {
    ttl: Duration,
    slot: Mutex<Option<(Instant, T)>>,
}

impl<T: Clone> TtlCache<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            slot: Mutex::new(None),
        }
    }

    /// Returns the cached value while it is fresh, else loads and stores a new one.
    pub fn get_or_load(&self, load: impl FnOnce() -> T) -> T {
        let mut slot = self.slot.lock().unwrap();
        if let Some((at, value)) = slot.as_ref() {
            if at.elapsed() < self.ttl {
                return value.clone();
            }
        }
        let value = load();
        *slot = Some((Instant::now(), value.clone()));
        value
    }

    /// Forces the next `get_or_load` to reload. Call after changing what is cached.
    pub fn invalidate(&self) {
        *self.slot.lock().unwrap() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn loads_once_while_fresh() {
        let c = TtlCache::new(Duration::from_secs(60));
        let n = Cell::new(0);
        let load = || {
            n.set(n.get() + 1);
            n.get()
        };
        assert_eq!(c.get_or_load(load), 1);
        assert_eq!(c.get_or_load(load), 1);
        c.invalidate();
        assert_eq!(c.get_or_load(load), 2);
    }

    #[test]
    fn reloads_after_ttl() {
        let c = TtlCache::new(Duration::ZERO);
        let n = Cell::new(0);
        let load = || {
            n.set(n.get() + 1);
            n.get()
        };
        assert_eq!(c.get_or_load(load), 1);
        assert_eq!(c.get_or_load(load), 2);
    }
}
