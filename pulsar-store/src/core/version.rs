//! Version comparison for packages from different ecosystems.
//!
//! Handles dotted numbers with an optional epoch (`1:`) and release (`-3`),
//! such as `158.0.1-1` or `2.10`. Versions it cannot read as numbers are only
//! compared for equality, so an odd format never hides or invents an update
//! by ordering alone.

use std::cmp::Ordering;

fn numbers(v: &str) -> Option<Vec<u64>> {
    let v = v.trim();
    let v = v.split_once(':').map_or(v, |(_, rest)| rest); // drop epoch
    let v = v.rsplit_once('-').map_or(v, |(main, rel)| {
        if rel.chars().all(|c| c.is_ascii_digit()) {
            main
        } else {
            v
        }
    }); // drop numeric release
    let v = v.strip_prefix('v').unwrap_or(v);
    if !v.chars().next()?.is_ascii_digit() {
        return None;
    }
    v.split('.')
        .map(|part| {
            let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .collect()
}

/// Orders two versions numerically, or `None` when either cannot be read.
pub fn compare(a: &str, b: &str) -> Option<Ordering> {
    let (mut x, mut y) = (numbers(a)?, numbers(b)?);
    let len = x.len().max(y.len());
    x.resize(len, 0);
    y.resize(len, 0);
    Some(x.cmp(&y))
}

/// True when `available` should be offered as an update over `installed`.
pub fn is_update(installed: &str, available: &str) -> bool {
    match compare(available, installed) {
        Some(order) => order == Ordering::Greater,
        None => installed != available,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_and_older() {
        assert!(is_update("1.0", "2.0"));
        assert!(is_update("1.9", "1.10"));
        assert!(!is_update("158.0.1-1", "135.0"));
        assert!(!is_update("1.14.4-2", "1.7.7"));
    }

    #[test]
    fn equal_forms_are_not_updates() {
        assert!(!is_update("1.0", "1.0.0"));
        assert!(!is_update("2.1-3", "2.1"));
        assert!(!is_update("1:2.0", "2.0"));
        assert!(!is_update("v1.2", "1.2"));
    }

    #[test]
    fn unreadable_versions_fall_back_to_inequality() {
        assert!(is_update("nightly", "stable"));
        assert!(!is_update("nightly", "nightly"));
    }

    #[test]
    fn compare_reports_none_for_text() {
        assert_eq!(compare("abc", "1.0"), None);
        assert_eq!(compare("2.0", "1.0"), Some(Ordering::Greater));
    }
}
