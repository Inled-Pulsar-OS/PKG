use super::*;
use Action as A;
use LayoutState::*;

fn ctx() -> Context {
    Context { mode_all: true, query_empty: true, at_root: true, ..Context::default() }
}

fn run(s: LayoutState, c: &Context, e: Event) -> (LayoutState, Action) {
    let o = transition(s, c, e);
    (o.state, o.action)
}

fn ptr(x: f64, y: f64) -> Event {
    Event::Pointer { x, y }
}

#[test]
fn first_pointer_event_only_records() {
    assert_eq!(run(Idle, &ctx(), ptr(500.0, 300.0)), (Idle, A::RecordPointer));
}

#[test]
fn pointer_travel_threshold() {
    let c = Context { pointer_origin: Some((100.0, 100.0)), ..ctx() };
    assert_eq!(run(Idle, &c, ptr(100.0, 100.0)), (Idle, A::Nothing));
    assert_eq!(run(Idle, &c, ptr(105.0, 105.0)), (Idle, A::Nothing)); // 7.07
    assert_eq!(run(Idle, &c, ptr(108.0, 100.0)), (IdleModes, A::Nothing));
    assert_eq!(run(Idle, &c, ptr(106.0, 106.0)), (IdleModes, A::Nothing)); // 8.49
}

#[test]
fn pointer_ignored_after_reveal_and_in_panel() {
    let c = Context { pointer_origin: Some((0.0, 0.0)), ..ctx() };
    assert_eq!(run(IdleModes, &c, ptr(50.0, 50.0)), (IdleModes, A::Nothing));
    assert_eq!(run(Panel, &c, ptr(50.0, 50.0)), (Panel, A::Nothing));
}

#[test]
fn tab_reveals_then_cycles() {
    assert_eq!(run(Idle, &ctx(), Event::Tab), (IdleModes, A::Nothing));
    assert_eq!(run(IdleModes, &ctx(), Event::Tab), (IdleModes, A::CycleMode));
    assert_eq!(run(Panel, &ctx(), Event::Tab), (Panel, A::CycleMode));
}

#[test]
fn bar_events_open_the_panel() {
    for s in [Idle, IdleModes] {
        assert_eq!(run(s, &ctx(), Event::Char), (Panel, A::ShowResults));
        assert_eq!(run(s, &ctx(), Event::Down), (Panel, A::ShowHome));
        assert_eq!(run(s, &ctx(), Event::Up), (Panel, A::ShowHistory));
        for m in [BarMode::Applications, BarMode::Files, BarMode::Actions, BarMode::Clipboard] {
            assert_eq!(run(s, &ctx(), Event::Mode(m)), (Panel, A::SwitchMode(m)));
        }
    }
}

#[test]
fn panel_keeps_default_handling() {
    for e in [Event::Char, Event::Down, Event::Up, Event::Mode(BarMode::Files)] {
        assert_eq!(run(Panel, &ctx(), e), (Panel, A::Default));
    }
}

#[test]
fn esc_closes_from_bar_and_root_steps_back_inside() {
    assert_eq!(run(Idle, &ctx(), Event::Esc), (Idle, A::Close));
    assert_eq!(run(IdleModes, &ctx(), Event::Esc), (IdleModes, A::Close));
    assert_eq!(run(Panel, &ctx(), Event::Esc), (Panel, A::Close));
    let inside = Context { at_root: false, ..ctx() };
    assert_eq!(run(Panel, &inside, Event::Esc), (Panel, A::StepBack));
}

#[test]
fn shift_esc_returns_to_bar_only_when_empty_and_all() {
    assert_eq!(run(Panel, &ctx(), Event::ShiftEsc), (Idle, A::Collapse));
    let shown = Context { revealed: true, ..ctx() };
    assert_eq!(run(Panel, &shown, Event::ShiftEsc), (IdleModes, A::Collapse));
    let typed = Context { query_empty: false, ..ctx() };
    assert_eq!(run(Panel, &typed, Event::ShiftEsc), (Panel, A::Default));
    let mode = Context { mode_all: false, ..ctx() };
    assert_eq!(run(Panel, &mode, Event::ShiftEsc), (Panel, A::Default));
}

#[test]
fn emptied_typed_query_collapses_to_bar() {
    let c = Context { query_typed: true, ..ctx() };
    assert_eq!(run(Panel, &c, Event::QueryEmptied), (Idle, A::Collapse));
}

#[test]
fn collapse_target_follows_revealed_flag() {
    let c = Context { query_typed: true, revealed: true, ..ctx() };
    assert_eq!(run(Panel, &c, Event::QueryEmptied), (IdleModes, A::Collapse));
}

#[test]
fn no_collapse_in_mode_after_down_or_up() {
    let base = Context { query_typed: true, ..ctx() };
    for c in [
        Context { mode_all: false, ..base },
        Context { home_opened: true, ..base },
        Context { history_opened: true, ..base },
        Context { query_typed: false, ..base },
    ] {
        assert_eq!(run(Panel, &c, Event::QueryEmptied), (Panel, A::Nothing), "{c:?}");
    }
}

#[test]
fn new_open_starts_clean() {
    let c = Context::default();
    assert!(!c.revealed && c.pointer_origin.is_none());
    assert_eq!(run(Idle, &c, ptr(1.0, 1.0)).1, A::RecordPointer);
}
