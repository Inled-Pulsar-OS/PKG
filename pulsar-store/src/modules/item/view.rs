use crate::core::editions::editions;
use crate::core::install::Action;
use crate::core::model::{EditionKey, InstallState, Item, SourceKind};
use crate::core::package_files::{package_files, FileRole};
use crate::core::remote::is_web_url;
use crate::ui::ctx::Change;
use crate::ui::page::{self, page};
use crate::ui::tokens::{SPACE_L, SPACE_M, SPACE_S};
use crate::ui::widget::{icon::item_icon, info_grid::info_grid, install_button::InstallButton};
use crate::ui::Ctx;
use crate::{ptr, tr};
use adw::prelude::*;
use gtk4 as gtk;

const README_LIMIT: usize = 64 * 1024;

pub fn kind_label(kind: SourceKind) -> String {
    match kind {
        SourceKind::Flatpak => tr!("Application"),
        SourceKind::System => tr!("System package"),
        SourceKind::GnomeExtension => tr!("Shell extension"),
        SourceKind::SayriSkill => tr!("Sayri skill"),
        SourceKind::SayriPlugin => tr!("Sayri plugin"),
        SourceKind::Unknown => tr!("Other"),
    }
}

pub fn edition_label(key: EditionKey) -> String {
    match key {
        EditionKey::Debian => tr!("Debian Edition"),
        EditionKey::Arch => tr!("Arch Edition"),
        EditionKey::Flatpak => ptr!("edition", "Flatpak"),
    }
}

fn str_field<'a>(item: &'a Item, key: &str) -> &'a str {
    item.extra.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

pub fn build(ctx: &Ctx, id: String) -> gtk::Widget {
    page(ctx, 860, move |ctx, column| {
        let Some(item) = ctx.item(&id) else {
            column.append(&page::empty(
                "dialog-question-symbolic",
                &tr!("Item not found"),
                &tr!("This item is not in the catalog."),
            ));
            return;
        };
        column.append(&header(ctx, &item));
        if let Some(shots) = screenshots(ctx, &item) {
            column.append(&shots);
        }
        let version = match ctx.state(&item.id) {
            InstallState::Installed { version: Some(v) } => v,
            InstallState::UpdateAvailable { installed, .. } => installed,
            _ => item.version.clone().unwrap_or_default(),
        };
        let score = item
            .security_report()
            .and_then(|r| r.score)
            .map(|s| format!("{s}/100"))
            .unwrap_or_default();
        column.append(&info_grid(&[
            (tr!("Version"), version),
            (tr!("Author"), str_field(&item, "author").to_string()),
            (tr!("Type"), kind_label(item.kind)),
            (tr!("Category"), item.category.clone()),
            (tr!("Security"), score),
        ]));
        if let Some(group) = editions_group(ctx, &item) {
            column.append(&group);
        }
        column.append(&about(ctx, &item));
        if let Some(group) = security_group(&item) {
            column.append(&group);
        }
        if let Some(group) = docs_group(ctx, &item) {
            column.append(&group);
        }
        column.append(&files_group(&item));
        column.append(&details_group(&item));
        if let Some(links) = links(&item) {
            column.append(&links);
        }
    })
}

fn header(ctx: &Ctx, item: &Item) -> gtk::Widget {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_L);
    row.append(&item_icon(ctx, item, 128));

    let meta = gtk::Box::new(gtk::Orientation::Vertical, SPACE_S);
    meta.set_hexpand(true);
    meta.set_valign(gtk::Align::Center);
    let name = gtk::Label::new(Some(&ctx.name(item)));
    name.add_css_class("store-title");
    name.set_xalign(0.0);
    name.set_wrap(true);
    name.set_accessible_role(gtk::AccessibleRole::Heading);
    meta.append(&name);
    let summary = gtk::Label::new(Some(&ctx.summary(item)));
    summary.add_css_class("store-subtitle");
    summary.set_xalign(0.0);
    summary.set_wrap(true);
    meta.append(&summary);
    row.append(&meta);

    let actions = gtk::Box::new(gtk::Orientation::Vertical, SPACE_S);
    actions.set_valign(gtk::Align::Center);
    actions.append(InstallButton::new(ctx, item).widget());

    let remove = gtk::Button::with_label(&tr!("Uninstall"));
    remove.add_css_class("destructive-action");
    remove.add_css_class("store-pill");
    let show = {
        let (ctx, id, remove) = (ctx.clone(), item.id.clone(), remove.clone());
        move || {
            let present = !matches!(ctx.state(&id), InstallState::NotInstalled);
            remove.set_visible(present && ctx.op(&id).is_none());
        }
    };
    show();
    let watch_id = item.id.clone();
    ctx.subscribe(&remove, move |c| {
        let mine = match c {
            Change::State(i) | Change::Op(i) => *i == watch_id,
            Change::States | Change::Catalog => true,
            Change::Icons => false,
        };
        if mine {
            show();
        }
    });
    {
        let (ctx, item) = (ctx.clone(), item.clone());
        remove.connect_clicked(move |_| ctx.act(Action::Uninstall, item.clone()));
    }
    actions.append(&remove);
    row.append(&actions);
    row.upcast()
}

