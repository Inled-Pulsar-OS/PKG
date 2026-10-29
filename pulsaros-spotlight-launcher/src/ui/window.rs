use crate::calculator::{CalcHistory, CalcResult, Calculator};
use crate::clipboard::ClipboardManager;
use crate::config::SpotlightConfig;
use crate::core::commands::{self, Command};
use crate::core::frecency::Frecency;
use crate::core::root_search::{AppEntry, Ctx, Row, RowKind, RootSearch};
use crate::core::settings::Settings;
use crate::core::shortcuts::{self, Act, Mods};
use crate::core::store::Store;
use crate::search::{SearchBackend, SearchResult, DEFAULT_LIMIT};
use crate::ui::results::{get_favorites, set_favorites, ResultView, RowData};
use crate::utils::{get_icon_dir, get_local_icon_dir, open_file};
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

mod extras;
use extras::{Extras, View};

/// (category id, mode name, glyph icon).
const CATEGORIES: &[(&str, &str, &str)] = &[
    ("all", "All", "spotlight"),
    ("applications", "Apps", "applications"),
    ("documents", "Docs", "files"),
    ("images", "Images", "files"),
    ("audio", "Music", "files"),
    ("video", "Videos", "files"),
    ("clipboard", "Clip", "clipboard"),
    ("actions", "Actions", "actions"),
    ("windows", "Windows", "actions"),
    ("snippets", "Snippets", "actions"),
    ("calc", "Calc", "spotlight"),
    ("web", "Web", "spotlight"),
];

const DEBOUNCE_MS: u64 = 80;
const BASE_W: i32 = 712;
const BASE_H: i32 = 580;
const ZOOM_MIN: f64 = 0.8;
const ZOOM_MAX: f64 = 1.6;

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

struct Panel {
    root: gtk4::Box,
    list: gtk4::ListBox,
    entry: gtk4::Entry,
    all: RefCell<Vec<UiAction>>,
    shown: RefCell<Vec<UiAction>>,
    target: RefCell<Option<RowData>>,
}

struct Dialog {
    root: gtk4::Box,
    title: gtk4::Label,
    body: gtk4::Label,
    entry: gtk4::Entry,
    ok: gtk4::Button,
    on_ok: RefCell<Option<Box<dyn Fn(String)>>>,
}

pub struct SpotlightWindow {
    window: gtk4::ApplicationWindow,
    search_entry: gtk4::Entry,
    mode_icon: gtk4::Image,
    result_view: Rc<ResultView>,
    config: Rc<RefCell<SpotlightConfig>>,
    backend: SearchBackend,
    category: RefCell<String>,
    current_dir: Rc<RefCell<Option<String>>>,
    debounce_id: Rc<RefCell<Option<glib::SourceId>>>,
    search_seq: Rc<Cell<u64>>,
    history_mode: Cell<bool>,
    calc_hist_mode: Cell<bool>,
    calc_history: RefCell<CalcHistory>,
    last_currency: Cell<Option<std::time::Instant>>,

    // core state
    store: Store,
    settings: RefCell<Settings>,
    frecency: RefCell<Frecency>,
    commands: RefCell<Vec<Command>>,
    x: Extras,
    states: RefCell<HashMap<String, bool>>,
    last_probe: Cell<Option<std::time::Instant>>,
    current: RefCell<SectionList>,

    // ui pieces
    panel: Panel,
    dialog: Dialog,
    dialog_backdrop: gtk4::Box,
    more_popover: gtk4::Popover,
    category_buttons: RefCell<HashMap<String, gtk4::ToggleButton>>,
    loading_bar: gtk4::ProgressBar,
    loading_count: Cell<u32>,
    loading_timer: RefCell<Option<glib::SourceId>>,
    footer_status: gtk4::Label,
    toast_pill: gtk4::Box,
    toast_undo: gtk4::Button,
    toast_cb: RefCell<Option<Rc<dyn Fn()>>>,
    toast_timer: RefCell<Option<glib::SourceId>>,
    zoom_provider: gtk4::CssProvider,

    indexing_revealer: gtk4::Revealer,
    indexing_label: gtk4::Label,
    indexing_pbar: gtk4::ProgressBar,
    indexing_timer_id: Rc<RefCell<Option<glib::SourceId>>>,

    progress_revealer: gtk4::Revealer,
    progress_spinner: gtk4::Spinner,
    progress_label: gtk4::Label,
}

fn pulsar_liquid_glass_enabled() -> bool {
    let Some(src) = gtk4::gio::SettingsSchemaSource::default() else { return true };
    if src.lookup("org.gnome.shell", true).is_none() {
        return true;
    }
    let settings = gtk4::gio::Settings::new("org.gnome.shell");
    let exts = settings.strv("enabled-extensions");
    exts.iter().any(|uuid| {
        let u = uuid.as_str();
        u == "liquid-glass@thinkingcoding1231.gmail.com"
            || u == "blur-my-shell@aunetx"
            || u.contains("blur")
            || u.contains("glass")
    }) || true
}

