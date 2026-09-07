//! Construcción de la interfaz GTK4.
use crate::demo::is_demo_mode;
use crate::icons::{create_button_with_icon, create_icon_widget, get_lucide_icon_path};
use crate::manifest::{fetch_release_manifest, local_manifest_fallback};
use crate::models::{
    BtrfsTarget, DownloadMsg, ManifestData, NetConnType, RecoveryMode, RecoveryUpdate,
    WifiNetwork,
};
use crate::network::{
    connect_wifi, get_network_status, open_external_network_settings, scan_wifi_networks,
};
use crate::restore::run_restoration;
use crate::system::{
    detect_system_base, detect_system_bootloader, find_btrfs_targets, format_file_size,
    is_valid_base_squashfs, log_msg, scan_usb_devices,
};
use crate::theme::APP_CSS;
use glib::clone;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, Box as GtkBox, Button, CenterBox, CssProvider, DropDown, GestureClick,
    Label, ListBox, ListBoxRow, Orientation, PasswordEntry, ProgressBar, ScrolledWindow,
    SelectionMode, Stack, StackTransitionType, StringList, TextView, WrapMode,
};
use libadwaita::prelude::*;
use libadwaita::ApplicationWindow;
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;
pub(crate) fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title(if is_demo_mode() { "Pulsar OS Recovery (MODO DEMO / PRUEBAS)" } else { "Pulsar OS Recovery" })
        .default_width(1024)
        .default_height(720)
        .resizable(true)
        .build();

    window.maximize();

    let style_mgr = libadwaita::StyleManager::default();
    style_mgr.set_color_scheme(libadwaita::ColorScheme::ForceDark);

    let provider = CssProvider::new();
    let _ = provider.load_from_string(APP_CSS);
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER + 500,
        );
    }

    let root_box = GtkBox::new(Orientation::Vertical, 0);
    root_box.add_css_class("root-container");
    root_box.set_hexpand(true);
    root_box.set_vexpand(true);

    let center_box = CenterBox::new();
    center_box.set_hexpand(true);
    center_box.set_vexpand(true);

    let card_box = GtkBox::new(Orientation::Vertical, 0);
    card_box.add_css_class("apple-box");
    card_box.set_size_request(680, 580);
    card_box.set_valign(Align::Center);
    card_box.set_halign(Align::Center);

    let stack = Stack::new();
    stack.set_transition_type(StackTransitionType::Crossfade);
    stack.set_transition_duration(250);
    card_box.append(&stack);

    center_box.set_center_widget(Some(&card_box));
    root_box.append(&center_box);

    // Bottom center control buttons: Restart & Shut Down
    let bottom_bar = GtkBox::new(Orientation::Horizontal, 16);
    bottom_bar.set_halign(Align::Center);
    bottom_bar.set_valign(Align::End);
    bottom_bar.set_margin_bottom(24);

    if is_demo_mode() {
        let demo_pill = GtkBox::new(Orientation::Horizontal, 6);
        demo_pill.add_css_class("badge-demo");
        let warn_icon = create_icon_widget("", "alert-triangle", 14);
        let warn_lbl = Label::new(Some("DEMO / SIMULATION MODE (No disk changes)"));
        demo_pill.append(&warn_icon);
        demo_pill.append(&warn_lbl);
        bottom_bar.append(&demo_pill);
    }

    let btn_restart = create_button_with_icon("Restart", "restart", 18, "bottom-power-btn");
    btn_restart.connect_clicked(|_| {
        let _ = Command::new("sh")
            .arg("-c")
            .arg("systemctl reboot || reboot || sudo reboot")
            .spawn();
    });

    let btn_shutdown = create_button_with_icon("Shut Down", "shutdown", 18, "bottom-power-btn");
    btn_shutdown.connect_clicked(|_| {
        let _ = Command::new("sh")
            .arg("-c")
            .arg("systemctl poweroff || poweroff || sudo poweroff")
            .spawn();
    });

    bottom_bar.append(&btn_restart);
    bottom_bar.append(&btn_shutdown);
    root_box.append(&bottom_bar);

    window.set_content(Some(&root_box));

    // Shared state
    let selected_action: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let selected_target: Rc<RefCell<Option<BtrfsTarget>>> = Rc::new(RefCell::new(None));
    let selected_image_path: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let recovery_mode: Rc<RefCell<RecoveryMode>> = Rc::new(RefCell::new(RecoveryMode::Local));
    let current_browser_dir: Rc<RefCell<PathBuf>> = Rc::new(RefCell::new(PathBuf::from("/media")));

    // ─────────────────────────────────────────────────────────────
    // 1. Utilities Screen (macOS Recovery main view)
    // ─────────────────────────────────────────────────────────────
    let util_box = GtkBox::new(Orientation::Vertical, 10);
    util_box.set_valign(Align::Center);

    let header_lbl = Label::new(Some("Pulsar OS Recovery Utilities"));
    header_lbl.add_css_class("welcome-title");
    util_box.append(&header_lbl);

    let sub_lbl = Label::new(Some("Select a recovery utility to restore or repair your system."));
    sub_lbl.add_css_class("welcome-subtitle");
    util_box.append(&sub_lbl);

    let listbox = ListBox::new();
    listbox.add_css_class("transparent-list");
    listbox.set_selection_mode(SelectionMode::Single);
    listbox.set_show_separators(false);

    let add_row = |id: &str, title: &str, desc: &str, icon_file: &str, icon_fallback: &str| {
        let row = ListBoxRow::new();
        row.set_widget_name(id);
        row.add_css_class("utility-item-row");

        let card = GtkBox::new(Orientation::Horizontal, 16);
        card.add_css_class("utility-row-card");
        card.set_margin_top(3);
        card.set_margin_bottom(3);
        card.set_margin_start(2);
        card.set_margin_end(2);

        let icon = create_icon_widget(icon_file, icon_fallback, 44);
        card.append(&icon);

        let vbox = GtkBox::new(Orientation::Vertical, 2);
        vbox.set_valign(Align::Center);

        let title_l = Label::new(Some(title));
        title_l.add_css_class("utility-title-lbl");
        title_l.set_halign(Align::Start);
        vbox.append(&title_l);

        let desc_l = Label::new(Some(desc));
        desc_l.add_css_class("utility-desc-lbl");
        desc_l.set_halign(Align::Start);
        desc_l.set_wrap(true);
        vbox.append(&desc_l);

        card.append(&vbox);
        row.set_child(Some(&card));
        listbox.append(&row);
    };

    add_row(
        "timemachine",
        "Restore from Time Machine (Btrfs + Restic)",
        "Restore full system or personal files from a Time Machine backup (USB, Samba/NAS or Cloud Rclone).",
        "/usr/share/pulsaros-recovery/timemachine.png",
        "restore",
    );
    add_row(
        "reinstall",
        "Reinstall Pulsar OS (Local Partition)",
        "Install a fresh copy of Pulsar OS from built-in recovery while keeping personal files intact.",
        "/usr/share/pulsaros-recovery/reinstall.png",
        "restore",
    );
    add_row(
        "usb_restore",
        "Restore from USB Flash Drive",
        "Scan connected USB drives to find and restore a downloaded .squashfs system image.",
        "",
        "usb",
    );
    add_row(
        "internet_info",
        "Pulsar Internet Recovery",
        "Download latest recovery image from SourceForge CDN and reinstall directly over the internet.",
        "/usr/share/pulsaros-recovery/safari.png",
        "safari",
    );
    add_row(
        "disk",
        "Disk Utility (GParted)",
        "Repair, inspect, format, or resize disk partitions with GParted.",
        "/usr/share/pulsaros-recovery/diskutility.png",
        "hard-drive",
    );
    add_row(
        "terminal",
        "Terminal / Root Console",
        "Open a root terminal for manual diagnosis and advanced commands.",
        "/usr/share/pulsaros-recovery/terminal.png",
        "terminal",
    );

    util_box.append(&listbox);

    let util_btn_box = GtkBox::new(Orientation::Horizontal, 0);
    util_btn_box.set_margin_top(14);
    let util_spacer = GtkBox::new(Orientation::Horizontal, 0);
    util_spacer.set_hexpand(true);
    util_btn_box.append(&util_spacer);

    let btn_util_continue = Button::with_label("Continue");
    btn_util_continue.add_css_class("suggested-action");
    btn_util_continue.set_sensitive(false);
    util_btn_box.append(&btn_util_continue);
    util_box.append(&util_btn_box);
    stack.add_named(&util_box, Some("utilities"));

    // ─────────────────────────────────────────────────────────────
    // 2. Internet Recovery Screen (Direct Cloud Download & Restore)
    // ─────────────────────────────────────────────────────────────
    let net_info_box = GtkBox::new(Orientation::Vertical, 8);
    net_info_box.set_valign(Align::Center);
    net_info_box.set_halign(Align::Center);

    let net_icon = create_icon_widget("/usr/share/pulsaros-recovery/safari.png", "globe", 48);
    net_info_box.append(&net_icon);

    let net_title = Label::new(Some("Pulsar Internet Recovery"));
    net_title.add_css_class("welcome-title");
    net_info_box.append(&net_title);

    let net_subtitle = Label::new(Some("Download official system recovery image from SourceForge CDN and reinstall."));
    net_subtitle.add_css_class("welcome-subtitle");
    net_info_box.append(&net_subtitle);

    let net_card = GtkBox::new(Orientation::Vertical, 10);
    net_card.add_css_class("info-card");
    net_card.set_size_request(580, -1);

    let detected_base = detect_system_base();
    let detected_boot = detect_system_bootloader();
    // Build the UI immediately from the local fallback manifest (no network
    // blocking), then refresh the version/mirror dropdowns in background once
    // the remote manifest (if any) is fetched.
    let manifest_rc = Rc::new(RefCell::new(local_manifest_fallback()));

    // Row 1: Edition and Version
    let net_row_1 = GtkBox::new(Orientation::Horizontal, 14);
    net_row_1.set_hexpand(true);

    let edition_box = GtkBox::new(Orientation::Vertical, 3);
    edition_box.set_hexpand(true);
    let lbl_edition = Label::new(Some("Distribution & Bootloader"));
    lbl_edition.add_css_class("setting-label");
    lbl_edition.set_halign(Align::Start);
    edition_box.append(&lbl_edition);

    let edition_entries = [
        "Arch Linux (GRUB)",
        "Arch Linux (rEFInd)",
        "Debian (GRUB)",
        "Debian (rEFInd)",
    ];
    let edition_list = StringList::new(&edition_entries);
    let combo_edition = DropDown::new(Some(edition_list), gtk4::Expression::NONE);
    combo_edition.set_hexpand(true);

    let default_edition_idx = match (detected_base.as_str(), detected_boot.as_str()) {
        ("arch", "grub") => 0,
        ("arch", "refind") => 1,
        ("debian", "grub") => 2,
        ("debian", "refind") => 3,
        _ => 0,
    };
    combo_edition.set_selected(default_edition_idx);
    edition_box.append(&combo_edition);
    net_row_1.append(&edition_box);

    let version_box = GtkBox::new(Orientation::Vertical, 3);
    version_box.set_hexpand(true);
    let lbl_version = Label::new(Some("Version"));
    lbl_version.add_css_class("setting-label");
    lbl_version.set_halign(Align::Start);
    version_box.append(&lbl_version);

    let ver_str = manifest_rc.borrow().latest_version.clone();
    let version_entries = [ver_str.as_str()];
    let version_list = StringList::new(&version_entries);
    let combo_version = DropDown::new(Some(version_list), gtk4::Expression::NONE);
    combo_version.set_hexpand(true);
    version_box.append(&combo_version);
    net_row_1.append(&version_box);
    net_card.append(&net_row_1);

    // Row 2: Mirror and Network status
    let net_row_2 = GtkBox::new(Orientation::Horizontal, 14);
    net_row_2.set_hexpand(true);

    let mirror_box = GtkBox::new(Orientation::Vertical, 3);
    mirror_box.set_hexpand(true);
    let lbl_mirror = Label::new(Some("SourceForge Mirror"));
    lbl_mirror.add_css_class("setting-label");
    lbl_mirror.set_halign(Align::Start);
    mirror_box.append(&lbl_mirror);

    let mirror_names: Vec<String> = manifest_rc.borrow().mirrors.iter().map(|m| m.name.clone()).collect();
    let mirror_strs: Vec<&str> = mirror_names.iter().map(|s| s.as_str()).collect();
    let mirror_list = StringList::new(&mirror_strs);
    let combo_mirror = DropDown::new(Some(mirror_list), gtk4::Expression::NONE);
    combo_mirror.set_hexpand(true);
    mirror_box.append(&combo_mirror);
    net_row_2.append(&mirror_box);

    let net_stat_box = GtkBox::new(Orientation::Vertical, 3);
    net_stat_box.set_hexpand(true);
    let lbl_net_hdr = Label::new(Some("Network Status"));
    lbl_net_hdr.add_css_class("setting-label");
    lbl_net_hdr.set_halign(Align::Start);
    net_stat_box.append(&lbl_net_hdr);

    let net_badge_row = GtkBox::new(Orientation::Horizontal, 8);
    let net_badge_inner = GtkBox::new(Orientation::Horizontal, 6);
    net_badge_inner.set_halign(Align::Start);
    net_badge_inner.set_hexpand(true);
    let img_net_badge = create_icon_widget("", "wifi-off", 16);
    let lbl_net_badge = Label::new(Some("Detecting network..."));
    lbl_net_badge.add_css_class("badge-net-warn");
    lbl_net_badge.set_halign(Align::Start);
    lbl_net_badge.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    net_badge_inner.append(&img_net_badge);
    net_badge_inner.append(&lbl_net_badge);
    net_badge_row.append(&net_badge_inner);

    let btn_configure_wifi = create_button_with_icon("Wi-Fi Settings...", "wifi", 16, "secondary-action");
    net_badge_row.append(&btn_configure_wifi);

    net_stat_box.append(&net_badge_row);
    net_row_2.append(&net_stat_box);
    net_card.append(&net_row_2);

    // Progress Section
    let pbar_net = ProgressBar::new();
    pbar_net.add_css_class("progress-bar-thin");
    pbar_net.set_fraction(0.0);
    pbar_net.set_margin_top(8);
    net_card.append(&pbar_net);

    let lbl_net_status = Label::new(Some("Ready to download (~3.1 GB). System root (@) will be restored, keeping @home intact."));
    lbl_net_status.add_css_class("progress-text");
    lbl_net_status.set_halign(Align::Start);
    lbl_net_status.set_wrap(true);
    net_card.append(&lbl_net_status);

    net_info_box.append(&net_card);

    // Refetch the release manifest in background. The UI already shows the
    // local fallback, so this never blocks; it only refreshes the version and
    // mirror dropdowns if a remote manifest is found.
    {
        let (tx, rx) = std::sync::mpsc::channel::<ManifestData>();
        let manifest_rc_c = manifest_rc.clone();
        let combo_version_c = combo_version.clone();
        let combo_mirror_c = combo_mirror.clone();

        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            match rx.try_recv() {
                Ok(manifest) => {
                    *manifest_rc_c.borrow_mut() = manifest.clone();
                    let ver = manifest.latest_version.clone();
                    let mirror_names: Vec<&str> = manifest.mirrors.iter().map(|m| m.name.as_str()).collect();
                    let version_list = StringList::new(&[ver.as_str()]);
                    combo_version_c.set_model(Some(&version_list));
                    let mirror_list = StringList::new(&mirror_names);
                    combo_mirror_c.set_model(Some(&mirror_list));
                    glib::ControlFlow::Break
                }
                Err(_) => glib::ControlFlow::Continue,
            }
        });

        std::thread::spawn(move || {
            if let Some(m) = fetch_release_manifest() {
                let _ = tx.send(m);
            }
        });
    }

    let net_btn_box = GtkBox::new(Orientation::Horizontal, 14);
    net_btn_box.set_halign(Align::Center);
    net_btn_box.set_margin_top(6);

    let btn_net_back = Button::with_label("Back to Utilities");
    btn_net_back.add_css_class("secondary-action");
    net_btn_box.append(&btn_net_back);

    let btn_net_cancel = Button::with_label("Cancel Download");
    btn_net_cancel.add_css_class("destructive-action");
    btn_net_cancel.set_visible(false);
    net_btn_box.append(&btn_net_cancel);

    let btn_net_download = Button::with_label("Download and Reinstall");
    btn_net_download.add_css_class("suggested-action");
    net_btn_box.append(&btn_net_download);

    net_info_box.append(&net_btn_box);
    stack.add_named(&net_info_box, Some("internet_info"));

    // ─────────────────────────────────────────────────────────────
    // 2b. Wi-Fi Configuration Screen (Scan, Select & Connect)
    // ─────────────────────────────────────────────────────────────
    let wifi_box = GtkBox::new(Orientation::Vertical, 8);
    wifi_box.set_valign(Align::Center);
    wifi_box.set_halign(Align::Center);

    let wifi_icon = create_icon_widget("", "wifi", 48);
    wifi_box.append(&wifi_icon);

    let wifi_title = Label::new(Some("Wi-Fi Networks"));
    wifi_title.add_css_class("welcome-title");
    wifi_box.append(&wifi_title);

    let wifi_subtitle = Label::new(Some("Select a wireless network to connect this device to the Internet."));
    wifi_subtitle.add_css_class("welcome-subtitle");
    wifi_box.append(&wifi_subtitle);

    // Top action bar
    let wifi_top_bar = GtkBox::new(Orientation::Horizontal, 10);
    wifi_top_bar.set_size_request(580, -1);
    wifi_top_bar.set_margin_bottom(4);

    let btn_wifi_refresh = create_button_with_icon("Scan Networks", "refresh", 16, "secondary-action");
    wifi_top_bar.append(&btn_wifi_refresh);

    let wifi_top_spacer = GtkBox::new(Orientation::Horizontal, 0);
    wifi_top_spacer.set_hexpand(true);
    wifi_top_bar.append(&wifi_top_spacer);

    let btn_wifi_advanced = create_button_with_icon("Advanced Settings...", "settings", 16, "secondary-action");
    wifi_top_bar.append(&btn_wifi_advanced);
    wifi_box.append(&wifi_top_bar);

    // Scrolled network list
    let wifi_scrolled = ScrolledWindow::new();
    wifi_scrolled.set_size_request(580, 200);
    wifi_scrolled.add_css_class("live-log-view");

    let wifi_listbox = ListBox::new();
    wifi_listbox.add_css_class("transparent-list");
    wifi_scrolled.set_child(Some(&wifi_listbox));
    wifi_box.append(&wifi_scrolled);

    // Inline password connection card
    let wifi_conn_card = GtkBox::new(Orientation::Vertical, 8);
    wifi_conn_card.add_css_class("info-card");
    wifi_conn_card.set_size_request(580, -1);
    wifi_conn_card.set_visible(false);

    let lbl_conn_target = Label::new(Some("Enter Network Password"));
    lbl_conn_target.add_css_class("setting-label");
    lbl_conn_target.set_halign(Align::Start);
    wifi_conn_card.append(&lbl_conn_target);

    let wifi_entry_row = GtkBox::new(Orientation::Horizontal, 10);
    let entry_wifi_pw = PasswordEntry::new();
    entry_wifi_pw.set_hexpand(true);
    entry_wifi_pw.set_placeholder_text(Some("Password..."));
    entry_wifi_pw.set_show_peek_icon(true);
    wifi_entry_row.append(&entry_wifi_pw);

    let btn_wifi_do_connect = Button::with_label("Connect");
    btn_wifi_do_connect.add_css_class("suggested-action");
    wifi_entry_row.append(&btn_wifi_do_connect);

    let btn_wifi_cancel_connect = Button::with_label("Cancel");
    btn_wifi_cancel_connect.add_css_class("secondary-action");
    wifi_entry_row.append(&btn_wifi_cancel_connect);
    wifi_conn_card.append(&wifi_entry_row);

    let lbl_wifi_status = Label::new(None);
    lbl_wifi_status.add_css_class("progress-text");
    lbl_wifi_status.set_halign(Align::Start);
    lbl_wifi_status.set_wrap(true);
    wifi_conn_card.append(&lbl_wifi_status);

    wifi_box.append(&wifi_conn_card);

    // Bottom navigation bar
    let wifi_nav_box = GtkBox::new(Orientation::Horizontal, 14);
    wifi_nav_box.set_halign(Align::Center);
    wifi_nav_box.set_margin_top(8);

    let btn_wifi_back = create_button_with_icon("Back to Internet Recovery", "arrow-left", 16, "secondary-action");
    wifi_nav_box.append(&btn_wifi_back);
    wifi_box.append(&wifi_nav_box);

    stack.add_named(&wifi_box, Some("wifi_select"));

    // ─────────────────────────────────────────────────────────────
    // 3. USB Image Selector Screen (Auto-detected USBs)
    // ─────────────────────────────────────────────────────────────
    let usb_box = GtkBox::new(Orientation::Vertical, 10);
    usb_box.set_valign(Align::Center);

    let usb_header = Label::new(Some("Select Recovery Image from USB"));
    usb_header.add_css_class("welcome-title");
    usb_box.append(&usb_header);

    let usb_sub = Label::new(Some("Plug in your USB drive with the .squashfs file, then select it below."));
    usb_sub.add_css_class("welcome-subtitle");
    usb_box.append(&usb_sub);

    // USB Actions bar
    let usb_actions_bar = GtkBox::new(Orientation::Horizontal, 10);
    usb_actions_bar.set_margin_bottom(6);

    let btn_scan_usb = create_button_with_icon("Scan / Refresh USBs", "refresh", 16, "secondary-action");
    usb_actions_bar.append(&btn_scan_usb);

    let btn_open_browser = create_button_with_icon("Browse Files / Drives...", "folder", 16, "secondary-action");
    usb_actions_bar.append(&btn_open_browser);

    usb_box.append(&usb_actions_bar);

    let usb_scrolled = ScrolledWindow::new();
    usb_scrolled.set_size_request(600, 230);
    usb_scrolled.add_css_class("live-log-view");

    let usb_listbox = ListBox::new();
    usb_listbox.add_css_class("transparent-list");
    usb_listbox.set_selection_mode(SelectionMode::Single);
    usb_scrolled.set_child(Some(&usb_listbox));
    usb_box.append(&usb_scrolled);

    let usb_nav_box = GtkBox::new(Orientation::Horizontal, 16);
    usb_nav_box.set_halign(Align::End);
    usb_nav_box.set_margin_top(12);

    let btn_usb_back = Button::with_label("Back");
    btn_usb_back.add_css_class("secondary-action");
    btn_usb_back.connect_clicked(clone!(@weak stack => move |_| {
        stack.set_visible_child_name("utilities");
    }));
    usb_nav_box.append(&btn_usb_back);

    let btn_usb_continue = Button::with_label("Continue");
    btn_usb_continue.add_css_class("suggested-action");
    btn_usb_continue.set_sensitive(false);
    usb_nav_box.append(&btn_usb_continue);
    usb_box.append(&usb_nav_box);

    stack.add_named(&usb_box, Some("usb_select"));

    // ─────────────────────────────────────────────────────────────
    // 4. Built-in File Browser Screen (No XDG portal required!)
    // ─────────────────────────────────────────────────────────────
    let browser_box = GtkBox::new(Orientation::Vertical, 8);
    browser_box.set_valign(Align::Center);

    let browser_header = Label::new(Some("Browse Storage for System Image"));
    browser_header.add_css_class("welcome-title");
    browser_box.append(&browser_header);

    // Current Path & Quick Jump buttons
    let browser_top_bar = GtkBox::new(Orientation::Horizontal, 8);
    browser_top_bar.set_valign(Align::Center);

    let lbl_current_path = Label::new(Some("Path: /media"));
    lbl_current_path.add_css_class("progress-text");
    lbl_current_path.set_hexpand(true);
    lbl_current_path.set_halign(Align::Start);
    lbl_current_path.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    browser_top_bar.append(&lbl_current_path);

    let shortcuts = [
        ("/media", "media"),
        ("/run/media", "run/media"),
        ("/mnt", "mnt"),
        ("/tmp", "tmp"),
        ("/", "root (/)"),
    ];

    let shortcuts_bar = GtkBox::new(Orientation::Horizontal, 4);
    shortcuts_bar.set_halign(Align::End);

    browser_top_bar.append(&shortcuts_bar);
    browser_box.append(&browser_top_bar);

    let browser_scrolled = ScrolledWindow::new();
    browser_scrolled.set_size_request(600, 240);
    browser_scrolled.add_css_class("live-log-view");

    let browser_listbox = ListBox::new();
    browser_listbox.add_css_class("transparent-list");
    browser_listbox.set_selection_mode(SelectionMode::Single);
    browser_scrolled.set_child(Some(&browser_listbox));
    browser_box.append(&browser_scrolled);

    let browser_nav_box = GtkBox::new(Orientation::Horizontal, 16);
    browser_nav_box.set_halign(Align::End);
    browser_nav_box.set_margin_top(10);

    let btn_browser_back = Button::with_label("Back to USB List");
    btn_browser_back.add_css_class("secondary-action");
    btn_browser_back.connect_clicked(clone!(@weak stack => move |_| {
        stack.set_visible_child_name("usb_select");
    }));
    browser_nav_box.append(&btn_browser_back);

    let btn_browser_select = Button::with_label("Select Image");
    btn_browser_select.add_css_class("suggested-action");
    btn_browser_select.set_sensitive(false);
    browser_nav_box.append(&btn_browser_select);
    browser_box.append(&browser_nav_box);

    stack.add_named(&browser_box, Some("file_browser"));

    // ─────────────────────────────────────────────────────────────
    // 5. Select Target Partition Screen
    // ─────────────────────────────────────────────────────────────
    let target_box = GtkBox::new(Orientation::Vertical, 10);
    target_box.set_valign(Align::Center);
    target_box.set_halign(Align::Center);

    let target_icon = create_icon_widget("/usr/share/pulsaros-recovery/diskutility.png", "hard-drive", 56);
    target_box.append(&target_icon);

    let target_title = Label::new(Some("Select Pulsar OS Partition"));
    target_title.add_css_class("welcome-title");
    target_box.append(&target_title);

    let target_desc = Label::new(Some("The root system (@) will be cleanly restored. Your user accounts and documents in /home (@home) will remain completely intact."));
    target_desc.add_css_class("welcome-subtitle");
    target_desc.set_wrap(true);
    target_desc.set_max_width_chars(50);
    target_desc.set_justify(gtk4::Justification::Center);
    target_box.append(&target_desc);

    let source_img_lbl = Label::new(Some("Source: Built-in Recovery Partition"));
    source_img_lbl.add_css_class("progress-text");
    target_box.append(&source_img_lbl);

    let targets_flow = GtkBox::new(Orientation::Horizontal, 10);
    targets_flow.set_halign(Align::Center);
    targets_flow.set_margin_top(8);
    target_box.append(&targets_flow);

    let target_nav_box = GtkBox::new(Orientation::Horizontal, 16);
    target_nav_box.set_halign(Align::Center);
    target_nav_box.set_margin_top(14);

    let btn_target_back = Button::with_label("Back");
    btn_target_back.add_css_class("secondary-action");
    target_nav_box.append(&btn_target_back);

    let btn_target_restore = Button::with_label("Restore System");
    btn_target_restore.add_css_class("suggested-action");
    btn_target_restore.set_sensitive(false);
    target_nav_box.append(&btn_target_restore);
    target_box.append(&target_nav_box);

    stack.add_named(&target_box, Some("target_select"));

    // ─────────────────────────────────────────────────────────────
    // 6. Progress Screen
    // ─────────────────────────────────────────────────────────────
    let prog_box = GtkBox::new(Orientation::Vertical, 10);
    prog_box.set_valign(Align::Center);
    prog_box.set_halign(Align::Center);

    let prog_icon = create_icon_widget("/usr/share/pulsaros-recovery/reinstall.png", "progress", 60);
    prog_box.append(&prog_icon);

    let prog_title = Label::new(Some("Restoring Pulsar OS..."));
    prog_title.add_css_class("welcome-title");
    prog_box.append(&prog_title);

    let prog_desc = Label::new(Some("Preparing disk and restoring root subvolume (@)..."));
    prog_desc.add_css_class("progress-text");
    prog_box.append(&prog_desc);

    let pbar = ProgressBar::new();
    pbar.add_css_class("progress-bar-thin");
    pbar.set_size_request(480, -1);
    prog_box.append(&pbar);

    let scrolled_log = ScrolledWindow::new();
    scrolled_log.set_size_request(520, 160);
    scrolled_log.add_css_class("live-log-view");

    let log_view = TextView::new();
    log_view.set_editable(false);
    log_view.set_monospace(true);
    log_view.set_wrap_mode(WrapMode::WordChar);
    log_view.add_css_class("live-log-text");
    scrolled_log.set_child(Some(&log_view));
    prog_box.append(&scrolled_log);

    stack.add_named(&prog_box, Some("progress"));

    // ─────────────────────────────────────────────────────────────
    // 7. Complete Screen
    // ─────────────────────────────────────────────────────────────
    let done_box = GtkBox::new(Orientation::Vertical, 14);
    done_box.set_valign(Align::Center);
    done_box.set_halign(Align::Center);

    let done_icon = create_icon_widget("", "complete", 72);
    done_box.append(&done_icon);

    let done_title = Label::new(Some("Restoration Complete"));
    done_title.add_css_class("welcome-title");
    done_box.append(&done_title);

    let done_desc = Label::new(Some("Pulsar OS has been successfully restored.\nYour personal files, settings, and apps in /home are intact.\n\nClick Restart to boot into your restored system."));
    done_desc.add_css_class("welcome-subtitle");
    done_desc.set_justify(gtk4::Justification::Center);
    done_box.append(&done_desc);

    let btn_reboot = Button::with_label("Restart System");
    btn_reboot.add_css_class("suggested-action");
    btn_reboot.connect_clicked(|_| {
        let _ = Command::new("sudo").args(&["-n", "systemctl", "reboot", "-i", "-f"]).spawn();
        let _ = Command::new("sudo").args(&["-n", "reboot", "-f"]).spawn();
    });
    done_box.append(&btn_reboot);

    stack.add_named(&done_box, Some("complete"));

    // ─────────────────────────────────────────────────────────────
    // 8. Error Screen
    // ─────────────────────────────────────────────────────────────
    let err_box = GtkBox::new(Orientation::Vertical, 10);
    err_box.set_valign(Align::Center);
    err_box.set_halign(Align::Center);

    let err_icon = create_icon_widget("", "error", 64);
    err_box.append(&err_icon);

    let err_title = Label::new(Some("Restoration Failed"));
    err_title.add_css_class("welcome-title");
    err_box.append(&err_title);

    let err_msg_lbl = Label::new(Some("An error occurred during system restoration."));
    err_msg_lbl.add_css_class("welcome-subtitle");
    err_msg_lbl.set_wrap(true);
    err_msg_lbl.set_max_width_chars(50);
    err_msg_lbl.set_justify(gtk4::Justification::Center);
    err_box.append(&err_msg_lbl);

    let err_scrolled_log = ScrolledWindow::new();
    err_scrolled_log.set_size_request(520, 140);
    err_scrolled_log.add_css_class("live-log-view");

    let err_log_view = TextView::new();
    err_log_view.set_editable(false);
    err_log_view.set_monospace(true);
    err_log_view.set_wrap_mode(WrapMode::WordChar);
    err_log_view.add_css_class("err-log-text");
    err_scrolled_log.set_child(Some(&err_log_view));
    err_box.append(&err_scrolled_log);

    let err_btn_box = GtkBox::new(Orientation::Horizontal, 12);
    err_btn_box.set_halign(Align::Center);
    err_btn_box.set_margin_top(8);

    let btn_err_back = Button::with_label("Back to Utilities");
    btn_err_back.add_css_class("secondary-action");
    btn_err_back.connect_clicked(clone!(@weak stack => move |_| {
        stack.set_visible_child_name("utilities");
    }));
    err_btn_box.append(&btn_err_back);

    let btn_try_usb = Button::with_label("Try USB Image");
    btn_try_usb.add_css_class("suggested-action");
    let sel_act_c = selected_action.clone();
    btn_try_usb.connect_clicked(clone!(@weak stack, @weak btn_util_continue => move |_| {
        *sel_act_c.borrow_mut() = Some("usb_restore".to_string());
        btn_util_continue.emit_clicked();
    }));
    err_btn_box.append(&btn_try_usb);

    err_box.append(&err_btn_box);
    stack.add_named(&err_box, Some("error"));

    // ─────────────────────────────────────────────────────────────
    // Helper: Refresh Target Partitions & Show Target Screen
    // ─────────────────────────────────────────────────────────────
    let show_target_screen = {
        let stack = stack.clone();
        let targets_flow = targets_flow.clone();
        let btn_target_restore = btn_target_restore.clone();
        let selected_target = selected_target.clone();
        let source_img_lbl = source_img_lbl.clone();

        move |source_desc: &str| {
            source_img_lbl.set_text(source_desc);

            while let Some(child) = targets_flow.first_child() {
                targets_flow.remove(&child);
            }
            *selected_target.borrow_mut() = None;
            btn_target_restore.set_sensitive(false);

            let mut targets = find_btrfs_targets();
            if targets.is_empty() && is_demo_mode() {
                targets.push(BtrfsTarget {
                    _disk_path: "/dev/demo-nvme0n1".to_string(),
                    part_path: "/dev/demo-nvme0n1p2 (Simulado)".to_string(),
                    label: "Pulsar OS Demo Pool".to_string(),
                    uuid: "demo-btrfs-uuid-0000".to_string(),
                    size: "500.0G".to_string(),
                });
            }

            if targets.is_empty() {
                let no_target_lbl = Label::new(Some("No Btrfs Pulsar OS partitions detected.\nUse Disk Utility to inspect drives."));
                no_target_lbl.add_css_class("welcome-subtitle");
                targets_flow.append(&no_target_lbl);
            } else {
                for target in targets {
                    let card = GtkBox::new(Orientation::Vertical, 6);
                    card.add_css_class("disk-card");
                    let disk_icon = create_icon_widget("", "hard-drive", 40);
                    card.append(&disk_icon);

                    let name_lbl = Label::new(Some(&format!("{} ({})", target.label, target.size)));
                    name_lbl.add_css_class("utility-title-lbl");
                    card.append(&name_lbl);

                    let dev_lbl = Label::new(Some(&target.part_path));
                    dev_lbl.add_css_class("utility-desc-lbl");
                    card.append(&dev_lbl);

                    let gesture = GestureClick::new();
                    let t_clone = target.clone();
                    let targets_flow_c = targets_flow.clone();
                    let btn_restore_c = btn_target_restore.clone();
                    let sel_target_c = selected_target.clone();
                    let card_c = card.clone();

                    gesture.connect_released(move |_, _, _, _| {
                        let mut next = targets_flow_c.first_child();
                        while let Some(w) = next {
                            w.remove_css_class("selected");
                            next = w.next_sibling();
                        }
                        card_c.add_css_class("selected");
                        *sel_target_c.borrow_mut() = Some(t_clone.clone());
                        btn_restore_c.set_sensitive(true);
                    });

                    card.add_controller(gesture);
                    targets_flow.append(&card);
                }
            }
            stack.set_visible_child_name("target_select");
        }
    };

    // ─────────────────────────────────────────────────────────────
    // Helper: Refresh USB Images list
    // ─────────────────────────────────────────────────────────────
    let populate_usb_images = {
        let usb_listbox = usb_listbox.clone();
        let btn_usb_continue = btn_usb_continue.clone();
        let selected_image_path = selected_image_path.clone();

        move || {
            while let Some(child) = usb_listbox.first_child() {
                usb_listbox.remove(&child);
            }
            *selected_image_path.borrow_mut() = None;
            btn_usb_continue.set_sensitive(false);

            let images = scan_usb_devices();
            if images.is_empty() {
                let row = ListBoxRow::new();
                row.set_selectable(false);
                let empty_box = GtkBox::new(Orientation::Vertical, 6);
                empty_box.set_margin_top(16);
                empty_box.set_margin_bottom(16);
                let empty_lbl = Label::new(Some("No .squashfs recovery images detected on connected USB drives."));
                empty_lbl.add_css_class("welcome-subtitle");
                let hint_lbl = Label::new(Some("Plug in your USB drive and click 'Scan / Refresh USBs' or use 'Browse Files / Drives...'"));
                hint_lbl.add_css_class("utility-desc-lbl");
                empty_box.append(&empty_lbl);
                empty_box.append(&hint_lbl);
                row.set_child(Some(&empty_box));
                usb_listbox.append(&row);
            } else {
                for img in images {
                    let row = ListBoxRow::new();
                    row.set_widget_name(&img.file_path);
                    row.add_css_class("utility-item-row");

                    let card = GtkBox::new(Orientation::Horizontal, 14);
                    card.add_css_class("utility-row-card");
                    card.set_margin_top(3);
                    card.set_margin_bottom(3);

                    let icon = create_icon_widget("", "usb", 36);
                    card.append(&icon);

                    let vbox = GtkBox::new(Orientation::Vertical, 2);
                    let title_l = Label::new(Some(&format!("{} ({})", img.filename, img.size_str)));
                    title_l.add_css_class("utility-title-lbl");
                    title_l.set_halign(Align::Start);
                    vbox.append(&title_l);

                    let desc_l = Label::new(Some(&format!("On {} • {}", img.device_label, img.file_path)));
                    desc_l.add_css_class("utility-desc-lbl");
                    desc_l.set_halign(Align::Start);
                    desc_l.set_wrap(true);
                    vbox.append(&desc_l);

                    card.append(&vbox);
                    row.set_child(Some(&card));
                    usb_listbox.append(&row);
                }
            }
        }
    };

    // ─────────────────────────────────────────────────────────────
    // Helper: Built-in Directory Browser Populator
    // ─────────────────────────────────────────────────────────────
    let populate_file_browser = {
        let browser_listbox = browser_listbox.clone();
        let lbl_current_path = lbl_current_path.clone();
        let btn_browser_select = btn_browser_select.clone();
        let selected_image_path = selected_image_path.clone();
        let current_browser_dir = current_browser_dir.clone();

        Rc::new(RefCell::new(move |dir: &Path| {
            while let Some(child) = browser_listbox.first_child() {
                browser_listbox.remove(&child);
            }
            *selected_image_path.borrow_mut() = None;
            btn_browser_select.set_sensitive(false);

            let canonical_dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
            *current_browser_dir.borrow_mut() = canonical_dir.clone();
            lbl_current_path.set_text(&format!("Path: {}", canonical_dir.display()));

            // Parent directory row if not root
            if let Some(parent) = canonical_dir.parent() {
                let row = ListBoxRow::new();
                row.set_widget_name(&format!("DIR:{}", parent.display()));
                row.add_css_class("utility-item-row");

                let card = GtkBox::new(Orientation::Horizontal, 12);
                card.add_css_class("utility-row-card");
                card.set_margin_top(2);
                card.set_margin_bottom(2);

                let icon = create_icon_widget("", "folder-up", 24);
                card.append(&icon);

                let lbl = Label::new(Some(".. (Go to parent directory)"));
                lbl.add_css_class("utility-title-lbl");
                card.append(&lbl);

                row.set_child(Some(&card));
                browser_listbox.append(&row);
            }

            // Read entries
            let mut dirs_list: Vec<PathBuf> = Vec::new();
            let mut squashfs_list: Vec<PathBuf> = Vec::new();

            if let Ok(entries) = fs::read_dir(&canonical_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                    if name.starts_with('.') {
                        continue;
                    }
                    if p.is_dir() {
                        dirs_list.push(p);
                    } else if p.is_file() && (name.ends_with(".squashfs") || name.ends_with(".sfs")) {
                        squashfs_list.push(p);
                    }
                }
            }

            dirs_list.sort();
            squashfs_list.sort();

            // Append directories
            for d in dirs_list {
                let row = ListBoxRow::new();
                row.set_widget_name(&format!("DIR:{}", d.display()));
                row.add_css_class("utility-item-row");

                let card = GtkBox::new(Orientation::Horizontal, 12);
                card.add_css_class("utility-row-card");
                card.set_margin_top(2);
                card.set_margin_bottom(2);

                let icon = create_icon_widget("", "folder", 24);
                card.append(&icon);

                let dname = d.file_name().and_then(|n| n.to_str()).unwrap_or("Directory");
                let lbl = Label::new(Some(&format!("{}/", dname)));
                lbl.add_css_class("utility-title-lbl");
                card.append(&lbl);

                row.set_child(Some(&card));
                browser_listbox.append(&row);
            }

            // Append squashfs files
            for f in squashfs_list {
                let full_p = f.to_string_lossy().to_string();
                let row = ListBoxRow::new();
                row.set_widget_name(&format!("FILE:{}", full_p));
                row.add_css_class("utility-item-row");

                let card = GtkBox::new(Orientation::Horizontal, 12);
                card.add_css_class("utility-row-card");
                card.set_margin_top(2);
                card.set_margin_bottom(2);

                let icon = create_icon_widget("", "package", 28);
                card.append(&icon);

                let vbox = GtkBox::new(Orientation::Vertical, 2);
                let fname = f.file_name().and_then(|n| n.to_str()).unwrap_or("squashfs");
                let size_str = if let Ok(meta) = fs::metadata(&f) { format_file_size(meta.len()) } else { "".to_string() };
                let is_valid = is_valid_base_squashfs(&full_p);

                let title_l = Label::new(Some(&format!("{} ({})", fname, size_str)));
                title_l.add_css_class("utility-title-lbl");
                title_l.set_halign(Align::Start);
                vbox.append(&title_l);

                let desc_l = Label::new(Some(if is_valid { "Valid Pulsar OS system image" } else { "Image smaller than standard base size" }));
                desc_l.add_css_class("utility-desc-lbl");
                desc_l.set_halign(Align::Start);
                vbox.append(&desc_l);

                card.append(&vbox);
                row.set_child(Some(&card));
                browser_listbox.append(&row);
            }
        }))
    };

    // Setup shortcuts buttons in file browser top bar
    for (path_str, btn_title) in shortcuts {
        let s_btn = Button::with_label(btn_title);
        s_btn.add_css_class("shortcut-btn");
        let pop_c = populate_file_browser.clone();
        let target_p = PathBuf::from(path_str);
        s_btn.connect_clicked(move |_| {
            (pop_c.borrow_mut())(&target_p);
        });
        shortcuts_bar.append(&s_btn);
    }

    // Connect file browser listbox row selection and activation
    let pop_c2 = populate_file_browser.clone();
    browser_listbox.connect_row_activated(clone!(
        @weak btn_browser_select,
        @strong selected_image_path
     => move |_, row| {
        let tag = row.widget_name().to_string();
        if let Some(dir_path) = tag.strip_prefix("DIR:") {
            (pop_c2.borrow_mut())(Path::new(dir_path));
        } else if let Some(file_path) = tag.strip_prefix("FILE:") {
            *selected_image_path.borrow_mut() = Some(file_path.to_string());
            btn_browser_select.set_sensitive(true);
            btn_browser_select.emit_clicked();
        }
    }));

    browser_listbox.connect_row_selected(clone!(
        @weak btn_browser_select,
        @strong selected_image_path
     => move |_, row| {
        if let Some(r) = row {
            let tag = r.widget_name().to_string();
            if let Some(file_path) = tag.strip_prefix("FILE:") {
                *selected_image_path.borrow_mut() = Some(file_path.to_string());
                btn_browser_select.set_sensitive(true);
                return;
            }
        }
        btn_browser_select.set_sensitive(false);
    }));

    // ─────────────────────────────────────────────────────────────
    // Callbacks & Connections
    // ─────────────────────────────────────────────────────────────
    listbox.connect_row_selected(clone!(@weak btn_util_continue, @strong selected_action => move |_, row| {
        if let Some(r) = row {
            let id = r.widget_name().to_string();
            *selected_action.borrow_mut() = Some(id);
            btn_util_continue.set_sensitive(true);
        }
    }));

    listbox.connect_row_activated(clone!(@weak btn_util_continue, @strong selected_action => move |_, row| {
        let id = row.widget_name().to_string();
        *selected_action.borrow_mut() = Some(id);
        btn_util_continue.emit_clicked();
    }));

    btn_scan_usb.connect_clicked(clone!(@strong populate_usb_images => move |_| {
        populate_usb_images();
    }));

    let pop_browser_first = populate_file_browser.clone();
    btn_open_browser.connect_clicked(clone!(@weak stack => move |_| {
        let initial_dir = if Path::new("/media").exists() {
            Path::new("/media")
        } else if Path::new("/run/media").exists() {
            Path::new("/run/media")
        } else {
            Path::new("/")
        };
        (pop_browser_first.borrow_mut())(initial_dir);
        stack.set_visible_child_name("file_browser");
    }));

    usb_listbox.connect_row_selected(clone!(@weak btn_usb_continue, @strong selected_image_path => move |_, row| {
        if let Some(r) = row {
            let path = r.widget_name().to_string();
            if !path.is_empty() {
                *selected_image_path.borrow_mut() = Some(path);
                btn_usb_continue.set_sensitive(true);
                return;
            }
        }
        btn_usb_continue.set_sensitive(false);
    }));

    usb_listbox.connect_row_activated(clone!(@weak btn_usb_continue, @strong selected_image_path => move |_, row| {
        let path = row.widget_name().to_string();
        if !path.is_empty() {
            *selected_image_path.borrow_mut() = Some(path);
            btn_usb_continue.emit_clicked();
        }
    }));

    let cancel_signal_holder = Rc::new(RefCell::new(None::<Arc<AtomicBool>>));

    // ─────────────────────────────────────────────────────────────
    // Real Network Status & Wi-Fi Management Callbacks
    // ─────────────────────────────────────────────────────────────
    let update_network_badge = {
        let lbl_badge = lbl_net_badge.clone();
        let img_badge = img_net_badge.clone();
        let btn_dl = btn_net_download.clone();
        let lbl_status = lbl_net_status.clone();
        Rc::new(move || {
            let st = get_network_status();
            lbl_badge.remove_css_class("badge-net-ok");
            lbl_badge.remove_css_class("badge-net-warn");
            lbl_badge.remove_css_class("badge-net-err");

            if st.is_connected {
                lbl_badge.add_css_class("badge-net-ok");
                let icon_name = match st.conn_type {
                    NetConnType::Wifi => "wifi",
                    NetConnType::Ethernet => "ethernet",
                    NetConnType::None => "wifi",
                };
                let icon_p = get_lucide_icon_path(icon_name);
                img_badge.set_from_file(Some(&icon_p));
                lbl_badge.set_text(&st.conn_name);
                btn_dl.set_sensitive(true);
            } else {
                lbl_badge.add_css_class("badge-net-warn");
                let icon_p = get_lucide_icon_path("wifi-off");
                img_badge.set_from_file(Some(&icon_p));
                lbl_badge.set_text("Offline / Disconnected");
                if !is_demo_mode() {
                    btn_dl.set_sensitive(false);
                    lbl_status.set_text("No internet connection detected. Please connect to Wi-Fi or Ethernet.");
                }
            }
        })
    };

    let selected_wifi_ssid = Rc::new(RefCell::new(String::new()));

    let populate_wifi_list = {
        let wifi_listbox = wifi_listbox.clone();
        let wifi_conn_card = wifi_conn_card.clone();
        let lbl_conn_target = lbl_conn_target.clone();
        let entry_wifi_pw = entry_wifi_pw.clone();
        let lbl_wifi_status = lbl_wifi_status.clone();
        let selected_wifi_ssid = selected_wifi_ssid.clone();
        let btn_wifi_do_connect = btn_wifi_do_connect.clone();

        Rc::new(move || {
            // clear current list
            while let Some(child) = wifi_listbox.first_child() {
                wifi_listbox.remove(&child);
            }
            wifi_conn_card.set_visible(false);

            // show a placeholder while scanning in the background
            let scanning_row = ListBoxRow::new();
            scanning_row.set_selectable(false);
            let lbl_scanning = Label::new(Some("Scanning Wi-Fi networks..."));
            lbl_scanning.add_css_class("progress-text");
            lbl_scanning.set_margin_top(20);
            lbl_scanning.set_margin_bottom(20);
            scanning_row.set_child(Some(&lbl_scanning));
            wifi_listbox.append(&scanning_row);

            // run scan on a background thread so the UI stays responsive
            let (tx, rx) = std::sync::mpsc::channel::<Vec<WifiNetwork>>();

            // clone the widgets for the inner timeout closure so the outer
            // closure stays `Fn` (its captures are not consumed)
            let box_c = wifi_listbox.clone();
            let card_c = wifi_conn_card.clone();
            let target_lbl_c = lbl_conn_target.clone();
            let pw_entry_c = entry_wifi_pw.clone();
            let status_lbl_c = lbl_wifi_status.clone();
            let sel_c = selected_wifi_ssid.clone();
            let do_btn_c = btn_wifi_do_connect.clone();

            glib::timeout_add_local(std::time::Duration::from_millis(80), move || {
                match rx.try_recv() {
                    Ok(nets) => {
                        // remove the scanning placeholder
                        while let Some(child) = box_c.first_child() {
                            box_c.remove(&child);
                        }

                        if nets.is_empty() {
                            let empty_row = ListBoxRow::new();
                            empty_row.set_selectable(false);
                            let lbl_empty = Label::new(Some("No Wi-Fi networks found. Click 'Scan Networks' or configure manually."));
                            lbl_empty.add_css_class("progress-text");
                            lbl_empty.set_margin_top(20);
                            lbl_empty.set_margin_bottom(20);
                            empty_row.set_child(Some(&lbl_empty));
                            box_c.append(&empty_row);
                            return glib::ControlFlow::Break;
                        }

                        for net in nets {
                            let row = ListBoxRow::new();
                            row.set_widget_name(&net.ssid);

                            let card = GtkBox::new(Orientation::Horizontal, 12);
                            card.add_css_class("wifi-card-row");

                            let sig_icon_name = if net.signal >= 50 { "wifi" } else { "wifi-low" };
                            let icon_sig = create_icon_widget("", sig_icon_name, 20);
                            card.append(&icon_sig);

                            let info_box = GtkBox::new(Orientation::Vertical, 2);
                            info_box.set_hexpand(true);

                            let lbl_ssid = Label::new(Some(&net.ssid));
                            lbl_ssid.add_css_class("wifi-ssid-text");
                            lbl_ssid.set_halign(Align::Start);
                            info_box.append(&lbl_ssid);

                            let sub_box = GtkBox::new(Orientation::Horizontal, 8);
                            let lbl_sig_pct = Label::new(Some(&format!("Signal: {}%", net.signal)));
                            lbl_sig_pct.add_css_class("wifi-signal-text");
                            sub_box.append(&lbl_sig_pct);

                            let sec_box = GtkBox::new(Orientation::Horizontal, 4);
                            sec_box.add_css_class("wifi-badge-security");
                            let is_open = net.security.to_lowercase().contains("open") || net.security.is_empty();
                            let lock_icon_name = if is_open { "unlock" } else { "lock" };
                            let lock_icon = create_icon_widget("", lock_icon_name, 12);
                            sec_box.append(&lock_icon);
                            let lbl_sec = Label::new(Some(if is_open { "Open" } else { &net.security }));
                            sec_box.append(&lbl_sec);
                            sub_box.append(&sec_box);

                            info_box.append(&sub_box);
                            card.append(&info_box);

                            if net.in_use {
                                let conn_box = GtkBox::new(Orientation::Horizontal, 6);
                                conn_box.add_css_class("wifi-badge-connected");
                                let check_icon = create_icon_widget("", "check", 14);
                                let lbl_conn = Label::new(Some("Connected"));
                                conn_box.append(&check_icon);
                                conn_box.append(&lbl_conn);
                                card.append(&conn_box);
                            } else {
                                let btn_row_connect = Button::with_label("Connect");
                                btn_row_connect.add_css_class("shortcut-btn");

                                let ssid_clone = net.ssid.clone();
                                let sec_clone = net.security.clone();
                                let card_row_c = card_c.clone();
                                let target_row_c = target_lbl_c.clone();
                                let pw_row_c = pw_entry_c.clone();
                                let status_row_c = status_lbl_c.clone();
                                let sel_row_c = sel_c.clone();
                                let do_row_c = do_btn_c.clone();

                                btn_row_connect.connect_clicked(move |_| {
                                    *sel_row_c.borrow_mut() = ssid_clone.clone();
                                    card_row_c.set_visible(true);
                                    status_row_c.set_text("");
                                    if sec_clone.to_lowercase().contains("open") || sec_clone.is_empty() {
                                        target_row_c.set_text(&format!("Connect to open network '{}'", ssid_clone));
                                        pw_row_c.set_visible(false);
                                    } else {
                                        target_row_c.set_text(&format!("Enter password for '{}'", ssid_clone));
                                        pw_row_c.set_visible(true);
                                        pw_row_c.set_text("");
                                        pw_row_c.grab_focus();
                                    }
                                    do_row_c.set_sensitive(true);
                                });

                                card.append(&btn_row_connect);
                            }

                            row.set_child(Some(&card));
                            box_c.append(&row);
                        }

                        glib::ControlFlow::Break
                    }
                    Err(_) => glib::ControlFlow::Continue,
                }
            });

            // launch the actual nmcli scan in a background thread
            std::thread::spawn(move || {
                let nets = scan_wifi_networks();
                let _ = tx.send(nets);
            });
        })
    };

    let do_connect_action = {
        let selected_wifi_ssid = selected_wifi_ssid.clone();
        let entry_wifi_pw = entry_wifi_pw.clone();
        let lbl_wifi_status = lbl_wifi_status.clone();
        let btn_wifi_do_connect = btn_wifi_do_connect.clone();
        let btn_wifi_cancel_connect = btn_wifi_cancel_connect.clone();
        let wifi_conn_card = wifi_conn_card.clone();
        let populate_wifi_list = populate_wifi_list.clone();
        let update_network_badge = update_network_badge.clone();

        Rc::new(move || {
            let ssid = selected_wifi_ssid.borrow().clone();
            if ssid.is_empty() {
                return;
            }

            let pw = entry_wifi_pw.text().to_string();
            let pw_opt = if pw.is_empty() { None } else { Some(pw) };

            btn_wifi_do_connect.set_sensitive(false);
            btn_wifi_cancel_connect.set_sensitive(false);
            lbl_wifi_status.set_text(&format!("Connecting to '{}'...", ssid));

            let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();

            let btn_do_c = btn_wifi_do_connect.clone();
            let btn_cancel_c = btn_wifi_cancel_connect.clone();
            let lbl_st_c = lbl_wifi_status.clone();
            let card_c = wifi_conn_card.clone();
            let pop_c = populate_wifi_list.clone();
            let badge_c = update_network_badge.clone();

            glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                match rx.try_recv() {
                    Ok(res) => {
                        btn_do_c.set_sensitive(true);
                        btn_cancel_c.set_sensitive(true);
                        match res {
                            Ok(()) => {
                                lbl_st_c.set_text("Connected successfully.");
                                badge_c();
                                let pop_delay = pop_c.clone();
                                let card_delay = card_c.clone();
                                glib::timeout_add_local(std::time::Duration::from_millis(1200), move || {
                                    pop_delay();
                                    card_delay.set_visible(false);
                                    glib::ControlFlow::Break
                                });
                            }
                            Err(e) => {
                                lbl_st_c.set_text(&format!("Connection error: {}", e));
                            }
                        }
                        glib::ControlFlow::Break
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        btn_do_c.set_sensitive(true);
                        btn_cancel_c.set_sensitive(true);
                        glib::ControlFlow::Break
                    }
                }
            });

            thread::spawn(move || {
                let res = connect_wifi(&ssid, pw_opt.as_deref());
                let _ = tx.send(res);
            });
        })
    };

    let do_conn_1 = do_connect_action.clone();
    btn_wifi_do_connect.connect_clicked(move |_| {
        do_conn_1();
    });

    let do_conn_2 = do_connect_action.clone();
    entry_wifi_pw.connect_activate(move |_| {
        do_conn_2();
    });

    btn_wifi_cancel_connect.connect_clicked(clone!(@weak wifi_conn_card => move |_| {
        wifi_conn_card.set_visible(false);
    }));

    btn_wifi_refresh.connect_clicked(clone!(@strong populate_wifi_list => move |_| {
        populate_wifi_list();
    }));

    btn_wifi_advanced.connect_clicked(move |_| {
        open_external_network_settings();
    });

    btn_configure_wifi.connect_clicked(clone!(@weak stack, @strong populate_wifi_list => move |_| {
        populate_wifi_list();
        stack.set_visible_child_name("wifi_select");
    }));

    btn_wifi_back.connect_clicked(clone!(@weak stack, @strong update_network_badge => move |_| {
        update_network_badge();
        stack.set_visible_child_name("internet_info");
    }));

    // Initial check and periodic polling every 3 seconds
    update_network_badge();
    glib::timeout_add_local(std::time::Duration::from_millis(3000), clone!(@strong update_network_badge => move || {
        update_network_badge();
        glib::ControlFlow::Continue
    }));

    btn_net_back.connect_clicked(clone!(@weak stack => move |_| {
        stack.set_visible_child_name("utilities");
    }));

    btn_net_cancel.connect_clicked(clone!(
        @strong cancel_signal_holder,
        @weak lbl_net_status,
        @weak pbar_net,
        @weak btn_net_download,
        @weak btn_net_back,
        @weak btn_net_cancel,
        @weak combo_edition,
        @weak combo_mirror,
        @weak combo_version,
        @weak stack
     => move |_| {
        if let Some(ref sig) = *cancel_signal_holder.borrow() {
            sig.store(true, Ordering::SeqCst);
        }
        let _ = fs::remove_file("/tmp/pulsaros-internet-recovery.squashfs.part");
        let _ = fs::remove_file("/tmp/pulsaros-internet-recovery.squashfs");
        pbar_net.set_fraction(0.0);
        lbl_net_status.set_text("Descarga cancelada por el usuario.");
        btn_net_download.set_visible(true);
        btn_net_download.set_sensitive(true);
        btn_net_back.set_visible(true);
        btn_net_back.set_sensitive(true);
        btn_net_cancel.set_visible(false);
        combo_edition.set_sensitive(true);
        combo_mirror.set_sensitive(true);
        combo_version.set_sensitive(true);
        stack.set_visible_child_name("utilities");
    }));

    btn_net_download.connect_clicked(clone!(
        @weak pbar_net,
        @weak lbl_net_status,
        @weak btn_net_download,
        @weak btn_net_back,
        @weak btn_net_cancel,
        @weak combo_edition,
        @weak combo_mirror,
        @weak combo_version,
        @strong manifest_rc,
        @strong selected_image_path,
        @strong recovery_mode,
        @strong show_target_screen,
        @strong cancel_signal_holder,
        @weak stack
     => move |_| {
        btn_net_download.set_visible(false);
        btn_net_back.set_visible(false);
        btn_net_cancel.set_visible(true);
        btn_net_cancel.set_sensitive(true);
        combo_edition.set_sensitive(false);
        combo_mirror.set_sensitive(false);
        combo_version.set_sensitive(false);

        let cancel_flag = Arc::new(AtomicBool::new(false));
        *cancel_signal_holder.borrow_mut() = Some(cancel_flag.clone());

        let (sel_base, sel_boot) = match combo_edition.selected() {
            0 => ("arch", "grub"),
            1 => ("arch", "refind"),
            2 => ("debian", "grub"),
            3 => ("debian", "refind"),
            _ => ("arch", "grub"),
        };

        let m_data = manifest_rc.borrow().clone();
        let ver = m_data.latest_version.clone();
        let mirror_id = m_data.mirrors.get(combo_mirror.selected() as usize)
            .map(|m| m.id.clone())
            .unwrap_or_else(|| "auto".to_string());

        let target_info = m_data.versions
            .get(&ver)
            .and_then(|b_map| b_map.get(sel_base))
            .and_then(|bt_map| bt_map.get(sel_boot))
            .cloned();

        let mut download_url = target_info.as_ref()
            .map(|t| t.squashfs.clone())
            .unwrap_or_else(|| {
                format!("https://downloads.sourceforge.net/project/pulsaros-inled/pulsaros-{}-{}-{}-{}.squashfs", ver, sel_base, sel_boot, ver)
            });

        if mirror_id != "auto" {
            download_url = format!("{}?use_mirror={}", download_url, mirror_id);
        }

        let expected_size = target_info.as_ref().and_then(|t| t.size_bytes).unwrap_or(3_145_728_000);
        let expected_hash = target_info.as_ref().and_then(|t| t.sha256.clone());

        pbar_net.set_fraction(0.01);
        lbl_net_status.set_text("Connecting to SourceForge CDN...");

        let (tx, rx) = std::sync::mpsc::channel::<DownloadMsg>();

        let pbar_dl = pbar_net.clone();
        let lbl_status_dl = lbl_net_status.clone();
        let btn_dl = btn_net_download.clone();
        let btn_bk = btn_net_back.clone();
        let btn_cn = btn_net_cancel.clone();
        let combo_ed = combo_edition.clone();
        let combo_mr = combo_mirror.clone();
        let combo_ver = combo_version.clone();
        let sel_img = selected_image_path.clone();
        let rec_mode = recovery_mode.clone();
        let show_tgt = show_target_screen.clone();
        let stack_dl = stack.clone();

        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    DownloadMsg::Progress { downloaded, total, speed_mb_s, fraction, eta_secs } => {
                        pbar_dl.set_fraction(fraction);
                        let down_mb = (downloaded as f64) / 1_048_576.0;
                        let tot_mb = (total as f64) / 1_048_576.0;
                        let eta_str = if eta_secs >= 60 {
                            format!("{} min {} s", eta_secs / 60, eta_secs % 60)
                        } else {
                            format!("{} s", eta_secs)
                        };
                        lbl_status_dl.set_text(&format!(
                            "Downloading: {:.1} MB / {:.1} MB ({:.0}%) — {:.2} MB/s (ETA: {})",
                            down_mb, tot_mb, fraction * 100.0, speed_mb_s, eta_str
                        ));
                    }
                    DownloadMsg::Verifying => {
                        pbar_dl.set_fraction(0.98);
                        lbl_status_dl.set_text("Verifying image checksum (SHA256)...");
                    }
                    DownloadMsg::Done => {
                        pbar_dl.set_fraction(1.0);
                        if is_demo_mode() {
                            lbl_status_dl.set_text("[DEMO MODE] Download and SHA256 verification completed! /tmp/pulsaros-internet-recovery.squashfs ready. No disk changes made.");
                        } else {
                            lbl_status_dl.set_text("Download complete and verified!");
                        }
                        btn_dl.set_visible(true);
                        btn_dl.set_sensitive(true);
                        btn_bk.set_visible(true);
                        btn_bk.set_sensitive(true);
                        btn_cn.set_visible(false);
                        *sel_img.borrow_mut() = Some("/tmp/pulsaros-internet-recovery.squashfs".to_string());
                        *rec_mode.borrow_mut() = RecoveryMode::CustomImage("/tmp/pulsaros-internet-recovery.squashfs".to_string());
                        let src_str = if is_demo_mode() {
                            "Source: Internet Recovery [DEMO MODE - Safe]"
                        } else {
                            "Source: Internet Recovery (SourceForge CDN)"
                        };
                        show_tgt(src_str);
                        return glib::ControlFlow::Break;
                    }
                    DownloadMsg::Cancelled => {
                        pbar_dl.set_fraction(0.0);
                        lbl_status_dl.set_text("Download cancelled.");
                        btn_dl.set_visible(true);
                        btn_dl.set_sensitive(true);
                        btn_bk.set_visible(true);
                        btn_bk.set_sensitive(true);
                        btn_cn.set_visible(false);
                        combo_ed.set_sensitive(true);
                        combo_mr.set_sensitive(true);
                        combo_ver.set_sensitive(true);
                        stack_dl.set_visible_child_name("utilities");
                        return glib::ControlFlow::Break;
                    }
                    DownloadMsg::Error(err) => {
                        pbar_dl.set_fraction(0.0);
                        lbl_status_dl.set_text(&format!("Error: {}", err));
                        btn_dl.set_visible(true);
                        btn_dl.set_sensitive(true);
                        btn_bk.set_visible(true);
                        btn_bk.set_sensitive(true);
                        btn_cn.set_visible(false);
                        combo_ed.set_sensitive(true);
                        combo_mr.set_sensitive(true);
                        combo_ver.set_sensitive(true);
                        return glib::ControlFlow::Break;
                    }
                }
            }
            glib::ControlFlow::Continue
        });

        thread::spawn(move || {
            log_msg(&format!("Starting Internet Recovery download from: {}", download_url));
            let tmp_part = "/tmp/pulsaros-internet-recovery.squashfs.part";
            let tmp_final = "/tmp/pulsaros-internet-recovery.squashfs";
            let _ = fs::remove_file(tmp_part);
            let _ = fs::remove_file(tmp_final);

            let mut child = match Command::new("curl")
                .args(&[
                    "-sSL",
                    "-L",
                    "--connect-timeout", "10",
                    "--retry", "3",
                    "-o", tmp_part,
                    &download_url,
                ])
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(DownloadMsg::Error(format!("Could not run curl: {}", e)));
                    return;
                }
            };

            let mut last_size: u64 = 0;
            let mut last_time = Instant::now();

            loop {
                thread::sleep(std::time::Duration::from_millis(200));

                if cancel_flag.load(Ordering::SeqCst) {
                    log_msg("Download cancelled by user. Terminating curl and cleaning temporary files...");
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = fs::remove_file(tmp_part);
                    let _ = fs::remove_file(tmp_final);
                    let _ = tx.send(DownloadMsg::Cancelled);
                    return;
                }

                match child.try_wait() {
                    Ok(Some(status)) => {
                        if cancel_flag.load(Ordering::SeqCst) {
                            let _ = fs::remove_file(tmp_part);
                            let _ = fs::remove_file(tmp_final);
                            let _ = tx.send(DownloadMsg::Cancelled);
                            return;
                        }

                        if status.success() {
                            let _ = fs::rename(tmp_part, tmp_final);
                            let _ = tx.send(DownloadMsg::Verifying);

                            // Calculate SHA-256
                            let mut calculated_hash = String::new();
                            if let Ok(out) = Command::new("sha256sum").arg(tmp_final).output() {
                                let hash_out = String::from_utf8_lossy(&out.stdout);
                                calculated_hash = hash_out.split_whitespace().next().unwrap_or("").to_string();
                                log_msg(&format!("Calculated SHA-256 for downloaded image: {}", calculated_hash));
                            }

                            if let Some(ref expected_h) = expected_hash {
                                let trimmed_exp = expected_h.trim();
                                if !trimmed_exp.is_empty() {
                                    if !calculated_hash.eq_ignore_ascii_case(trimmed_exp) {
                                        log_msg(&format!("Error: SHA-256 mismatch! Got: '{}', Expected: '{}'", calculated_hash, trimmed_exp));
                                        let _ = fs::remove_file(tmp_final);
                                        let _ = tx.send(DownloadMsg::Error(format!(
                                             "SHA256 checksum mismatch!\nCalculated: {}\nExpected: {}",
                                             calculated_hash, trimmed_exp
                                        )));
                                        return;
                                    } else {
                                        log_msg("SHA-256 checksum verified successfully.");
                                    }
                                }
                            }

                            let _ = tx.send(DownloadMsg::Done);
                        } else {
                            let _ = tx.send(DownloadMsg::Error(format!("curl failed with status {:?}", status)));
                        }
                        break;
                    }
                    Ok(None) => {
                        let cur_size = fs::metadata(tmp_part).map(|m| m.len()).unwrap_or(0);
                        let now = Instant::now();
                        let elapsed = now.duration_since(last_time).as_secs_f64();
                        if elapsed >= 0.3 {
                            let diff = cur_size.saturating_sub(last_size);
                            let speed_mb_s = (diff as f64 / 1_048_576.0) / elapsed;
                            let rem_bytes = expected_size.saturating_sub(cur_size);
                            let eta_secs = if speed_mb_s > 0.05 {
                                ((rem_bytes as f64 / 1_048_576.0) / speed_mb_s) as u64
                            } else {
                                0
                            };
                            let fraction = if expected_size > 0 {
                                (cur_size as f64 / expected_size as f64).clamp(0.0, 0.95)
                            } else {
                                0.0
                            };
                            let _ = tx.send(DownloadMsg::Progress {
                                downloaded: cur_size,
                                total: expected_size,
                                speed_mb_s,
                                fraction,
                                eta_secs,
                            });
                            last_size = cur_size;
                            last_time = now;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(DownloadMsg::Error(format!("Error monitoring download: {}", e)));
                        break;
                    }
                }
            }
        });
    }));

    btn_usb_continue.connect_clicked(clone!(
        @strong selected_image_path,
        @strong recovery_mode,
        @strong show_target_screen
     => move |_| {
        if let Some(img_path) = selected_image_path.borrow().clone() {
            *recovery_mode.borrow_mut() = RecoveryMode::CustomImage(img_path.clone());
            let fname = Path::new(&img_path).file_name().and_then(|n| n.to_str()).unwrap_or(&img_path);
            let desc = format!("Source: USB Image ({})", fname);
            show_target_screen(&desc);
        }
    }));

    btn_browser_select.connect_clicked(clone!(
        @strong selected_image_path,
        @strong recovery_mode,
        @strong show_target_screen
     => move |_| {
        if let Some(img_path) = selected_image_path.borrow().clone() {
            *recovery_mode.borrow_mut() = RecoveryMode::CustomImage(img_path.clone());
            let fname = Path::new(&img_path).file_name().and_then(|n| n.to_str()).unwrap_or(&img_path);
            let desc = format!("Source: Selected Image ({})", fname);
            show_target_screen(&desc);
        }
    }));

    btn_target_back.connect_clicked(clone!(
        @weak stack,
        @strong selected_action
     => move |_| {
        let action = selected_action.borrow().clone().unwrap_or_default();
        if action == "usb_restore" {
            stack.set_visible_child_name("usb_select");
        } else if action == "internet_info" {
            stack.set_visible_child_name("internet_info");
        } else {
            stack.set_visible_child_name("utilities");
        }
    }));

    btn_util_continue.connect_clicked(clone!(
        @weak stack,
        @strong selected_action,
        @strong recovery_mode,
        @strong show_target_screen,
        @strong populate_usb_images
     => move |_| {
        let action = selected_action.borrow().clone().unwrap_or_default();
        match action.as_str() {
            "timemachine" => {
                log_msg("Launching Pulsar OS Time Machine Recovery Suite...");
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg("export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin; xhost +SI:localuser:root >/dev/null 2>&1 || xhost +local: >/dev/null 2>&1 || xhost + >/dev/null 2>&1 || true; (pulsaros-timemachine gui || python3 /usr/share/pulsaros-timemachine/cli.py gui || python3 /usr/share/pulsaros-timemachine/cli.py restore --help) >/tmp/timemachine-recovery.log 2>&1 &")
                    .spawn();
            }
            "disk" => {
                log_msg("Launching elevated Disk Utility (GParted)...");
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg("export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin; xhost +SI:localuser:root >/dev/null 2>&1 || xhost +local: >/dev/null 2>&1 || xhost + >/dev/null 2>&1 || true; (sudo -E /usr/sbin/gparted || sudo -E gparted || sudo /usr/sbin/gparted || sudo gparted || /usr/sbin/gparted || gparted || gnome-disks || gnome-disk-utility) >/tmp/gparted.log 2>&1 &")
                    .spawn();
            }
            "terminal" => {
                log_msg("Launching recovery root terminal...");
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg("export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin; xhost +SI:localuser:root >/dev/null 2>&1 || xhost +local: >/dev/null 2>&1 || xhost + >/dev/null 2>&1 || true; (xterm -title 'Pulsar OS Recovery Terminal' -bg '#18181b' -fg '#ffffff' -fa Monospace -fs 11 -e sudo bash || gnome-terminal -- sudo bash || alacritty -e sudo bash || x-terminal-emulator -e sudo bash || xterm -e sudo bash) &")
                    .spawn();
            }
            "internet_info" => {
                stack.set_visible_child_name("internet_info");
            }
            "usb_restore" => {
                populate_usb_images();
                stack.set_visible_child_name("usb_select");
            }
            "reinstall" => {
                *recovery_mode.borrow_mut() = RecoveryMode::Local;
                show_target_screen("Source: Built-in Recovery Partition");
            }
            _ => {}
        }
    }));

    btn_target_restore.connect_clicked(clone!(
        @weak stack,
        @weak pbar,
        @weak prog_desc,
        @weak log_view,
        @weak scrolled_log,
        @weak err_msg_lbl,
        @weak err_log_view,
        @strong selected_target,
        @strong recovery_mode
     => move |_| {
        let target = match selected_target.borrow().clone() {
            Some(t) => t,
            None => return,
        };
        let mode = recovery_mode.borrow().clone();
        stack.set_visible_child_name("progress");

        let (sender, receiver) = std::sync::mpsc::channel::<RecoveryUpdate>();

        let pbar_c = pbar.clone();
        let desc_c = prog_desc.clone();
        let stack_c = stack.clone();
        let buffer = log_view.buffer();
        let err_buffer = err_log_view.buffer();
        let err_lbl_c = err_msg_lbl.clone();
        let scroll_c = scrolled_log.clone();

        buffer.set_text("");
        err_buffer.set_text("");

        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            while let Ok(msg) = receiver.try_recv() {
                match msg {
                    RecoveryUpdate::Progress(fraction, text) => {
                        pbar_c.set_fraction(fraction);
                        desc_c.set_label(&text);
                    }
                    RecoveryUpdate::Log(line) => {
                        let mut end = buffer.end_iter();
                        buffer.insert(&mut end, &format!("{}\n", line));
                        let mut err_end = err_buffer.end_iter();
                        err_buffer.insert(&mut err_end, &format!("{}\n", line));
                        let adj = scroll_c.vadjustment();
                        adj.set_value(adj.upper());
                    }
                    RecoveryUpdate::Finished(res) => {
                        match res {
                            Ok(_) => {
                                stack_c.set_visible_child_name("complete");
                            }
                            Err(e) => {
                                log_msg(&format!("Restoration error: {}", e));
                                err_lbl_c.set_label(&format!("Failed: {}", e));
                                stack_c.set_visible_child_name("error");
                            }
                        }
                        return glib::ControlFlow::Break;
                    }
                }
            }
            glib::ControlFlow::Continue
        });

        thread::spawn(move || {
            let sender_p = sender.clone();
            let update_ui = move |pct: f64, msg: &str| {
                let _ = sender_p.send(RecoveryUpdate::Progress(pct, msg.to_string()));
            };

            let sender_l = sender.clone();
            let append_log = move |text: &str| {
                let _ = sender_l.send(RecoveryUpdate::Log(text.to_string()));
            };

            update_ui(0.05, "Scanning target partition...");
            append_log(&format!("Target partition: {}", target.part_path));

            // Run restoration steps
            let res = run_restoration(&target, mode, update_ui, append_log);
            let _ = sender.send(RecoveryUpdate::Finished(res));
        });
    }));

    stack.set_visible_child_name("utilities");
    window.present();
}