fn screenshots(ctx: &Ctx, item: &Item) -> Option<gtk::Widget> {
    let urls: Vec<String> = item
        .screenshots()
        .into_iter()
        .filter(|u| is_web_url(u))
        .map(str::to_string)
        .collect();
    if urls.is_empty() {
        return None;
    }
    let carousel = adw::Carousel::new();
    carousel.set_allow_scroll_wheel(true);
    carousel.set_height_request(260);
    for url in urls.iter().cloned() {
        let picture = gtk::Picture::new();
        picture.add_css_class("store-screenshot");
        picture.set_content_fit(gtk::ContentFit::Contain);
        picture.set_accessible_role(gtk::AccessibleRole::Img);
        carousel.append(&picture);
        let weak = picture.downgrade();
        ctx.load_image(url, move |path| {
            if let (Some(p), Some(pic)) = (path, weak.upgrade()) {
                pic.set_filename(Some(p));
            }
        });
    }

    // Arrow keys move between screenshots.
    carousel.set_focusable(true);
    let keys = gtk::EventControllerKey::new();
    {
        let carousel = carousel.clone();
        keys.connect_key_pressed(move |_, key, _, _| {
            let step = match key {
                gtk::gdk::Key::Left => -1.0,
                gtk::gdk::Key::Right => 1.0,
                _ => return gtk::glib::Propagation::Proceed,
            };
            let target = (carousel.position() + step)
                .clamp(0.0, f64::from(carousel.n_pages().saturating_sub(1)));
            carousel.scroll_to(&carousel.nth_page(target as u32), true);
            gtk::glib::Propagation::Stop
        });
    }
    carousel.add_controller(keys);

    let col = gtk::Box::new(gtk::Orientation::Vertical, SPACE_S);
    col.append(&carousel);
    if urls.len() > 1 {
        let dots = adw::CarouselIndicatorDots::new();
        dots.set_carousel(Some(&carousel));
        col.append(&dots);
    }
    Some(col.upcast())
}

/// Lets the user pick which edition to install when the item offers several.
fn editions_group(ctx: &Ctx, item: &Item) -> Option<gtk::Widget> {
    let list = editions(item);
    if list.len() < 2 || !matches!(ctx.state(&item.id), InstallState::NotInstalled) {
        return None;
    }
    let group = adw::PreferencesGroup::new();
    group.set_title(&tr!("Install as"));
    for edition in list {
        let row = adw::ActionRow::new();
        row.set_title(&edition_label(edition.key));
        let button = gtk::Button::with_label(&ptr!("button", "Install"));
        button.add_css_class("store-pill");
        button.set_valign(gtk::Align::Center);
        let (ctx, item) = (ctx.clone(), item.clone());
        button.connect_clicked(move |_| ctx.act(Action::Install(Some(edition.key)), item.clone()));
        row.add_suffix(&button);
        group.add(&row);
    }
    Some(group.upcast())
}

fn about(ctx: &Ctx, item: &Item) -> gtk::Widget {
    let col = gtk::Box::new(gtk::Orientation::Vertical, SPACE_M);
    let heading = gtk::Label::new(Some(&tr!("About")));
    heading.add_css_class("store-section-title");
    heading.set_xalign(0.0);
    col.append(&heading);

    let loc = ctx.locales();
    let text = item
        .story()
        .map(|s| s.resolve(&loc).to_string())
        .filter(|s| !s.is_empty());
    let description = text.unwrap_or_else(|| item.description.resolve(&loc).to_string());
    let body = gtk::Label::new(Some(&description));
    body.set_wrap(true);
    body.set_xalign(0.0);
    body.set_selectable(true);
    col.append(&body);

    col.upcast()
}

fn links(item: &Item) -> Option<gtk::Widget> {
    let url = str_field(item, "github_url");
    if !is_web_url(url) {
        return None;
    }
    let button = gtk::Button::with_label(&tr!("View source code"));
    button.add_css_class("flat");
    button.set_halign(gtk::Align::Start);
    button.set_tooltip_text(Some(url));
    let url = url.to_string();
    button.connect_clicked(move |_| {
        let _ =
            gtk::gio::AppInfo::launch_default_for_uri(&url, None::<&gtk::gio::AppLaunchContext>);
    });
    Some(button.upcast())
}

