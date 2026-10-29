//! Application shell: window, sidebar, floating glass toolbar, router wiring,
//! module registry, toasts, shortcuts, and single-instance command line handling.

use adw::prelude::*;
use gtk4 as gtk;
use gtk4::gio;
use pulsar_store::core::deeplink::install_request;
use pulsar_store::core::install::Action;
use pulsar_store::core::prefs::WindowState;
use pulsar_store::core::store::Store;
use pulsar_store::core::{i18n, model::Item};
use pulsar_store::modules::{self, Module};
use pulsar_store::tr;
use pulsar_store::ui::glass::GlassSurface;
use pulsar_store::ui::tokens::{self, INSET_TOOLBAR, RADIUS_WINDOW, SPACE_M, SPACE_S};
use pulsar_store::ui::{theme, Change, Ctx, Route, Router};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

pub const APP_ID: &str = "es.inled.PulsarStore";

struct Shell {
    window: adw::ApplicationWindow,
    ctx: Ctx,
    modules: Vec<Box<dyn Module>>,
    router: RefCell<Router>,
    nav: adw::NavigationView,
    sidebar: gtk::ListBox,
    sidebar_routes: Vec<Route>,
    back: gtk::Button,
    search: gtk::SearchEntry,
    /// True while the code (not the user) changes the sidebar selection or search text.
    syncing: Cell<bool>,
    pending_install: RefCell<Option<String>>,
}

impl Shell {
    fn new(app: &adw::Application) -> Rc<Self> {
        theme::install();
        let ctx = Ctx::new(Arc::new(Store::new()));
        let modules = modules::all();
        let entries = modules::sidebar_entries(&modules);

        // ---- content: pages under a floating glass toolbar ----
        let nav = adw::NavigationView::new();
        let back = gtk::Button::from_icon_name("go-previous-symbolic");
        back.set_tooltip_text(Some(&tr!("Back")));
        back.set_visible(false);
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some(&tr!("Search apps, extensions, Sayri AI…")));
        search.update_property(&[gtk::accessible::Property::Label(&tr!("Search"))]);
        search.set_hexpand(true);
        search.set_max_width_chars(36);
        let refresh = gtk::Button::from_icon_name("view-refresh-symbolic");
        refresh.set_tooltip_text(Some(&tr!("Refresh Store")));

        let header = adw::HeaderBar::new();
        header.add_css_class("flat");
        header.pack_start(&back);
        header.pack_end(&refresh);
        header.set_title_widget(Some(&search));

