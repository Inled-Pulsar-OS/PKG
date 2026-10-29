//! Shared application state handed to every module: the store, the catalog,
//! install state, running operations, and hooks for navigation and toasts.
//! Single-threaded (main thread only); background work reports back through it.

use crate::core::i18n;
use crate::core::install::{Action, Installer};
use crate::core::model::{InstallState, Item, Progress};
use crate::core::ports::{CancelToken, CatalogProvider};
use crate::core::prefs::Prefs;
use crate::core::store::Store;
use crate::tr;
use crate::ui::{router::Route, task};
use gtk4 as gtk;
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::Arc;

/// A running install, uninstall, or update.
#[derive(Clone)]
pub struct Op {
    pub action: Action,
    pub fraction: Option<f32>,
    pub cancel: CancelToken,
    /// Waiting for an earlier operation to finish.
    pub queued: bool,
}

/// What changed, so widgets can refresh only what they show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Catalog,
    /// Install states were recomputed for all items.
    States,
    /// The state of one item changed.
    State(String),
    /// An operation started, progressed, or ended for one item.
    Op(String),
    Icons,
}

type Listener = Rc<dyn Fn(&Change)>;
type Navigator = Rc<dyn Fn(Route)>;
type Toaster = Rc<dyn Fn(String)>;
type ErrorPresenter = Rc<dyn Fn(String, String)>;

struct Inner {
    store: Arc<Store>,
    locales: Vec<String>,
    items: RefCell<Arc<Vec<Item>>>,
    announcement: RefCell<Option<serde_json::Value>>,
    states: RefCell<HashMap<String, InstallState>>,
    ops: RefCell<HashMap<String, Op>>,
    /// Package managers lock their databases, so operations run one at a time.
    queue: RefCell<VecDeque<(Action, Item)>>,
    running: Cell<bool>,
    prefs: Arc<Prefs>,
    navigate: RefCell<Option<Navigator>>,
    toast: RefCell<Option<Toaster>>,
    errors: RefCell<Option<ErrorPresenter>>,
    listeners: RefCell<Vec<(u64, Listener)>>,
    next_id: Cell<u64>,
}

#[derive(Clone)]
pub struct Ctx(Rc<Inner>);

impl Ctx {
    pub fn new(store: Arc<Store>) -> Self {
        Self(Rc::new(Inner {
            store,
            locales: i18n::locales(),
            items: RefCell::default(),
            announcement: RefCell::default(),
            states: RefCell::default(),
            ops: RefCell::default(),
            queue: RefCell::default(),
            running: Cell::new(false),
            prefs: Arc::new(Prefs::user()),
            navigate: RefCell::default(),
            toast: RefCell::default(),
            errors: RefCell::default(),
            listeners: RefCell::default(),
            next_id: Cell::new(0),
        }))
    }

    pub fn store(&self) -> &Arc<Store> {
        &self.0.store
    }

    pub fn prefs(&self) -> &Arc<Prefs> {
        &self.0.prefs
    }

    // ---- wiring done by the shell ----

    pub fn set_navigator(&self, f: impl Fn(Route) + 'static) {
        *self.0.navigate.borrow_mut() = Some(Rc::new(f));
    }

    pub fn set_toaster(&self, f: impl Fn(String) + 'static) {
        *self.0.toast.borrow_mut() = Some(Rc::new(f));
    }

    /// Shows a failure with details the user can copy.
    pub fn set_error_presenter(&self, f: impl Fn(String, String) + 'static) {
        *self.0.errors.borrow_mut() = Some(Rc::new(f));
    }

    fn show_error(&self, heading: String, details: String) {
        let f = self.0.errors.borrow().clone();
        match f {
            Some(f) => f(heading, details),
            None => self.toast(heading),
        }
    }

    // ---- what modules use ----

    pub fn navigate(&self, route: Route) {
        let f = self.0.navigate.borrow().clone();
        if let Some(f) = f {
            f(route);
        }
    }

    pub fn toast(&self, message: String) {
        let f = self.0.toast.borrow().clone();
        if let Some(f) = f {
            f(message);
        }
    }