fn security_group(item: &Item) -> Option<gtk::Widget> {
    let report = item.security_report()?;
    let group = adw::PreferencesGroup::new();
    group.set_title(&tr!("Security Audit"));

    let score = adw::ActionRow::new();
    score.set_title(&tr!("Audit Score"));
    let text = match (report.score, report.status.as_deref()) {
        (Some(s), Some(st)) => format!("{s}/100 ({st})"),
        (Some(s), None) => format!("{s}/100"),
        (None, Some(st)) => st.to_string(),
        (None, None) => String::new(),
    };
    if !text.is_empty() {
        let label = gtk::Label::new(Some(&text));
        label.add_css_class("store-info-value");
        score.add_suffix(&label);
        group.add(&score);
    }
    if let Some(by) = report.audited_by.filter(|b| !b.is_empty()) {
        let row = adw::ActionRow::new();
        row.set_title(&tr!("Audited By"));
        row.set_subtitle(&by);
        group.add(&row);
    }
    if let Some(summary) = report.summary.filter(|s| !s.is_empty()) {
        let row = adw::ActionRow::new();
        let label = gtk::Label::new(Some(&summary));
        label.set_wrap(true);
        label.set_xalign(0.0);
        label.set_selectable(true);
        label.set_margin_top(SPACE_M);
        label.set_margin_bottom(SPACE_M);
        label.set_margin_start(SPACE_L);
        label.set_margin_end(SPACE_L);
        row.set_child(Some(&label));
        group.add(&row);
    }
    Some(group.upcast())
}

/// The item's embedded docs, or its README downloaded from the repository.
fn docs_group(ctx: &Ctx, item: &Item) -> Option<gtk::Widget> {
    let readme = str_field(item, "readme_url");
    let inline = item.inline_docs();
    if inline.is_none() && !is_web_url(readme) {
        return None;
    }
    let view = gtk::TextView::new();
    view.set_editable(false);
    view.set_cursor_visible(false);
    view.set_monospace(true);
    view.set_wrap_mode(gtk::WrapMode::WordChar);
    view.set_top_margin(SPACE_M);
    view.set_bottom_margin(SPACE_M);
    view.set_left_margin(SPACE_L);
    view.set_right_margin(SPACE_L);

    match inline {
        Some(text) => view.buffer().set_text(text),
        None => {
            view.buffer().set_text(&tr!("Loading documentation…"));
            let weak = view.downgrade();
            ctx.load_text(readme.to_string(), README_LIMIT, move |text| {
                if let Some(v) = weak.upgrade() {
                    v.buffer().set_text(
                        &text.unwrap_or_else(|| tr!("Could not load the documentation.")),
                    );
                }
            });
        }
    }

    let scroll = gtk::ScrolledWindow::new();
    scroll.set_min_content_height(140);
    scroll.set_max_content_height(280);
    scroll.set_propagate_natural_height(true);
    scroll.set_child(Some(&view));
    scroll.add_css_class("store-card");

    let group = adw::PreferencesGroup::new();
    group.set_title(&tr!("Documentation & Preview"));
    group.add(&scroll);
    Some(group.upcast())
}

fn role_label(role: FileRole) -> String {
    match role {
        FileRole::SkillPrompt => tr!("Skill prompt, persona, and tool definitions"),
        FileRole::Manifest => tr!("Plugin manifest and configuration"),
        FileRole::PluginFiles => tr!("Plugin files"),
        FileRole::ExtensionMetadata => tr!("Extension metadata"),
        FileRole::ExtensionCode => tr!("Extension code"),
        FileRole::ExtensionStyle => tr!("Extension stylesheet"),
        FileRole::ExtensionSchemas => tr!("Settings schemas"),
        FileRole::AppFiles => tr!("Application files and runtime"),
        FileRole::DesktopEntry => tr!("Desktop launch entry"),
        FileRole::Binary => tr!("Application executable"),
    }
}

fn files_group(item: &Item) -> gtk::Widget {
    let group = adw::PreferencesGroup::new();
    group.set_title(&tr!("Package Files & Destination"));
    for f in package_files(item) {
        let row = adw::ActionRow::new();
        row.set_title(&f.path);
        row.set_subtitle(&role_label(f.role));
        row.set_title_selectable(true);
        row.add_prefix(&gtk::Image::from_icon_name(f.role.icon()));
        group.add(&row);
    }
    group.upcast()
}

fn details_group(item: &Item) -> gtk::Widget {
    let group = adw::PreferencesGroup::new();
    group.set_title(&tr!("Package Details"));
    for (title, value) in [
        (tr!("Package ID"), item.id.clone()),
        (tr!("Format"), kind_label(item.kind)),
        (tr!("Version"), item.version.clone().unwrap_or_default()),
        (tr!("Developer"), str_field(item, "author").to_string()),
    ] {
        if value.is_empty() {
            continue;
        }
        let row = adw::ActionRow::new();
        row.set_title(&title);
        row.set_subtitle(&value);
        row.set_subtitle_selectable(true);
        group.add(&row);
    }
    group.upcast()
}
