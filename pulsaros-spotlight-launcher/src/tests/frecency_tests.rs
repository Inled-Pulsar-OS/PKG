use super::*;
use crate::core::store::temp_store;

const DAY: i64 = 86400;

#[test]
fn launch_adds_100() {
    let mut f = Frecency::default();
    f.record_launch("a", "", 1000);
    assert_eq!(f.score("a", 1000), 100.0);
    f.record_launch("a", "", 1000);
    assert_eq!(f.score("a", 1000), 200.0);
}

#[test]
fn half_life_ten_days() {
    let mut f = Frecency::default();
    f.record_launch("a", "", 0);
    assert!((f.score("a", 10 * DAY) - 50.0).abs() < 1e-9);
    assert!((f.score("a", 20 * DAY) - 25.0).abs() < 1e-9);
}

#[test]
fn decay_applied_before_adding() {
    let mut f = Frecency::default();
    f.record_launch("a", "", 0);
    f.record_launch("a", "", 10 * DAY);
    assert!((f.score("a", 10 * DAY) - 150.0).abs() < 1e-9);
}

#[test]
fn unknown_is_zero_and_clock_backwards_safe() {
    let mut f = Frecency::default();
    assert_eq!(f.score("x", 5), 0.0);
    f.record_launch("a", "", 100);
    assert_eq!(f.score("a", 50), 100.0);
}

#[test]
fn learned_prefix_and_exact() {
    let mut f = Frecency::default();
    f.record_launch("ff", "ff", 0);
    f.record_launch("ff", "ff", 0);
    assert_eq!(f.learned("ff", "ff"), 4);
    assert_eq!(f.learned("f", "ff"), 2);
    assert_eq!(f.learned("fi", "ff"), 0);
    assert_eq!(f.learned("", "ff"), 0);
    assert_eq!(f.learned("ff", "other"), 0);
}

#[test]
fn learned_is_case_insensitive_and_skips_empty_query() {
    let mut f = Frecency::default();
    f.record_launch("a", " FF ", 0);
    f.record_launch("a", "", 0);
    assert_eq!(f.learned("ff", "a"), 2);
    assert_eq!(f.learned.len(), 1);
}

#[test]
fn top_filters_and_sorts() {
    let mut f = Frecency::default();
    f.record_launch("a", "", 0);
    f.record_launch("b", "", 0);
    f.record_launch("b", "", 0);
    f.record_launch("old", "", -100 * DAY);
    assert_eq!(f.top(0, 50.0, 6), vec!["b", "a"]);
    assert_eq!(f.top(0, 50.0, 1), vec!["b"]);
}

#[test]
fn learned_map_evicts_least_recently_used_beyond_cap() {
    let mut f = Frecency::default();
    for i in 0..=LEARNED_CAP {
        f.record_launch("a", &format!("q{}", i), 0);
    }
    assert_eq!(f.learned.len(), LEARNED_CAP);
    // q0 was the least-recently-used query and should have been evicted.
    assert!(!f.learned.contains_key("q0"));
    // the most recently added query is retained.
    assert!(f.learned.contains_key(&format!("q{}", LEARNED_CAP)));
}

#[test]
fn learned_map_cap_respects_recency_on_reuse() {
    let mut f = Frecency::default();
    for i in 0..LEARNED_CAP {
        f.record_launch("a", &format!("q{}", i), 0);
    }
    // Re-recording q0 bumps it to most-recently-used.
    f.record_launch("a", "q0", 0);
    // Adding one more distinct query now evicts q1, not q0.
    f.record_launch("a", "new-query", 0);

    assert_eq!(f.learned.len(), LEARNED_CAP);
    assert!(f.learned.contains_key("q0"));
    assert!(!f.learned.contains_key("q1"));
}

#[test]
fn legacy_learned_without_recency_queue_does_not_evict_newly_typed_query() {
    // Mirrors deserializing a frecency file persisted before
    // `learned_recency` existed: `learned` is already at LEARNED_CAP
    // from legacy data, but the LRU queue is empty (its
    // `#[serde(default)]` value).
    let mut f = Frecency::default();
    for i in 0..LEARNED_CAP {
        f.learned.insert(format!("legacy{}", i), BTreeMap::new());
    }
    assert_eq!(f.learned.len(), LEARNED_CAP);
    assert!(f.learned_recency.is_empty());

    // A brand-new query typed after upgrading must survive, not be
    // evicted immediately as the only entry in the (otherwise empty)
    // LRU queue.
    f.record_launch("a", "new-query", 0);

    assert!(f.learned.contains_key("new-query"));
    assert_eq!(f.learned.len(), LEARNED_CAP);
}

#[test]
fn persists() {
    let s = temp_store("frec");
    let mut f = Frecency::default();
    f.record_launch("a", "q", 7);
    f.save(&s).unwrap();
    let g = Frecency::load(&s);
    assert_eq!(g.score("a", 7), 100.0);
    assert_eq!(g.learned("q", "a"), 2);
}
