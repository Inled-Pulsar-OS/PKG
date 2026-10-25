//! dmenu mode window: reads items from stdin, fuzzy filters, prints the pick (exit 0) or cancels (exit 1).
//! Runs as a separate non-unique application so it never talks to the normal launcher instance.
use crate::core::dmenu::{self, Args, Outcome};
use crate::core::root_search::{Row, RowKind};
use crate::core::shortcuts::{self, Act, Mods};
use crate::ui::results::{ResultView, RowData};
use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

const APP_ID: &str = "es.inled.pulsaros-spotlight.dmenu";

fn rows(items: &[String], query: &str) -> Vec<(String, Vec<RowData>)> {
    let rows = dmenu::filter(query, items)
        .into_iter()
        .map(|t| RowData {
            row: Row::simple(RowKind::Command, &format!("dmenu:{t}"), &t, "", "go-next-symbolic"),
            payload: None,
            calc: None,
        })
        .collect();
    vec![(String::new(), rows)]
}

/// Runs the picker; the returned code is `Outcome::exit_code()`.
pub fn run(args: Args, load_css: fn()) -> ! {
    adw::init().expect("Failed to initialize Libadwaita");
    let app = gtk4::Application::builder().application_id(APP_ID).flags(gio::ApplicationFlags::NON_UNIQUE).build();

    let (tx, rx) = mpsc::channel::<Vec<String>>();
    std::thread::spawn(move || {
        let _ = tx.send(dmenu::read_items(std::io::stdin().lock()));
    });
    let rx = Rc::new(RefCell::new(Some(rx)));
    let code = Rc::new(Cell::new(Outcome::Cancelled.exit_code()));

    let code_c = code.clone();
    app.connect_activate(move |app| {
        let Some(rx) = rx.borrow_mut().take() else { return };
        load_css();
        build(app, &args, rx, code_c.clone());
    });
    app.run_with_args::<&str>(&[]);
    std::process::exit(code.get())
}

fn build(app: &gtk4::Application, args: &Args, rx: mpsc::Receiver<Vec<String>>, code: Rc<Cell<i32>>) {
    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("dmenu")
        .default_width(712)
        .default_height(560)
        .resizable(false)
        .decorated(false)
        .build();
    window.add_css_class("spotlight-window");
    let sm = adw::StyleManager::default();
    window.add_css_class(if sm.is_dark() { "dark" } else { "light" });

    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    root.add_css_class("spotlight-root");
    let main = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    main.add_css_class("spotlight-main");
    main.set_vexpand(true);
    main.set_overflow(gtk4::Overflow::Hidden);
    root.append(&main);
    window.set_child(Some(&root));

    let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    header.add_css_class("search-header");
    let icon = gtk4::Image::from_icon_name("system-search-symbolic");
    icon.set_pixel_size(28);
    icon.add_css_class("search-icon");
    header.append(&icon);
    if let Some(p) = &args.prompt {
        let l = gtk4::Label::new(Some(p));
        l.add_css_class("dmenu-prompt");
        header.append(&l);
    }
    let entry = gtk4::Entry::builder().hexpand(true).has_frame(false).valign(gtk4::Align::Center).build();
    entry.add_css_class("search-input");
    entry.set_placeholder_text(Some(args.placeholder.as_deref().unwrap_or("Select")));
    header.append(&entry);
    main.append(&header);

    let activate: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
    let act_c = activate.clone();
    let view = Rc::new(ResultView::new(
        move || {
            let cb = act_c.borrow().clone();
            if let Some(cb) = cb {
                cb();
            }
        },
        || {},
        |_| {},
    ));
    main.append(view.widget());

    let items: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let refresh = {
        let (view, items, entry) = (view.clone(), items.clone(), entry.clone());
        Rc::new(move |keep: bool| view.set_sections(rows(&items.borrow(), &entry.text()), keep))
    };
    let r = refresh.clone();
    entry.connect_changed(move |_| r(false));

    let finish = {
        let (app, code, view, entry) = (app.clone(), code.clone(), view.clone(), entry.clone());
        Rc::new(move |n: Option<usize>| {
            if let Some(n) = n {
                if !view.select_nth_row(n) {
                    return;
                }
            }
            let pick = view.selected_row().map(|r| r.row.title).or_else(|| {
                let t = entry.text().to_string();
                (!t.is_empty()).then_some(t)
            });
            let Some(pick) = pick else { return };
            let outcome = Outcome::Selected(pick);
            if let Some(out) = outcome.stdout() {
                use std::io::Write;
                let mut so = std::io::stdout();
                let _ = so.write_all(out.as_bytes());
                let _ = so.flush();
            }
            code.set(outcome.exit_code());
            app.quit();
        })
    };
    let f = finish.clone();
    *activate.borrow_mut() = Some(Rc::new(move || f(None)));

    let keys = gtk4::EventControllerKey::new();
    keys.set_propagation_phase(gtk4::PropagationPhase::Capture);
    let (app_c, v, f) = (app.clone(), view.clone(), finish.clone());
    keys.connect_key_pressed(move |_, keyval, _, state| {
        let name = keyval.to_lower().name().map(|n| n.to_lowercase()).unwrap_or_default();
        let mods = Mods {
            ctrl: state.contains(gdk::ModifierType::CONTROL_MASK),
            shift: state.contains(gdk::ModifierType::SHIFT_MASK),
            alt: state.contains(gdk::ModifierType::ALT_MASK),
        };
        match shortcuts::lookup(&name, mods) {
            Some(Act::Back) | Some(Act::Root) => app_c.quit(), // exit code stays 1
            Some(Act::Run(0)) => f(None),
            Some(Act::Quick(n)) => f(Some(n)),
            Some(Act::Up) => {
                v.step(-1);
            }
            Some(Act::Down) => {
                v.step(1);
            }
            Some(Act::PageUp) => {
                v.step(-8);
            }
            Some(Act::PageDown) => {
                v.step(8);
            }
            _ => return glib::Propagation::Proceed,
        }
        glib::Propagation::Stop
    });
    window.add_controller(keys);

    // Items arrive off the UI thread.
    let (items_c, refresh_c) = (items.clone(), refresh.clone());
    glib::timeout_add_local(Duration::from_millis(20), move || match rx.try_recv() {
        Ok(list) => {
            *items_c.borrow_mut() = list;
            refresh_c(false);
            glib::ControlFlow::Break
        }
        Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
        Err(_) => glib::ControlFlow::Break,
    });

    window.present();
    entry.grab_focus();
}
