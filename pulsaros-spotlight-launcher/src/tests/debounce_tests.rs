use super::*;

#[test]
fn burst_of_20_events_triggers_exactly_one_load() {
    let mut d = Debounce::default();
    let mut timers_armed = 0;
    for _ in 0..20 {
        if d.trigger() {
            timers_armed += 1;
        }
    }
    assert_eq!(timers_armed, 1);

    // One timer expiry = one load_apps() call.
    let mut loads = 0;
    for _ in 0..timers_armed {
        d.fire();
        loads += 1;
    }
    assert_eq!(loads, 1);
}

#[test]
fn event_after_fire_arms_a_new_timer() {
    let mut d = Debounce::default();
    assert!(d.trigger());
    assert!(!d.trigger());
    d.fire();
    assert!(d.trigger());
}
