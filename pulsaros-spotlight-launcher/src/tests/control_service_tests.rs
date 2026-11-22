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
        on_pointer: Arc::new(|_, _| {}),
    };
    iface.hide();
    iface.hide();
    assert_eq!(n.load(Ordering::SeqCst), 2);
}

#[test]
fn pointer_moved_forwards_the_coordinates() {
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen2 = seen.clone();
    let iface = ControlIface {
        on_hide: Arc::new(|| {}),
        on_pointer: Arc::new(move |x, y| seen2.lock().unwrap().push((x, y))),
    };
    iface.pointer_moved(10.0, 20.5);
    assert_eq!(*seen.lock().unwrap(), vec![(10.0, 20.5)]);
}
