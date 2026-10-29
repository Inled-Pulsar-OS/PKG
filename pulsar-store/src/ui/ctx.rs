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

struct Inner {
    store: Arc<Store>,
    locales: Vec<String>,
    items: RefCell<Arc<Vec<Item>>>,
    states: RefCell<HashMap<String, InstallState>>,
    ops: RefCell<HashMap<String, Op>>,
    /// Package managers lock their databases, so operations run one at a time.
    queue: RefCell<VecDeque<(Action, Item)>>,
    running: Cell<bool>,
    prefs: Arc<Prefs>,
    navigate: RefCell<Option<Navigator>>,
    toast: RefCell<Option<Toaster>>,
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
            states: RefCell::default(),
            ops: RefCell::default(),
            queue: RefCell::default(),
            running: Cell::new(false),
            prefs: Arc::new(Prefs::user()),
            navigate: RefCell::default(),
            toast: RefCell::default(),
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
                    me.emit(Change::Catalog);
                    me.refresh_states();
                    me.prefetch_icons();
                }
            },
        );
    }

    /// Fetches the catalog from the network and reloads it when it changed.
    pub fn refresh_remote(&self, force: bool) {
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
        let msg = match (&result, action) {
            (Ok(()), Action::Install(_)) => {
                i18n::format(&tr!("{name} installed"), &[("name", &name)])
            }
            (Ok(()), Action::Uninstall) => i18n::format(&tr!("{name} removed"), &[("name", &name)]),
            (Ok(()), Action::Update) => i18n::format(&tr!("{name} updated"), &[("name", &name)]),
            (Err(crate::core::error::StoreError::Cancelled), _) => {
                i18n::format(&tr!("Cancelled {name}"), &[("name", &name)])
            }
            (Err(e), _) => i18n::format(
                &tr!("Could not complete {name}: {reason}"),
                &[("name", &name), ("reason", &e.to_string())],
            ),
        };
        self.toast(msg);
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

fn compute_states(installer: &Installer, items: &[Item]) -> HashMap<String, InstallState> {
    items
        .iter()
        .filter_map(|i| installer.state(i).ok().map(|s| (i.id.clone(), s)))
        .collect()
}
