use adw::prelude::AnimationExt as _;
use crate::calculator::{CalcHistory, CalcResult, Calculator};
use crate::category::Category;
use crate::clipboard::ClipboardManager;
use crate::config::SpotlightConfig;
use crate::core::apps_provider::AppsProvider;
use crate::core::calc_provider::{history_result, CalcHistoryProvider, CalculatorProvider};
use crate::core::clipboard_provider::{self, ClipboardProvider};
use crate::core::commands::{self, Command};
use crate::core::commands_provider::CommandsProvider;
use crate::core::frecency::Frecency;
use crate::core::nav::{Browse, Nav, NavEvent, Opened};
use crate::core::layout::{self, Action as LayoutAction, BarMode, Context as LayoutCtx, Event as LayoutEvent, LayoutState};
use crate::core::provider::{Provider, Query, Registry};
use crate::core::root_run::RootRun;
use crate::core::root_search::{AppEntry, Ctx, Row, RowKind, RootSearch, SearchIndex, Section};
use crate::core::settings::Settings;
use crate::core::shortcuts::{self, Act, Mods};
use crate::core::store::Store;
use crate::search::{file_data, run_job, SearchBackend};
use crate::search_providers::{FilesProvider, HistoryProvider, WebProvider};
use crate::ui::results::{get_favorites, result_row, set_favorites, Payload, ResultView, RowData};
use crate::utils::{get_icon_dir, get_local_icon_dir, open_file};
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

mod actions;
mod args;
mod chrome;
mod dialog;
mod emoji;
mod header;
use header::{build_category_bar, build_mode_buttons, build_toast, SearchHeader};
mod keys;
mod panel;
mod pointer;
#[cfg(test)]
use pointer::parse_cursor_pos;
mod scripts;
mod search;
mod snippets;
mod toast;
mod views;
use actions::UiAction;
use chrome::*;
use dialog::Dialog;
use views::{Extras, View};
#[cfg(test)]
use keys::bar_event;
pub(crate) use panel::primary_title;
use panel::Panel;

type SectionList = Vec<(String, Vec<RowData>)>;

pub struct SpotlightWindow {
    window: gtk4::ApplicationWindow,
    search_entry: gtk4::Entry,
    mode_icon: gtk4::Image,
    result_view: Rc<ResultView>,
    config: Rc<RefCell<SpotlightConfig>>,
    backend: SearchBackend,
    category: Cell<Category>,
    /// Which list is shown, how the Panel opened, whether the query was typed.
    nav: RefCell<Nav>,
    search_state: SearchState,

    // core state
    store: Store,
    settings: RefCell<Settings>,
    frecency: RefCell<Frecency>,
    commands: RefCell<Vec<Command>>,
    x: Extras,
    current: RefCell<SectionList>,

    // ui pieces
    panel: Panel,
    dialog: Dialog,
    more_popover: gtk4::Popover,
    category_buttons: RefCell<HashMap<Category, gtk4::ToggleButton>>,
    loading: Loading,
    toast_ui: ToastUi,
    zoom_provider: gtk4::CssProvider,

    layout: Cell<LayoutState>,
    bar: Bar,
    progress: Progress,
}

/// Debounce, search sequencing and the caches the search flow reads.
struct SearchState {
    debounce_id: Rc<RefCell<Option<glib::SourceId>>>,
    search_seq: Rc<Cell<u64>>,
    calc_history: Rc<RefCell<CalcHistory>>,
    last_currency: Cell<Option<std::time::Instant>>,
    /// Apps and their ranker index, rebuilt after the next app reload (`None` = stale).
    search_data: RefCell<Option<Rc<(Vec<AppEntry>, SearchIndex)>>>,
    states: RefCell<HashMap<String, bool>>,
    last_probe: Cell<Option<std::time::Instant>>,
}

struct Loading {
    bar: gtk4::ProgressBar,
    count: Cell<u32>,
    timer: RefCell<Option<glib::SourceId>>,
}