        let glass = GlassSurface::new();
        glass.set_halign(gtk::Align::Fill);
        glass.set_valign(gtk::Align::Start);
        for set in [
            gtk::Widget::set_margin_top,
            gtk::Widget::set_margin_start,
            gtk::Widget::set_margin_end,
        ] {
            set(glass.upcast_ref(), INSET_TOOLBAR as i32);
        }
        glass.set_corner_radius(tokens::concentric(RADIUS_WINDOW, 0.0) + 4.0);
        glass.set_child(Some(&header));
        glass.set_backdrop(Some(&nav));

        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&nav));
        overlay.add_overlay(&glass);
        let content_page = adw::NavigationPage::new(&overlay, &tr!("Pulsar Store"));

        // ---- sidebar ----
        let sidebar = gtk::ListBox::new();
        sidebar.add_css_class("navigation-sidebar");
        sidebar.set_selection_mode(gtk::SelectionMode::Single);
        let mut sidebar_routes = Vec::new();
        for entry in &entries {
            let row = gtk::ListBoxRow::new();
            row.add_css_class("store-sidebar-row");
            let line = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_M);
            line.set_margin_top(SPACE_S);
            line.set_margin_bottom(SPACE_S);
            line.set_margin_start(SPACE_S);
            line.append(&gtk::Image::from_icon_name(entry.icon));
            let label = gtk::Label::new(Some(&entry.title));
            label.set_xalign(0.0);
            line.append(&label);
            row.set_child(Some(&line));
            sidebar.append(&row);
            sidebar_routes.push(entry.route.clone());
        }
        let side_scroll = gtk::ScrolledWindow::new();
        side_scroll.set_child(Some(&sidebar));
        side_scroll.set_vexpand(true);
        let side_view = adw::ToolbarView::new();
        side_view.add_top_bar(&adw::HeaderBar::new());
        side_view.set_content(Some(&side_scroll));
        let sidebar_page = adw::NavigationPage::new(&side_view, &tr!("Pulsar Store"));

        let split = adw::NavigationSplitView::new();
        split.set_sidebar(Some(&sidebar_page));
        split.set_content(Some(&content_page));
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&split));

        let prefs = ctx.prefs().clone();
        let size = prefs.window().unwrap_or(WindowState {
            width: 1100,
            height: 720,
            maximized: false,
        });
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(tr!("Pulsar Store"))
            .default_width(size.width.max(480))
            .default_height(size.height.max(360))
            .maximized(size.maximized)
            .content(&toasts)
            .build();

        {
            let window = window.clone();
            ctx.set_error_presenter(move |heading, details| {
                let dialog = adw::AlertDialog::new(Some(&heading), Some(&details));
                dialog.add_response("copy", &tr!("Copy Details"));
                dialog.add_response("close", &tr!("Close"));
                dialog.set_default_response(Some("close"));
                dialog.set_close_response("close");
                let text = format!("{heading}\n{details}");
                dialog.connect_response(Some("copy"), move |_, _| {
                    if let Some(display) = gtk::gdk::Display::default() {
                        display.clipboard().set_text(&text);
                    }
                });
                dialog.present(Some(&window));
            });
        }
        {
            let toasts = toasts.clone();
            ctx.set_toaster(move |msg| toasts.add_toast(adw::Toast::new(&msg)));
        }

        let shell = Rc::new(Self {
            window,
            ctx,
            modules,
            router: RefCell::new(Router::new(Route::Discover)),
            nav,
            sidebar,
            sidebar_routes,
            back,
            search,
            syncing: Cell::new(false),
            pending_install: RefCell::new(None),
        });
        shell.wire(&refresh);
        shell.go(Route::Discover, Nav::Root);
        shell.ctx.load_catalog();
        shell.ctx.refresh_remote(false);
        shell
    }

    fn wire(self: &Rc<Self>, refresh: &gtk::Button) {
        let weak = Rc::downgrade(self);
        self.ctx.set_navigator({
            let weak = weak.clone();
            move |route| {
                if let Some(s) = weak.upgrade() {
                    let nav = if s.sidebar_routes.contains(&route) {
                        Nav::Root
                    } else {
                        Nav::Push
                    };
                    s.go(route, nav);
                }
            }
        });

        self.sidebar.connect_row_selected({
            let weak = weak.clone();
            move |_, row| {
                let (Some(s), Some(row)) = (weak.upgrade(), row) else {
                    return;
                };
                if s.syncing.get() {
                    return;
                }
                if let Some(route) = s.sidebar_routes.get(row.index() as usize) {
                    s.clear_search();
                    s.go(route.clone(), Nav::Root);
                }
            }
        });

        self.back.connect_clicked({
            let weak = weak.clone();
            move |_| {
                if let Some(s) = weak.upgrade() {
                    s.nav.pop();
                }
            }
        });
        self.nav.connect_popped({
            let weak = weak.clone();
            move |_, _| {
                if let Some(s) = weak.upgrade() {
                    s.router.borrow_mut().pop();
                    s.sync_chrome();
                }
            }
        });

        self.search.connect_search_changed({
            let weak = weak.clone();
            move |entry| {
                let Some(s) = weak.upgrade() else { return };
                if s.syncing.get() {
                    return;
                }
                let text = entry.text().to_string();
                let on_search = matches!(s.router.borrow().current(), Some(Route::Search(_)));
                s.go(
                    Route::Search(text),
                    if on_search {
                        Nav::ReplaceTop
                    } else {
                        Nav::Push
                    },
                );
            }
        });
        self.search.connect_activate({
            let weak = weak.clone();
            move |entry| {
                if let Some(s) = weak.upgrade() {
                    s.ctx.prefs().add_recent_search(&entry.text());
                }
            }
        });
        self.search.connect_stop_search({
            let weak = weak.clone();
            move |_| {
                if let Some(s) = weak.upgrade() {
                    s.clear_search();
                }
            }
        });

        refresh.connect_clicked({
            let weak = weak.clone();
            move |_| {
                if let Some(s) = weak.upgrade() {
                    s.ctx.refresh_remote(true);
                    s.ctx.refresh_states();
                }
            }
        });

        // Shortcuts.
        let add = |name: &str, accel: &str, f: ShellAction| {
            let action = gio::SimpleAction::new(name, None);
            let weak = weak.clone();
            action.connect_activate(move |_, _| {
                if let Some(s) = weak.upgrade() {
                    f(&s);
                }
            });
            self.window.add_action(&action);
            if let Some(app) = self.window.application() {
                app.set_accels_for_action(&format!("win.{name}"), &[accel]);
            }
        };
        add(
            "search",
            "<primary>f",
            Box::new(|s| {
                s.search.grab_focus();
            }),
        );
        add(
            "refresh",
            "<primary>r",
            Box::new(|s| s.ctx.refresh_remote(true)),
        );
        add(
            "back",
            "<alt>Left",
            Box::new(|s| {
                s.nav.pop();
            }),
        );

        // Remember the window size.
        self.window.connect_close_request({
            let weak = weak.clone();
            move |w| {
                if let Some(s) = weak.upgrade() {
                    let old = s.ctx.prefs().window();
                    let (width, height) = if w.is_maximized() {
                        old.map_or((w.width(), w.height()), |o| (o.width, o.height))
                    } else {
                        (w.width(), w.height())
                    };
                    s.ctx.prefs().set_window(WindowState {
                        width,
                        height,
                        maximized: w.is_maximized(),
                    });
                }
                gtk::glib::Propagation::Proceed
            }
        });

        // Run a queued deep-link install once its item is in the catalog.
        self.ctx.subscribe(&self.window, {
            let weak = weak.clone();
            move |c| {
                if let (Change::Catalog, Some(s)) = (c, weak.upgrade()) {
                    s.run_pending_install();
                }
            }
        });
    }

    fn clear_search(&self) {
        self.syncing.set(true);
        self.search.set_text("");
        self.syncing.set(false);
    }

    fn sync_chrome(&self) {
        self.back.set_visible(self.router.borrow().can_go_back());
    }

    fn title_for(&self, route: &Route) -> String {
        match route {
            Route::Item(id) => self
                .ctx
                .item(id)
                .map_or_else(|| id.clone(), |i| self.ctx.name(&i)),
            _ => tr!("Pulsar Store"),
        }
    }

    fn page_for(&self, route: &Route) -> Option<adw::NavigationPage> {
        let module = modules::for_route(&self.modules, route)?;
        Some(adw::NavigationPage::new(
            &module.build(route, &self.ctx),
            &self.title_for(route),
        ))
    }

    fn go(&self, route: Route, how: Nav) {
        let Some(page) = self.page_for(&route) else {
            return;
        };
        match how {
            Nav::Root => {
                self.router.borrow_mut().reset(route.clone());
                self.nav.replace(&[page]);
                if let Some(i) = self.sidebar_routes.iter().position(|r| *r == route) {
                    self.syncing.set(true);
                    self.sidebar
                        .select_row(self.sidebar.row_at_index(i as i32).as_ref());
                    self.syncing.set(false);
                }
            }
            Nav::Push => {
                self.router.borrow_mut().push(route);
                self.nav.push(&page);
            }
            Nav::ReplaceTop => {
                self.router.borrow_mut().replace(route);
                let stack = self.nav.navigation_stack();
                let mut pages: Vec<adw::NavigationPage> = (0..stack.n_items().saturating_sub(1))
                    .filter_map(|i| stack.item(i).and_downcast())
                    .collect();
                pages.push(page);
                self.nav.replace(&pages);
            }
        }
        self.sync_chrome();
    }

    /// Opens the item and starts installing it, now or when the catalog arrives.
    fn queue_install(&self, id: String) {
        *self.pending_install.borrow_mut() = Some(id);
        self.run_pending_install();
    }

    fn run_pending_install(&self) {
        let Some(id) = self.pending_install.borrow().clone() else {
            return;
        };
        let item: Option<Item> = self.ctx.item(&id);
        if let Some(item) = item {
            self.pending_install.borrow_mut().take();
            self.go(Route::Item(id), Nav::Push);
            self.confirm_install(item);
        }
    }

    /// A link or command line can ask for an install, so the user must agree first.
    fn confirm_install(&self, item: Item) {
        let name = self.ctx.name(&item);
        let dialog = adw::AlertDialog::new(
            Some(&i18n::format(&tr!("Install {name}?"), &[("name", &name)])),
            Some(&tr!(
                "This install was requested by a link or command. Install only software you trust."
            )),
        );
        dialog.add_response("cancel", &tr!("Cancel"));
        dialog.add_response("install", &tr!("Install"));
        dialog.set_response_appearance("install", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        let ctx = self.ctx.clone();
        dialog.connect_response(Some("install"), move |_, _| {
            ctx.act(Action::Install(None), item.clone());
        });
        dialog.present(Some(&self.window));
    }
}

