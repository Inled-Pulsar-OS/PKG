use crate::core::editions::editions;
use crate::core::i18n;
use crate::core::install::Action;
use crate::core::model::{EditionKey, InstallState, Item, SourceKind};
use crate::core::package_files::{package_files, FileRole};
use crate::core::remote::is_web_url;
use crate::ui::ctx::Change;
use crate::ui::page::{self, live_page};
use crate::ui::tokens::{SPACE_L, SPACE_M, SPACE_S, SPACE_XL};
use crate::ui::widget::{
    icon::item_icon,
    info_grid::{info_strip, InfoCell},
    install_button::InstallButton,
};
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
    // Live: the facts strip and the buttons depend on install state.
    live_page(ctx, 860, move |ctx, column| {
        let Some(item) = ctx.item(&id) else {
            column.append(&page::empty(
                "dialog-question-symbolic",
                &tr!("Item not found"),
                &tr!("This item is not in the catalog."),
            ));
            return;
        };
        column.append(&header(ctx, &item));
        column.append(&rule());
        column.append(&info_strip(&info_cells(
            &item,
            &ctx.state(&item.id),
            &ctx.locales(),
        )));
        column.append(&rule());
        if let Some(shots) = screenshots(ctx, &item) {
            column.append(&shots);
        }
        column.append(&platform_line());
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

/// A hairline between the header, the facts, and the screenshots.
fn rule() -> gtk::Widget {
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    line.add_css_class("store-rule");
    line.set_hexpand(true);
    line.upcast()
}

/// The "Linux" line under the screenshots, as the App Store shows "Mac".
fn platform_line() -> gtk::Widget {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_S);
    row.add_css_class("store-platform");
    row.append(&gtk::Image::from_icon_name("computer-symbolic"));
    row.append(&gtk::Label::new(Some(&tr!("Linux"))));
    row.upcast()
}

/// Bytes as a value and unit, such as `1.6` and `GB`. Uses powers of 1000, like the App Store.
pub fn format_size(bytes: u64) -> (String, &'static str) {
    const UNITS: [&str; 5] = ["bytes", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    let text = if unit == 0 || value >= 100.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    };
    // "43.0" reads as "43".
    let text = text.strip_suffix(".0").map_or(text.clone(), str::to_string);
    (text, UNITS[unit])
}

/// One short word for the big value in the facts strip.
fn short_kind(kind: SourceKind) -> String {
    match kind {
        SourceKind::Flatpak => tr!("App"),
        SourceKind::System => tr!("Package"),
        SourceKind::GnomeExtension => tr!("Add-on"),
        SourceKind::SayriSkill => tr!("Skill"),
        SourceKind::SayriPlugin => tr!("Plugin"),
        SourceKind::Unknown => String::new(),
    }
}

/// Short name of how an item is delivered, shown under its type.
fn delivery_label(kind: SourceKind) -> String {
    match kind {
        SourceKind::Flatpak => ptr!("edition", "Flatpak"),
        SourceKind::System => tr!("Native"),
        SourceKind::GnomeExtension => tr!("GNOME Shell"),
        SourceKind::SayriSkill | SourceKind::SayriPlugin => tr!("Sayri AI"),
        SourceKind::Unknown => String::new(),
    }
}

/// The facts strip: security score, version, type, developer, language, and size.
/// Each cell appears only when the catalog has the data.
pub fn info_cells(item: &Item, state: &InstallState, locales: &[&str]) -> Vec<InfoCell> {
    let mut cells = Vec::new();

    if let Some(score) = item.security_report().and_then(|r| r.score) {
        cells.push(InfoCell::text(
            tr!("Security"),
            score.to_string(),
            tr!("out of 100"),
        ));
    }

    let (version, status) = match state {
        InstallState::Installed { version: Some(v) } => (v.clone(), tr!("Installed")),
        InstallState::Installed { version: None } => {
            (item.version.clone().unwrap_or_default(), tr!("Installed"))
        }
        InstallState::UpdateAvailable { installed, .. } => {
            (installed.clone(), tr!("Update available"))
        }
        InstallState::NotInstalled => (item.version.clone().unwrap_or_default(), tr!("Available")),
    };
    cells.push(InfoCell::text(tr!("Version"), version, status));

    cells.push(InfoCell::text(
        tr!("Type"),
        short_kind(item.kind),
        delivery_label(item.kind),
    ));

    let author = str_field(item, "author");
    cells.push(InfoCell::icon(
        tr!("Developer"),
        "avatar-default-symbolic",
        author.to_string(),
    ));

    // Languages the catalog has text for.
    if let crate::core::model::LocalizedText::Map(map) = &item.summary {
        let first = map
            .keys()
            .find(|k| locales.iter().any(|l| l.eq_ignore_ascii_case(k)))
            .or_else(|| map.keys().next());
        if let Some(first) = first {
            let more = map.len().saturating_sub(1);
            let sub = if more > 0 {
                i18n::format(&tr!("+ {count} more"), &[("count", &more.to_string())])
            } else {
                String::new()
            };
            let lang = first
                .split(['-', '_'])
                .next()
                .unwrap_or(first)
                .to_uppercase();
            cells.push(InfoCell::text(tr!("Language"), lang, sub));
        }
    }

    if let Some(bytes) = item.extra.get("size").and_then(|v| v.as_u64()) {
        let (value, unit) = format_size(bytes);
        cells.push(InfoCell::text(tr!("Size"), value, unit.to_string()));
    }
    cells
}

fn header(ctx: &Ctx, item: &Item) -> gtk::Widget {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_XL);
    row.append(&item_icon(ctx, item, 128));

    let meta = gtk::Box::new(gtk::Orientation::Vertical, SPACE_S);
    meta.set_hexpand(true);
    meta.set_valign(gtk::Align::Center);
    let name = gtk::Label::new(Some(&ctx.name(item)));
    name.add_css_class("store-product-title");
    name.set_xalign(0.0);
    name.set_wrap(true);
    name.set_accessible_role(gtk::AccessibleRole::Heading);
    meta.append(&name);
    let summary = gtk::Label::new(Some(&ctx.summary(item)));
    summary.add_css_class("store-product-subtitle");
    summary.set_xalign(0.0);
    summary.set_wrap(true);
    summary.set_lines(2);
    summary.set_ellipsize(gtk::pango::EllipsizeMode::End);
    meta.append(&summary);

    // Get sits under the text, with Uninstall beside it once the item is installed.
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_M);
    actions.set_margin_top(SPACE_M);
    actions.set_halign(gtk::Align::Start);
    actions.append(InstallButton::prominent(ctx, item).widget());

    let remove = gtk::Button::with_label(&tr!("Uninstall"));
    remove.add_css_class("flat");
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
    meta.append(&actions);
    row.append(&meta);
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
    // Large rounded images side by side, scrolling sideways when they do not fit.
    let strip = gtk::Box::new(gtk::Orientation::Horizontal, SPACE_XL);
    strip.add_css_class("store-shots");
    for url in urls.iter().cloned() {
        let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
        frame.add_css_class("store-shot");
        frame.set_overflow(gtk::Overflow::Hidden);
        frame.set_size_request(SHOT_WIDTH, SHOT_HEIGHT);
        let picture = gtk::Picture::new();
        picture.set_content_fit(gtk::ContentFit::Cover);
        picture.set_hexpand(true);
        picture.set_vexpand(true);
        picture.set_accessible_role(gtk::AccessibleRole::Img);
        frame.append(&picture);
        strip.append(&frame);
        let weak = picture.downgrade();
        ctx.load_image(url, move |path| {
            if let (Some(p), Some(pic)) = (path, weak.upgrade()) {
                pic.set_filename(Some(p));
            }
        });
    }

    let scroll = gtk::ScrolledWindow::new();
    scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
    scroll.set_child(Some(&strip));
    scroll.set_min_content_height(SHOT_HEIGHT + 16);

    // Arrow keys scroll to the neighbouring screenshot.
    scroll.set_focusable(true);
    let keys = gtk::EventControllerKey::new();
    {
        let scroll = scroll.clone();
        keys.connect_key_pressed(move |_, key, _, _| {
            let step = match key {
                gtk::gdk::Key::Left => -1.0,
                gtk::gdk::Key::Right => 1.0,
                _ => return gtk::glib::Propagation::Proceed,
            };
            let adj = scroll.hadjustment();
            let page = f64::from(SHOT_WIDTH + SPACE_XL);
            adj.set_value(
                (adj.value() + step * page).clamp(adj.lower(), adj.upper() - adj.page_size()),
            );
            gtk::glib::Propagation::Stop
        });
    }
    scroll.add_controller(keys);
    Some(scroll.upcast())
}