struct ToastUi {
    status: gtk4::Label,
    pill: gtk4::Box,
    undo: gtk4::Button,
    cb: RefCell<Option<Rc<dyn Fn()>>>,
    timer: RefCell<Option<glib::SourceId>>,
}

/// The mode-button bar and the pointer tracking that reveals it.
struct Bar {
    /// The four round mode buttons; visible only in `IdleModes`.
    mode_buttons: gtk4::Box,
    buttons_revealer: gtk4::Revealer,
    size_anim: RefCell<Option<adw::TimedAnimation>>,
    /// True once the Shell extension reports pointer motion; the window's own motion events
    /// (local coordinates) are then ignored so the two coordinate spaces never mix.
    global_pointer: Cell<bool>,
    hypr_poll: RefCell<Option<glib::SourceId>>,
    /// The buttons were shown at some point since the window opened.
    revealed: Cell<bool>,
    /// First pointer position seen since the window opened.
    pointer_origin: Cell<Option<(f64, f64)>>,
    /// Everything below the header row; hidden while the layout is Idle.
    panel_parts: Vec<gtk4::Widget>,
}

struct Progress {
    revealer: gtk4::Revealer,
    spinner: gtk4::Spinner,
    label: gtk4::Label,
}

fn app_entry(a: &crate::apps::DesktopApp) -> AppEntry {
    AppEntry {
        id: format!("app:{}", a.filename),
        name: a.name.clone(),
        comment: a.comment.clone(),
        icon: a.icon.clone(),
        keywords: a.keywords.split(';').filter(|k| !k.is_empty()).map(String::from).collect(),
    }
}

fn clean_uninstall_message(message: &str, app_name: &str) -> String {
    let mut clean = message.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ");
    if clean.to_lowercase().contains("dependencias") || clean.to_lowercase().contains("dependencies") {
        clean = format!("Cannot remove {}: required by other system packages", app_name);
    } else if let Some(err_part) = clean.split("error:").nth(1) {
        clean = err_part.trim().to_string();
    }
    clean
}

impl SpotlightWindow {
    pub fn new(
        app: &gtk4::Application,
        config: SpotlightConfig,
        backend: SearchBackend,
        clipboard_mgr: Option<ClipboardManager>,
    ) -> Rc<Self> {
        let _ = clipboard_mgr; // the backend already owns the clipboard manager
        let zoom = config.zoom.clamp(ZOOM_MIN, ZOOM_MAX);
        let config_rc = Rc::new(RefCell::new(config));

        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Spotlight")
            .default_width(BASE_W)
            .default_height(BASE_H)
            .resizable(false)
            .decorated(false)
            .build();
        window.set_size_request((BASE_W as f64 * zoom) as i32, (BASE_H as f64 * zoom) as i32);
        window.add_css_class("spotlight-window");

        if pulsar_liquid_glass_enabled() {
            window.add_css_class("liquid-glass");
        }

        // Follow the libadwaita color scheme live (colors and accent come from
        // libadwaita named colors; these classes only tune alpha values).
        let style_manager = adw::StyleManager::default();
        let apply_scheme = {
            let window = window.clone();
            move |dark: bool| {
                window.remove_css_class(if dark { "light" } else { "dark" });
                window.add_css_class(if dark { "dark" } else { "light" });
            }
        };
        apply_scheme(style_manager.is_dark());
        style_manager.connect_notify_local(Some("dark"), move |sm, _| apply_scheme(sm.is_dark()));

        let zoom_provider = gtk4::CssProvider::new();
        zoom_provider.load_from_string(&zoom_css(zoom));
        if let Some(display) = gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(&display, &zoom_provider, 1001);
        }

        let store = Store::new(Store::default_dir());
        let x = Extras::new(&store, commands::builtin());

