//! Construcción de la interfaz GTK4.
use crate::demo::is_demo_mode;
use crate::icons::{create_button_with_icon, create_icon_widget, get_lucide_icon_path};
use crate::manifest::{fetch_release_manifest, local_manifest_fallback};
use crate::models::{
    AppType, BtrfsTarget, DownloadMsg, EncryptedTarget, ExternalDrive, InstalledApp, ManifestData,
    NetConnType, RecoveryMode, RecoveryUpdate, WifiNetwork,
};
use crate::network::{
    connect_wifi, get_network_status, open_external_network_settings, scan_wifi_networks,
};
use crate::restore::run_restoration;
use crate::system::{
    copy_folder_with_progress, detect_system_base, detect_system_bootloader, exec_cmd,
    find_btrfs_targets, find_encrypted_targets, format_file_size, is_valid_base_squashfs,
    launch_chroot_terminal, log_msg, scan_external_drives, scan_installed_applications,
    scan_usb_devices, set_keyboard_layout, unlock_luks_partition,
};
use crate::theme::APP_CSS;
use glib::clone;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, Box as GtkBox, Button, CenterBox, CheckButton, CssProvider, DropDown,
    GestureClick, Label, ListBox, ListBoxRow, Orientation, PasswordEntry, ProgressBar,
    ScrolledWindow, SelectionMode, Spinner, Stack, StackTransitionType, StringList, TextView,
    WrapMode,
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
        .title(if is_demo_mode() { "Pulsar OS Recovery (DEMO / SIMULATION MODE)" } else { "Pulsar OS Recovery" })
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

    let kb_box = GtkBox::new(Orientation::Horizontal, 6);
    kb_box.add_css_class("keyboard-selector-box");
    let kb_icon = create_icon_widget("", "keyboard", 16);
    let kb_layouts = [
        "Spanish (ES)",
        "English (US)",
        "Spanish (Latin America)",
        "English (UK)",
        "French (FR)",
        "German (DE)",
        "Italian (IT)",
        "Portuguese (PT)",
    ];
    let kb_list = StringList::new(&kb_layouts);
    let combo_kb = DropDown::new(Some(kb_list), gtk4::Expression::NONE);
    combo_kb.add_css_class("secondary-action");
    combo_kb.connect_selected_notify(|dd| {
        let code = match dd.selected() {
            0 => "es",
            1 => "us",
            2 => "latam",
            3 => "gb",
            4 => "fr",
            5 => "de",
            6 => "it",
            7 => "pt",
            _ => "es",
        };
        let _ = set_keyboard_layout(code);
    });
    kb_box.append(&kb_icon);
    kb_box.append(&combo_kb);
    bottom_bar.append(&kb_box);

    root_box.append(&bottom_bar);

    window.set_content(Some(&root_box));

    // Shared state
    let selected_action: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let selected_target: Rc<RefCell<Option<BtrfsTarget>>> = Rc::new(RefCell::new(None));
    let selected_encrypted_target: Rc<RefCell<Option<EncryptedTarget>>> = Rc::new(RefCell::new(None));
    let current_source_desc: Rc<RefCell<String>> = Rc::new(RefCell::new("Source: Built-in Recovery Partition".to_string()));
    let selected_image_path: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let recovery_mode: Rc<RefCell<RecoveryMode>> = Rc::new(RefCell::new(RecoveryMode::Local));
    let current_browser_dir: Rc<RefCell<PathBuf>> = Rc::new(RefCell::new(PathBuf::from("/media")));
    let selected_apps_to_reinstall: Rc<RefCell<Vec<InstalledApp>>> = Rc::new(RefCell::new(Vec::new()));
    let _discovered_rescue_drives: Rc<RefCell<Vec<ExternalDrive>>> = Rc::new(RefCell::new(Vec::new()));

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
    add_row(
        "chroot_terminal",
        "Open System Terminal (Chroot)",
        "Mount your installed Pulsar OS system and open a root terminal directly inside your system.",
        "/usr/share/pulsaros-recovery/terminal.png",
        "terminal",
    );
    add_row(
        "rescue_files",
        "Rescue Files to External Storage",
        "Browse and copy personal folders from your user system to a connected USB drive.",
        "",
        "folder",
    );
    add_row(
        "logs",
        "System & Installation Logs",
        "View and inspect installer, recovery and setup logs stored in /var/log/pulsaros.",
        "/usr/share/pulsaros-recovery/terminal.png",
        "file-text",
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
    
    let ver_header_row = GtkBox::new(Orientation::Horizontal, 8);
    let lbl_version = Label::new(Some("Version"));
    lbl_version.add_css_class("setting-label");
    lbl_version.set_halign(Align::Start);
    lbl_version.set_hexpand(true);
    ver_header_row.append(&lbl_version);

    let btn_refresh_versions = create_button_with_icon("Reload Images List", "refresh", 14, "secondary-action");
    ver_header_row.append(&btn_refresh_versions);
    version_box.append(&ver_header_row);

    let ver_str = manifest_rc.borrow().latest_version();
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

    // Function to reload/refresh the release manifest in background
    let reload_manifest = {
        let manifest_rc_c = manifest_rc.clone();
        let combo_version_c = combo_version.clone();
        let combo_mirror_c = combo_mirror.clone();
        let lbl_status_c = lbl_net_status.clone();

        Rc::new(move || {
            lbl_status_c.set_text("Connecting to mirrors and reloading releases manifest...");
            let (tx, rx) = std::sync::mpsc::channel::<ManifestData>();
            let manifest_rc_inner = manifest_rc_c.clone();
            let combo_version_inner = combo_version_c.clone();
            let combo_mirror_inner = combo_mirror_c.clone();
            let lbl_status_inner = lbl_status_c.clone();

            glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                match rx.try_recv() {
                    Ok(manifest) => {
                        *manifest_rc_inner.borrow_mut() = manifest.clone();
                        
                        // Collect all available versions newest first
                        let versions: Vec<String> = manifest.available_versions();
                        
                        let ver_ptrs: Vec<&str> = versions.iter().map(|s| s.as_str()).collect();
                        let version_list = StringList::new(&ver_ptrs);
                        combo_version_inner.set_model(Some(&version_list));
                        combo_version_inner.set_selected(0);

                        let mirror_names: Vec<&str> = manifest.mirrors.iter().map(|m| m.name.as_str()).collect();
                        let mirror_list = StringList::new(&mirror_names);
                        combo_mirror_inner.set_model(Some(&mirror_list));

                        lbl_status_inner.set_text("Releases manifest reloaded successfully. Images ready to download.");
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
        })
    };

    // Initial background fetch
    reload_manifest();

    // Connect refresh button
    {
        let reload_fn = reload_manifest.clone();
        btn_refresh_versions.connect_clicked(move |_| {
            reload_fn();
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
    // 5b. Unlock LUKS Encrypted Partition Screen
    // ─────────────────────────────────────────────────────────────
    let unlock_box = GtkBox::new(Orientation::Vertical, 10);
    unlock_box.set_valign(Align::Center);
    unlock_box.set_halign(Align::Center);

    let unlock_icon = create_icon_widget("", "lock", 56);
    unlock_box.append(&unlock_icon);

    let unlock_title = Label::new(Some("Unlock Encrypted Disk"));
    unlock_title.add_css_class("welcome-title");
    unlock_box.append(&unlock_title);

    let unlock_desc = Label::new(Some("This disk partition is encrypted with LUKS.\nEnter your passphrase to unlock and access the system for recovery."));
    unlock_desc.add_css_class("welcome-subtitle");
    unlock_desc.set_wrap(true);
    unlock_desc.set_max_width_chars(50);
    unlock_desc.set_justify(gtk4::Justification::Center);
    unlock_box.append(&unlock_desc);

    let lbl_unlock_part_info = Label::new(Some("Partition: /dev/..."));
    lbl_unlock_part_info.add_css_class("progress-text");
    unlock_box.append(&lbl_unlock_part_info);

    let unlock_card = GtkBox::new(Orientation::Vertical, 10);
    unlock_card.add_css_class("info-card");
    unlock_card.set_size_request(440, -1);

    let entry_luks_pw = PasswordEntry::new();
    entry_luks_pw.set_show_peek_icon(true);
    entry_luks_pw.set_placeholder_text(Some("Disk encryption passphrase"));
    entry_luks_pw.set_hexpand(true);
    unlock_card.append(&entry_luks_pw);

    let unlock_spinner_box = GtkBox::new(Orientation::Horizontal, 8);
    unlock_spinner_box.set_halign(Align::Center);
    let spinner_unlock = Spinner::new();
    spinner_unlock.set_spinning(false);
    spinner_unlock.set_visible(false);
    let lbl_unlock_status = Label::new(None);
    lbl_unlock_status.add_css_class("progress-text");
    unlock_spinner_box.append(&spinner_unlock);
    unlock_spinner_box.append(&lbl_unlock_status);
    unlock_card.append(&unlock_spinner_box);

    let lbl_unlock_err = Label::new(None);
    lbl_unlock_err.add_css_class("badge-net-err");
    lbl_unlock_err.set_wrap(true);
    lbl_unlock_err.set_max_width_chars(45);
    lbl_unlock_err.set_visible(false);
    unlock_card.append(&lbl_unlock_err);

    let unlock_nav_box = GtkBox::new(Orientation::Horizontal, 14);
    unlock_nav_box.set_halign(Align::Center);
    unlock_nav_box.set_margin_top(8);

    let btn_unlock_back = Button::with_label("Back");
    btn_unlock_back.add_css_class("secondary-action");
    unlock_nav_box.append(&btn_unlock_back);

    let btn_unlock_do = Button::with_label("Unlock Disk");
    btn_unlock_do.add_css_class("suggested-action");
    unlock_nav_box.append(&btn_unlock_do);
    unlock_card.append(&unlock_nav_box);

    unlock_box.append(&unlock_card);
    stack.add_named(&unlock_box, Some("unlock_luks"));

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
    // 9. Rescue Files to External Storage Screen
    // ─────────────────────────────────────────────────────────────
    let rescue_box = GtkBox::new(Orientation::Vertical, 8);
    rescue_box.set_valign(Align::Center);
    rescue_box.set_halign(Align::Center);

    let rescue_icon = create_icon_widget("", "folder", 48);
    rescue_box.append(&rescue_icon);

    let rescue_title = Label::new(Some("Rescue Files to External Storage"));
    rescue_title.add_css_class("welcome-title");
    rescue_box.append(&rescue_title);

    let rescue_desc = Label::new(Some("Select a user folder to copy and choose your destination USB drive."));
    rescue_desc.add_css_class("welcome-subtitle");
    rescue_box.append(&rescue_desc);

    let rescue_card = GtkBox::new(Orientation::Vertical, 8);
    rescue_card.add_css_class("info-card");
    rescue_card.set_size_request(580, -1);

    let lbl_src_folder = Label::new(Some("Source Folder:"));
    lbl_src_folder.add_css_class("setting-label");
    lbl_src_folder.set_halign(Align::Start);
    rescue_card.append(&lbl_src_folder);

    let combo_src_folder = DropDown::new(None::<StringList>, gtk4::Expression::NONE);
    combo_src_folder.set_hexpand(true);
    rescue_card.append(&combo_src_folder);

    let dst_hdr_box = GtkBox::new(Orientation::Horizontal, 8);
    let lbl_dst_drive = Label::new(Some("Destination External Drive:"));
    lbl_dst_drive.add_css_class("setting-label");
    lbl_dst_drive.set_halign(Align::Start);
    lbl_dst_drive.set_hexpand(true);
    dst_hdr_box.append(&lbl_dst_drive);

    let btn_refresh_drives = create_button_with_icon("Refresh Drives", "refresh", 14, "secondary-action");
    dst_hdr_box.append(&btn_refresh_drives);
    rescue_card.append(&dst_hdr_box);

    let combo_dst_drive = DropDown::new(None::<StringList>, gtk4::Expression::NONE);
    combo_dst_drive.set_hexpand(true);
    rescue_card.append(&combo_dst_drive);

    let pbar_rescue = ProgressBar::new();
    pbar_rescue.add_css_class("progress-bar-thin");
    pbar_rescue.set_fraction(0.0);
    pbar_rescue.set_margin_top(6);
    rescue_card.append(&pbar_rescue);

    let lbl_rescue_status = Label::new(Some("Ready to copy files."));
    lbl_rescue_status.add_css_class("progress-text");
    lbl_rescue_status.set_halign(Align::Start);
    lbl_rescue_status.set_wrap(true);
    rescue_card.append(&lbl_rescue_status);

    let rescue_nav_box = GtkBox::new(Orientation::Horizontal, 14);
    rescue_nav_box.set_halign(Align::Center);
    rescue_nav_box.set_margin_top(8);

    let btn_rescue_back = Button::with_label("Back to Utilities");
    btn_rescue_back.add_css_class("secondary-action");
    rescue_nav_box.append(&btn_rescue_back);

    let btn_rescue_start = Button::with_label("Start Rescue Copy");
    btn_rescue_start.add_css_class("suggested-action");
    rescue_nav_box.append(&btn_rescue_start);

    rescue_card.append(&rescue_nav_box);
    rescue_box.append(&rescue_card);
    stack.add_named(&rescue_box, Some("rescue_files"));

    // ─────────────────────────────────────────────────────────────
    // 10. Installed Applications & Drivers Review Screen
    // ─────────────────────────────────────────────────────────────
    let app_box = GtkBox::new(Orientation::Vertical, 8);
    app_box.set_valign(Align::Center);
    app_box.set_halign(Align::Center);

    let app_icon = create_icon_widget("", "package", 48);
    app_box.append(&app_icon);

    let app_title = Label::new(Some("Preserve Installed Applications & Drivers"));
    app_title.add_css_class("welcome-title");
    app_box.append(&app_title);

    let app_sub = Label::new(Some("The following applications and hardware drivers will be reinstalled.\nUncheck any items you wish to remove."));
    app_sub.add_css_class("welcome-subtitle");
    app_sub.set_justify(gtk4::Justification::Center);
    app_box.append(&app_sub);

    let app_card = GtkBox::new(Orientation::Vertical, 8);
    app_card.add_css_class("info-card");
    app_card.set_size_request(600, -1);

    let app_actions_box = GtkBox::new(Orientation::Horizontal, 8);
    let btn_select_all = Button::with_label("Select All");
    btn_select_all.add_css_class("secondary-action");
    let btn_deselect_all = Button::with_label("Deselect All");
    btn_deselect_all.add_css_class("secondary-action");
    let lbl_app_count = Label::new(Some("Detected: 0 items"));
    lbl_app_count.add_css_class("progress-text");
    lbl_app_count.set_hexpand(true);
    lbl_app_count.set_halign(Align::End);
    app_actions_box.append(&btn_select_all);
    app_actions_box.append(&btn_deselect_all);
    app_actions_box.append(&lbl_app_count);
    app_card.append(&app_actions_box);

    let scrolled_apps = ScrolledWindow::new();
    scrolled_apps.set_size_request(580, 220);
    scrolled_apps.add_css_class("live-log-view");

    let app_listbox = ListBox::new();
    app_listbox.add_css_class("transparent-list");
    app_listbox.set_selection_mode(SelectionMode::None);
    scrolled_apps.set_child(Some(&app_listbox));
    app_card.append(&scrolled_apps);

    let app_nav_box = GtkBox::new(Orientation::Horizontal, 14);
    app_nav_box.set_halign(Align::Center);
    app_nav_box.set_margin_top(10);

    let btn_app_back = Button::with_label("Back");
    btn_app_back.add_css_class("secondary-action");
    app_nav_box.append(&btn_app_back);

    let btn_app_continue = Button::with_label("Confirm & Reinstall System");
    btn_app_continue.add_css_class("suggested-action");
    app_nav_box.append(&btn_app_continue);

    app_card.append(&app_nav_box);
    app_box.append(&app_card);
    stack.add_named(&app_box, Some("app_review"));

    // ─────────────────────────────────────────────────────────────
    // Helper: Refresh Target Partitions & Show Target Screen
    // ─────────────────────────────────────────────────────────────
    let show_target_screen = {
        let stack = stack.clone();
        let targets_flow = targets_flow.clone();
        let btn_target_restore = btn_target_restore.clone();
        let selected_target = selected_target.clone();
        let selected_encrypted_target = selected_encrypted_target.clone();
        let current_source_desc = current_source_desc.clone();
        let source_img_lbl = source_img_lbl.clone();
        let lbl_unlock_part_info = lbl_unlock_part_info.clone();
        let entry_luks_pw = entry_luks_pw.clone();
        let lbl_unlock_err = lbl_unlock_err.clone();
        let lbl_unlock_status = lbl_unlock_status.clone();
        let spinner_unlock = spinner_unlock.clone();
        let btn_unlock_do = btn_unlock_do.clone();

        Rc::new(move |source_desc: &str| {
            *current_source_desc.borrow_mut() = source_desc.to_string();
            source_img_lbl.set_text(source_desc);

            while let Some(child) = targets_flow.first_child() {
                targets_flow.remove(&child);
            }
            *selected_target.borrow_mut() = None;
            btn_target_restore.set_sensitive(false);

            let mut targets = find_btrfs_targets();
            let mut enc_targets = find_encrypted_targets();

            if targets.is_empty() && enc_targets.is_empty() && is_demo_mode() {
                targets.push(BtrfsTarget {
                    _disk_path: "/dev/demo-nvme0n1".to_string(),
                    part_path: "/dev/demo-nvme0n1p2 (Simulado)".to_string(),
                    label: "Pulsar OS Demo Pool".to_string(),
                    uuid: "demo-btrfs-uuid-0000".to_string(),
                    size: "500.0G".to_string(),
                });
                enc_targets.push(EncryptedTarget {
                    disk_path: "/dev/demo-nvme0n1".to_string(),
                    part_path: "/dev/demo-nvme0n1p3 (Simulado Encrypted)".to_string(),
                    label: "Pulsar OS Demo Encrypted".to_string(),
                    uuid: "demo-luks-uuid-1111".to_string(),
                    size: "250.0G".to_string(),
                    is_unlocked: false,
                    mapper_name: "pulsar_cryptroot".to_string(),
                });
            }

            let locked_enc_targets: Vec<EncryptedTarget> = enc_targets.into_iter().filter(|e| !e.is_unlocked).collect();

            if targets.is_empty() && locked_enc_targets.is_empty() {
                let no_target_lbl = Label::new(Some("No Btrfs or LUKS encrypted Pulsar OS partitions detected.\nUse Disk Utility to inspect drives."));
                no_target_lbl.add_css_class("welcome-subtitle");
                targets_flow.append(&no_target_lbl);
            } else {
                // Render unencrypted or unlocked Btrfs partitions
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

                // Render locked LUKS encrypted partitions
                for enc in locked_enc_targets {
                    let card = GtkBox::new(Orientation::Vertical, 6);
                    card.add_css_class("disk-card");
                    card.add_css_class("encrypted");

                    let lock_icon = create_icon_widget("", "lock", 40);
                    card.append(&lock_icon);

                    let name_lbl = Label::new(Some(&format!("Encrypted Disk ({})", enc.size)));
                    name_lbl.add_css_class("utility-title-lbl");
                    card.append(&name_lbl);

                    let dev_lbl = Label::new(Some(&enc.part_path));
                    dev_lbl.add_css_class("utility-desc-lbl");
                    card.append(&dev_lbl);

                    let badge = Label::new(Some("Locked (Click to Unlock)"));
                    badge.add_css_class("badge-encrypted");
                    card.append(&badge);

                    let gesture = GestureClick::new();
                    let enc_clone = enc.clone();
                    let sel_enc_c = selected_encrypted_target.clone();
                    let stack_c = stack.clone();
                    let info_lbl_c = lbl_unlock_part_info.clone();
                    let pw_entry_c = entry_luks_pw.clone();
                    let err_lbl_c = lbl_unlock_err.clone();
                    let status_lbl_c = lbl_unlock_status.clone();
                    let spin_c = spinner_unlock.clone();
                    let btn_do_c = btn_unlock_do.clone();

                    gesture.connect_released(move |_, _, _, _| {
                        *sel_enc_c.borrow_mut() = Some(enc_clone.clone());
                        info_lbl_c.set_text(&format!("Partition: {}  |  Size: {}", enc_clone.part_path, enc_clone.size));
                        pw_entry_c.set_text("");
                        err_lbl_c.set_text("");
                        err_lbl_c.set_visible(false);
                        status_lbl_c.set_text("");
                        spin_c.set_spinning(false);
                        spin_c.set_visible(false);
                        btn_do_c.set_sensitive(true);
                        stack_c.set_visible_child_name("unlock_luks");
                        pw_entry_c.grab_focus();
                    });

                    card.add_controller(gesture);
                    targets_flow.append(&card);
                }
            }
            stack.set_visible_child_name("target_select");
        })
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
        let ver = combo_version
            .selected_item()
            .and_then(|item| item.downcast::<gtk4::StringObject>().ok())
            .map(|s| s.string().to_string())
            .unwrap_or_else(|| m_data.latest_version());
        let mirror_id = m_data.mirrors.get(combo_mirror.selected() as usize)
            .map(|m| m.id.clone())
            .unwrap_or_else(|| "auto".to_string());

        let target_info = m_data.image_for(&ver, sel_base, sel_boot);

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

            let has_aria2c = Command::new("which").arg("aria2c").status().map(|s| s.success()).unwrap_or(false);
            let mut child = if has_aria2c {
                log_msg("Using aria2c download manager (segmented download with resume support)...");
                match Command::new("aria2c")
                    .args(&[
                        "-c",
                        "-x", "4",
                        "-s", "4",
                        "--connect-timeout=15",
                        "--max-tries=5",
                        "--dir=/tmp",
                        "-o", "pulsaros-internet-recovery.squashfs.part",
                        &download_url,
                    ])
                    .spawn()
                {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx.send(DownloadMsg::Error(format!("Could not run aria2c: {}", e)));
                        return;
                    }
                }
            } else {
                log_msg("Using curl download engine with range resume support (-C -)...");
                match Command::new("curl")
                    .args(&[
                        "-C", "-",
                        "-sSL",
                        "-L",
                        "--connect-timeout", "15",
                        "--retry", "5",
                        "--retry-connrefused",
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
                }
            };

            let mut last_size: u64 = 0;
            let mut last_time = Instant::now();

            loop {
                thread::sleep(std::time::Duration::from_millis(200));

                if cancel_flag.load(Ordering::SeqCst) {
                    log_msg("Download cancelled by user. Terminating download manager and cleaning temporary files...");
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = fs::remove_file(tmp_part);
                    let _ = fs::remove_file(format!("{}.aria2", tmp_part));
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

    let rescue_src_paths: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let rescue_dst_paths: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

    let populate_rescue_options = {
        let combo_src = combo_src_folder.clone();
        let combo_dst = combo_dst_drive.clone();
        let src_paths = rescue_src_paths.clone();
        let dst_paths = rescue_dst_paths.clone();
        let lbl_status = lbl_rescue_status.clone();

        Rc::new(move || {
            lbl_status.set_text("Scanning storage devices and user folders...");
            let targets = find_btrfs_targets();
            let mut detected_sources: Vec<String> = Vec::new();
            let mut display_sources: Vec<String> = Vec::new();

            if let Some(target) = targets.first() {
                let mnt_rescue = "/tmp/pulsar_rescue_pool";
                let _ = fs::create_dir_all(mnt_rescue);
                let _ = exec_cmd(&format!("mountpoint -q {} || mount -t btrfs -o ro,subvol=@home {} {}", mnt_rescue, target.part_path, mnt_rescue));

                if let Ok(entries) = fs::read_dir(mnt_rescue) {
                    for entry in entries.flatten() {
                        let uname = entry.file_name().to_string_lossy().to_string();
                        if uname != "lost+found" && !uname.starts_with('.') {
                            let user_root = format!("{}/{}", mnt_rescue, uname);
                            display_sources.push(format!("Full Home: /home/{}", uname));
                            detected_sources.push(user_root.clone());

                            for sub in &["Documents", "Desktop", "Downloads", "Pictures", "Projects", "Videos", "Music"] {
                                let sub_path = format!("{}/{}", user_root, sub);
                                if Path::new(&sub_path).exists() {
                                    display_sources.push(format!("Folder: /home/{}/{}", uname, sub));
                                    detected_sources.push(sub_path);
                                }
                            }
                        }
                    }
                }
            }

            if display_sources.is_empty() {
                display_sources.push("No user folders found in @home".to_string());
                detected_sources.push("/tmp".to_string());
            }

            *src_paths.borrow_mut() = detected_sources;
            let src_strs: Vec<&str> = display_sources.iter().map(|s| s.as_str()).collect();
            let src_list = StringList::new(&src_strs);
            combo_src.set_model(Some(&src_list));
            combo_src.set_selected(0);

            // Scan external drives
            let ext_drives = scan_external_drives();
            let mut display_dsts: Vec<String> = Vec::new();
            let mut detected_dsts: Vec<String> = Vec::new();

            for d in ext_drives {
                display_dsts.push(format!("{} ({} - {})", d.label, d.size, d.mount_point));
                detected_dsts.push(d.mount_point);
            }

            if display_dsts.is_empty() {
                display_dsts.push("No external USB drives detected (Connect one and click Refresh)".to_string());
                detected_dsts.push("/tmp".to_string());
            }

            *dst_paths.borrow_mut() = detected_dsts;
            let dst_strs: Vec<&str> = display_dsts.iter().map(|s| s.as_str()).collect();
            let dst_list = StringList::new(&dst_strs);
            combo_dst.set_model(Some(&dst_list));
            combo_dst.set_selected(0);

            lbl_status.set_text("Select source folder and external drive to begin copying.");
        })
    };

    btn_refresh_drives.connect_clicked(clone!(@strong populate_rescue_options => move |_| {
        populate_rescue_options();
    }));

    btn_rescue_back.connect_clicked(clone!(@weak stack => move |_| {
        stack.set_visible_child_name("utilities");
    }));

    btn_rescue_start.connect_clicked(clone!(
        @weak pbar_rescue,
        @weak lbl_rescue_status,
        @weak btn_rescue_start,
        @weak btn_rescue_back,
        @weak combo_src_folder,
        @weak combo_dst_drive,
        @strong rescue_src_paths,
        @strong rescue_dst_paths
     => move |_| {
        let src_idx = combo_src_folder.selected() as usize;
        let dst_idx = combo_dst_drive.selected() as usize;

        let src = match rescue_src_paths.borrow().get(src_idx).cloned() {
            Some(s) => s,
            None => return,
        };
        let dst = match rescue_dst_paths.borrow().get(dst_idx).cloned() {
            Some(d) => d,
            None => return,
        };

        if dst == "/tmp" || !Path::new(&dst).exists() {
            lbl_rescue_status.set_text("Please connect and select a valid external drive destination.");
            return;
        }

        btn_rescue_start.set_sensitive(false);
        btn_rescue_back.set_sensitive(false);
        pbar_rescue.set_fraction(0.05);
        lbl_rescue_status.set_text("Starting copy to external drive...");

        let (tx, rx) = std::sync::mpsc::channel::<(f64, String, bool)>();
        let pbar_c = pbar_rescue.clone();
        let lbl_c = lbl_rescue_status.clone();
        let btn_s_c = btn_rescue_start.clone();
        let btn_b_c = btn_rescue_back.clone();

        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok((fraction, msg, is_done)) = rx.try_recv() {
                pbar_c.set_fraction(fraction);
                lbl_c.set_text(&msg);
                if is_done {
                    btn_s_c.set_sensitive(true);
                    btn_b_c.set_sensitive(true);
                    return glib::ControlFlow::Break;
                }
            }
            glib::ControlFlow::Continue
        });

        thread::spawn(move || {
            let tx_p = tx.clone();
            let update_fn = move |pct: f64, text: &str| {
                let _ = tx_p.send((pct, text.to_string(), false));
            };
            let log_fn = move |text: &str| {
                log_msg(text);
            };

            match copy_folder_with_progress(&src, &dst, update_fn, log_fn) {
                Ok(_) => {
                    let _ = tx.send((1.0, "Rescue copy finished successfully!".to_string(), true));
                }
                Err(e) => {
                    let _ = tx.send((0.0, format!("Copy failed: {}", e), true));
                }
            }
        });
    }));

    let app_checkboxes: Rc<RefCell<Vec<CheckButton>>> = Rc::new(RefCell::new(Vec::new()));

    btn_select_all.connect_clicked(clone!(@strong app_checkboxes => move |_| {
        for chk in app_checkboxes.borrow().iter() {
            chk.set_active(true);
        }
    }));

    btn_deselect_all.connect_clicked(clone!(@strong app_checkboxes => move |_| {
        for chk in app_checkboxes.borrow().iter() {
            chk.set_active(false);
        }
    }));

    btn_app_back.connect_clicked(clone!(@weak stack => move |_| {
        stack.set_visible_child_name("target_select");
    }));

    let run_restore_job = {
        let stack = stack.clone();
        let pbar = pbar.clone();
        let prog_desc = prog_desc.clone();
        let log_view = log_view.clone();
        let scrolled_log = scrolled_log.clone();
        let err_msg_lbl = err_msg_lbl.clone();
        let err_log_view = err_log_view.clone();

        Rc::new(move |target: BtrfsTarget, mode: RecoveryMode, apps: Vec<InstalledApp>| {
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

                // Run restoration steps with preserved apps
                let res = run_restoration(&target, mode, apps, update_ui, append_log);
                let _ = sender.send(RecoveryUpdate::Finished(res));
            });
        })
    };

    btn_app_continue.connect_clicked(clone!(
        @strong run_restore_job,
        @strong selected_target,
        @strong recovery_mode,
        @strong selected_apps_to_reinstall,
        @strong app_checkboxes
     => move |_| {
        let target = match selected_target.borrow().clone() {
            Some(t) => t,
            None => return,
        };
        let mode = recovery_mode.borrow().clone();

        let mut kept_apps: Vec<InstalledApp> = Vec::new();
        let all_apps = selected_apps_to_reinstall.borrow().clone();
        let chks = app_checkboxes.borrow();

        for (idx, chk) in chks.iter().enumerate() {
            if chk.is_active() {
                if let Some(app) = all_apps.get(idx) {
                    kept_apps.push(app.clone());
                }
            }
        }

        run_restore_job(target, mode, kept_apps);
    }));

    btn_util_continue.connect_clicked(clone!(
        @weak stack,
        @strong selected_action,
        @strong recovery_mode,
        @strong show_target_screen,
        @strong populate_usb_images,
        @strong populate_rescue_options
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
            "chroot_terminal" => {
                log_msg("Initiating 1-Click Chroot Terminal session...");
                let targets = find_btrfs_targets();
                if let Some(first) = targets.first() {
                    let part = first.part_path.clone();
                    std::thread::spawn(move || {
                        if let Err(e) = launch_chroot_terminal(&part) {
                            log_msg(&format!("Error opening chroot terminal: {}", e));
                        }
                    });
                } else {
                    let enc = find_encrypted_targets();
                    if let Some(first_enc) = enc.first() {
                        if first_enc.is_unlocked {
                            let part = format!("/dev/mapper/{}", first_enc.mapper_name);
                            std::thread::spawn(move || {
                                let _ = launch_chroot_terminal(&part);
                            });
                        } else {
                            stack.set_visible_child_name("unlock_luks");
                        }
                    } else {
                        log_msg("No Pulsar OS system partition detected to chroot into.");
                    }
                }
            }
            "rescue_files" => {
                log_msg("Opening File Rescue utility...");
                populate_rescue_options();
                stack.set_visible_child_name("rescue_files");
            }
            "logs" => {
                log_msg("Opening Pulsar OS logs directory...");
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg("mkdir -p /var/log/pulsaros; xhost +SI:localuser:root >/dev/null 2>&1 || true; (nautilus /var/log/pulsaros || xdg-open /var/log/pulsaros || gnome-text-editor /var/log/pulsaros/installer.log || xterm -title 'Pulsar OS Logs' -e less /var/log/pulsaros/installer.log) &")
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
        @weak app_listbox,
        @weak lbl_app_count,
        @strong selected_target,
        @strong recovery_mode,
        @strong selected_apps_to_reinstall,
        @strong app_checkboxes,
        @strong run_restore_job
     => move |_| {
        let target = match selected_target.borrow().clone() {
            Some(t) => t,
            None => return,
        };
        let mode = recovery_mode.borrow().clone();

        // Scan installed applications from existing system @ subvolume
        let audit_mnt = "/tmp/pulsar_audit_root";
        let _ = fs::create_dir_all(audit_mnt);
        let _ = exec_cmd(&format!("umount -l {} 2>/dev/null || true", audit_mnt));
        let _ = exec_cmd(&format!("mount -t btrfs -o ro,subvol=@ {} {}", target.part_path, audit_mnt));
        let discovered = scan_installed_applications(audit_mnt);
        let _ = exec_cmd(&format!("umount -l {} 2>/dev/null || true", audit_mnt));

        if !discovered.is_empty() {
            while let Some(child) = app_listbox.first_child() {
                app_listbox.remove(&child);
            }

            lbl_app_count.set_text(&format!("Detected: {} items", discovered.len()));
            app_checkboxes.borrow_mut().clear();

            for app in &discovered {
                let row = ListBoxRow::new();
                row.add_css_class("app-item-row");

                let h_box = GtkBox::new(Orientation::Horizontal, 12);
                let chk = CheckButton::with_label(&app.name);
                chk.set_active(true);
                chk.set_hexpand(true);
                h_box.append(&chk);

                let badge_lbl = Label::new(Some(match app.app_type {
                    AppType::Driver => "Driver",
                    AppType::Application => "App",
                    AppType::Flatpak => "Flatpak",
                }));
                badge_lbl.add_css_class(match app.app_type {
                    AppType::Driver => "badge-driver",
                    AppType::Application => "badge-app",
                    AppType::Flatpak => "badge-flatpak",
                });
                h_box.append(&badge_lbl);

                let ver_lbl = Label::new(Some(&app.version));
                ver_lbl.add_css_class("setting-label");
                h_box.append(&ver_lbl);

                row.set_child(Some(&h_box));
                app_listbox.append(&row);

                app_checkboxes.borrow_mut().push(chk);
            }

            *selected_apps_to_reinstall.borrow_mut() = discovered;
            stack.set_visible_child_name("app_review");
        } else {
            run_restore_job(target, mode, Vec::new());
        }
    }));

    // ─────────────────────────────────────────────────────────────
    // Unlock LUKS Encrypted Disk Callbacks
    // ─────────────────────────────────────────────────────────────
    let do_unlock_action = {
        let selected_encrypted_target = selected_encrypted_target.clone();
        let entry_luks_pw = entry_luks_pw.clone();
        let lbl_unlock_err = lbl_unlock_err.clone();
        let lbl_unlock_status = lbl_unlock_status.clone();
        let spinner_unlock = spinner_unlock.clone();
        let btn_unlock_do = btn_unlock_do.clone();
        let btn_unlock_back = btn_unlock_back.clone();
        let show_target_screen = show_target_screen.clone();
        let current_source_desc = current_source_desc.clone();

        Rc::new(move || {
            let enc_opt = selected_encrypted_target.borrow().clone();
            let enc = match enc_opt {
                Some(e) => e,
                None => return,
            };
            let pw = entry_luks_pw.text().to_string();
            if pw.is_empty() {
                lbl_unlock_err.set_text("Please enter your encryption passphrase.");
                lbl_unlock_err.set_visible(true);
                return;
            }

            lbl_unlock_err.set_visible(false);
            lbl_unlock_err.set_text("");
            lbl_unlock_status.set_text("Unlocking encrypted container...");
            spinner_unlock.set_visible(true);
            spinner_unlock.set_spinning(true);
            btn_unlock_do.set_sensitive(false);
            btn_unlock_back.set_sensitive(false);

            let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
            let part_path = enc.part_path.clone();
            let mapper_name = enc.mapper_name.clone();
            let pw_clone = pw.clone();

            thread::spawn(move || {
                let res = unlock_luks_partition(&part_path, &pw_clone, &mapper_name);
                let _ = tx.send(res);
            });

            let spinner_c = spinner_unlock.clone();
            let status_c = lbl_unlock_status.clone();
            let err_c = lbl_unlock_err.clone();
            let btn_do_c = btn_unlock_do.clone();
            let btn_back_c = btn_unlock_back.clone();
            let show_tgt_c = show_target_screen.clone();
            let cur_src_c = current_source_desc.clone();
            let pw_entry_c = entry_luks_pw.clone();

            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                match rx.try_recv() {
                    Ok(res) => {
                        spinner_c.set_spinning(false);
                        spinner_c.set_visible(false);
                        status_c.set_text("");
                        btn_do_c.set_sensitive(true);
                        btn_back_c.set_sensitive(true);

                        match res {
                            Ok(mapper_dev) => {
                                log_msg(&format!("Unlocked {}, refreshing target screen...", mapper_dev));
                                let src = cur_src_c.borrow().clone();
                                show_tgt_c(&src);
                            }
                            Err(e) => {
                                err_c.set_text(&e);
                                err_c.set_visible(true);
                                pw_entry_c.grab_focus();
                            }
                        }
                        glib::ControlFlow::Break
                    }
                    Err(_) => glib::ControlFlow::Continue,
                }
            });
        })
    };

    let do_unlock_c1 = do_unlock_action.clone();
    btn_unlock_do.connect_clicked(move |_| {
        do_unlock_c1();
    });

    let do_unlock_c2 = do_unlock_action.clone();
    entry_luks_pw.connect_activate(move |_| {
        do_unlock_c2();
    });

    btn_unlock_back.connect_clicked(clone!(@weak stack => move |_| {
        stack.set_visible_child_name("target_select");
    }));

    stack.set_visible_child_name("utilities");
    window.present();
}