const SHOT_WIDTH: i32 = 410;
const SHOT_HEIGHT: i32 = 256;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_use_powers_of_one_thousand() {
        assert_eq!(format_size(1_600_000_000), ("1.6".to_string(), "GB"));
        assert_eq!(format_size(512), ("512".to_string(), "bytes"));
        assert_eq!(format_size(43_000_000), ("43".to_string(), "MB"));
        assert_eq!(format_size(250_000_000), ("250".to_string(), "MB"));
    }

    fn item(json: &str) -> Item {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn info_cells_only_include_what_the_catalog_knows() {
        let bare = item(r#"{"id":"a","type":"flatpak","version":"1.0"}"#);
        let labels = |cells: &[InfoCell]| cells.iter().map(|c| c.label.clone()).collect::<Vec<_>>();
        let cells = info_cells(&bare, &InstallState::NotInstalled, &[]);
        assert_eq!(labels(&cells), ["Version", "Type", "Developer"]);

        let rich = item(
            r#"{"id":"b","type":"sayri_plugin","version":"2.0","author":"Ana","size":1600000000,
                "summary":{"en":"x","es":"y","fr":"z"},"security_report":{"score":90}}"#,
        );
        let cells = info_cells(
            &rich,
            &InstallState::Installed {
                version: Some("2.0".into()),
            },
            &["es"],
        );
        assert_eq!(
            labels(&cells),
            [
                "Security",
                "Version",
                "Type",
                "Developer",
                "Language",
                "Size"
            ]
        );
        let language = cells.iter().find(|c| c.label == "Language").unwrap();
        assert!(
            matches!(&language.value, crate::ui::widget::info_grid::InfoValue::Text(t) if t == "ES")
        );
        assert_eq!(language.sub, "+ 2 more");
    }
}