        let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        main_box.add_css_class("spotlight-main");
        main_box.set_vexpand(true);
        main_box.set_hexpand(true);
        // Clip children to the rounded corners (the CSS `overflow` property is not accepted by GTK).
        main_box.set_overflow(gtk4::Overflow::Hidden);
        // The bar buttons sit beside the pill, not inside it, so the pill contracts when they show.
        let outer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        outer.append(&main_box);
        window.set_child(Some(&outer));

        let SearchHeader { container: search_container, mode_icon, entry: search_entry, more_button } = SearchHeader::build(&x);

        let (mode_buttons, buttons_revealer) = build_mode_buttons();
        outer.append(&buttons_revealer);
        main_box.append(&search_container);

        let (category_scroll, cat_buttons) = build_category_bar();
        main_box.append(&category_scroll);

        // -- More popover anchored to more_button --
        let more_popover = gtk4::Popover::new();
        more_popover.set_parent(&more_button);
        more_popover.set_position(gtk4::PositionType::Bottom);
        more_popover.set_has_arrow(true);
        more_popover.add_css_class("spotlight-popover");

        // -- Loading bar --
        let loading_bar = gtk4::ProgressBar::new();
        loading_bar.add_css_class("loading-bar");
        loading_bar.set_opacity(0.0);
        main_box.append(&loading_bar);

        let (toast_pill, footer_status, toast_undo) = build_toast();