    pub fn items(&self) -> Arc<Vec<Item>> {
        self.0.items.borrow().clone()
    }

    /// The catalog's announcement banner, if any.
    pub fn announcement(&self) -> Option<serde_json::Value> {
        self.0.announcement.borrow().clone()
    }

    /// Downloads an image in the background and calls `done` with its cached path.
    pub fn load_image(&self, url: String, done: impl FnOnce(Option<std::path::PathBuf>) + 'static) {
        let paths = self.0.store.paths.clone();
        task::run(
            move || {
                crate::core::remote::fetch_image_cached(&paths, &crate::core::http::UreqFetch, &url)
                    .ok()
            },
            done,
        );
    }

    /// Downloads text in the background and calls `done` with it.
    pub fn load_text(
        &self,
        url: String,
        max_bytes: usize,
        done: impl FnOnce(Option<String>) + 'static,
    ) {
        task::run(
            move || {
                crate::core::remote::fetch_text(&crate::core::http::UreqFetch, &url, max_bytes).ok()
            },
            done,
        );
    }

    pub fn item(&self, id: &str) -> Option<Item> {
        self.items().iter().find(|i| i.id == id).cloned()
    }

    pub fn locales(&self) -> Vec<&str> {
        self.0.locales.iter().map(String::as_str).collect()
    }

    pub fn name(&self, item: &Item) -> String {
        let n = item.name.resolve(&self.locales());
        if n.is_empty() {
            item.id.clone()
        } else {
            n.to_string()
        }
    }

    pub fn summary(&self, item: &Item) -> String {
        let loc = self.locales();
        let s = item.summary.resolve(&loc);
        if s.is_empty() {
            item.description.resolve(&loc).to_string()
        } else {
            s.to_string()
        }
    }

    pub fn state(&self, id: &str) -> InstallState {
        self.0
            .states
            .borrow()
            .get(id)
            .cloned()
            .unwrap_or(InstallState::NotInstalled)
    }

    /// How many items have a newer version in the catalog.
    pub fn update_count(&self) -> usize {
        self.0
            .states
            .borrow()
            .values()
            .filter(|s| matches!(s, InstallState::UpdateAvailable { .. }))
            .count()
    }

    pub fn op(&self, id: &str) -> Option<Op> {
        self.0.ops.borrow().get(id).cloned()
    }

    pub fn has_active_ops(&self) -> bool {
        !self.0.ops.borrow().is_empty()
    }

    /// Calls `cb` on every change until `widget` is destroyed.
    pub fn subscribe(&self, widget: &impl IsA<gtk::Widget>, cb: impl Fn(&Change) + 'static) {
        let id = self.0.next_id.get();
        self.0.next_id.set(id + 1);
        self.0.listeners.borrow_mut().push((id, Rc::new(cb)));
        let me = self.clone();
        widget.connect_destroy(move |_| me.0.listeners.borrow_mut().retain(|(i, _)| *i != id));
    }

    fn emit(&self, change: Change) {
        let listeners: Vec<Listener> = self
            .0
            .listeners
            .borrow()
            .iter()
            .map(|(_, f)| f.clone())
            .collect();
        for f in listeners {
            f(&change);
        }
    }

    // ---- loading ----

    /// Loads the catalog (cache or bundled), then its states and icons.
    pub fn load_catalog(&self) {
        let store = self.0.store.clone();
        let me = self.clone();
        task::run(
            move || store.catalog.load(),
            move |result| {
                if let Ok(catalog) = result {
                    *me.0.items.borrow_mut() = Arc::new(catalog.packages);
                    *me.0.announcement.borrow_mut() = catalog.announcement;
                    me.emit(Change::Catalog);
                    me.refresh_states();
                    me.prefetch_icons();
                }
            },
        );
    }

