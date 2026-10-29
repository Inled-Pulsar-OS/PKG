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
use pulsar_store::ui::tokens::{
    RADIUS_WINDOW, SIDEBAR_CLEARANCE, SIDEBAR_INSET, SIDEBAR_WIDTH, SPACE_L, SPACE_M, SPACE_S,
    SPACE_XL, TOOLBAR_HEIGHT,
};
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
    sidebar_titles: Vec<String>,
    back: gtk::Button,
    /// The round glass around the back button, shown only when there is somewhere to go back to.
    back_holder: GlassSurface,
    /// The round glass around the share button, shown only on an item page.
    share_holder: GlassSurface,
    share: gtk::Button,
    /// The red, yellow, and green window buttons at the top of the sidebar.
    traffic: [gtk::Button; 3],
    /// Count of pending updates, shown on the Updates row.
    badge: gtk::Label,
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

        // ---- content: pages, with the sidebar and toolbar floating above ----
        let nav = adw::NavigationView::new();
        nav.add_css_class("store-nav");

        // Two round glass buttons float over the content, as in the Tahoe App Store:
        // back at the left, and share at the right. Both appear only on an item page
        // (back also whenever there is somewhere to go back to). There is no toolbar,
        // no title, and no refresh or close button here; refresh is Ctrl+R.
        let round = |icon: &str, tooltip: &str| {
            let button = gtk::Button::from_icon_name(icon);
            button.add_css_class("flat");
            button.add_css_class("circular");
            button.set_tooltip_text(Some(tooltip));
            button
        };
        let back = round("go-previous-symbolic", &tr!("Back"));
        let share = round("emblem-shared-symbolic", &tr!("Share"));

        let wrap_round = |child: &gtk::Widget| {
            let glass = GlassSurface::new();
            glass.add_css_class("store-round-glass");
            glass.set_corner_radius(TOOLBAR_HEIGHT as f32 / 2.0);
            glass.set_child(Some(child));
            glass.set_backdrop(Some(&nav));
            glass
        };
        let back_holder = wrap_round(back.upcast_ref());
        back_holder.set_halign(gtk::Align::Start);
        back_holder.set_valign(gtk::Align::Start);
        back_holder.set_margin_top(SIDEBAR_INSET);
        back_holder.set_margin_start(SIDEBAR_CLEARANCE);
        back_holder.set_visible(false);

        let share_holder = wrap_round(share.upcast_ref());
        share_holder.set_halign(gtk::Align::End);
        share_holder.set_valign(gtk::Align::Start);
        share_holder.set_margin_top(SIDEBAR_INSET);
        share_holder.set_margin_end(SIDEBAR_INSET);
        share_holder.set_visible(false);

        // An invisible strip along the top lets the window be dragged.
        let drag = gtk::WindowHandle::new();
        drag.set_halign(gtk::Align::Fill);
        drag.set_valign(gtk::Align::Start);
        drag.set_size_request(-1, TOOLBAR_HEIGHT + SIDEBAR_INSET);

        // ---- sidebar: search, destinations, and the user at the bottom ----
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some(&tr!("Search")));
        search.update_property(&[gtk::accessible::Property::Label(&tr!(
            "Search apps, extensions, Sayri AI…"
        ))]);
        search.set_margin_start(SPACE_M);
        search.set_margin_end(SPACE_M);

        let sidebar = gtk::ListBox::new();
        sidebar.add_css_class("navigation-sidebar");
        sidebar.add_css_class("store-sidebar-list");
        sidebar.set_selection_mode(gtk::SelectionMode::Single);
        let badge = gtk::Label::new(None);
        badge.add_css_class("store-badge");
        badge.set_visible(false);
        let mut sidebar_routes = Vec::new();
        let mut sidebar_titles = Vec::new();
        for entry in &entries {
            let row = gtk::ListBoxRow::new();
            row.add_css_class("store-sidebar-row");
            let line = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_M);
            line.set_margin_top(SPACE_S);
            line.set_margin_bottom(SPACE_S);
            line.set_margin_start(SPACE_S);
            line.set_margin_end(SPACE_S);
            line.append(&gtk::Image::from_icon_name(entry.icon));
            let label = gtk::Label::new(Some(&entry.title));
            label.set_xalign(0.0);
            label.set_hexpand(true);
            line.append(&label);
            if entry.route == Route::Updates {
                line.append(&badge);
            }
            row.set_child(Some(&line));
            sidebar.append(&row);
            sidebar_routes.push(entry.route.clone());
            sidebar_titles.push(entry.title.clone());
        }
        let side_scroll = gtk::ScrolledWindow::new();
        side_scroll.set_child(Some(&sidebar));
        side_scroll.set_vexpand(true);
        side_scroll.set_hscrollbar_policy(gtk::PolicyType::Never);

        // Window buttons are red, yellow, and green circles at the top of the sidebar,
        // as in the picture. Their actions are connected once the window exists.
        let traffic_button = |class: &str, tooltip: String| {
            let button = gtk::Button::new();
            button.add_css_class("store-traffic");
            button.add_css_class(class);
            button.set_tooltip_text(Some(&tooltip));
            button.update_property(&[gtk::accessible::Property::Label(&tooltip)]);
            button.set_valign(gtk::Align::Center);
            button
        };
        let traffic = [
            traffic_button("close", tr!("Close")),
            traffic_button("minimize", tr!("Minimize")),
            traffic_button("maximize", tr!("Maximize")),
        ];
        let lights = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_S);
        lights.set_margin_start(SPACE_L);
        lights.set_margin_top(SPACE_M);
        lights.set_halign(gtk::Align::Start);
        traffic.iter().for_each(|b| lights.append(b));
        let top_handle = gtk::WindowHandle::new();
        top_handle.set_child(Some(&lights));
        top_handle.set_size_request(-1, SPACE_XL + SPACE_M);

        let side = gtk::Box::new(gtk::Orientation::Vertical, SPACE_S);
        side.append(&top_handle);
        side.append(&search);
        side.append(&side_scroll);
        side.append(&user_footer());

        let sidebar_glass = GlassSurface::new();
        sidebar_glass.add_css_class("store-sidebar-glass");
        sidebar_glass.set_halign(gtk::Align::Start);
        sidebar_glass.set_valign(gtk::Align::Fill);
        sidebar_glass.set_size_request(SIDEBAR_WIDTH, -1);
        for set in [
            gtk::Widget::set_margin_top,
            gtk::Widget::set_margin_bottom,
            gtk::Widget::set_margin_start,
        ] {
            set(sidebar_glass.upcast_ref(), SIDEBAR_INSET);
        }
        sidebar_glass.set_corner_radius(RADIUS_WINDOW);
        sidebar_glass.set_child(Some(&side));
        sidebar_glass.set_backdrop(Some(&nav));

        let root = gtk::Overlay::new();
        root.add_css_class("store-backdrop");
        root.set_child(Some(&nav));
        root.add_overlay(&sidebar_glass);
        root.add_overlay(&drag);
        root.add_overlay(&back_holder);
        root.add_overlay(&share_holder);
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&root));

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
        window.set_size_request(760, 480);

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
            sidebar_titles,
            back,
            back_holder,
            share_holder,
            share,
            traffic,
            badge,
            search,
            syncing: Cell::new(false),
            pending_install: RefCell::new(None),
        });
        shell.wire();
        shell.go(Route::Discover, Nav::Root);
        shell.ctx.load_catalog();
        shell.ctx.refresh_remote(false);
        shell
    }

    fn wire(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);

        // Window buttons.
        let [close, minimize, maximize] = &self.traffic;
        close.connect_clicked({
            let w = self.window.clone();
            move |_| w.close()
        });
        minimize.connect_clicked({
            let w = self.window.clone();
            move |_| w.minimize()
        });
        maximize.connect_clicked({
            let w = self.window.clone();
            move |_| {
                if w.is_maximized() {
                    w.unmaximize();
                } else {
                    w.maximize();
                }
            }
        });

        self.share.connect_clicked({
            let weak = weak.clone();
            move |_| {
                if let Some(s) = weak.upgrade() {
                    s.share_current();
                }
            }
        });
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
                    let current = s.router.borrow().current().cloned();
                    if let Some(route) = current {
                        s.select_sidebar(Some(&route));
                    }
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

        // Keep the Updates badge in step with install states.
        self.ctx.subscribe(&self.window, {
            let weak = weak.clone();
            move |c| {
                if matches!(c, Change::States | Change::State(_) | Change::Catalog) {
                    if let Some(s) = weak.upgrade() {
                        s.update_badge();
                        // An item page opened before the catalog loaded shows the id until now.
                        s.sync_chrome();
                    }
                }
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

    /// Highlights the sidebar row for `route`, or none when it is not a destination.
    fn select_sidebar(&self, route: Option<&Route>) {
        self.syncing.set(true);
        match route.and_then(|r| self.sidebar_routes.iter().position(|x| x == r)) {
            Some(i) => self
                .sidebar
                .select_row(self.sidebar.row_at_index(i as i32).as_ref()),
            None => self.sidebar.unselect_all(),
        }
        self.syncing.set(false);
    }

    fn clear_search(&self) {
        self.syncing.set(true);
        self.search.set_text("");
        self.syncing.set(false);
    }

    fn sync_chrome(&self) {
        self.back_holder
            .set_visible(self.router.borrow().can_go_back());
        let on_item = matches!(self.router.borrow().current(), Some(Route::Item(_)));
        self.share_holder.set_visible(on_item);
        // The page the user came to stays named for screen readers.
        let route = self.router.borrow().current().cloned();
        if let Some(route) = route {
            self.window.set_title(Some(&self.title_for(&route)));
        }
    }

    /// Copies a `pulsar://install/<id>` link for the open item.
    fn share_current(&self) {
        let id = match self.router.borrow().current() {
            Some(Route::Item(id)) => id.clone(),
            _ => return,
        };
        if let Some(display) = gtk::gdk::Display::default() {
            display
                .clipboard()
                .set_text(&format!("pulsar://install/{id}"));
        }
        self.ctx.toast(tr!("Link copied"));
    }

    fn update_badge(&self) {
        let n = self.ctx.update_count();
        self.badge.set_text(&n.to_string());
        self.badge.set_visible(n > 0);
    }

    fn title_for(&self, route: &Route) -> String {
        if let Some(i) = self.sidebar_routes.iter().position(|r| r == route) {
            return self.sidebar_titles[i].clone();
        }
        match route {
            Route::Item(id) => self
                .ctx
                .item(id)
                .map_or_else(|| id.clone(), |i| self.ctx.name(&i)),
            Route::Search(_) => tr!("Search"),
            Route::Category(key) => {
                let mut c = key.chars();
                c.next()
                    .map_or_else(String::new, |f| f.to_uppercase().chain(c).collect())
            }
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
                self.select_sidebar(Some(&route));
            }
            Nav::Push => {
                // A page that is not a sidebar destination leaves no row highlighted.
                self.select_sidebar(None);
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

/// The signed-in user at the bottom of the sidebar: an initial in a circle and the name.
fn user_footer() -> gtk::Widget {
    let name = {
        let real = gtk::glib::real_name().to_string_lossy().into_owned();
        if real.is_empty() || real == "Unknown" {
            gtk::glib::user_name().to_string_lossy().into_owned()
        } else {
            real
        }
    };
    let row = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_M);
    row.set_margin_start(SPACE_M);
    row.set_margin_end(SPACE_M);
    row.set_margin_bottom(SPACE_M);
    row.set_margin_top(SPACE_S);

    let initial = name
        .chars()
        .next()
        .map_or(String::new(), |c| c.to_uppercase().collect());
    let avatar = gtk::Label::new(Some(&initial));
    avatar.add_css_class("store-avatar");
    avatar.set_accessible_role(gtk::AccessibleRole::Presentation);
    row.append(&avatar);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 0);
    text.set_valign(gtk::Align::Center);
    let who = gtk::Label::new(Some(&name));
    who.set_xalign(0.0);
    who.set_ellipsize(gtk::pango::EllipsizeMode::End);
    text.append(&who);
    let os = gtk::Label::new(Some(&tr!("Pulsar OS")));
    os.add_css_class("store-caption");
    os.set_xalign(0.0);
    text.append(&os);
    row.append(&text);
    row.upcast()
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
