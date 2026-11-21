//! Layout State of the launcher window: how much of it is drawn (see GLOSSARY).
//!
//! One pure function, `transition`, decides every change. The window wiring only
//! feeds it events and performs the returned `Action`. No GTK here.
//!
//! The caller keeps the `Context`. On every open it starts from `Context::default()`
//! and `LayoutState::Idle`. It sets `Context::revealed` once a transition returns
//! `LayoutState::IdleModes`, and `Context::pointer_origin` when told to record it.

/// Pointer travel (pixels) that counts as real movement.
pub const POINTER_THRESHOLD: f64 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutState {
    /// Pill alone. Shown on every open.
    Idle,
    /// Pill plus the four mode buttons.
    IdleModes,
    /// The expanded window with results.
    Panel,
}

/// The four modes reachable from the bar buttons and Alt+1..4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarMode {
    Applications,
    Files,
    Actions,
    Clipboard,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Context {
    /// Current mode is All.
    pub mode_all: bool,
    pub query_empty: bool,
    /// The query text was typed by the user (not set by a mode or a view).
    pub query_typed: bool,
    /// The Panel shows its root view (nothing pushed, no dialog, no folder, no history).
    pub at_root: bool,
    /// The Panel was opened by Down (home view).
    pub home_opened: bool,
    /// The Panel was opened by Up (history).
    pub history_opened: bool,
    /// The buttons were shown at some point this session.
    pub revealed: bool,
    /// First pointer position seen this session.
    pub pointer_origin: Option<(f64, f64)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Pointer { x: f64, y: f64 },
    Tab,
    Char,
    Down,
    Up,
    /// A button click or Alt+1..4.
    Mode(BarMode),
    Esc,
    ShiftEsc,
    /// The query just became empty.
    QueryEmptied,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Nothing,
    /// Store this pointer position as `Context::pointer_origin`.
    RecordPointer,
    Close,
    StepBack,
    CycleMode,
    ShowResults,
    ShowHome,
    ShowHistory,
    SwitchMode(BarMode),
    /// Shrink to the bar; the next state says which one.
    Collapse,
    /// The event keeps the meaning it has today.
    Default,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outcome {
    pub state: LayoutState,
    pub action: Action,
}

fn out(state: LayoutState, action: Action) -> Outcome {
    Outcome { state, action }
}

fn bar_target(ctx: &Context) -> LayoutState {
    if ctx.revealed { LayoutState::IdleModes } else { LayoutState::Idle }
}

pub fn transition(state: LayoutState, ctx: &Context, event: Event) -> Outcome {
    use LayoutState::*;
    let bar = state != Panel;
    match event {
        Event::Pointer { x, y } if state == Idle => match ctx.pointer_origin {
            None => out(Idle, Action::RecordPointer),
            Some((ox, oy)) if (x - ox).hypot(y - oy) >= POINTER_THRESHOLD => {
                out(IdleModes, Action::Nothing)
            }
            Some(_) => out(Idle, Action::Nothing),
        },
        Event::Pointer { .. } => out(state, Action::Nothing),
        Event::Tab if state == Idle => out(IdleModes, Action::Nothing),
        Event::Tab => out(state, Action::CycleMode),
        Event::Char if bar => out(Panel, Action::ShowResults),
        Event::Down if bar => out(Panel, Action::ShowHome),
        Event::Up if bar => out(Panel, Action::ShowHistory),
        Event::Mode(m) if bar => out(Panel, Action::SwitchMode(m)),
        Event::Char | Event::Down | Event::Up | Event::Mode(_) => out(state, Action::Default),
        Event::Esc if bar || ctx.at_root => out(state, Action::Close),
        Event::Esc => out(state, Action::StepBack),
        Event::ShiftEsc if state == Panel && ctx.query_empty && ctx.mode_all => {
            out(bar_target(ctx), Action::Collapse)
        }
        Event::ShiftEsc => out(state, Action::Default),
        Event::QueryEmptied
            if state == Panel
                && ctx.mode_all
                && ctx.query_typed
                && !ctx.home_opened
                && !ctx.history_opened =>
        {
            out(bar_target(ctx), Action::Collapse)
        }
        Event::QueryEmptied => out(state, Action::Nothing),
    }
}

#[cfg(test)]
#[path = "../tests/layout_tests.rs"]
mod tests;