    /// Fetches the catalog from the network and reloads it when it changed.
    pub fn refresh_remote(&self, force: bool) {
        // For development and screenshots with a local catalog: never replace it.
        if std::env::var_os("PULSAR_STORE_OFFLINE").is_some_and(|v| v != "0") {
            return;
        }
        let store = self.0.store.clone();
        let me = self.clone();
        task::run(
            move || store.catalog.refresh(force),
            move |result| match result {
                Ok(true) => me.load_catalog(),
                Ok(false) => {}
                Err(e) => me.toast(i18n::format(
                    &tr!("Could not refresh the catalog: {reason}"),
                    &[("reason", &e.to_string())],
                )),
            },
        );
    }

    /// Recomputes install state for every item in the background.
    pub fn refresh_states(&self) {
        let (installer, items) = (self.0.store.installer.clone(), self.items());
        let me = self.clone();
        task::run(
            move || compute_states(&installer, &items),
            move |states| {
                *me.0.states.borrow_mut() = states;
                me.emit(Change::States);
            },
        );
    }

    fn refresh_state_of(&self, item: Item) {
        let installer = self.0.store.installer.clone();
        let me = self.clone();
        task::run(
            move || {
                (
                    item.id.clone(),
                    installer.state(&item).unwrap_or(InstallState::NotInstalled),
                )
            },
            move |(id, state)| {
                me.0.states.borrow_mut().insert(id.clone(), state);
                me.emit(Change::State(id));
            },
        );
    }

    fn prefetch_icons(&self) {
        let (icons, items) = (self.0.store.icons.clone(), self.items());
        let me = self.clone();
        task::run(
            move || icons.prefetch(&items),
            move |fetched| {
                if fetched > 0 {
                    me.emit(Change::Icons);
                }
            },
        );
    }

    // ---- actions ----

    /// Queues an install, uninstall, or update. It starts when earlier ones finish.
    pub fn act(&self, action: Action, item: Item) {
        let id = item.id.clone();
        if self.0.ops.borrow().contains_key(&id) {
            return;
        }
        self.0.ops.borrow_mut().insert(
            id.clone(),
            Op {
                action,
                fraction: None,
                cancel: CancelToken::new(),
                queued: true,
            },
        );
        self.0.queue.borrow_mut().push_back((action, item));
        self.emit(Change::Op(id));
        self.pump();
    }

    /// Starts the next queued operation if none is running.
    fn pump(&self) {
        if self.0.running.get() {
            return;
        }
        let Some((action, item)) = self.0.queue.borrow_mut().pop_front() else {
            return;
        };
        let id = item.id.clone();
        let token = self.0.ops.borrow().get(&id).map(|o| o.cancel.clone());
        if token.as_ref().is_some_and(CancelToken::is_cancelled) {
            // Cancelled while waiting: finish it without starting work.
            self.finish(action, item, Err(crate::core::error::StoreError::Cancelled));
            return self.pump();
        }
        self.0.running.set(true);
        let handle = self.0.store.installer.start(action, item.clone());
        if let Some(op) = self.0.ops.borrow_mut().get_mut(&id) {
            op.cancel = handle.cancel.clone();
            op.queued = false;
        }
        self.emit(Change::Op(id.clone()));

        let (me_p, me_d) = (self.clone(), self.clone());
        let id_p = id.clone();
        task::watch(
            handle,
            move |p: Progress| {
                if let Some(op) = me_p.0.ops.borrow_mut().get_mut(&id_p) {
                    op.fraction = p.fraction;
                }
                me_p.emit(Change::Op(id_p.clone()));
            },
            move |result| {
                me_d.0.running.set(false);
                me_d.finish(action, item, result);
                me_d.pump();
            },
        );
    }

    fn finish(&self, action: Action, item: Item, result: crate::core::error::Result<()>) {
        let id = item.id.clone();
        self.0.ops.borrow_mut().remove(&id);
        self.emit(Change::Op(id));
        self.report(action, &item, result);
        self.refresh_state_of(item);
    }

    pub fn cancel(&self, id: &str) {
        if let Some(op) = self.0.ops.borrow().get(id) {
            op.cancel.cancel();
        }
    }

