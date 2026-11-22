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
use crate::search::{SearchBackend, DEFAULT_LIMIT};
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

mod chrome;
mod dialog;
mod extras;
mod panel;
mod toast;
use chrome::*;
use dialog::Dialog;
use extras::{Extras, View};
pub(crate) use panel::primary_title;
use panel::Panel;

type SectionList = Vec<(String, Vec<RowData>)>;

/// One entry of the Ctrl+K panel.
#[derive(Clone, Debug)]
struct UiAction {
    id: String,
    title: String,
    keys: Vec<String>,
    danger: bool,
    argv: Option<Vec<String>>,
}

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
    debounce_id: Rc<RefCell<Option<glib::SourceId>>>,
    search_seq: Rc<Cell<u64>>,
    calc_history: Rc<RefCell<CalcHistory>>,
    last_currency: Cell<Option<std::time::Instant>>,

    // core state
    store: Store,
    settings: RefCell<Settings>,
    frecency: RefCell<Frecency>,
    commands: RefCell<Vec<Command>>,
    /// Apps and their ranker index, rebuilt after the next app reload (`None` = stale).
    search_data: RefCell<Option<Rc<(Vec<AppEntry>, SearchIndex)>>>,
    x: Extras,
    states: RefCell<HashMap<String, bool>>,
    last_probe: Cell<Option<std::time::Instant>>,
    current: RefCell<SectionList>,

    // ui pieces
    panel: Panel,
    dialog: Dialog,
    dialog_backdrop: gtk4::Box,
    more_popover: gtk4::Popover,
    category_buttons: RefCell<HashMap<Category, gtk4::ToggleButton>>,
    loading_bar: gtk4::ProgressBar,
    loading_count: Cell<u32>,
    loading_timer: RefCell<Option<glib::SourceId>>,
    footer_status: gtk4::Label,
    toast_pill: gtk4::Box,
    toast_undo: gtk4::Button,
    toast_cb: RefCell<Option<Rc<dyn Fn()>>>,
    toast_timer: RefCell<Option<glib::SourceId>>,
    zoom_provider: gtk4::CssProvider,

    layout: Cell<LayoutState>,
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

    progress_revealer: gtk4::Revealer,
    progress_spinner: gtk4::Spinner,
    progress_label: gtk4::Label,
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
        zoom_provider.load_from_data(&zoom_css(zoom));
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

        // -- Search header: mode glyph, big light input, `...` disc --
        let search_container = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        search_container.add_css_class("search-header");

        let mode_icon = gtk4::Image::new();
        mode_icon.set_pixel_size(28);
        mode_icon.add_css_class("search-icon");
        set_mode_icon(&mode_icon, "spotlight");

        let search_entry = gtk4::Entry::builder()
            .placeholder_text(Category::All.label())
            .hexpand(true)
            .has_frame(false)
            .valign(gtk4::Align::Center)
            .build();
        search_entry.add_css_class("search-input");

        let more_button = gtk4::Button::new();
        let more_img = gtk4::Image::new();
        more_img.set_pixel_size(22);
        set_mode_icon(&more_img, "more");
        more_button.set_child(Some(&more_img));
        more_button.add_css_class("disc-button");
        more_button.set_valign(gtk4::Align::Center);
        more_button.set_focusable(false);
        more_button.set_tooltip_text(Some("Actions (Ctrl+K)"));

        search_container.append(&mode_icon);
        search_container.append(&search_entry);
        search_container.append(&x.arg_box);
        search_container.append(&more_button);

        let mode_buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, MODE_GAP as i32);
        mode_buttons.add_css_class("mode-buttons");
        mode_buttons.set_margin_start(MODE_GAP as i32);
        let buttons_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideLeft)
            .transition_duration(200)
            .reveal_child(false)
            .child(&mode_buttons)
            .build();
        for (n, tip) in BAR_BUTTONS {
            let img = gtk4::Image::new();
            img.set_pixel_size(26);
            let glyph = shortcuts::mode_id(n).and_then(Category::from_id).map_or("spotlight", Category::glyph);
            set_mode_icon(&img, glyph);
            let b = gtk4::Button::new();
            b.set_child(Some(&img));
            b.add_css_class("mode-button");
            b.set_focusable(false);
            b.set_valign(gtk4::Align::Center);
            b.set_tooltip_text(Some(tip));
            mode_buttons.append(&b);
        }
        outer.append(&buttons_revealer);
        main_box.append(&search_container);

        // -- Category bar (tabs) --
        let category_bar = gtk4::Box::new(gtk4::Orientation::Horizontal, 5);
        category_bar.add_css_class("category-bar");
        let mut cat_buttons = HashMap::new();

        for cat in Category::ALL {
            let btn = gtk4::ToggleButton::builder()
                .label(cat.label())
                .active(cat == Category::All)
                .focusable(false)
                .build();
            btn.add_css_class("category-btn");
            category_bar.append(&btn);
            cat_buttons.insert(cat, btn);
        }

        let category_scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::External)
            .vscrollbar_policy(gtk4::PolicyType::Never)
            .propagate_natural_width(false)
            .propagate_natural_height(false)
            .hexpand(true)
            .build();
        category_scroll.add_css_class("category-scroll");
        category_scroll.set_child(Some(&category_bar));
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

        // -- Toast: floating pill over results list --
        let toast_pill = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        toast_pill.add_css_class("toast-pill");
        toast_pill.set_halign(gtk4::Align::Center);
        toast_pill.set_valign(gtk4::Align::End);
        toast_pill.set_margin_bottom(18);
        toast_pill.set_visible(false);
        let footer_status = gtk4::Label::builder().xalign(0.0).build();
        footer_status.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        footer_status.set_max_width_chars(48);
        let toast_undo = gtk4::Button::with_label("Undo");
        toast_undo.add_css_class("toast-undo");
        toast_undo.set_visible(false);
        toast_undo.set_focusable(false);
        toast_pill.append(&footer_status);
        toast_pill.append(&toast_undo);

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

        // -- Action panel --
        let panel_root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        panel_root.add_css_class("action-panel");
        panel_root.set_halign(gtk4::Align::End);
        panel_root.set_valign(gtk4::Align::End);
        panel_root.set_visible(false);
        let panel_list = gtk4::ListBox::builder().selection_mode(gtk4::SelectionMode::Single).build();
        panel_list.add_css_class("action-list");
        let filter_wrap = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        filter_wrap.add_css_class("action-filter-wrap");
        let panel_entry = gtk4::Entry::builder()
            .placeholder_text("Search for actions\u{2026}")
            .hexpand(true)
            .has_frame(false)
            .build();
        panel_entry.add_css_class("action-filter");
        filter_wrap.append(&panel_entry);
        panel_root.append(&panel_list);
        panel_root.append(&filter_wrap);

        // -- Dialog card (confirm / alias editor) --
        let dialog_root = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
        dialog_root.add_css_class("dialog-card");
        dialog_root.set_halign(gtk4::Align::Center);
        dialog_root.set_valign(gtk4::Align::Center);
        dialog_root.set_visible(false);
        let dialog_title = gtk4::Label::builder().xalign(0.0).wrap(true).build();
        dialog_title.add_css_class("dialog-title");
        let dialog_body = gtk4::Label::builder().xalign(0.0).wrap(true).build();
        dialog_body.add_css_class("dialog-body");
        let dialog_entry = gtk4::Entry::new();
        let dialog_buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        dialog_buttons.set_halign(gtk4::Align::End);
        let dialog_cancel = gtk4::Button::with_label("Cancel");
        dialog_cancel.add_css_class("dialog-cancel-btn");
        let dialog_ok = gtk4::Button::with_label("OK");
        dialog_ok.add_css_class("dialog-ok-btn");
        dialog_ok.add_css_class("suggested-action");
        dialog_buttons.append(&dialog_cancel);
        dialog_buttons.append(&dialog_ok);
        dialog_root.append(&dialog_title);
        dialog_root.append(&dialog_body);
        dialog_root.append(&x.form);
        dialog_root.append(&dialog_entry);
        dialog_root.append(&dialog_buttons);

        let dialog_backdrop = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        dialog_backdrop.add_css_class("dialog-backdrop");
        dialog_backdrop.set_hexpand(true);
        dialog_backdrop.set_vexpand(true);
        dialog_backdrop.set_visible(false);

        let click_backdrop = gtk4::GestureClick::new();
        dialog_backdrop.add_controller(click_backdrop.clone());

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
        overlay.add_overlay(&panel_root);
        overlay.add_overlay(&dialog_backdrop);
        overlay.add_overlay(&dialog_root);
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
            debounce_id: Rc::new(RefCell::new(None)),
            search_seq: Rc::new(Cell::new(0)),
            calc_history: Rc::new(RefCell::new(CalcHistory::open())),
            last_currency: Cell::new(None),
            store,
            settings: RefCell::new(settings),
            frecency: RefCell::new(frecency),
            commands: RefCell::new(Vec::new()),
            search_data: RefCell::new(None),
            x,
            states: RefCell::new(HashMap::new()),
            last_probe: Cell::new(None),
            current: RefCell::new(Vec::new()),
            panel: Panel {
                root: panel_root,
                list: panel_list,
                entry: panel_entry,
                all: RefCell::new(Vec::new()),
                shown: RefCell::new(Vec::new()),
                target: RefCell::new(None),
            },
            dialog: Dialog {
                root: dialog_root,
                title: dialog_title,
                body: dialog_body,
                entry: dialog_entry,
                ok: dialog_ok.clone(),
                on_ok: RefCell::new(None),
            },
            dialog_backdrop,
            more_popover,
            category_buttons: RefCell::new(cat_buttons),
            loading_bar,
            loading_count: Cell::new(0),
            loading_timer: RefCell::new(None),
            footer_status,
            toast_pill,
            toast_undo,
            toast_cb: RefCell::new(None),
            toast_timer: RefCell::new(None),
            zoom_provider,
            layout: Cell::new(LayoutState::Idle),
            mode_buttons,
            buttons_revealer,
            size_anim: RefCell::new(None),
            global_pointer: Cell::new(false),
            hypr_poll: RefCell::new(None),
            revealed: Cell::new(false),
            pointer_origin: Cell::new(None),
            panel_parts,
            progress_revealer,
            progress_spinner,
            progress_label,
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
        spotlight.toast_undo.connect_clicked(move |_| {
            let cb = s.toast_cb.borrow_mut().take();
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

    /// Same two checks the focus-loss handler has always used: the `--keep-open` debug
    /// escape hatch and the `hide_on_focus_loss` config toggle. Any path that wants to hide
    /// the window because of an "outside" interaction (focus-loss today, click-outside via
    /// the Shell extension) must gate on this first, so the two paths can never drift apart.
    fn may_hide_on_outside_event(&self) -> bool {
        let keep_open = std::env::args().any(|a| a == "--keep-open");
        !keep_open && self.config.borrow().hide_on_focus_loss
    }

    /// The one place that actually hides the window and cleans up. Called by both the
    /// focus-loss handler and the click-outside D-Bus request from the Shell extension.
    fn hide_window(self: &Rc<Self>) {
        self.window.set_visible(false);
    }

    /// Entry point for the Shell extension's "click landed outside the window" report
    /// (see `src/control_service.rs`). Routes through the exact same gating and hide logic
    /// as the existing focus-loss path — no separate hide implementation.
    pub fn handle_outside_click(self: &Rc<Self>) {
        if self.window.is_visible() && self.may_hide_on_outside_event() {
            self.hide_window();
        }
    }

    /// Pointer motion anywhere on the screen, reported by the Shell extension (global
    /// coordinates). Reveals the mode buttons from any pointer travel, not only over the bar.
    pub fn handle_global_pointer(&self, x: f64, y: f64) {
        if self.window.is_visible() {
            self.global_pointer.set(true);
            self.on_pointer(x, y);
        }
    }

    /// Hyprland has no extension that reports the pointer, so ask its IPC socket for the cursor
    /// position every 30 ms while the window is visible and still in the Idle bar state. The
    /// timer stops itself when the window hides. GNOME gets the position from the Shell
    /// extension instead (`handle_global_pointer`), so this only runs under Hyprland.
    fn start_hypr_pointer_poll(self: &Rc<Self>) {
        let Some(path) = hypr_socket_path() else { return };
        if self.hypr_poll.borrow().is_some() {
            return;
        }
        let s = self.clone();
        let id = glib::timeout_add_local(Duration::from_millis(30), move || {
            if !s.window.is_visible() {
                s.hypr_poll.borrow_mut().take();
                return glib::ControlFlow::Break;
            }
            if s.layout.get() == LayoutState::Idle {
                if let Some((x, y)) = hypr_cursor_pos(&path) {
                    s.handle_global_pointer(x, y);
                }
            }
            glib::ControlFlow::Continue
        });
        *self.hypr_poll.borrow_mut() = Some(id);
    }

    pub fn present_with_focus(self: &Rc<Self>) {
        // Follow the Liquid Glass switch from Pulsar Settings.
        if pulsar_liquid_glass_enabled() {
            self.window.add_css_class("liquid-glass");
        } else {
            self.window.remove_css_class("liquid-glass");
        }

        self.revealed.set(false);
        self.pointer_origin.set(None);
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
        self.apply_layout(if self.revealed.get() { LayoutState::IdleModes } else { LayoutState::Idle });
        self.set_mode(Category::All);
        self.search_entry.set_text("");
    }

    /// Show only the header row in a bar state, the full window in the Panel.
    fn apply_layout(&self, state: LayoutState) {
        self.layout.set(state);
        let idle = state != LayoutState::Panel;
        self.buttons_revealer.set_reveal_child(state == LayoutState::IdleModes);
        if state == LayoutState::IdleModes {
            self.revealed.set(true);
        }
        if idle {
            self.nav_apply(NavEvent::Collapsed);
        }
        for w in &self.panel_parts {
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
        if let Some(a) = self.size_anim.borrow_mut().take() {
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
        *self.size_anim.borrow_mut() = Some(anim);
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
        if let Some(t) = self.last_currency.get() {
            if t.elapsed() < Duration::from_secs(1800) {
                return;
            }
        }
        self.last_currency.set(Some(std::time::Instant::now()));
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
        if let Some(t) = self.last_probe.get() {
            if t.elapsed() < Duration::from_secs(5) {
                return;
            }
        }
        self.last_probe.set(Some(std::time::Instant::now()));
        self.loading_begin();
        let (tx, rx) = std::sync::mpsc::channel::<HashMap<String, bool>>();
        let cmds = self.commands.borrow().clone();
        std::thread::spawn(move || {
            let _ = tx.send(commands::probe_states(&cmds, &commands::system_runner));
        });
        let s = self.clone();
        glib::timeout_add_local(Duration::from_millis(30), move || match rx.try_recv() {
            Ok(map) => {
                *s.states.borrow_mut() = map;
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
            if !s.global_pointer.get() {
                s.on_pointer(x, y);
            }
        });
        self.window.add_controller(motion);

        let mut child = self.mode_buttons.first_child();
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
                *s.search_data.borrow_mut() = None;
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

    fn on_search_changed(self: &Rc<Self>) {
        self.nav_apply(NavEvent::QueryChanged);
        let empty = self.search_entry.text().is_empty();
        if self.layout.get() != LayoutState::Panel && !empty {
            self.apply_layout(LayoutState::Panel);
        }
        if empty {
            let typed = std::mem::replace(&mut self.nav.borrow_mut().typed, false);
            let ctx = LayoutCtx { query_typed: typed, ..self.panel_ctx() };
            let o = layout::transition(self.layout.get(), &ctx, LayoutEvent::QueryEmptied);
            if o.action == LayoutAction::Collapse {
                self.reset_to_root();
                return;
            }
        } else {
            self.nav_apply(NavEvent::Typed(true));
        }
        if let Some(id) = self.debounce_id.borrow_mut().take() {
            id.remove();
        }
        let s = self.clone();
        let id = glib::timeout_add_local(Duration::from_millis(DEBOUNCE_MS), move || {
            // The source has fired: drop the stale handle so a later
            // on_search_changed never removes an already-destroyed source
            *s.debounce_id.borrow_mut() = None;
            s.do_search(false);
            glib::ControlFlow::Break
        });
        *self.debounce_id.borrow_mut() = Some(id);
    }

    // ---- searching ---------------------------------------------------------------

    fn render(&self, keep: bool) {
        let sections = self.current.borrow().clone();
        self.result_view.set_sections(sections, keep);
    }

    /// Append backend rows to a named section (creating it) and re-render.
    fn append_rows(&self, seq: u64, title: &str, rows: Vec<RowData>) {
        if self.search_seq.get() != seq || rows.is_empty() {
            return;
        }
        {
            let mut cur = self.current.borrow_mut();
            let have: std::collections::HashSet<String> =
                cur.iter().flat_map(|(_, r)| r.iter().map(|x| x.row.id.clone())).collect();
            let rows: Vec<RowData> = rows.into_iter().filter(|r| !have.contains(&r.row.id)).collect();
            if rows.is_empty() {
                return;
            }
            match cur.iter_mut().find(|(t, _)| t == title) {
                Some((_, existing)) => existing.extend(rows),
                None => cur.push((title.to_string(), rows)),
            }
        }
        self.render(true);
    }

    fn core_sections(&self, query: &str, extra: &[Row]) -> Vec<Section> {
        let commands = self.commands.borrow();
        let data = self
            .search_data
            .borrow_mut()
            .get_or_insert_with(|| {
                let apps: Vec<AppEntry> = self.backend.apps_snapshot().iter().map(app_entry).collect();
                let index = SearchIndex::new(&apps, &commands);
                Rc::new((apps, index))
            })
            .clone();
        let settings = self.settings.borrow();
        let frecency = self.frecency.borrow();
        let states = self.states.borrow();
        let ctx = Ctx {
            apps: &data.0,
            commands: &commands,
            states: &states,
            settings: &settings,
            frecency: &frecency,
            now: now_secs(),
            extra,
        };
        RootSearch::sections_indexed(query, &ctx, &data.1)
    }

    fn do_search(self: &Rc<Self>, keep: bool) {
        // A bar state draws no results: skip the root search (also from state probes and app updates).
        if self.layout.get() != LayoutState::Panel {
            return;
        }
        let seq = self.search_seq.get() + 1;
        self.search_seq.set(seq);

        let query = self.search_entry.text().to_string();
        let category = self.category.get();
        if self.search_view(&query, keep) {
            return;
        }

        // If in media/document category and query empty and not browsing, start browsing the category dir
        if query.trim().is_empty() && self.nav.borrow().is_root() {
            if let Some(d) = category.browse_dir() {
                if d.is_dir() {
                    self.nav_apply(NavEvent::OpenDir(d.display().to_string()));
                }
            }
        }

        let browsing_dir = self.nav.borrow().dir().map(str::to_string);
        if let Some(dir_path) = browsing_dir.as_ref() {
            let rows = browse_directory_plain(dir_path, query.trim());
            *self.current.borrow_mut() = vec![("Files".to_string(), rows)];
            self.render(keep);
            return;
        }

        let is_empty = query.trim().is_empty();
        if self.in_calc_history() {
            let rows = CalcHistoryProvider::new(self.calc_history.clone()).instant(&Query::new(&query));
            *self.current.borrow_mut() = vec![("Calculator History".to_string(), self.attach_all(rows, &None))];
            self.render(keep);
            return;
        }
        let sections: SectionList = match category {
            Category::All | Category::Applications | Category::Actions => {
                let show_extra = category == Category::All && !is_empty;
                let calc = if show_extra { Calculator::calculate(&query) } else { None };
                let q = Query::new(&query);
                let mut registry = Registry::default();
                registry.register(Box::new(CalculatorProvider));
                if show_extra {
                    registry.register(Box::new(self.clipboard_provider(3)));
                }
                let mut extra = registry.instant(&q);
                if !show_extra {
                    extra.retain(|r| r.kind == RowKind::Command);
                }
                let run = {
                    let (this, q) = (self.clone(), query.clone());
                    Rc::new(RootRun::new(move || this.core_sections(&q, &extra)))
                };
                let mut secs: SectionList = run
                    .sections()
                    .iter()
                    .map(|s| {
                        (s.title.clone(), self.attach_all(s.rows.clone(), &calc))
                    })
                    .collect();
                if show_extra {
                    self.add_extra_rows(&query, &mut secs);
                }
                if calc.is_some() {
                    secs.retain(|(t, _)| t != "Search the web");
                }
                if category != Category::All {
                    let provider: Box<dyn Provider> = if category == Category::Actions {
                        Box::new(CommandsProvider::new(run.clone()))
                    } else {
                        Box::new(AppsProvider::new(run.clone()))
                    };
                    let mut registry = Registry::default();
                    registry.register(provider);
                    let ids: std::collections::HashSet<String> =
                        registry.instant(&Query::new(&query)).into_iter().map(|r| r.id).collect();
                    for (_, rows) in secs.iter_mut() {
                        rows.retain(|r| ids.contains(&r.row.id));
                    }
                    secs.retain(|(t, rows)| !rows.is_empty() && t != "Favorites" && t != "Suggestions");
                }
                secs
            }
            Category::Clipboard => {
                let rows = self.clipboard_provider(usize::MAX).instant(&Query::new(&query));
                vec![("Clipboard".to_string(), self.attach_all(rows, &None))]
            }
            Category::Web => {
                let rows = self.backend.search_web(&query);
                vec![("Web".to_string(), rows)]
            }
            Category::Calc => {
                let calc_query = if is_empty { "1+1" } else { &query };
                let mut rows = CalculatorProvider.instant(&Query::new(calc_query));
                rows.retain(|r| r.kind == RowKind::Calc);
                rows.extend(CalcHistoryProvider::new(self.calc_history.clone()).instant(&Query::new(&query)));
                vec![("Calculator".to_string(), self.attach_all(rows, &Calculator::calculate(calc_query)))]
            }
            _ => Vec::new(),
        };
        *self.current.borrow_mut() = sections;
        self.render(keep);

        // Async Nautilus file search (appended to Results, or its own section)
        let needs_files = (!is_empty || category.is_file_tab()) && category.needs_file_search();
        let target = category.section_title();
        if needs_files {
            self.loading_begin();
            let s = self.clone();
            self.backend.search_async(&query, category, DEFAULT_LIMIT, move |file_results| {
                s.loading_end();
                s.append_rows(seq, target, file_results);
            });
        }

        // Async browser history (web category, or web results within "all")
        let wants_history = category == Category::Web || (category == Category::All && !is_empty);
        if wants_history {
            let limit = if category == Category::Web { 6 } else { 3 };
            self.loading_begin();
            let s = self.clone();
            let title = if category == Category::Web { "Web" } else { "Results" };
            self.backend.search_browser_history_async(&query, limit, move |hist| {
                s.loading_end();
                s.append_rows(seq, title, hist);
            });
        }
    }

    fn clipboard_provider(&self, limit: usize) -> ClipboardProvider {
        let mgr = self.backend.clipboard();
        let items = move || mgr.as_ref().map(|m| m.get_items().into_iter().map(|i| i.text).collect()).unwrap_or_default();
        ClipboardProvider::new(items, limit)
    }

    /// Rows plus what activating them needs: the clipboard payload, or the calculator result
    /// (`live` for a calculation, the saved row for a history entry).
    fn attach_all(&self, rows: Vec<Row>, live: &Option<CalcResult>) -> Vec<RowData> {
        rows.into_iter()
            .map(|row| {
                let payload = (row.kind == RowKind::Clipboard)
                    .then(|| Payload { url: row.id.clone(), mime: clipboard_provider::MIME.to_string(), app: None });
                let calc = match (row.kind, row.id.strip_prefix("calchist:")) {
                    (RowKind::Calc, Some(id)) => {
                        id.parse().ok().and_then(|id| self.calc_history.borrow().get(id)).map(|h| history_result(&h))
                    }
                    (RowKind::Calc, None) => live.clone(),
                    _ => None,
                };
                RowData { row, payload, calc }
            })
            .collect()
    }

    fn show_history(self: &Rc<Self>) {
        let rows: Vec<RowData> = self
            .settings
            .borrow()
            .history()
            .iter()
            .map(|q| {
                let r = Row::simple(RowKind::Fallback, &format!("history:{q}"), q, "", "document-open-recent-symbolic");
                RowData { row: r, payload: None, calc: None }
            })
            .collect();
        if rows.is_empty() {
            return;
        }
        self.nav_apply(NavEvent::ShowRecent);
        *self.current.borrow_mut() = vec![("Recent searches".to_string(), rows)];
        self.render(false);
    }

    fn enter_calc_history(self: &Rc<Self>) {
        self.close_panel();
        self.nav_apply(NavEvent::ShowCalcHistory);
        self.search_entry.set_placeholder_text(Some("Search calculator history\u{2026}"));
        self.search_entry.set_text("");
        self.do_search(false);
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

    // ---- actions -------------------------------------------------------------------

    fn actions_for(&self, rd: &RowData) -> Vec<UiAction> {
        let mk = |id: &str, title: &str, keys: Vec<String>, danger: bool, argv: Option<Vec<String>>| UiAction {
            id: id.into(),
            title: title.into(),
            keys,
            danger,
            argv,
        };
        let mut out = Vec::new();
        if let Some(c) = &rd.calc {
            let hist = rd.row.id.starts_with("calchist:");
            let _ = c;
            out.push(mk("calc_copy", "Copy Answer", shortcuts::label(Act::Run(0)), false, None));
            out.push(mk("calc_paste", "Paste Answer", shortcuts::label(Act::Run(1)), false, None));
            out.push(mk("calc_full", "Copy Question = Answer", shortcuts::label(Act::CopyFull), false, None));
            if hist {
                out.push(mk("calc_del", "Delete Entry", shortcuts::label(Act::DelItem), true, None));
                out.push(mk("calc_delall", "Delete All History", shortcuts::label(Act::DelAll), true, None));
            } else {
                out.push(mk("calc_history", "Calculator History", Vec::new(), false, None));
            }
            return out;
        }
        if let Some(a) = self.extra_actions(rd) {
            return a;
        }
        let primary = rd.row.actions.first().cloned();
        let primary_id = primary.as_ref().map(|a| a.id.clone()).unwrap_or_else(|| "open".into());
        out.push(mk(
            &primary_id,
            &primary_title(rd),
            shortcuts::label(Act::Run(0)),
            false,
            primary.and_then(|a| a.argv),
        ));
        match rd.row.kind {
            RowKind::App | RowKind::Command => {
                out.push(mk("copy_name", "Copy Name", shortcuts::label(Act::Run(1)), false, None));
            }
            RowKind::File => {
                out.push(mk("copy_path", "Copy Path", shortcuts::label(Act::Run(1)), false, None));
                if rd.payload.as_ref().map_or(false, |p| p.url.starts_with("file://")) {
                    out.push(mk("open_folder", "Open Containing Folder", shortcuts::label(Act::Run(2)), false, None));
                }
            }
            RowKind::Clipboard => {
                out.push(mk("copy_name", "Copy Text", shortcuts::label(Act::Run(1)), false, None));
            }
            _ => {}
        }
        for a in rd.row.actions.iter().skip(1) {
            let keys = match a.id.as_str() {
                "favorite" => shortcuts::label(Act::Favorite),
                "disable" => shortcuts::label(Act::Disable),
                _ => Vec::new(),
            };
            out.push(mk(&a.id, &a.title, keys, false, a.argv.clone()));
        }
        if rd.row.kind == RowKind::App {
            let desktop = rd.row.id.strip_prefix("app:").unwrap_or(&rd.row.id);
            let pinned = get_favorites().iter().any(|f| f == desktop);
            out.push(mk("pin", if pinned { "Unpin from Dock" } else { "Pin to Dock" }, Vec::new(), false, None));
            out.push(mk("uninstall", "Uninstall", Vec::new(), true, None));
        }
        out
    }

    fn run_action_index(self: &Rc<Self>, n: usize) {
        if self.in_recent() {
            if let Some(rd) = self.result_view.selected_row() {
                self.perform(rd, "open", None);
            }
            return;
        }
        let Some(rd) = self.result_view.selected_row() else { return };
        if let Some(a) = self.actions_for(&rd).get(n).cloned() {
            self.perform(rd, &a.id, a.argv);
        }
    }

    fn perform(self: &Rc<Self>, rd: RowData, action: &str, argv: Option<Vec<String>>) {
        let id = rd.row.id.clone();
        if id == "calc:history" && matches!(action, "open" | "run") {
            self.enter_calc_history();
            return;
        }
        if self.perform_extra(&rd, action) {
            return;
        }
        match action {
            "calc_copy" | "calc_paste" | "calc_full" => {
                let Some(c) = rd.calc.clone() else { return };
                if !self.in_calc_history() {
                    self.calc_history.borrow_mut().add(&c);
                }
                match action {
                    "calc_copy" => self.copy(&c.answer),
                    "calc_full" => self.copy(&format!("{} = {}", c.question, c.answer)),
                    _ => {
                        self.window.set_visible(false);
                        if let Some(mgr) = self.backend_clipboard() {
                            mgr.paste_clip(&c.answer);
                        } else {
                            self.copy(&c.answer);
                        }
                        return;
                    }
                }
                self.window.set_visible(false);
            }
            "calc_history" => self.enter_calc_history(),
            "calc_del" => {
                if let Some(n) = id.strip_prefix("calchist:").and_then(|n| n.parse::<u64>().ok()) {
                    self.calc_history.borrow_mut().delete(n);
                    self.toast("Deleted from history", None);
                    self.do_search(true);
                }
            }
            "calc_delall" => {
                let s = self.clone();
                self.show_dialog(
                    "Delete all calculator history?",
                    "This cannot be undone.",
                    None,
                    "Delete All",
                    true,
                    Box::new(move |_| {
                        s.calc_history.borrow_mut().delete_all();
                        s.toast("History cleared", None);
                        s.do_search(true);
                    }),
                );
            }
            "open" | "run" | "open_url" => {
                if let Some(q) = id.strip_prefix("history:") {
                    self.nav_apply(NavEvent::QueryChanged);
                    self.search_entry.set_text(q);
                    self.search_entry.set_position(-1);
                    return;
                }
                if rd.row.confirm {
                    let s = self.clone();
                    let title = rd.row.title.clone();
                    self.show_dialog(
                        &format!("{title}?"),
                        "This action can interrupt your session. Continue?",
                        None,
                        "Run",
                        true,
                        Box::new(move |_| s.launch(rd.clone(), argv.clone())),
                    );
                } else {
                    self.launch(rd, argv);
                }
            }
            "favorite" => {
                let added = self.settings.borrow_mut().toggle_favorite(&id);
                self.save_settings();
                let s = self.clone();
                let id2 = id.clone();
                self.toast(
                    &format!("{} {}", if added { "Added to Favorites:" } else { "Removed from Favorites:" }, rd.row.title),
                    Some(Rc::new(move || {
                        s.settings.borrow_mut().toggle_favorite(&id2);
                        s.save_settings();
                        s.do_search(true);
                    })),
                );
                self.do_search(true);
            }
            "alias" => {
                let current = self.settings.borrow().alias(&id).unwrap_or("").to_string();
                let s = self.clone();
                self.show_dialog(
                    &format!("Alias for {}", rd.row.title),
                    "Type the alias to put this item first. Leave empty to remove it.",
                    Some(current),
                    "Save",
                    false,
                    Box::new(move |text| {
                        let res = s.settings.borrow_mut().set_alias(&id, &text);
                        match res {
                            Ok(()) => {
                                s.save_settings();
                                s.toast("Alias saved", None);
                                s.do_search(true);
                            }
                            Err(e) => s.toast(&e, None),
                        }
                    }),
                );
            }
            "disable" => {
                self.settings.borrow_mut().disable(&id);
                self.save_settings();
                let s = self.clone();
                let id2 = id.clone();
                self.toast(
                    &format!("Disabled {}", rd.row.title),
                    Some(Rc::new(move || {
                        s.settings.borrow_mut().enable(&id2);
                        s.save_settings();
                        s.do_search(true);
                    })),
                );
                self.do_search(true);
            }
            "enable" => {
                self.settings.borrow_mut().enable(&id);
                self.save_settings();
                self.do_search(true);
            }
            "copy_name" => {
                self.copy(&rd.row.title);
                self.toast("Copied", None);
            }
            "copy_path" => {
                let text = rd.payload.as_ref().map(|p| p.url.trim_start_matches("file://").to_string()).unwrap_or_default();
                self.copy(&text);
                self.toast("Copied path", None);
            }
            "open_folder" => {
                if let Some(p) = &rd.payload {
                    let clean = p.url.trim_start_matches("file://");
                    if let Some(parent) = Path::new(clean).parent() {
                        open_file(&format!("file://{}", parent.display()));
                    }
                }
                self.window.set_visible(false);
            }
            "pin" => {
                let desktop = id.strip_prefix("app:").unwrap_or(&id).to_string();
                let mut favs = get_favorites();
                let msg = if favs.contains(&desktop) {
                    favs.retain(|x| x != &desktop);
                    "Unpinned from dock"
                } else {
                    favs.push(desktop);
                    "Pinned to dock"
                };
                set_favorites(&favs);
                self.toast(msg, None);
            }
            "uninstall" => {
                let s = self.clone();
                let name = rd.row.title.clone();
                self.show_dialog(
                    &format!("Uninstall {name}?"),
                    "The package will be removed from the system.",
                    None,
                    "Uninstall",
                    true,
                    Box::new(move |_| s.uninstall(id.strip_prefix("app:").unwrap_or(&id).to_string(), name.clone())),
                );
            }
            _ => {}
        }
    }

    fn copy(&self, text: &str) {
        if let Some(display) = gdk::Display::default() {
            display.clipboard().set_text(text);
        }
    }

    /// Run the primary action: launch / open / paste / browse.
    fn launch(self: &Rc<Self>, rd: RowData, argv: Option<Vec<String>>) {
        let query = self.search_entry.text().to_string();
        let mut hide = true;
        let mut ok = true;
        match rd.row.kind {
            RowKind::App => {
                let desktop = rd.row.id.strip_prefix("app:").unwrap_or(&rd.row.id);
                ok = open_file(&format!("app://{desktop}"));
            }
            RowKind::Command | RowKind::Fallback => {
                if let Some(argv) = argv.as_ref().filter(|a| !a.is_empty()) {
                    match std::process::Command::new(&argv[0]).args(&argv[1..]).spawn() {
                        Ok(mut child) => {
                            std::thread::spawn(move || {
                                let _ = child.wait();
                            });
                        }
                        Err(e) => {
                            self.toast(&format!("Could not run {}: {e}", argv[0]), None);
                            return;
                        }
                    }
                } else {
                    ok = false;
                }
            }
            RowKind::Calc | RowKind::Clipboard | RowKind::File => {
                if let Some(res) = rd.payload.clone() {
                    hide = self.open_result(&res);
                }
            }
        }
        if !ok {
            self.toast(&format!("Could not open {}", rd.row.title), None);
            return;
        }
        if matches!(rd.row.kind, RowKind::App | RowKind::Command) {
            self.frecency.borrow_mut().record_launch(&rd.row.id, &query, now_secs());
            self.save_frecency();
        }
        if !query.trim().is_empty() && hide {
            self.settings.borrow_mut().push_history(&query);
            self.save_settings();
        }
        if hide {
            self.window.set_visible(false);
        }
    }

    /// Returns true when the window should hide afterwards.
    fn open_result(self: &Rc<Self>, res: &Payload) -> bool {
        if let Some(val) = res.url.strip_prefix("calc://") {
            self.copy(val);
            return true;
        }
        if let Some(idx_str) = res.url.strip_prefix("clipboard://") {
            if let (Some(mgr), Ok(idx)) = (self.backend_clipboard(), idx_str.parse::<usize>()) {
                if let Some(text) = mgr.get_clip_by_index(idx) {
                    self.window.set_visible(false);
                    mgr.paste_clip(&text);
                }
            }
            return true;
        }
        let clean_path = res.url.trim_start_matches("file://");
        if Path::new(clean_path).is_dir() && (res.mime == "inode/directory" || res.mime == "folder") {
            self.nav_apply(NavEvent::OpenDir(clean_path.to_string()));
            self.search_entry.set_text("");
            self.search_entry.set_placeholder_text(Some(&format!("Browsing: {}", clean_path)));
            // set_text only emits 'changed' when the text differs
            self.search_entry.emit_by_name::<()>("changed", &[]);
            return false;
        }
        open_file(&res.url);
        true
    }

    fn backend_clipboard(&self) -> Option<ClipboardManager> {
        self.backend.clipboard()
    }

    fn uninstall(self: &Rc<Self>, desktop_id: String, app_name: String) {
        self.progress_label.set_label(&format!("Uninstalling {}...", app_name));
        self.progress_spinner.start();
        self.progress_revealer.set_reveal_child(true);

        let (sender, receiver) = std::sync::mpsc::channel::<(bool, String)>();
        let s = self.clone();
        glib::timeout_add_local(Duration::from_millis(50), move || match receiver.try_recv() {
            Ok((success, message)) => {
                s.progress_spinner.stop();
                let (text, delay) = if success {
                    (format!("\u{2713} {} uninstalled successfully", app_name), 1800)
                } else {
                    s.progress_label.set_tooltip_text(Some(&message));
                    (format!("\u{2717} {}", clean_uninstall_message(&message, &app_name)), 4000)
                };
                s.progress_label.set_label(&text);
                let rev = s.progress_revealer.clone();
                glib::timeout_add_local(Duration::from_millis(delay), move || {
                    rev.set_reveal_child(false);
                    glib::ControlFlow::Break
                });
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(_) => glib::ControlFlow::Break,
        });

        std::thread::spawn(move || {
            let mut cmd = std::process::Command::new("pkm");
            cmd.args(["--uninstall", &desktop_id]);
            cmd.env("APPINSTALL_SUDO", "pkexec");
            let output = cmd.output();
            let success = output.as_ref().map_or(false, |o| o.status.success());
            let message = output.map_or_else(
                |e| e.to_string(),
                |o| String::from_utf8_lossy(&[o.stdout, o.stderr].concat()).into_owned(),
            );
            let _ = sender.send((success, message));
        });
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

    fn on_key_pressed(self: &Rc<Self>, keyval: gdk::Key, state: gdk::ModifierType) -> glib::Propagation {
        let name = keyval.to_lower().name().map(|n| n.to_lowercase()).unwrap_or_default();
        let mods = Mods {
            ctrl: state.contains(gdk::ModifierType::CONTROL_MASK),
            shift: state.contains(gdk::ModifierType::SHIFT_MASK),
            alt: state.contains(gdk::ModifierType::ALT_MASK),
        };
        let act = shortcuts::lookup(&name, mods);

        if self.dialog_open() {
            if act == Some(Act::Back) || keyval == gdk::Key::Escape {
                self.close_dialog();
                return glib::Propagation::Stop;
            }
            if (keyval == gdk::Key::Return || keyval == gdk::Key::KP_Enter) && !mods.ctrl && !mods.alt && !mods.shift {
                self.accept_dialog();
                return glib::Propagation::Stop;
            }
            return glib::Propagation::Proceed;
        }

        if self.panel_open() {
            match act {
                Some(Act::Up) => self.panel_step(-1),
                Some(Act::Down) => self.panel_step(1),
                Some(Act::Run(_)) => self.panel_activate(),
                Some(Act::Back) | Some(Act::ActionPanel) => self.close_panel(),
                _ => return glib::Propagation::Proceed,
            }
            return glib::Propagation::Stop;
        }

        if self.layout.get() != LayoutState::Panel {
            if let Some(p) = self.bar_key(keyval, mods, act) {
                return p;
            }
        }

        if let Some(p) = self.emoji_key(&name, mods) {
            return p;
        }
        if let Some(p) = self.arg_key(&name, mods) {
            return p;
        }

        // Enter on result: immediately run action index 0
        if (keyval == gdk::Key::Return || keyval == gdk::Key::KP_Enter || keyval == gdk::Key::ISO_Enter)
            && !mods.ctrl && !mods.alt && !mods.shift
        {
            if self.result_view.selected_row().is_some() {
                self.run_action_index(0);
                return glib::Propagation::Stop;
            }
        }

        // Down / Up arrows
        if keyval == gdk::Key::Down && !mods.ctrl && !mods.alt && !mods.shift {
            self.result_view.step(1);
            return glib::Propagation::Stop;
        }
        if keyval == gdk::Key::Up && !mods.ctrl && !mods.alt && !mods.shift {
            if !self.result_view.step(-1)
                && self.search_entry.text().is_empty()
                && self.nav.borrow().is_root()
            {
                self.show_history();
            }
            return glib::Propagation::Stop;
        }

        // Left / Right arrows when search entry is empty: cycle category tabs!
        if keyval == gdk::Key::Left && self.search_entry.text().is_empty() && !mods.ctrl && !mods.alt {
            self.cycle_category(-1);
            return glib::Propagation::Stop;
        }
        if keyval == gdk::Key::Right && self.search_entry.text().is_empty() && !mods.ctrl && !mods.alt {
            self.cycle_category(1);
            return glib::Propagation::Stop;
        }

        if let Some(act) = act {
            return self.handle_act(act);
        }

        // Backspace on empty: ascend directory
        if keyval == gdk::Key::BackSpace && self.search_entry.text().is_empty() {
            let before = self.nav.borrow().dir().map(str::to_string);
            self.nav_apply(NavEvent::ParentDir);
            let after = self.nav.borrow().dir().map(str::to_string);
            if let Some(parent) = after.filter(|p| Some(p) != before.as_ref()) {
                self.search_entry.set_placeholder_text(Some(&format!("Browsing: {}", parent)));
                self.do_search(false);
                return glib::Propagation::Stop;
            }
        }

        // Typing while a list row has focus: redirect into the search entry
        if !self.search_entry.has_focus() && !self.arg_has_focus() && !mods.ctrl && !mods.alt && !state.contains(gdk::ModifierType::SUPER_MASK) {
            if let Some(ch) = keyval.to_unicode() {
                if ch.is_alphanumeric() || ch.is_ascii_punctuation() || ch == ' ' {
                    self.search_entry.grab_focus();
                    let mut text = self.search_entry.text().to_string();
                    text.push(ch);
                    self.search_entry.set_text(&text);
                    self.search_entry.set_position(-1);
                    return glib::Propagation::Stop;
                }
            } else if keyval == gdk::Key::BackSpace {
                self.search_entry.grab_focus();
                let mut text = self.search_entry.text().to_string();
                text.pop();
                self.search_entry.set_text(&text);
                self.search_entry.set_position(-1);
                return glib::Propagation::Stop;
            }
        }

        glib::Propagation::Proceed
    }

    fn layout_ctx(&self) -> LayoutCtx {
        LayoutCtx { revealed: self.revealed.get(), pointer_origin: self.pointer_origin.get(), ..LayoutCtx::default() }
    }

    fn on_pointer(&self, x: f64, y: f64) {
        let o = layout::transition(self.layout.get(), &self.layout_ctx(), LayoutEvent::Pointer { x, y });
        if o.action == LayoutAction::RecordPointer {
            self.pointer_origin.set(Some((x, y)));
        }
        if o.state != self.layout.get() {
            self.apply_layout(o.state);
        }
    }

    /// Open the Panel in mode `n` (1-based, as Alt+n) with an empty query.
    fn open_bar_mode(self: &Rc<Self>, n: usize) {
        let Some(cat) = shortcuts::mode_id(n).and_then(Category::from_id) else { return };
        self.apply_layout(LayoutState::Panel);
        self.set_category(cat);
        self.search_entry.grab_focus();
    }

    /// Keys in a bar state (Idle, IdleModes): expand to the Panel as the layout module says.
    /// Left and Right do nothing there. `None` leaves the key to the normal handling.
    fn bar_key(self: &Rc<Self>, keyval: gdk::Key, mods: Mods, act: Option<Act>) -> Option<glib::Propagation> {
        if matches!(keyval, gdk::Key::Left | gdk::Key::Right) && !mods.ctrl && !mods.alt {
            return Some(glib::Propagation::Stop);
        }
        if let Some(step @ (Act::CatNext | Act::CatPrev)) = act {
            let o = layout::transition(self.layout.get(), &self.layout_ctx(), LayoutEvent::Tab);
            if o.state != self.layout.get() {
                self.apply_layout(o.state);
            } else {
                self.apply_layout(LayoutState::Panel);
                self.cycle_category(if step == Act::CatNext { 1 } else { -1 });
            }
            return Some(glib::Propagation::Stop);
        }
        let event = bar_event(keyval, mods, act)?;
        match layout::transition(self.layout.get(), &self.layout_ctx(), event).action {
            LayoutAction::ShowHome => {
                self.apply_layout(LayoutState::Panel);
                self.nav_apply(NavEvent::PanelOpened(Opened::Home));
                self.do_search(false);
            }
            LayoutAction::ShowHistory => {
                self.apply_layout(LayoutState::Panel);
                self.nav_apply(NavEvent::PanelOpened(Opened::History));
                if self.settings.borrow().history().is_empty() {
                    self.do_search(false);
                } else {
                    self.show_history();
                }
            }
            LayoutAction::SwitchMode(_) => {
                let Some(Act::Mode(n)) = act else { return None };
                self.open_bar_mode(n);
            }
            _ => return None,
        }
        Some(glib::Propagation::Stop)
    }

    fn handle_act(self: &Rc<Self>, act: Act) -> glib::Propagation {
        let stop = glib::Propagation::Stop;
        match act {
            Act::ActionPanel => self.open_more_menu(),
            Act::Run(n) => {
                if self.result_view.selected_row().is_none() {
                    return glib::Propagation::Proceed;
                }
                self.run_action_index(n);
            }
            Act::Quick(n) => {
                if self.result_view.select_nth_row(n) {
                    self.run_action_index(0);
                }
            }
            Act::Up => {
                if !self.result_view.step(-1)
                    && self.search_entry.text().is_empty()
                    && self.nav.borrow().is_root()
                {
                    self.show_history();
                }
            }
            Act::Down => {
                self.result_view.step(1);
            }
            Act::PageUp => {
                self.result_view.step(-8);
            }
            Act::PageDown => {
                self.result_view.step(8);
            }
            Act::SectionPrev => self.result_view.jump_section(false),
            Act::SectionNext => self.result_view.jump_section(true),
            Act::Back => self.go_back(),
            Act::Root => {
                // Collapse and the old reset both land on the bar: reset_to_root follows the revealed flag.
                self.reset_to_root();
                self.search_entry.grab_focus();
            }
            Act::Favorite => self.perform_selected("favorite"),
            Act::Disable => self.perform_selected("disable"),
            Act::CopyFull => self.perform_selected("calc_full"),
            Act::DelItem => self.perform_selected("calc_del"),
            Act::DelAll => self.perform_selected("calc_delall"),
            Act::FavUp | Act::FavDown => {
                if let Some(rd) = self.result_view.selected_row() {
                    if rd.row.favorite {
                        self.settings.borrow_mut().move_favorite(&rd.row.id, if act == Act::FavUp { -1 } else { 1 });
                        self.save_settings();
                        self.do_search(true);
                    }
                }
            }
            Act::ZoomIn => self.set_zoom(self.config.borrow().zoom + 0.1),
            Act::ZoomOut => self.set_zoom(self.config.borrow().zoom - 0.1),
            Act::ZoomReset => self.set_zoom(1.0),
            Act::Quit => {
                if let Some(app) = self.window.application() {
                    app.quit();
                }
            }
            Act::CatNext => self.cycle_category(1),
            Act::CatPrev => self.cycle_category(-1),
            Act::Mode(n) => {
                if let Some(cat) = shortcuts::mode_id(n).and_then(Category::from_id) {
                    self.set_category(cat);
                }
            }
        }
        stop
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

/// Layout event for a key pressed in a bar state: Down, Up, or Alt+1..4.
fn bar_event(keyval: gdk::Key, mods: Mods, act: Option<Act>) -> Option<LayoutEvent> {
    if let Some(Act::Mode(n)) = act {
        let mode = [BarMode::Applications, BarMode::Files, BarMode::Actions, BarMode::Clipboard].get(n.checked_sub(1)?)?;
        return Some(LayoutEvent::Mode(*mode));
    }
    if mods.ctrl || mods.alt || mods.shift {
        return None;
    }
    match keyval {
        gdk::Key::Down => Some(LayoutEvent::Down),
        gdk::Key::Up => Some(LayoutEvent::Up),
        _ => None,
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

            let is_dir = entry.file_type().map_or(false, |t| t.is_dir());
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

/// Path of Hyprland's command socket, when running under Hyprland.
fn hypr_socket_path() -> Option<std::path::PathBuf> {
    let sig = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    let run = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(std::path::PathBuf::from(run).join("hypr").join(sig).join(".socket.sock"))
}

/// Cursor position from Hyprland (`cursorpos` replies "x, y").
fn hypr_cursor_pos(socket: &Path) -> Option<(f64, f64)> {
    use std::io::{Read, Write};
    let mut stream = std::os::unix::net::UnixStream::connect(socket).ok()?;
    stream.set_read_timeout(Some(Duration::from_millis(50))).ok()?;
    stream.write_all(b"cursorpos").ok()?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply).ok()?;
    parse_cursor_pos(&reply)
}

fn parse_cursor_pos(reply: &str) -> Option<(f64, f64)> {
    let (x, y) = reply.trim().split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}