        // -- Uninstall progress banner --
        let progress_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideDown)
            .transition_duration(200)
            .reveal_child(false)
            .build();
        let progress_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
        progress_box.add_css_class("uninstall-progress");
        let progress_spinner = gtk4::Spinner::new();
        let progress_label = gtk4::Label::builder()
            .hexpand(true)
            .xalign(0.0)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();
        progress_box.append(&progress_spinner);
        progress_box.append(&progress_label);
        progress_revealer.set_child(Some(&progress_box));

        let panel = Panel::build();
        let (dialog, dialog_cancel, click_backdrop) = Dialog::build(&x.form);
        let dialog_ok = dialog.ok.clone();

        // -- Result view --
        let result_view_cell: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let activate_cell = result_view_cell.clone();
        let context_cell: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let context_cell_c = context_cell.clone();
        let select_cell: Rc<RefCell<Option<Rc<dyn Fn(Option<RowData>)>>>> = Rc::new(RefCell::new(None));
        let select_cell_c = select_cell.clone();
        let result_view = Rc::new(ResultView::new(
            move || {
                let cb = activate_cell.borrow().clone();
                if let Some(cb) = cb {
                    cb();
                }
            },
            move || {
                let cb = context_cell_c.borrow().clone();
                if let Some(cb) = cb {
                    cb();
                }
            },
            move |rd| {
                let cb = select_cell_c.borrow().clone();
                if let Some(cb) = cb {
                    cb(rd);
                }
            },
        ));

        x.stack.add_named(result_view.widget(), Some("list"));
        let overlay = gtk4::Overlay::new();
        overlay.set_vexpand(true);
        overlay.set_child(Some(&x.stack));
        overlay.add_overlay(&toast_pill);
        overlay.add_overlay(&panel.root);
        overlay.add_overlay(&dialog.backdrop);
        overlay.add_overlay(&dialog.root);
        main_box.append(&overlay);
        main_box.append(&progress_revealer);

        let panel_parts: Vec<gtk4::Widget> = vec![
            more_button.clone().upcast(),
            category_scroll.clone().upcast(),
            loading_bar.clone().upcast(),
            overlay.clone().upcast(),
            progress_revealer.clone().upcast(),
        ];

        let settings = Settings::load(&store);
        let frecency = Frecency::load(&store);

        let spotlight = Rc::new(Self {
            window,
            search_entry,
            mode_icon,
            result_view,
            config: config_rc,
            backend,
            category: Cell::new(Category::All),
            nav: RefCell::new(Nav::default()),
            search_state: SearchState {
                debounce_id: Rc::new(RefCell::new(None)),
                search_seq: Rc::new(Cell::new(0)),
                calc_history: Rc::new(RefCell::new(CalcHistory::open())),
                last_currency: Cell::new(None),
                search_data: RefCell::new(None),
                states: RefCell::new(HashMap::new()),
                last_probe: Cell::new(None),
            },
            store,
            settings: RefCell::new(settings),
            frecency: RefCell::new(frecency),
            commands: RefCell::new(Vec::new()),
            x,
            current: RefCell::new(Vec::new()),
            panel,
            dialog,
            more_popover,
            category_buttons: RefCell::new(cat_buttons),
            loading: Loading { bar: loading_bar, count: Cell::new(0), timer: RefCell::new(None) },
            toast_ui: ToastUi {
                status: footer_status,
                pill: toast_pill,
                undo: toast_undo,
                cb: RefCell::new(None),
                timer: RefCell::new(None),
            },
            zoom_provider,
            layout: Cell::new(LayoutState::Idle),
            bar: Bar {
                mode_buttons,
                buttons_revealer,
                size_anim: RefCell::new(None),
                global_pointer: Cell::new(false),
                hypr_poll: RefCell::new(None),
                revealed: Cell::new(false),
                pointer_origin: Cell::new(None),
                panel_parts,
            },
            progress: Progress { revealer: progress_revealer, spinner: progress_spinner, label: progress_label },
        });

        // Late-bound callbacks that need the Rc<Self>.
        let s = spotlight.clone();
        *result_view_cell.borrow_mut() = Some(Rc::new(move || s.run_action_index(0)));
        let s = spotlight.clone();
        *context_cell.borrow_mut() = Some(Rc::new(move || s.open_panel()));

        let s = spotlight.clone();
        *select_cell.borrow_mut() = Some(Rc::new(move |rd| s.on_row_selected(rd)));

        let s = spotlight.clone();
        click_backdrop.connect_pressed(move |_, _, _, _| s.close_dialog());

        let s = spotlight.clone();
        dialog_cancel.connect_clicked(move |_| s.close_dialog());
        let s = spotlight.clone();
        dialog_ok.connect_clicked(move |_| s.accept_dialog());
        let s = spotlight.clone();
        spotlight.dialog.entry.connect_activate(move |_| s.accept_dialog());
        let s = spotlight.clone();
        spotlight.toast_ui.undo.connect_clicked(move |_| {
            let cb = s.toast_ui.cb.borrow_mut().take();
            if let Some(cb) = cb {
                cb();
            }
            s.reset_toast();
        });

        let s = spotlight.clone();
        more_button.connect_clicked(move |_| s.open_more_menu());

        let s = spotlight.clone();
        spotlight.search_entry.connect_activate(move |_| {
            s.run_action_index(0);
        });

        for (cat_id, btn) in spotlight.category_buttons.borrow().iter() {
            let s = spotlight.clone();
            let cid = *cat_id;
            let btn_c = btn.clone();
            btn.connect_toggled(move |_| {
                if btn_c.is_active() {
                    s.set_category(cid);
                }
            });
        }

        spotlight.setup_panel_events();
        spotlight.setup_ui_interactions();
        spotlight.setup_extras();
        spotlight
    }






    pub fn present_with_focus(self: &Rc<Self>) {
        // Follow the Liquid Glass switch from Pulsar Settings.
        if pulsar_liquid_glass_enabled() {
            self.window.add_css_class("liquid-glass");
        } else {
            self.window.remove_css_class("liquid-glass");
        }

        self.bar.revealed.set(false);
        self.bar.pointer_origin.set(None);
        self.reset_to_root();
        self.window.present();
        self.start_hypr_pointer_poll();
        self.search_entry.grab_focus();
        self.refresh_states();
        self.refresh_currencies();
        self.refresh_extras();
    }

    fn nav_apply(&self, e: NavEvent) {
        self.nav.borrow_mut().apply(e);
    }

    fn in_recent(&self) -> bool {
        self.nav.borrow().browse == Browse::RecentSearches
    }

    fn in_calc_history(&self) -> bool {
        self.nav.borrow().browse == Browse::CalcHistory
    }

    fn reset_to_root(self: &Rc<Self>) {
        self.close_panel();
        self.close_dialog();
        self.leave_view();
        self.nav_apply(NavEvent::ToRoot);
        self.category.set(Category::All);
        self.apply_layout(if self.bar.revealed.get() { LayoutState::IdleModes } else { LayoutState::Idle });
        self.set_mode(Category::All);
        self.search_entry.set_text("");
    }

    /// Show only the header row in a bar state, the full window in the Panel.
    fn apply_layout(&self, state: LayoutState) {
        self.layout.set(state);
        let idle = state != LayoutState::Panel;
        self.bar.buttons_revealer.set_reveal_child(state == LayoutState::IdleModes);
        if state == LayoutState::IdleModes {
            self.bar.revealed.set(true);
        }
        if idle {
            self.nav_apply(NavEvent::Collapsed);
        }
        for w in &self.bar.panel_parts {
            w.set_visible(!idle);
        }
        self.mode_icon.set_pixel_size(if idle { 24 } else { 28 });
        if idle {
            self.window.add_css_class("idle");
        } else {
            self.window.remove_css_class("idle");
        }
        self.apply_size(self.config.borrow().zoom.clamp(ZOOM_MIN, ZOOM_MAX));
    }

    fn apply_size(&self, zoom: f64) {
        let (w, h) = if self.layout.get() != LayoutState::Panel { (PILL_W, PILL_H) } else { (BASE_W, BASE_H) };
        let (w, h) = ((w as f64 * zoom) as i32, (h as f64 * zoom) as i32);
        // The size request is only a minimum, so the default size must follow too: a
        // non-resizable window keeps its default height otherwise.
        let set = {
            let win = self.window.clone();
            move |height: i32| {
                win.set_size_request(w, height);
                win.set_default_size(w, height);
            }
        };
        if let Some(a) = self.bar.size_anim.borrow_mut().take() {
            a.pause();
        }
        let from = self.window.height();
        if from <= 0 || from == h || !self.window.is_visible() {
            set(h);
            return;
        }
        // Grow or shrink between the pill and the Panel in 200 ms (libadwaita honours
        // `gtk-enable-animations`, so this snaps when animations are off).
        let target = adw::CallbackAnimationTarget::new(move |v| set(v.round() as i32));
        let anim = adw::TimedAnimation::new(&self.window, from as f64, h as f64, 200, target);
        anim.set_easing(adw::Easing::EaseOutCubic);
        anim.play();
        *self.bar.size_anim.borrow_mut() = Some(anim);
    }

    // ---- state persistence ---------------------------------------------------

    fn save_settings(self: &Rc<Self>) {
        if let Err(e) = self.settings.borrow().save(&self.store) {
            self.toast(&format!("Could not save settings: {e}"), None);
        }
    }

    fn save_frecency(&self) {
        let _ = self.frecency.borrow().save(&self.store);
    }

    /// Fetch currency rates off the UI thread (cache-aware, honours config.currency_rates).
    fn refresh_currencies(self: &Rc<Self>) {
        if let Some(t) = self.search_state.last_currency.get() {
            if t.elapsed() < Duration::from_secs(1800) {
                return;
            }
        }
        self.search_state.last_currency.set(Some(std::time::Instant::now()));
        self.loading_begin();
        let (tx, rx) = std::sync::mpsc::channel::<bool>();
        std::thread::spawn(move || {
            let _ = tx.send(crate::calculator::refresh_currencies());
        });
        let s = self.clone();
        glib::timeout_add_local(Duration::from_millis(100), move || match rx.try_recv() {
            Ok(fresh) => {
                s.loading_end();
                if fresh && s.window.is_visible() {
                    s.do_search(true);
                }
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(_) => {
                s.loading_end();
                glib::ControlFlow::Break
            }
        });
    }

    /// Probe command states off the UI thread, then refresh the list.
    fn refresh_states(self: &Rc<Self>) {
        if let Some(t) = self.search_state.last_probe.get() {
            if t.elapsed() < Duration::from_secs(5) {
                return;
            }
        }
        self.search_state.last_probe.set(Some(std::time::Instant::now()));
        self.loading_begin();
        let (tx, rx) = std::sync::mpsc::channel::<HashMap<String, bool>>();
        let cmds = self.commands.borrow().clone();
        std::thread::spawn(move || {
            let _ = tx.send(commands::probe_states(&cmds, &commands::system_runner));
        });
        let s = self.clone();
        glib::timeout_add_local(Duration::from_millis(30), move || match rx.try_recv() {
            Ok(map) => {
                *s.search_state.states.borrow_mut() = map;
                s.loading_end();
                if s.window.is_visible() {
                    s.do_search(true);
                }
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(_) => {
                s.loading_end();
                glib::ControlFlow::Break
            }
        });
    }

    // ---- wiring ----------------------------------------------------------------

    fn setup_ui_interactions(self: &Rc<Self>) {
        let s = self.clone();
        self.search_entry.connect_changed(move |_| s.on_search_changed());

        // Window focus out -> hide
        let s = self.clone();
        let focus_ctrl = gtk4::EventControllerFocus::new();
        focus_ctrl.connect_leave(move |_| {
            if !s.may_hide_on_outside_event() {
                return;
            }
            let s = s.clone();
            glib::timeout_add_local(Duration::from_millis(100), move || {
                if s.window.is_visible() && !s.window.is_active() {
                    s.hide_window();
                }
                glib::ControlFlow::Break
            });
        });
        self.window.add_controller(focus_ctrl);

        // Capture phase: see keys before the focused widget can consume them
        let s = self.clone();
        let key_ctrl = gtk4::EventControllerKey::new();
        key_ctrl.set_propagation_phase(gtk4::PropagationPhase::Capture);
        key_ctrl.connect_key_pressed(move |_, keyval, _, state| s.on_key_pressed(keyval, state));
        self.window.add_controller(key_ctrl);

        // Pointer travel reveals the mode buttons (anywhere over the bar).
        let s = self.clone();
        let motion = gtk4::EventControllerMotion::new();
        motion.set_propagation_phase(gtk4::PropagationPhase::Capture);
        motion.connect_motion(move |_, x, y| {
            if !s.bar.global_pointer.get() {
                s.on_pointer(x, y);
            }
        });
        self.window.add_controller(motion);

        let mut child = self.bar.mode_buttons.first_child();
        for (n, _) in BAR_BUTTONS {
            let Some(w) = child else { break };
            child = w.next_sibling();
            if let Some(b) = w.downcast_ref::<gtk4::Button>() {
                let s = self.clone();
                b.connect_clicked(move |_| s.open_bar_mode(n));
            }
        }

        let s = self.clone();
        self.backend.set_on_apps_updated(move || {
            let s = s.clone();
            glib::idle_add_local(move || {
                *s.search_state.search_data.borrow_mut() = None;
                if s.window.is_visible() {
                    s.do_search(true);
                }
                glib::ControlFlow::Break
            });
        });

        self.window.connect_close_request(move |win| {
            win.set_visible(false);
            glib::Propagation::Stop
        });
    }

    fn mode_index(&self) -> usize {
        let cur = self.category.get();
        Category::ALL.iter().position(|c| *c == cur).unwrap_or(0)
    }

    fn mode_name(&self) -> &'static str {
        self.category.get().label()
    }

    /// Switch mode: updates category, glyph and placeholder (no search).
    fn set_mode(&self, cat: Category) {
        self.nav_apply(NavEvent::Typed(false));
        self.category.set(cat);
        set_mode_icon(&self.mode_icon, cat.glyph());
        self.search_entry.set_placeholder_text(Some(cat.placeholder()));

        if let Ok(buttons) = self.category_buttons.try_borrow() {
            for (cid, btn) in buttons.iter() {
                let active = *cid == cat;
                if btn.is_active() != active {
                    btn.set_active(active);
                }
            }
        }
    }

    fn set_category(self: &Rc<Self>, cat: Category) {
        self.goto_mode(cat);
    }

    fn cycle_category(self: &Rc<Self>, step: i32) {
        let n = Category::ALL.len() as i32;
        self.goto_mode(Category::ALL[(self.mode_index() as i32 + step).rem_euclid(n) as usize]);
    }

    fn goto_mode(self: &Rc<Self>, cat: Category) {
        self.leave_view();
        self.set_mode(cat);
        self.nav_apply(NavEvent::ToRoot);
        match cat {
            Category::Windows => {
                self.push_view(View::Windows, "Filter open windows\u{2026}");
            }
            Category::Snippets => {
                self.push_view(View::Snippets, "Search snippets\u{2026}");
            }
            _ => {
                self.do_search(false);
            }
        }
        self.search_entry.grab_focus();
    }

    // ---- keyboard --------------------------------------------------------------------

    fn go_back(self: &Rc<Self>) {
        let o = layout::transition(self.layout.get(), &self.panel_ctx(), LayoutEvent::Esc);
        if o.action == LayoutAction::Close {
            self.window.set_visible(false);
            return;
        }
        // Clearing the query here is a step back, not a typed deletion.
        self.nav_apply(NavEvent::Typed(false));
        if self.dialog_open() {
            self.close_dialog();
        } else if self.panel_open() {
            self.close_panel();
        } else if self.x.view.get() != View::Root {
            self.pop_view();
        } else if self.in_calc_history() {
            self.nav_apply(NavEvent::Back);
            self.search_entry.set_placeholder_text(Some(self.mode_name()));
            self.search_entry.set_text("");
            self.do_search(false);
        } else if self.in_recent() {
            self.nav_apply(NavEvent::Back);
            self.do_search(false);
        } else if self.nav.borrow().dir().is_some() {
            self.nav_apply(NavEvent::Back);
            self.search_entry.set_text("");
            self.search_entry.set_placeholder_text(Some(self.mode_name()));
            self.do_search(false);
        } else if !self.search_entry.text().is_empty() {
            self.search_entry.set_text("");
        }
    }

    /// Run a named action on the selected row if it offers it.
    fn perform_selected(self: &Rc<Self>, action: &str) {
        if let Some(rd) = self.result_view.selected_row() {
            if self.actions_for(&rd).iter().any(|a| a.id == action) {
                self.perform(rd, action, None);
            }
        }
    }
}