/// Load `data/icons/<name>` from the install dirs, falling back to the source tree.
fn icon_file(name: &str) -> Option<PathBuf> {
    [get_local_icon_dir(), get_icon_dir(), PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/data/icons"))]
        .into_iter()
        .map(|d| d.join(name))
        .find(|f| f.exists())
}

fn set_mode_icon(img: &gtk4::Image, glyph: &str) {
    match icon_file(&format!("mode-{glyph}-symbolic.svg")) {
        Some(f) => img.set_from_gicon(&gtk4::gio::FileIcon::new(&gtk4::gio::File::for_path(f))),
        None => img.set_icon_name(Some(if glyph == "more" { "view-more-symbolic" } else { "system-search-symbolic" })),
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn keycap(text: &str) -> gtk4::Label {
    let l = gtk4::Label::new(Some(text));
    l.add_css_class("keycap");
    l
}

fn keycap_row(keys: &[String]) -> gtk4::Box {
    let b = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    for k in keys {
        b.append(&keycap(k));
    }
    b
}

/// Title of the primary action for the panel and the selected-row hint.
pub(crate) fn primary_title(r: &RowData) -> String {
    for (prefix, title) in [("dmenu:", "Select"), ("snippet:", "Paste Snippet"), ("window:", "Switch to Window"), ("script:", "Run Script")] {
        if r.row.id.starts_with(prefix) {
            return title.into();
        }
    }
    match r.row.kind {
        RowKind::App => "Open Application".into(),
        RowKind::Command => "Run".into(),
        RowKind::Clipboard => "Paste".into(),
        RowKind::Calc => "Copy Answer".into(),
        RowKind::File => match &r.payload {
            Some(p) if p.mime == "inode/directory" || p.mime == "folder" => "Open Folder".into(),
            Some(p) if p.url.starts_with("http") => "Open in Browser".into(),
            _ => "Open".into(),
        },
        RowKind::Fallback => {
            if r.row.id.starts_with("history:") {
                "Search Again".into()
            } else {
                r.row.actions.first().map(|a| a.title.clone()).unwrap_or_else(|| "Open".into())
            }
        }
    }
}

fn row_from_result(res: &SearchResult) -> RowData {
    let (kind, id, icon) = if let Some(v) = res.url.strip_prefix("calc://") {
        (RowKind::Calc, format!("calc:{v}"), "accessories-calculator-symbolic")
    } else if res.url.starts_with("clipboard://") {
        (RowKind::Clipboard, res.url.clone(), "edit-paste-symbolic")
    } else {
        (RowKind::File, res.url.clone(), "text-x-generic")
    };
    let mut sub = res.snippet.clone();
    if sub.is_empty() {
        sub = res.url.trim_start_matches("file://").to_string();
    }
    RowData { row: Row::simple(kind, &id, &res.title, &sub, icon), payload: Some(res.clone()), calc: None }
}

fn calc_row(c: &CalcResult, id: String) -> RowData {
    RowData {
        row: Row::simple(RowKind::Calc, &id, &c.answer, &c.question, "accessories-calculator-symbolic"),
        payload: None,
        calc: Some(c.clone()),
    }
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

fn zoom_css(z: f64) -> String {
    let p = |v: f64| (v * z).round();
    format!(
        "window.spotlight-window .spotlight-main {{ font-size: {}px; }}
         window.spotlight-window entry.search-input, window.spotlight-window entry.search-input > text {{ font-size: {}px; }}
         window.spotlight-window .result-row {{ min-height: {}px; }}
         window.spotlight-window .result-title {{ font-size: {}px; }}
         window.spotlight-window .result-subtitle, window.spotlight-window .result-kind {{ font-size: {}px; }}
         window.spotlight-window .section-header {{ font-size: {}px; }}
         window.spotlight-window .icon-tile {{ min-width: {}px; min-height: {}px; }}
         window.spotlight-window .result-icon {{ -gtk-icon-size: {}px; }}
         window.spotlight-window .search-header {{ min-height: {}px; }}",
        p(13.0),
        p(30.0),
        p(56.0),
        p(17.0),
        p(13.0),
        p(12.0),
        p(44.0),
        p(44.0),
        p(40.0),
        p(76.0)
    )
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
        window.set_child(Some(&main_box));

        // -- Search header: mode glyph, big light input, `...` disc --
        let search_container = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        search_container.add_css_class("search-header");

        let mode_icon = gtk4::Image::new();
        mode_icon.set_pixel_size(28);
        mode_icon.add_css_class("search-icon");
        set_mode_icon(&mode_icon, "spotlight");

        let search_entry = gtk4::Entry::builder()
            .placeholder_text(CATEGORIES[0].1)
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
        main_box.append(&search_container);

        // -- Category bar (tabs) --
        let category_bar = gtk4::Box::new(gtk4::Orientation::Horizontal, 5);
        category_bar.add_css_class("category-bar");
        let mut cat_buttons = HashMap::new();

        for &(cat_id, cat_label, _) in CATEGORIES {
            let btn = gtk4::ToggleButton::builder()
                .label(cat_label)
                .active(cat_id == "all")
                .focusable(false)
                .build();
            btn.add_css_class("category-btn");
            category_bar.append(&btn);
            cat_buttons.insert(cat_id.to_string(), btn);
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

        // -- Indexing banner --
        let indexing_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideDown)
            .transition_duration(200)
            .reveal_child(false)
            .build();
        let indexing_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        indexing_box.add_css_class("indexing-status-bar");
        let indexing_spinner = gtk4::Spinner::new();
        indexing_spinner.start();
        let indexing_label = gtk4::Label::builder()
            .label("Indexing files in background...")
            .xalign(0.0)
            .hexpand(true)
            .build();
        let indexing_pbar = gtk4::ProgressBar::builder()
            .valign(gtk4::Align::Center)
            .width_request(100)
            .height_request(4)
            .build();
        indexing_box.append(&indexing_spinner);
        indexing_box.append(&indexing_label);
        indexing_box.append(&indexing_pbar);
        indexing_revealer.set_child(Some(&indexing_box));

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
        main_box.append(&indexing_revealer);

        let settings = Settings::load(&store);
        let frecency = Frecency::load(&store);

        let spotlight = Rc::new(Self {
            window,
            search_entry,
            mode_icon,
            result_view,
            config: config_rc,
            backend,
            category: RefCell::new("all".to_string()),
            current_dir: Rc::new(RefCell::new(None)),
            debounce_id: Rc::new(RefCell::new(None)),
            search_seq: Rc::new(Cell::new(0)),
            history_mode: Cell::new(false),
            calc_hist_mode: Cell::new(false),
            calc_history: RefCell::new(CalcHistory::open()),
            last_currency: Cell::new(None),
            store,
            settings: RefCell::new(settings),
            frecency: RefCell::new(frecency),
            commands: RefCell::new(Vec::new()),
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
            indexing_revealer,
            indexing_label,
            indexing_pbar,
            indexing_timer_id: Rc::new(RefCell::new(None)),
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
            let cid = cat_id.clone();
            let btn_c = btn.clone();
            btn.connect_toggled(move |_| {
                if btn_c.is_active() {
                    s.set_category_by_id(&cid);
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

        self.reset_to_root();
        self.window.present();
        self.search_entry.grab_focus();
        self.start_indexing_timer();
        self.refresh_states();
        self.refresh_currencies();
        self.refresh_extras();
    }

    fn reset_to_root(self: &Rc<Self>) {
        self.close_panel();
        self.close_dialog();
        self.leave_view();
        *self.current_dir.borrow_mut() = None;
        self.history_mode.set(false);
        self.calc_hist_mode.set(false);
        *self.category.borrow_mut() = "all".to_string();
        self.set_mode(0);
        self.search_entry.set_text("");
        self.do_search(false);
    }

    // ---- loading bar / toast -------------------------------------------------

    fn loading_begin(self: &Rc<Self>) {
        let n = self.loading_count.get() + 1;
        self.loading_count.set(n);
        if n == 1 {
            self.loading_bar.set_opacity(1.0);
            let bar = self.loading_bar.clone();
            let id = glib::timeout_add_local(Duration::from_millis(50), move || {
                bar.pulse();
                glib::ControlFlow::Continue
            });
            *self.loading_timer.borrow_mut() = Some(id);
        }
    }

    fn loading_end(&self) {
        let n = self.loading_count.get().saturating_sub(1);
        self.loading_count.set(n);
        if n == 0 {
            if let Some(id) = self.loading_timer.borrow_mut().take() {
                id.remove();
            }
            self.loading_bar.set_opacity(0.0);
            self.loading_bar.set_fraction(0.0);
        }
    }

    fn toast(self: &Rc<Self>, msg: &str, undo: Option<Rc<dyn Fn()>>) {
        self.footer_status.set_label(msg);
        self.toast_pill.set_visible(true);
        self.toast_undo.set_visible(undo.is_some());
        *self.toast_cb.borrow_mut() = undo;
        if let Some(id) = self.toast_timer.borrow_mut().take() {
            id.remove();
        }
        let s = self.clone();
        let id = glib::timeout_add_local(Duration::from_millis(4000), move || {
            *s.toast_timer.borrow_mut() = None;
            s.reset_toast();
            glib::ControlFlow::Break
        });
        *self.toast_timer.borrow_mut() = Some(id);
    }

    fn reset_toast(&self) {
        self.toast_pill.set_visible(false);
        self.toast_undo.set_visible(false);
        *self.toast_cb.borrow_mut() = None;
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

    // ---- indexing banner -----------------------------------------------------

    fn start_indexing_timer(self: &Rc<Self>) {
        if self.indexing_timer_id.borrow().is_none() && self.window.is_visible() {
            let self_c = self.clone();
            let id = glib::timeout_add_seconds_local(3, move || {
                if !self_c.window.is_visible() {
                    // Drop the handle and stop ticking; removing the source
                    // from within its own callback is not safe
                    *self_c.indexing_timer_id.borrow_mut() = None;
                    return glib::ControlFlow::Break;
                }
                self_c.check_indexing_status();
                glib::ControlFlow::Continue
            });
            *self.indexing_timer_id.borrow_mut() = Some(id);
        }
    }

    fn stop_indexing_timer(&self) {
        if let Some(id) = self.indexing_timer_id.borrow_mut().take() {
            id.remove();
        }
    }

    fn check_indexing_status(&self) {
        let (sender, receiver) = std::sync::mpsc::channel::<(bool, String, f64)>();

        let ind_revealer = self.indexing_revealer.clone();
        let ind_label = self.indexing_label.clone();
        let ind_pbar = self.indexing_pbar.clone();

        glib::timeout_add_local(Duration::from_millis(5), move || match receiver.try_recv() {
            Ok((is_indexing, _status, progress)) => {
                if is_indexing {
                    let percent = (progress * 100.0) as i32;
                    let label_text = if percent > 0 {
                        format!("Indexing files in background... ({}%)", percent)
                    } else {
                        "Indexing files in background...".to_string()
                    };
                    ind_label.set_text(&label_text);
                    ind_pbar.set_fraction(progress.clamp(0.0, 1.0));
                    ind_revealer.set_reveal_child(true);
                } else {
                    ind_revealer.set_reveal_child(false);
                }
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
        });

        std::thread::spawn(move || {
            let res = crate::search::get_indexing_status_plain();
            let _ = sender.send(res);
        });
    }

    // ---- wiring ----------------------------------------------------------------

    fn setup_ui_interactions(self: &Rc<Self>) {
        let s = self.clone();
        self.search_entry.connect_changed(move |_| s.on_search_changed());

        // Window focus out -> hide
        let s = self.clone();
        let keep_open = std::env::args().any(|a| a == "--keep-open");
        let focus_ctrl = gtk4::EventControllerFocus::new();
        focus_ctrl.connect_leave(move |_| {
            if keep_open || !s.config.borrow().hide_on_focus_loss {
                return;
            }
            let s = s.clone();
            glib::timeout_add_local(Duration::from_millis(100), move || {
                if s.window.is_visible() && !s.window.is_active() {
                    s.window.set_visible(false);
                    s.stop_indexing_timer();
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

        let s = self.clone();
        self.backend.set_on_apps_updated(move || {
            let s = s.clone();
            glib::idle_add_local(move || {
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
        self.history_mode.set(false);
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

    fn core_sections(&self, query: &str, extra: &[Row]) -> SectionList {
        let apps: Vec<AppEntry> = self.backend.apps_snapshot().iter().map(app_entry).collect();
        let settings = self.settings.borrow();
        let frecency = self.frecency.borrow();
        let states = self.states.borrow();
        let commands = self.commands.borrow();
        let ctx = Ctx {
            apps: &apps,
            commands: &commands,
            states: &states,
            settings: &settings,
            frecency: &frecency,
            now: now_secs(),
            extra,
        };
        RootSearch::sections(query, &ctx)
            .into_iter()
            .map(|s| (s.title, s.rows.into_iter().map(|row| RowData { row, payload: None, calc: None }).collect()))
            .collect()
    }

    fn do_search(self: &Rc<Self>, keep: bool) {
        let seq = self.search_seq.get() + 1;
        self.search_seq.set(seq);

        let query = self.search_entry.text().to_string();
        let category = self.category.borrow().clone();
        if self.search_view(&query, keep) {
            return;
        }

        // If in media/document category and query empty and not browsing, start browsing the category dir
        if ["documents", "images", "audio", "video"].contains(&category.as_str()) && query.trim().is_empty() && self.current_dir.borrow().is_none() {
            let target_dir = match category.as_str() {
                "documents" => dirs::document_dir(),
                "images" => dirs::picture_dir(),
                "audio" => dirs::audio_dir(),
                "video" => dirs::video_dir(),
                _ => None,
            }.or_else(|| dirs::home_dir());
            if let Some(d) = target_dir {
                if d.is_dir() {
                    *self.current_dir.borrow_mut() = Some(d.display().to_string());
                }
            }
        }

        let browsing_dir = self.current_dir.borrow().clone();
        if let Some(dir_path) = browsing_dir.as_ref() {
            let rows: Vec<RowData> =
                browse_directory_plain(dir_path, query.trim()).iter().map(row_from_result).collect();
            *self.current.borrow_mut() = vec![("Files".to_string(), rows)];
            self.render(keep);
            return;
        }

        let is_empty = query.trim().is_empty();
        if self.calc_hist_mode.get() {
            let rows: Vec<RowData> = self
                .calc_history
                .borrow()
                .search(&query)
                .iter()
                .map(|h| {
                    calc_row(
                        &CalcResult {
                            question: h.question.clone(),
                            answer: h.answer.clone(),
                            caption_question: h.caption_question.clone(),
                            caption_answer: h.caption_answer.clone(),
                        },
                        format!("calchist:{}", h.id),
                    )
                })
                .collect();
            *self.current.borrow_mut() = vec![("Calculator History".to_string(), rows)];
            self.render(keep);
            return;
        }
        let sections: SectionList = match category.as_str() {
            "all" | "applications" | "actions" => {
                let found: Vec<SearchResult> = if category == "all" && !is_empty {
                    self.backend
                        .search_instant(&query, "all", DEFAULT_LIMIT)
                        .into_iter()
                        .filter(|r| r.app.is_none() && r.url.starts_with("clipboard://"))
                        .collect()
                } else {
                    Vec::new()
                };
                let calc = if category == "all" && !is_empty { Calculator::calculate(&query) } else { None };
                let ql = query.trim().to_lowercase();
                let mut extra: Vec<Row> = Vec::new();
                if let Some(c) = &calc {
                    extra.push(calc_row(c, format!("calc:{}", c.answer)).row);
                }
                if ql.len() >= 3 && "calculator history".contains(&ql) {
                    extra.push(Row::simple(RowKind::Command, "calc:history", "Calculator History", "Calculator", "accessories-calculator-symbolic"));
                }
                extra.extend(found.iter().map(|r| row_from_result(r).row));
                let payloads: HashMap<String, SearchResult> =
                    found.into_iter().map(|r| (row_from_result(&r).row.id, r)).collect();
                let mut secs = self.core_sections(&query, &extra);
                if category == "all" && !is_empty {
                    self.add_extra_rows(&query, &mut secs);
                }
                for (_, rows) in secs.iter_mut() {
                    for r in rows.iter_mut() {
                        r.payload = payloads.get(&r.row.id).cloned();
                        if r.row.kind == RowKind::Calc {
                            r.calc = calc.clone();
                        }
                    }
                }
                if calc.is_some() {
                    secs.retain(|(t, _)| t != "Search the web");
                }
                if category == "actions" {
                    for (_, rows) in secs.iter_mut() {
                        rows.retain(|r| r.row.kind == RowKind::Command);
                    }
                    secs.retain(|(t, rows)| !rows.is_empty() && t != "Favorites" && t != "Suggestions");
                }
                if category == "applications" {
                    for (_, rows) in secs.iter_mut() {
                        rows.retain(|r| r.row.kind == RowKind::App);
                    }
                    secs.retain(|(t, rows)| !rows.is_empty() && t != "Favorites" && t != "Suggestions");
                }
                secs
            }
            "clipboard" => {
                let rows: Vec<RowData> =
                    self.backend.search_instant(&query, "clipboard", DEFAULT_LIMIT).iter().map(row_from_result).collect();
                vec![("Clipboard".to_string(), rows)]
            }
            "web" => {
                let rows: Vec<RowData> = self.backend.search_web(&query).iter().map(row_from_result).collect();
                vec![("Web".to_string(), rows)]
            }
            "calc" => {
                let mut rows = Vec::new();
                if let Some(c) = Calculator::calculate(if is_empty { "1+1" } else { &query }) {
                    rows.push(calc_row(&c, format!("calc:{}", c.answer)));
                }
                let hist_rows: Vec<RowData> = self.calc_history.borrow().search(&query).iter().map(|h| {
                    calc_row(
                        &CalcResult {
                            question: h.question.clone(),
                            answer: h.answer.clone(),
                            caption_question: h.caption_question.clone(),
                            caption_answer: h.caption_answer.clone(),
                        },
                        format!("calchist:{}", h.id),
                    )
                }).collect();
                rows.extend(hist_rows);
                vec![("Calculator".to_string(), rows)]
            }
            _ => Vec::new(),
        };
        *self.current.borrow_mut() = sections;
        self.render(keep);

        // Async Nautilus file search (appended to Results, or its own section)
        let needs_files = (!is_empty || ["documents", "images", "audio", "video"].contains(&category.as_str()))
            && !["applications", "actions", "clipboard", "web", "calc", "windows", "snippets"].contains(&category.as_str());
        let target = match category.as_str() {
            "documents" => "Documents",
            "images" => "Images",
            "audio" => "Music",
            "video" => "Videos",
            "all" => "Results",
            _ => "Files",
        };
        if needs_files {
            self.loading_begin();
            let s = self.clone();
            self.backend.search_async(&query, &category, DEFAULT_LIMIT, move |file_results| {
                s.loading_end();
                let rows = file_results.iter().map(row_from_result).collect();
                s.append_rows(seq, target, rows);
            });
        }

        // Async browser history (web category, or web results within "all")
        let wants_history = category == "web" || (category == "all" && !is_empty);
        if wants_history {
            let limit = if category == "web" { 6 } else { 3 };
            self.loading_begin();
            let s = self.clone();
            let title = if category == "web" { "Web" } else { "Results" };
            self.backend.search_browser_history_async(&query, limit, move |hist| {
                s.loading_end();
                let rows = hist.iter().map(row_from_result).collect();
                s.append_rows(seq, title, rows);
            });
        }
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
        self.history_mode.set(true);
        *self.current.borrow_mut() = vec![("Recent searches".to_string(), rows)];
        self.render(false);
    }

    fn enter_calc_history(self: &Rc<Self>) {
        self.close_panel();
        self.calc_hist_mode.set(true);
        self.history_mode.set(false);
        self.search_entry.set_placeholder_text(Some("Search calculator history\u{2026}"));
        self.search_entry.set_text("");
        self.do_search(false);
    }

    fn mode_index(&self) -> usize {
        let cur = self.category.borrow().clone();
        CATEGORIES.iter().position(|(id, _, _)| *id == cur).unwrap_or(0)
    }

    fn mode_name(&self) -> &'static str {
        CATEGORIES[self.mode_index()].1
    }

    /// Switch mode: updates category, glyph and placeholder (no search).
    fn set_mode(&self, idx: usize) {
        let (id, name, glyph) = CATEGORIES[idx % CATEGORIES.len()];
        *self.category.borrow_mut() = id.to_string();
        set_mode_icon(&self.mode_icon, glyph);
        let ph = match id {
            "all" => "Spotlight Search...",
            "applications" => "Search Applications...",
            "documents" => "Search Documents...",
            "images" => "Search Images...",
            "audio" => "Search Music...",
            "video" => "Search Videos...",
            "clipboard" => "Search Clipboard...",
            "actions" => "Search Commands & Settings...",
            "windows" => "Filter Open Windows...",
            "snippets" => "Search Snippets...",
            "calc" => "Calculate (e.g. 24*15, $50 in EUR)...",
            "web" => "Search the Web...",
            _ => name,
        };
        self.search_entry.set_placeholder_text(Some(ph));

        if let Ok(buttons) = self.category_buttons.try_borrow() {
            for (cid, btn) in buttons.iter() {
                let active = cid == id;
                if btn.is_active() != active {
                    btn.set_active(active);
                }
            }
        }
    }

    fn set_category_by_id(self: &Rc<Self>, cid: &str) {
        if let Some(pos) = CATEGORIES.iter().position(|(id, _, _)| *id == cid) {
            self.goto_mode(pos);
        }
    }

    fn cycle_category(self: &Rc<Self>, step: i32) {
        let n = CATEGORIES.len() as i32;
        self.goto_mode((self.mode_index() as i32 + step).rem_euclid(n) as usize);
    }

    fn goto_mode(self: &Rc<Self>, idx: usize) {
        let (id, _, _) = CATEGORIES[idx % CATEGORIES.len()];
        self.leave_view();
        self.set_mode(idx);
        *self.current_dir.borrow_mut() = None;
        self.history_mode.set(false);
        self.calc_hist_mode.set(false);
        match id {
            "windows" => {
                self.push_view(View::Windows, "Filter open windows\u{2026}");
            }
            "snippets" => {
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
        if self.history_mode.get() {
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
                if !self.calc_hist_mode.get() {
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
                    self.history_mode.set(false);
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
    fn open_result(self: &Rc<Self>, res: &SearchResult) -> bool {
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
            *self.current_dir.borrow_mut() = Some(clean_path.to_string());
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

    // ---- action panel ----------------------------------------------------------------

    fn panel_open(&self) -> bool {
        self.panel.root.is_visible()
    }

    fn open_more_menu(self: &Rc<Self>) {
        let menu_box = gtk4::Box::new(gtk4::Orientation::Vertical, 3);
        menu_box.add_css_class("popover-menu-box");

        // 1. Actions for currently selected item (if any)
        if let Some(rd) = self.result_view.selected_row() {
            let actions = self.actions_for(&rd);
            if !actions.is_empty() {
                for act in actions.into_iter().take(4) {
                    let btn = gtk4::Button::new();
                    btn.add_css_class("popover-item");
                    let row_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
                    let label = gtk4::Label::builder().label(&act.title).xalign(0.0).hexpand(true).build();
                    row_box.append(&label);
                    if !act.keys.is_empty() {
                        let k = gtk4::Label::builder().label(&act.keys.join("+")).build();
                        k.add_css_class("keycap");
                        row_box.append(&k);
                    }
                    btn.set_child(Some(&row_box));
                    let s = self.clone();
                    let a_id = act.id.clone();
                    let a_argv = act.argv.clone();
                    let rd_c = rd.clone();
                    btn.connect_clicked(move |_| {
                        s.more_popover.popdown();
                        s.perform(rd_c.clone(), &a_id, a_argv.clone());
                    });
                    menu_box.append(&btn);
                }
                let sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
                menu_box.append(&sep);
            }
        }

        // 2. Global tools & options
        let tools: &[(&str, &str, &str)] = &[
            ("ui:quicklink_create", "Create Quicklink", "list-add-symbolic"),
            ("ui:quicklink_manage", "Manage Quicklinks", "emblem-system-symbolic"),
            ("ui:snippet_create", "Create Snippet", "insert-text-symbolic"),
            ("ui:snippets", "Snippets", "edit-paste-symbolic"),
            ("ui:windows", "Open Windows", "preferences-system-windows-symbolic"),
            ("ui:emoji", "Emoji & Symbols", "face-smile-symbolic"),
            ("cmd:settings", "System Settings", "preferences-system-symbolic"),
        ];

        for &(id, label_text, icon_name) in tools {
            let btn = gtk4::Button::new();
            btn.add_css_class("popover-item");
            let row_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            let img = gtk4::Image::from_icon_name(icon_name);
            img.set_pixel_size(16);
            let label = gtk4::Label::builder().label(label_text).xalign(0.0).hexpand(true).build();
            row_box.append(&img);
            row_box.append(&label);
            btn.set_child(Some(&row_box));

            let s = self.clone();
            btn.connect_clicked(move |_| {
                s.more_popover.popdown();
                match id {
                    "ui:quicklink_create" => s.quicklink_form(),
                    "ui:quicklink_manage" => s.quicklink_manage(),
                    "ui:snippet_create" => s.snippet_form(),
                    "ui:snippets" => s.push_view(View::Snippets, "Search snippets\u{2026}"),
                    "ui:windows" => s.push_view(View::Windows, "Filter open windows\u{2026}"),
                    "ui:emoji" => s.open_emoji_panel(),
                    "cmd:settings" => {
                        let _ = std::process::Command::new("gnome-control-center").spawn();
                    }
                    _ => {}
                }
            });
            menu_box.append(&btn);
        }

        self.more_popover.set_child(Some(&menu_box));
        self.more_popover.popup();
    }

    fn open_panel(self: &Rc<Self>) {
        if self.x.view.get() == View::Emoji {
            self.open_emoji_panel();
            return;
        }
        let Some(rd) = self.result_view.selected_row() else { return };
        if self.history_mode.get() {
            return;
        }
        *self.panel.all.borrow_mut() = self.actions_for(&rd);
        *self.panel.target.borrow_mut() = Some(rd);
        self.panel.entry.set_text("");
        self.fill_panel("");
        self.panel.root.set_visible(true);
        self.panel.entry.grab_focus();
    }

    fn close_panel(&self) {
        if self.panel.root.is_visible() {
            self.panel.root.set_visible(false);
            self.search_entry.grab_focus();
        }
    }

    fn fill_panel(&self, filter: &str) {
        let f = filter.trim().to_lowercase();
        while let Some(c) = self.panel.list.first_child() {
            self.panel.list.remove(&c);
        }
        let shown: Vec<UiAction> =
            self.panel.all.borrow().iter().filter(|a| f.is_empty() || a.title.to_lowercase().contains(&f)).cloned().collect();
        for a in &shown {
            let row = gtk4::ListBoxRow::new();
            let b = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
            b.add_css_class("action-item");
            if a.danger {
                b.add_css_class("danger");
            }
            let t = gtk4::Label::builder().label(&a.title).xalign(0.0).hexpand(true).build();
            t.add_css_class("action-title");
            b.append(&t);
            if !a.keys.is_empty() {
                b.append(&keycap_row(&a.keys));
            }
            row.set_child(Some(&b));
            self.panel.list.append(&row);
        }
        *self.panel.shown.borrow_mut() = shown;
        if let Some(first) = self.panel.list.row_at_index(0) {
            self.panel.list.select_row(Some(&first));
        }
    }

    fn panel_step(&self, delta: i32) {
        let n = self.panel.shown.borrow().len() as i32;
        if n == 0 {
            return;
        }
        let cur = self.panel.list.selected_row().map(|r| r.index()).unwrap_or(0);
        let next = (cur + delta).rem_euclid(n);
        if let Some(r) = self.panel.list.row_at_index(next) {
            self.panel.list.select_row(Some(&r));
        }
    }

    fn panel_activate(self: &Rc<Self>) {
        let idx = self.panel.list.selected_row().map(|r| r.index()).unwrap_or(-1);
        let act = self.panel.shown.borrow().get(idx.max(0) as usize).cloned();
        let target = self.panel.target.borrow().clone();
        if let (Some(a), Some(rd)) = (act, target) {
            self.close_panel();
            self.perform(rd, &a.id, a.argv);
        }
    }

    fn setup_panel_events(self: &Rc<Self>) {
        let s = self.clone();
        self.panel.entry.connect_changed(move |e| s.fill_panel(&e.text()));
        let s = self.clone();
        self.panel.list.connect_row_activated(move |_, _| s.panel_activate());
    }

    // ---- dialog --------------------------------------------------------------------

    fn show_dialog(
        self: &Rc<Self>,
        title: &str,
        body: &str,
        entry: Option<String>,
        ok_label: &str,
        destructive: bool,
        on_ok: Box<dyn Fn(String)>,
    ) {
        self.close_panel();
        self.dialog_reset_form();
        self.dialog.title.set_label(title);
        self.dialog.body.set_label(body);
        self.dialog.ok.set_label(ok_label);
        if destructive {
            self.dialog.ok.add_css_class("destructive");
        } else {
            self.dialog.ok.remove_css_class("destructive");
        }
        match entry {
            Some(text) => {
                self.dialog.entry.set_text(&text);
                self.dialog.entry.set_visible(true);
            }
            None => self.dialog.entry.set_visible(false),
        }
        *self.dialog.on_ok.borrow_mut() = Some(on_ok);
        self.x.stack.set_visible(false);
        self.dialog_backdrop.set_visible(true);
        self.dialog.root.set_visible(true);
        if gtk4::prelude::WidgetExt::is_visible(&self.dialog.entry) {
            self.dialog.entry.grab_focus();
            self.dialog.entry.set_position(-1);
        } else {
            self.dialog.ok.grab_focus();
        }
    }

    fn dialog_open(&self) -> bool {
        self.dialog.root.is_visible()
    }

    fn close_dialog(&self) {
        if self.dialog.root.is_visible() {
            self.dialog_backdrop.set_visible(false);
            self.dialog.root.set_visible(false);
            self.x.stack.set_visible(true);
            self.dialog_reset_form();
            *self.dialog.on_ok.borrow_mut() = None;
            self.search_entry.grab_focus();
        }
    }

    fn accept_dialog(self: &Rc<Self>) {
        if self.accept_form() {
            return;
        }
        let text = self.dialog.entry.text().to_string();
        let cb = self.dialog.on_ok.borrow_mut().take();
        self.close_dialog();
        if let Some(cb) = cb {
            cb(text);
        }
    }

    // ---- zoom ------------------------------------------------------------------------

    fn set_zoom(&self, z: f64) {
        let z = ((z * 10.0).round() / 10.0).clamp(ZOOM_MIN, ZOOM_MAX);
        self.zoom_provider.load_from_data(&zoom_css(z));
        self.window.set_size_request((BASE_W as f64 * z) as i32, (BASE_H as f64 * z) as i32);
        let mut c = self.config.borrow_mut();
        c.zoom = z;
        c.save();
    }

    // ---- keyboard --------------------------------------------------------------------

    fn go_back(self: &Rc<Self>) {
        if self.dialog_open() {
            self.close_dialog();
        } else if self.panel_open() {
            self.close_panel();
        } else if self.x.view.get() != View::Root {
            self.pop_view();
        } else if self.calc_hist_mode.get() {
            self.calc_hist_mode.set(false);
            self.search_entry.set_placeholder_text(Some(self.mode_name()));
            self.search_entry.set_text("");
            self.do_search(false);
        } else if self.history_mode.get() {
            self.history_mode.set(false);
            self.do_search(false);
        } else if self.current_dir.borrow().is_some() {
            *self.current_dir.borrow_mut() = None;
            self.search_entry.set_text("");
            self.search_entry.set_placeholder_text(Some(self.mode_name()));
            self.do_search(false);
        } else if !self.search_entry.text().is_empty() {
            self.search_entry.set_text("");
        } else {
            self.window.set_visible(false);
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
                && !self.history_mode.get()
                && !self.calc_hist_mode.get()
                && self.current_dir.borrow().is_none()
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
            let parent = self
                .current_dir
                .borrow()
                .as_ref()
                .and_then(|d| Path::new(d).parent().map(|p| (p.display().to_string(), d.clone())))
                .filter(|(p, d)| p != d);
            if let Some((parent, _)) = parent {
                self.search_entry.set_placeholder_text(Some(&format!("Browsing: {}", parent)));
                *self.current_dir.borrow_mut() = Some(parent);
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
                    && !self.history_mode.get()
                    && !self.calc_hist_mode.get()
                    && self.current_dir.borrow().is_none()
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
            Act::Mode(n) => self.goto_mode(n),
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

fn browse_directory_plain(path_str: &str, filter: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let p = Path::new(path_str);
    if !p.is_dir() {
        return results;
    }

    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    // Parent navigation
    if p != home && p.parent().is_some() {
        results.push(SearchResult {
            url: format!("file://{}", p.parent().unwrap().display()),
            title: ".. (Parent directory)".to_string(),
            mime: "inode/directory".to_string(),
            snippet: p.parent().unwrap().display().to_string(),
            app: None,
        });
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

            results.push(SearchResult {
                url: format!("file://{}", entry.path().display()),
                title: name,
                mime,
                snippet: entry.path().display().to_string(),
                app: None,
            });
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calc_result_becomes_calc_row() {
        let r = row_from_result(&SearchResult {
            url: "calc://4".into(),
            title: "4".into(),
            mime: "application/x-calculator".into(),
            snippet: "2+2".into(),
            app: None,
        });
        assert_eq!(r.row.kind, RowKind::Calc);
        assert_eq!(primary_title(&r), "Copy Answer");
    }

    #[test]
    fn uninstall_message_is_cleaned() {
        assert!(clean_uninstall_message("foo\nerror: bar baz", "X").contains("bar baz"));
        assert!(clean_uninstall_message("unmet dependencies", "X").starts_with("Cannot remove X"));
    }

    #[test]
    fn zoom_css_scales() {
        assert!(zoom_css(1.5).contains("min-height: 84px"));
    }
}