    fn report(&self, action: Action, item: &Item, result: crate::core::error::Result<()>) {
        let name = self.name(item);
        if result.is_ok()
            && matches!(action, Action::Install(_) | Action::Update)
            && item.kind == crate::core::model::SourceKind::GnomeExtension
        {
            self.report_extension(item.clone(), name);
            return;
        }
        let msg = match (&result, action) {
            (Ok(()), Action::Install(_)) => {
                i18n::format(&tr!("{name} installed"), &[("name", &name)])
            }
            (Ok(()), Action::Uninstall) => i18n::format(&tr!("{name} removed"), &[("name", &name)]),
            (Ok(()), Action::Update) => i18n::format(&tr!("{name} updated"), &[("name", &name)]),
            (Err(crate::core::error::StoreError::Cancelled), _) => {
                i18n::format(&tr!("Cancelled {name}"), &[("name", &name)])
            }
            (Err(e), _) => {
                let heading = i18n::format(&tr!("Could not complete {name}"), &[("name", &name)]);
                self.show_error(heading, error_details(e));
                return;
            }
        };
        self.toast(msg);
    }

    /// Tells the user whether GNOME Shell switched the new extension on. The Shell on
    /// Wayland only sees extensions that were installed before it started, so a fresh
    /// one usually needs the user to log out and back in.
    fn report_extension(&self, item: Item, name: String) {
        let installer = self.0.store.installer.clone();
        let me = self.clone();
        task::run(
            move || installer.is_active(&item),
            move |active| {
                let msg = match active {
                    Some(true) => {
                        i18n::format(&tr!("{name} installed and enabled"), &[("name", &name)])
                    }
                    Some(false) => i18n::format(
                        &tr!("{name} installed. Log out and back in to turn it on."),
                        &[("name", &name)],
                    ),
                    None => i18n::format(&tr!("{name} installed"), &[("name", &name)]),
                };
                me.toast(msg);
            },
        );
    }

    /// Launches an installed app. Reports a toast when it cannot be found.
    pub fn open_item(&self, item: &Item) {
        use gtk4::gio;
        let id = item.id.clone();
        let app = gio::DesktopAppInfo::new(&format!("{id}.desktop"))
            .map(|a| a.upcast::<gio::AppInfo>())
            .or_else(|| {
                let wanted = self.name(item).to_lowercase();
                gio::AppInfo::all().into_iter().find(|a| {
                    let name = a.name().to_lowercase();
                    name == wanted
                        || a.id().is_some_and(|i| {
                            i.trim_end_matches(".desktop").eq_ignore_ascii_case(&id)
                        })
                })
            });
        match app {
            Some(app) if app.launch(&[], None::<&gio::AppLaunchContext>).is_ok() => {}
            _ => self.toast(i18n::format(
                &tr!("Could not open {name}"),
                &[("name", &self.name(item))],
            )),
        }
    }
}

/// What to tell the user about a failure. A missing package manager says how to get it,
/// instead of the bare "not available" the error carries for logs.
fn error_details(e: &crate::core::error::StoreError) -> String {
    use crate::core::error::StoreError;
    use crate::core::model::SourceKind;
    match e {
        StoreError::SourceUnavailable(SourceKind::Flatpak) => tr!(
            "Flatpak is not installed on this system. Install it (for example with “sudo pacman -S flatpak” or “sudo apt install flatpak”), then try again."
        ),
        other => other.to_string(),
    }
}

fn compute_states(installer: &Installer, items: &[Item]) -> HashMap<String, InstallState> {
    items
        .iter()
        .filter_map(|i| installer.state(i).ok().map(|s| (i.id.clone(), s)))
        .collect()
}

#[cfg(test)]
mod error_text_tests {
    use super::error_details;
    use crate::core::error::StoreError;
    use crate::core::model::SourceKind;

    #[test]
    fn missing_flatpak_says_how_to_install_it() {
        let text = error_details(&StoreError::SourceUnavailable(SourceKind::Flatpak));
        assert!(text.contains("pacman -S flatpak") && text.contains("apt install flatpak"));
    }

    #[test]
    fn other_errors_keep_their_own_text() {
        let e = StoreError::CommandFailed("boom".into());
        assert_eq!(error_details(&e), e.to_string());
    }
}