fn browse_directory_plain(path_str: &str, filter: &str) -> Vec<RowData> {
    let mut results = Vec::new();
    let p = Path::new(path_str);
    if !p.is_dir() {
        return results;
    }

    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    // Parent navigation
    if p != home && p.parent().is_some() {
        results.push(result_row(
            format!("file://{}", p.parent().unwrap().display()),
            ".. (Parent directory)".to_string(),
            "inode/directory".to_string(),
            p.parent().unwrap().display().to_string(),
            None,
        ));
    }

    if let Ok(entries) = std::fs::read_dir(p) {
        let mut list: Vec<_> = entries.filter_map(Result::ok).collect();
        list.sort_by_key(|e| (e.file_type().map(|t| !t.is_dir()).unwrap_or(true), e.file_name().to_string_lossy().to_lowercase()));

        for entry in list {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            if !filter.is_empty() && !name.to_lowercase().contains(&filter.to_lowercase()) {
                continue;
            }

            let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
            let mime = if is_dir { "inode/directory".to_string() } else { "application/octet-stream".to_string() };

            results.push(result_row(
                format!("file://{}", entry.path().display()),
                name,
                mime,
                entry.path().display().to_string(),
                None,
            ));
        }
    }

    results
}

#[cfg(test)]
#[path = "../tests/window_tests.rs"]
mod tests;

