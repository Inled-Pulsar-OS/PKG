use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn hide_calls_the_callback_once_per_call() {
    let n = Arc::new(AtomicUsize::new(0));
    let n2 = n.clone();
    let iface = ControlIface {
        on_hide: Arc::new(move || {
            n2.fetch_add(1, Ordering::SeqCst);
        }),
    };
    iface.hide();
    iface.hide();
    assert_eq!(n.load(Ordering::SeqCst), 2);
}