/// Opens a page directly: `--page <route>` or `pulsar://page/<route>`.
fn page_request(args: &[String]) -> Option<Route> {
    pulsar_store::core::deeplink::page_request(args).and_then(|v| Route::from_arg(&v))
}

type ShellAction = Box<dyn Fn(&Rc<Shell>)>;

#[derive(Clone, Copy)]
enum Nav {
    /// A sidebar destination: clears history.
    Root,
    Push,
    /// Swap the top page, as when search text changes.
    ReplaceTop,
}

pub fn run() -> gtk::glib::ExitCode {
    i18n::init();
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    // GTK is initialized once the application starts, so the layout direction is set here.
    app.connect_startup(|_| {
        if i18n::wants_rtl() {
            gtk::Widget::set_default_direction(gtk::TextDirection::Rtl);
        }
    });

    let shell: Rc<RefCell<Option<Rc<Shell>>>> = Rc::default();
    {
        let shell = shell.clone();
        app.connect_command_line(move |app, cmd| {
            let mut slot = shell.borrow_mut();
            let s = slot.get_or_insert_with(|| Shell::new(app)).clone();
            drop(slot);
            s.window.present();
            let args: Vec<String> = cmd
                .arguments()
                .iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect();
            if let Some(id) = install_request(&args) {
                s.queue_install(id);
            } else if let Some(route) = page_request(&args) {
                let nav = if s.sidebar_routes.contains(&route) {
                    Nav::Root
                } else {
                    Nav::Push
                };
                s.go(route, nav);
            }
            0
        });
    }
    app.run()
}
