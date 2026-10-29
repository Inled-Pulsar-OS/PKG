//! Logic for the background update checker (`pulsar-store-updated`), kept free
//! of D-Bus so it can be tested.

use std::collections::BTreeSet;
use std::time::Duration;

pub const DEFAULT_INTERVAL_HOURS: u64 = 6;
const MIN_INTERVAL_HOURS: u64 = 1;
const MAX_INTERVAL_HOURS: u64 = 24 * 7;

/// Check interval from `--interval-hours N` or `PULSAR_STORE_UPDATE_INTERVAL_HOURS`,
/// clamped to between one hour and one week. Bad values fall back to the default.
pub fn interval_from(args: &[String], env: Option<&str>) -> Duration {
    let from_args = args
        .windows(2)
        .find(|w| w[0] == "--interval-hours")
        .map(|w| w[1].as_str());
    let hours = from_args
        .or(env)
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_INTERVAL_HOURS)
        .clamp(MIN_INTERVAL_HOURS, MAX_INTERVAL_HOURS);
    Duration::from_secs(hours * 3600)
}

/// NetworkManager's `Metered` property: 1 is "yes" and 3 is "guess yes".
pub fn is_metered(nm_value: u32) -> bool {
    matches!(nm_value, 1 | 3)
}

/// Remembers which updates the user was already told about, so a check that
/// finds the same updates does not notify again.
#[derive(Debug, Default)]
pub struct UpdateTracker {
    known: BTreeSet<String>,
}

impl UpdateTracker {
    /// Records the current update ids. Returns true when some id is new.
    pub fn observe(&mut self, ids: &[String]) -> bool {
        let now: BTreeSet<String> = ids.iter().cloned().collect();
        let has_new = now.iter().any(|id| !self.known.contains(id));
        self.known = now;
        has_new
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn interval_defaults_clamps_and_reads_args_first() {
        let day = |h: u64| Duration::from_secs(h * 3600);
        assert_eq!(interval_from(&[], None), day(6));
        assert_eq!(
            interval_from(&s(&["--interval-hours", "12"]), Some("3")),
            day(12)
        );
        assert_eq!(interval_from(&[], Some("3")), day(3));
        assert_eq!(interval_from(&s(&["--interval-hours", "0"]), None), day(1));
        assert_eq!(
            interval_from(&s(&["--interval-hours", "99999"]), None),
            day(168)
        );
        assert_eq!(
            interval_from(&s(&["--interval-hours", "abc"]), None),
            day(6)
        );
    }

    #[test]
    fn metered_values() {
        assert!(is_metered(1) && is_metered(3));
        assert!(!is_metered(0) && !is_metered(2) && !is_metered(4));
    }

    #[test]
    fn tracker_notifies_only_for_new_updates() {
        let mut t = UpdateTracker::default();
        assert!(!t.observe(&[]));
        assert!(t.observe(&s(&["a", "b"])));
        assert!(!t.observe(&s(&["a", "b"])));
        assert!(!t.observe(&s(&["a"])));
        assert!(t.observe(&s(&["a", "c"])));
    }
}
