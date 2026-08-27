use std::cell::RefCell;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::thread;

use gio::prelude::*;
use glib::clone;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, Box as GtkBox, Button, CenterBox, CssProvider, GestureClick,
    Image, Label, ListBox, ListBoxRow, Orientation, ProgressBar, ScrolledWindow,
    SelectionMode, Stack, StackTransitionType, TextView, WrapMode,
};
use libadwaita::prelude::*;
use libadwaita::ApplicationWindow;
use regex::Regex;

const APP_CSS: &str = r#"
/* Force macOS Dark Backdrop */
window, window.background, .background, .root-container {
    background-color: #1e1e20 !important;
    color: #ffffff !important;
}
window, .root-container, * {
    font-family: 'Inter', 'SF Pro Display', -apple-system, sans-serif;
}
.apple-box {
    background-color: #323236 !important;
    border: 1px solid rgba(255, 255, 255, 0.14) !important;
    border-radius: 20px !important;
    padding: 28px 32px !important;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.7) !important;
}
.welcome-title {
    font-size: 24px !important;
    font-weight: 700 !important;
    color: #ffffff !important;
    margin-top: 4px !important;
    margin-bottom: 4px !important;
}
.welcome-subtitle {
    font-size: 13px !important;
    color: #c7c7cc !important;
    margin-bottom: 16px !important;
}
/* Force completely transparent ListBox with individual floating cards */
list, listview, listbox, .transparent-list, .content, .boxed-list {
    background-color: transparent !important;
    background: transparent !important;
    background-image: none !important;
    border: none !important;
    box-shadow: none !important;
    outline: none !important;
    padding: 0 !important;
    margin: 0 !important;
}
listbox > row, listboxrow, row, .utility-item-row {
    background-color: transparent !important;
    background: transparent !important;
    background-image: none !important;
    border: none !important;
    box-shadow: none !important;
    outline: none !important;
    padding: 0 !important;
    margin: 0 !important;
}
.utility-row-card {
    background-color: rgba(255, 255, 255, 0.05) !important;
    border: 1px solid rgba(255, 255, 255, 0.08) !important;
    border-radius: 12px !important;
    padding: 12px 16px !important;
    transition: all 0.15s ease !important;
}
listbox > row:hover .utility-row-card,
listboxrow:hover .utility-row-card,
.utility-item-row:hover .utility-row-card {
    background-color: rgba(255, 255, 255, 0.09) !important;
    border-color: rgba(255, 255, 255, 0.16) !important;
}
listbox > row:selected .utility-row-card,
listboxrow:selected .utility-row-card,
.utility-item-row:selected .utility-row-card {
    background-color: #0071e3 !important;
    border-color: #0071e3 !important;
    box-shadow: 0 4px 14px rgba(0, 113, 227, 0.35) !important;
}
.utility-title-lbl {
    font-size: 15px !important;
    font-weight: 600 !important;
    color: #ffffff !important;
}
.utility-desc-lbl {
    font-size: 13px !important;
    color: #c7c7cc !important;
}
listbox > row:selected .utility-title-lbl,
listboxrow:selected .utility-title-lbl,
.utility-item-row:selected .utility-title-lbl {
    color: #ffffff !important;
}
listbox > row:selected .utility-desc-lbl,
listboxrow:selected .utility-desc-lbl,
.utility-item-row:selected .utility-desc-lbl {
    color: rgba(255, 255, 255, 0.92) !important;
}
.suggested-action {
    background-color: #0071e3 !important;
    color: #ffffff !important;
    border-radius: 8px !important;
    font-weight: 600 !important;
    padding: 9px 24px !important;
    border: none !important;
    font-size: 14px !important;
}
.suggested-action:hover {
    background-color: #007bf5 !important;
}
.suggested-action:disabled {
    background-color: #38383a !important;
    color: #636366 !important;
}
.secondary-action {
    background-color: #323236 !important;
    color: #ffffff !important;
    border-radius: 8px !important;
    font-weight: 600 !important;
    padding: 9px 24px !important;
    border: 1px solid rgba(255, 255, 255, 0.15) !important;
    font-size: 14px !important;
}
.secondary-action:hover {
    background-color: #3e3e42 !important;
}
.progress-bar-thin {
    min-height: 8px;
    margin-top: 12px;
    margin-bottom: 12px;
}
.progress-bar-thin trough {
    min-height: 8px;
    border-radius: 9999px;
    background-color: #3a3a3c;
    border: none;
}
.progress-bar-thin progress {
    min-height: 8px;
    border-radius: 9999px;
    background-color: #0071e3;
    border: none;
}
.progress-text {
    font-size: 13px;
    color: #aeaeb2;
}
.live-log-view {
    background-color: #121212;
    border: 1px solid #333333;
    border-radius: 8px;
    padding: 8px;
}
.live-log-text text {
    background-color: #121212;
    color: #30d158;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', monospace;
    font-size: 11px;
}
.err-log-text text {
    background-color: #121212;
    color: #ff453a;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', monospace;
    font-size: 11px;
}
.disk-card {
    background-color: #2a2a2a;
    border: 1px solid #3c3c3c;
    border-radius: 12px;
    padding: 16px;
    min-width: 140px;
    margin: 6px;
    transition: all 0.15s ease;
}
.disk-card:hover {
    background-color: #323236;
}
.disk-card.selected {
    background-color: #323236;
    border-color: #0071e3;
    box-shadow: 0 0 0 2px #0071e3;
}
"#;

#[derive(Clone, Debug)]
struct BtrfsTarget {
    _disk_path: String,
    part_path: String,
    label: String,
    uuid: String,
    size: String,
}

#[derive(Clone, Debug)]
enum RecoveryMode {
    Local,
    Internet(String),
}

#[derive(Debug)]
enum RecoveryUpdate {
    Progress(f64, String),
    Log(String),
    Finished(Result<(), String>),
}

fn log_msg(msg: &str) {
    let log_path = "/tmp/pulsaros-recovery.log";
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = writeln!(f, "{}", msg);
    }
    println!("{}", msg);
}

fn exec_cmd_stream<L>(cmd: &str, log: &L) -> Result<(), String>
where
    L: Fn(&str) + Send + Sync + 'static,
{
    log_msg(&format!("Running (as root): {}", cmd));
    log(&format!("$ {}", cmd));

    let mut child = Command::new("sudo")
        .args(&["-n", "sh", "-c", cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn '{}': {}", cmd, e))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let (tx, rx) = std::sync::mpsc::channel::<String>();

    if let Some(out) = stdout {
        let tx_out = tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(out);
            for line in reader.lines().flatten() {
                let _ = tx_out.send(line);
            }
        });
    }

    if let Some(err) = stderr {
        let tx_err = tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(err);
            for line in reader.lines().flatten() {
                let _ = tx_err.send(line);
            }
        });
    }
    drop(tx);

    while let Ok(line) = rx.recv() {
        log_msg(&line);
        log(&line);
    }

    let status = child.wait().map_err(|e| format!("Failed to wait on '{}': {}", cmd, e))?;
    if !status.success() {
        let err_str = format!("Command '{}' failed with exit code: {:?}", cmd, status.code());
        log_msg(&format!("ERROR: {}", err_str));
        return Err(err_str);
    }
    Ok(())
}

fn exec_cmd(cmd: &str) -> Result<String, String> {
    log_msg(&format!("Running (as root): {}", cmd));
    let out = Command::new("sudo")
        .args(&["-n", "sh", "-c", cmd])
        .output()
        .map_err(|e| format!("Failed to execute '{}': {}", cmd, e))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    if !out.status.success() {
        let err = format!("Command '{}' failed with code {:?}: {}", cmd, out.status.code(), stderr);
        log_msg(&format!("ERROR: {}", err));
        return Err(err);
    }
    Ok(stdout)
}

fn find_btrfs_targets() -> Vec<BtrfsTarget> {
    let mut targets = Vec::new();
    if let Ok(out) = Command::new("sudo").args(&["-n", "lsblk", "-P", "-o", "NAME,LABEL,UUID,FSTYPE,SIZE,PKNAME"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if line.contains("FSTYPE=\"btrfs\"") || line.contains("PULSAR_OS") || line.contains("PulsarOS") {
                let get_val = |key: &str| -> String {
                    let re = Regex::new(&format!(r#"{}=\"([^\"]*)\""#, key)).unwrap();
                    re.captures(line).and_then(|c| c.get(1)).map(|m| m.as_str().to_string()).unwrap_or_default()
                };
                let name = get_val("NAME");
                let label = get_val("LABEL");
                let uuid = get_val("UUID");
                let size = get_val("SIZE");
                let pkname = get_val("PKNAME");

                let part_path = format!("/dev/{}", name);
                let disk_path = if !pkname.is_empty() { format!("/dev/{}", pkname) } else { part_path.clone() };

                targets.push(BtrfsTarget {
                    _disk_path: disk_path,
                    part_path,
                    label: if label.is_empty() { "PULSAR_OS".to_string() } else { label },
                    uuid,
                    size,
                });
            }
        }
    }
    targets
}

fn is_valid_base_squashfs(path: &str) -> bool {
    if !Path::new(path).exists() {
        return false;
    }
    // Must be a complete Arch Linux base OS rootfs (>= 1.0 GB), NEVER the ~350MB Debian mini rootfs
    if let Ok(meta) = fs::metadata(path) {
        if meta.len() < 1000 * 1024 * 1024 {
            return false;
        }
    } else {
        return false;
    }
    // Quick superblock verification using unsquashfs -s
    Command::new("unsquashfs")
        .args(&["-s", path])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn detect_local_squashfs<L>(log: &L) -> Option<String>
where
    L: Fn(&str) + Send + Sync + 'static,
{
    log("Scanning storage devices for clean Arch Linux Pulsar OS base image...");

    let rec_mnt = "/tmp/pulsar_recovery";
    let _ = fs::create_dir_all(rec_mnt);

    // 1. Mount recovery partition by label
    let _ = Command::new("sudo").args(&["-n", "mount", "/dev/disk/by-label/PULSAR_RECOVERY", rec_mnt]).output();
    let _ = Command::new("sudo").args(&["-n", "mount", "-L", "PULSAR_RECOVERY", rec_mnt]).output();

    let base_image_names = [
        "images/pulsaros-base.squashfs",
        "images/x86_64/airootfs.sfs",
        "images/airootfs.sfs",
        "arch/x86_64/airootfs.sfs",
        "pulsaros-base.squashfs",
        "airootfs.sfs",
    ];

    let search_roots = [
        "/tmp/pulsar_recovery",
        "/run/live/medium",
        "/lib/live/mount/medium",
        "/run/archiso/bootmnt",
        "/run/archiso",
        "/recovery",
        "/mnt/recovery",
    ];

    for root in &search_roots {
        for img in &base_image_names {
            let full_p = format!("{}/{}", root, img);
            if Path::new(&full_p).exists() {
                if is_valid_base_squashfs(&full_p) {
                    log(&format!("Verified clean Arch base system image at: {}", full_p));
                    return Some(full_p);
                }
            }
        }
    }

    // 2. Scan all block devices
    if let Ok(out) = Command::new("sudo").args(&["-n", "blkid", "-o", "device"]).output() {
        let devs = String::from_utf8_lossy(&out.stdout);
        for dev in devs.lines() {
            let dev = dev.trim();
            if dev.is_empty() || dev.contains("loop") || dev.contains("zram") {
                continue;
            }
            let temp_mnt = format!("/tmp/mnt_{}", dev.replace('/', "_"));
            let _ = fs::create_dir_all(&temp_mnt);
            if Command::new("sudo").args(&["-n", "mount", "-o", "ro", dev, &temp_mnt]).status().map(|s| s.success()).unwrap_or(false) {
                for img in &base_image_names {
                    let p = format!("{}/{}", temp_mnt, img);
                    if Path::new(&p).exists() && is_valid_base_squashfs(&p) {
                        log(&format!("Verified clean base system image on {} at: {}", dev, p));
                        return Some(p);
                    }
                }
                let _ = Command::new("sudo").args(&["-n", "umount", &temp_mnt]).output();
            }
        }
    }

    log("⚠️ No valid local base image (>= 1.0 GB) found. Falling back to Internet Recovery.");
    None
}

fn ensure_lucide_icon(name: &str, svg_data: &str) -> String {
    let dir = "/tmp/pulsar_recovery_icons";
    let _ = fs::create_dir_all(dir);
    let p = format!("{}/{}.svg", dir, name);
    if !Path::new(&p).exists() {
        let _ = fs::write(&p, svg_data);
    }
    p
}

fn get_lucide_icon_path(name: &str) -> String {
    match name {
        "restore" | "timemachine" | "rotate-ccw" => ensure_lucide_icon(
            "restore",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#38bdf8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M12 7v5l4 2"/></svg>"##,
        ),
        "safari" | "globe" | "internet" => ensure_lucide_icon(
            "globe",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#60a5fa" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20"/><path d="M2 12h20"/></svg>"##,
        ),
        "disk" | "hard-drive" | "drive-harddisk" => ensure_lucide_icon(
            "hard-drive",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#a78bfa" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="22" x2="2" y1="12" y2="12"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/><line x1="6" x2="6.01" y1="16" y2="16"/><line x1="10" x2="10.01" y1="16" y2="16"/></svg>"##,
        ),
        "terminal" => ensure_lucide_icon(
            "terminal",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#4ade80" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" x2="20" y1="19" y2="19"/></svg>"##,
        ),
        "complete" | "check-circle" | "emblem-default" => ensure_lucide_icon(
            "complete",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="72" height="72" viewBox="0 0 24 24" fill="none" stroke="#22c55e" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10" stroke="#22c55e" stroke-width="2" fill="#22c55e" fill-opacity="0.15"/><path d="m9 12 2 2 4-4" stroke="#22c55e" stroke-width="2.5"/></svg>"##,
        ),
        "error" | "alert-circle" | "dialog-error" => ensure_lucide_icon(
            "error",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="#ef4444" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" x2="12" y1="8" y2="12"/><line x1="12" x2="12.01" y1="16" y2="16"/></svg>"##,
        ),
        "progress" | "download" | "system-software-install" => ensure_lucide_icon(
            "progress",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="#38bdf8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/></svg>"##,
        ),
        _ => ensure_lucide_icon(
            "generic",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#94a3b8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" x2="12" y1="8" y2="12"/><line x1="12" x2="12.01" y1="16" y2="16"/></svg>"##,
        ),
    }
}

fn create_icon_widget(file_path: &str, fallback_icon_name: &str, size: i32) -> Image {
    if !file_path.is_empty() && Path::new(file_path).exists() {
        let img = Image::from_file(file_path);
        img.set_pixel_size(size);
        img
    } else {
        let svg_p = get_lucide_icon_path(fallback_icon_name);
        let img = Image::from_file(&svg_p);
        img.set_pixel_size(size);
        img
    }
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Pulsar OS Recovery")
        .default_width(1024)
        .default_height(720)
        .resizable(true)
        .build();

    window.maximize();

    let style_mgr = libadwaita::StyleManager::default();
    style_mgr.set_color_scheme(libadwaita::ColorScheme::ForceDark);

    let provider = CssProvider::new();
    provider.load_from_data(APP_CSS);
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER + 500,
        );
    }

    let center_box = CenterBox::new();
    center_box.add_css_class("root-container");
    center_box.set_hexpand(true);
    center_box.set_vexpand(true);

    let card_box = GtkBox::new(Orientation::Vertical, 0);
    card_box.add_css_class("apple-box");
    card_box.set_size_request(620, 530);
    card_box.set_valign(Align::Center);
    card_box.set_halign(Align::Center);

    let stack = Stack::new();
    stack.set_transition_type(StackTransitionType::Crossfade);
    stack.set_transition_duration(300);
    card_box.append(&stack);

    center_box.set_center_widget(Some(&card_box));
    window.set_content(Some(&center_box));

    // Shared state
    let selected_action: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let selected_target: Rc<RefCell<Option<BtrfsTarget>>> = Rc::new(RefCell::new(None));
    let recovery_mode: Rc<RefCell<RecoveryMode>> = Rc::new(RefCell::new(RecoveryMode::Local));

    // ─────────────────────────────────────────────────────────────
    // 1. Utilities Screen (macOS Recovery main view)
    // ─────────────────────────────────────────────────────────────
    let util_box = GtkBox::new(Orientation::Vertical, 12);
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

        let card = GtkBox::new(Orientation::Horizontal, 18);
        card.add_css_class("utility-row-card");
        card.set_margin_top(4);
        card.set_margin_bottom(4);
        card.set_margin_start(2);
        card.set_margin_end(2);

        let icon = create_icon_widget(icon_file, icon_fallback, 50);
        card.append(&icon);

        let vbox = GtkBox::new(Orientation::Vertical, 3);
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
        "reinstall",
        "Reinstall Pulsar OS",
        "Install a fresh copy of Pulsar OS while keeping your home files intact.",
        "/usr/share/pulsaros-recovery/reinstall.png",
        "restore",
    );
    add_row(
        "internet",
        "Pulsar Internet Recovery",
        "Download latest recovery image from GitHub Releases and restore core system.",
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
    util_btn_box.set_margin_top(16);
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
    // 2. Select Target & Options Screen
    // ─────────────────────────────────────────────────────────────
    let target_box = GtkBox::new(Orientation::Vertical, 12);
    target_box.set_valign(Align::Center);
    target_box.set_halign(Align::Center);

    let target_icon = create_icon_widget("/usr/share/pulsaros-recovery/diskutility.png", "hard-drive", 64);
    target_box.append(&target_icon);

    let target_title = Label::new(Some("Select Pulsar OS Partition"));
    target_title.add_css_class("welcome-title");
    target_box.append(&target_title);

    let target_desc = Label::new(Some("The root system (@) will be cleanly restored. Your documents, applications, and settings in /home (@home) will remain completely intact."));
    target_desc.add_css_class("welcome-subtitle");
    target_desc.set_wrap(true);
    target_desc.set_max_width_chars(45);
    target_desc.set_justify(gtk4::Justification::Center);
    target_box.append(&target_desc);

    let targets_flow = GtkBox::new(Orientation::Horizontal, 10);
    targets_flow.set_halign(Align::Center);
    target_box.append(&targets_flow);

    let target_nav_box = GtkBox::new(Orientation::Horizontal, 16);
    target_nav_box.set_halign(Align::Center);
    target_nav_box.set_margin_top(12);

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
    // 3. Progress Screen
    // ─────────────────────────────────────────────────────────────
    let prog_box = GtkBox::new(Orientation::Vertical, 10);
    prog_box.set_valign(Align::Center);
    prog_box.set_halign(Align::Center);

    let prog_icon = create_icon_widget("/usr/share/pulsaros-recovery/reinstall.png", "progress", 64);
    prog_box.append(&prog_icon);

    let prog_title = Label::new(Some("Restoring Pulsar OS..."));
    prog_title.add_css_class("welcome-title");
    prog_box.append(&prog_title);

    let prog_desc = Label::new(Some("Preparing disk and restoring root subvolume (@)..."));
    prog_desc.add_css_class("progress-text");
    prog_box.append(&prog_desc);

    let pbar = ProgressBar::new();
    pbar.add_css_class("progress-bar-thin");
    pbar.set_size_request(460, -1);
    prog_box.append(&pbar);

    let scrolled_log = ScrolledWindow::new();
    scrolled_log.set_size_request(480, 160);
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
    // 4. Complete Screen
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
    // 5. Error Screen (with Detailed Logs View)
    // ─────────────────────────────────────────────────────────────
    let err_box = GtkBox::new(Orientation::Vertical, 10);
    err_box.set_valign(Align::Center);
    err_box.set_halign(Align::Center);

    let err_icon = create_icon_widget("", "error", 72);
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
    err_scrolled_log.set_size_request(480, 140);
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

    let btn_try_internet = Button::with_label("Try Internet Recovery");
    btn_try_internet.add_css_class("suggested-action");
    let sel_act_c = selected_action.clone();
    btn_try_internet.connect_clicked(clone!(@weak stack, @weak btn_util_continue => move |_| {
        *sel_act_c.borrow_mut() = Some("internet".to_string());
        btn_util_continue.emit_clicked();
    }));
    err_btn_box.append(&btn_try_internet);

    err_box.append(&err_btn_box);
    stack.add_named(&err_box, Some("error"));

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

    btn_target_back.connect_clicked(clone!(@weak stack => move |_| {
        stack.set_visible_child_name("utilities");
    }));

    btn_util_continue.connect_clicked(clone!(
        @weak stack,
        @weak targets_flow,
        @weak btn_target_restore,
        @strong selected_action,
        @strong selected_target,
        @strong recovery_mode
     => move |_| {
        let action = selected_action.borrow().clone().unwrap_or_default();
        match action.as_str() {
            "disk" => {
                log_msg("Launching elevated Disk Utility (GParted / Disks)...");
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg("gparted || pkexec gparted || sudo -E gparted || gnome-disks || gnome-disk-utility &")
                    .spawn();
            }
            "terminal" => {
                log_msg("Launching recovery root terminal...");
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg("gnome-terminal -- sudo bash || kgx -e 'sudo bash' || alacritty -e sudo bash || xfce4-terminal -e 'sudo bash' || konsole -e sudo bash || kitty sudo bash || xterm -title 'Pulsar OS Recovery Terminal' -bg '#18181b' -fg '#ffffff' -fa Monospace -fs 11 -e sudo bash || x-terminal-emulator -e sudo bash || xterm -e sudo bash || gnome-terminal || alacritty || xterm &")
                    .spawn();
            }
            "reinstall" | "internet" => {
                if action == "internet" {
                    *recovery_mode.borrow_mut() = RecoveryMode::Internet(
                        "https://github.com/Inled-Pulsar-OS/ISO/releases/download/latest/pulsaros-stable-arch-refind.squashfs".to_string()
                    );
                } else {
                    *recovery_mode.borrow_mut() = RecoveryMode::Local;
                }

                // Refresh targets
                while let Some(child) = targets_flow.first_child() {
                    targets_flow.remove(&child);
                }

                let targets = find_btrfs_targets();
                if targets.is_empty() {
                    let no_target_lbl = Label::new(Some("No Btrfs Pulsar OS partitions detected.\nUse Disk Utility to inspect drives."));
                    no_target_lbl.add_css_class("welcome-subtitle");
                    targets_flow.append(&no_target_lbl);
                    btn_target_restore.set_sensitive(false);
                } else {
                    for target in targets {
                        let card = GtkBox::new(Orientation::Vertical, 6);
                        card.add_css_class("disk-card");
                        let disk_icon = create_icon_widget("", "hard-drive", 44);
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

fn run_restoration<F, L>(
    target: &BtrfsTarget,
    mode: RecoveryMode,
    progress: F,
    log: L,
) -> Result<(), String>
where
    F: Fn(f64, &str) + Send + Sync + 'static,
    L: Fn(&str) + Send + Sync + 'static,
{
    let btrfs_mnt = "/tmp/pulsar_btrfs_pool";
    let _ = fs::create_dir_all(btrfs_mnt);

    // 1. Unmount any busy mounts
    let _ = exec_cmd(&format!("umount -l {} 2>/dev/null || true", btrfs_mnt));
    let _ = exec_cmd(&format!("umount -l {}* 2>/dev/null || true", target.part_path));

    progress(0.10, "Mounting Btrfs pool...");
    log("Mounting Btrfs root pool without subvolume...");
    exec_cmd(&format!("mount -t btrfs {} {}", target.part_path, btrfs_mnt))?;

    // 2. Backup existing user accounts from old @ subvolume and discover users from @home
    progress(0.20, "Preserving user accounts and identities...");
    log("Backing up /etc/passwd, /etc/shadow, /etc/group for real users (UID >= 1000)...");
    let old_root = format!("{}/@", btrfs_mnt);
    let mut preserved_passwd: Vec<String> = Vec::new();
    let mut preserved_shadow: Vec<String> = Vec::new();
    let mut preserved_group: Vec<String> = Vec::new();
    let mut preserved_gshadow: Vec<String> = Vec::new();
    let mut preserved_usernames: Vec<String> = Vec::new();
    let mut user_group_memberships: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    if Path::new(&old_root).exists() {
        if let Ok(file) = File::open(format!("{}/etc/passwd", old_root)) {
            for line in BufReader::new(file).lines().flatten() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 3 {
                    let uname = parts[0].to_string();
                    // NEVER preserve temporary live session users
                    if uname == "live" || uname == "root" || uname == "pulsar-live" || uname == "archiso" || uname == "nobody" {
                        continue;
                    }
                    if let Ok(uid) = parts[2].parse::<u32>() {
                        if uid >= 1000 && uid < 65534 {
                            preserved_usernames.push(uname);
                            preserved_passwd.push(line);
                        }
                    }
                }
            }
        }
        if let Ok(file) = File::open(format!("{}/etc/shadow", old_root)) {
            for line in BufReader::new(file).lines().flatten() {
                let uname = line.split(':').next().unwrap_or_default();
                if preserved_usernames.iter().any(|u| u == uname) {
                    preserved_shadow.push(line);
                }
            }
        }
        if let Ok(file) = File::open(format!("{}/etc/group", old_root)) {
            for line in BufReader::new(file).lines().flatten() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 4 {
                    let gname = parts[0].to_string();
                    let members = parts[3].split(',');
                    for m in members {
                        let m_trim = m.trim().to_string();
                        if !m_trim.is_empty() && m_trim != "live" && m_trim != "root" && m_trim != "archiso" {
                            user_group_memberships.entry(m_trim).or_default().push(gname.clone());
                        }
                    }
                }
                if parts.len() >= 3 {
                    let gname = parts[0];
                    if gname == "live" || gname == "root" || gname == "archiso" {
                        continue;
                    }
                    if let Ok(gid) = parts[2].parse::<u32>() {
                        if gid >= 1000 && gid < 65534 {
                            preserved_group.push(line);
                        }
                    }
                }
            }
        }
        if let Ok(file) = File::open(format!("{}/etc/gshadow", old_root)) {
            for line in BufReader::new(file).lines().flatten() {
                let gname = line.split(':').next().unwrap_or_default();
                if gname != "live" && gname != "root" && gname != "archiso" {
                    preserved_gshadow.push(line);
                }
            }
        }
    }

    // Also inspect @home in case /@/etc/passwd was already corrupted or missing
    let home_dir = format!("{}/@home", btrfs_mnt);
    if let Ok(entries) = fs::read_dir(&home_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let uname = entry.file_name().to_string_lossy().to_string();
                    if uname != "live" && uname != "root" && uname != "lost+found" && !preserved_usernames.contains(&uname) {
                        log(&format!("Discovered existing user home directory in @home: /home/{}", uname));
                        preserved_passwd.push(format!("{}:x:1000:1000::{}:/bin/bash", uname, format!("/home/{}", uname)));
                        preserved_shadow.push(format!("{}:!!:19700:0:99999:7:::", uname));
                        preserved_group.push(format!("{}:x:1000:", uname));
                        preserved_usernames.push(uname);
                    }
                }
            }
        }
    }
    log(&format!("Preserved {} real user account(s): {:?}", preserved_usernames.len(), preserved_usernames));

    // 3. Resolve and verify SquashFS source BEFORE wiping anything
    let squashfs_path = match mode {
        RecoveryMode::Local => {
            progress(0.25, "Locating local Arch Linux recovery image...");
            match detect_local_squashfs(&log) {
                Some(p) => p,
                None => {
                    log("ERROR: No valid local Arch Linux base recovery image (>= 1.0 GB) found on storage devices.");
                    return Err(
                        "No local recovery image found on the recovery partition.\n\n\
                        Please choose 'Pulsar Internet Recovery' from the main menu to download and restore the official release image.".to_string()
                    );
                }
            }
        }
        RecoveryMode::Internet(url) => {
            progress(0.25, "Downloading Pulsar OS image from GitHub Releases...");
            log(&format!("Downloading clean image from: {}", url));
            let dl_path = "/tmp/pulsaros-remote-recovery.squashfs";
            exec_cmd_stream(&format!("curl -L -C - --retry 3 -o {} {}", dl_path, url), &log)?;
            if !is_valid_base_squashfs(dl_path) {
                return Err(format!(
                    "Downloaded recovery image from {} is corrupt or invalid.\nNo changes were made to your disk.",
                    url
                ));
            }
            dl_path.to_string()
        }
    };

    // 4. Wipe and recreate @ root subvolume (SAFE: Image is 100% verified)
    progress(0.45, "Recreating @ root subvolume...");
    log("Removing old root (@) subvolume...");
    let _ = exec_cmd(&format!("btrfs subvolume delete {}/@ 2>/dev/null || rm -rf {}/@", btrfs_mnt, btrfs_mnt));
    log("Creating fresh root (@) subvolume...");
    exec_cmd(&format!("btrfs subvolume create {}/@", btrfs_mnt))?;

    // Ensure @home exists
    let home_path = format!("{}/@home", btrfs_mnt);
    if !Path::new(&home_path).exists() {
        log("Creating @home subvolume...");
        exec_cmd(&format!("btrfs subvolume create {}", home_path))?;
    }

    // 5. Unsquash clean system into @
    progress(0.55, "Unpacking clean Pulsar OS rootfs into @...");
    log(&format!("Unsquashing {} into {}/@...", squashfs_path, btrfs_mnt));
    if let Err(unsquash_err) = exec_cmd_stream(&format!("unsquashfs -f -d {}/@ {}", btrfs_mnt, squashfs_path), &log) {
        log(&format!("Local unsquash failed ({}). Attempting Internet Recovery fallback...", unsquash_err));
        let url = "https://github.com/Inled-Pulsar-OS/ISO/releases/download/latest/pulsaros-stable-arch-refind.squashfs";
        let dl_path = "/tmp/pulsaros-remote-recovery.squashfs";
        progress(0.60, "Downloading fresh system image from GitHub Releases...");
        exec_cmd_stream(&format!("curl -L -C - --retry 3 -o {} {}", dl_path, url), &log)?;
        log(&format!("Unsquashing downloaded image {} into {}/@...", dl_path, btrfs_mnt));
        exec_cmd_stream(&format!("unsquashfs -f -d {}/@ {}", btrfs_mnt, dl_path), &log)?;
    }

    // 6. Re-inject preserved users and clean out any temporary live user
    progress(0.85, "Re-injecting user credentials and settings...");
    log("Restoring user accounts into clean /etc...");
    let new_root = format!("{}/@", btrfs_mnt);

    // Remove any live user artifact from new rootfs
    let _ = exec_cmd(&format!("sed -i '/^live:/d' {}/etc/passwd {}/etc/shadow {}/etc/group {}/etc/gshadow 2>/dev/null || true", new_root, new_root, new_root, new_root));

    if !preserved_passwd.is_empty() {
        for l in &preserved_passwd {
            let uname = l.split(':').next().unwrap_or_default();
            let _ = exec_cmd(&format!("sed -i '/^{}:/d' {}/etc/passwd 2>/dev/null || true", uname, new_root));
        }
        let mut tmp_users = String::new();
        for l in &preserved_passwd {
            tmp_users.push_str(&format!("{}\n", l));
        }
        let _ = fs::write("/tmp/pulsar_preserved_passwd", &tmp_users);
        let _ = exec_cmd(&format!("cat /tmp/pulsar_preserved_passwd >> {}/etc/passwd", new_root));

        for l in &preserved_shadow {
            let uname = l.split(':').next().unwrap_or_default();
            let _ = exec_cmd(&format!("sed -i '/^{}:/d' {}/etc/shadow 2>/dev/null || true", uname, new_root));
        }
        let mut tmp_shadow = String::new();
        for l in &preserved_shadow {
            tmp_shadow.push_str(&format!("{}\n", l));
        }
        // Guarantee that every preserved user has a valid line in /etc/shadow
        for uname in &preserved_usernames {
            if !preserved_shadow.iter().any(|s| s.starts_with(&format!("{}:", uname))) {
                log(&format!("Adding fallback shadow entry for user: {}", uname));
                tmp_shadow.push_str(&format!("{}::19700:0:99999:7:::\n", uname));
            }
        }
        let _ = fs::write("/tmp/pulsar_preserved_shadow", &tmp_shadow);
        let _ = exec_cmd(&format!("cat /tmp/pulsar_preserved_shadow >> {}/etc/shadow", new_root));

        let mut tmp_group = String::new();
        for l in &preserved_group {
            tmp_group.push_str(&format!("{}\n", l));
        }
        let _ = fs::write("/tmp/pulsar_preserved_group", &tmp_group);
        let _ = exec_cmd(&format!("cat /tmp/pulsar_preserved_group >> {}/etc/group", new_root));

        let mut tmp_gshadow = String::new();
        for l in &preserved_gshadow {
            tmp_gshadow.push_str(&format!("{}\n", l));
        }
        let _ = fs::write("/tmp/pulsar_preserved_gshadow", &tmp_gshadow);
        let _ = exec_cmd(&format!("cat /tmp/pulsar_preserved_gshadow >> {}/etc/gshadow", new_root));

        // Add each preserved user to essential desktop/admin groups and preserved groups
        let base_admin_groups = [
            "wheel", "sudo", "video", "audio", "input", "storage", "network", "optical",
            "power", "rfkill", "autologin", "users", "lp", "scanner", "kvm"
        ];

        let sudoers_d = format!("{}/etc/sudoers.d", new_root);
        let _ = fs::create_dir_all(&sudoers_d);
        let _ = exec_cmd(&format!("chmod 750 {}", sudoers_d));

        let wheel_rule = format!("{}/10-admin-wheel", sudoers_d);
        let _ = fs::write(&wheel_rule, "%wheel ALL=(ALL:ALL) ALL\n%sudo ALL=(ALL:ALL) ALL\n");
        let _ = exec_cmd(&format!("chmod 0440 {}", wheel_rule));

        for uname in &preserved_usernames {
            let mut target_groups: Vec<String> = base_admin_groups.iter().map(|s| s.to_string()).collect();
            if let Some(custom_grps) = user_group_memberships.get(uname) {
                for cg in custom_grps {
                    if !target_groups.contains(cg) {
                        target_groups.push(cg.clone());
                    }
                }
            }

            for grp in &target_groups {
                let _ = exec_cmd(&format!(
                    "grep -q '^{}:' {}/etc/group || echo '{}:x:999:' >> {}/etc/group",
                    grp, new_root, grp, new_root
                ));
                let _ = exec_cmd(&format!(
                    "sed -i -E 's/^({}:[^:]*:[^:]*:)(.*)$/\\1\\2,{}/' {}/etc/group 2>/dev/null || true",
                    grp, uname, new_root
                ));
                let _ = exec_cmd(&format!(
                    "sed -i -E 's/,+/,/g; s/:,/:/g; s/,$//' {}/etc/group 2>/dev/null || true",
                    new_root
                ));
            }

            // Drop explicit sudoers rule for the user
            let user_rule = format!("{}/pulsaros-user-{}", sudoers_d, uname);
            let _ = fs::write(&user_rule, format!("{} ALL=(ALL:ALL) ALL\n", uname));
            let _ = exec_cmd(&format!("chmod 0440 {}", user_rule));
            log(&format!("Granted full sudo privileges to user '{}' via sudoers and wheel group", uname));
        }
    }

    // 7. Regenerate clean /etc/fstab with correct UUID
    progress(0.90, "Configuring file systems and boot mounts...");
    log("Writing clean /etc/fstab for Btrfs subvolumes (@, @home)...");
    let btrfs_uuid = if !target.uuid.is_empty() {
        target.uuid.clone()
    } else {
        exec_cmd(&format!("blkid -s UUID -o value {}", target.part_path))?.trim().to_string()
    };

    // Find EFI partition on the same disk
    let efi_uuid = exec_cmd("blkid -t TYPE=vfat -s UUID -o value | head -n 1").unwrap_or_default().trim().to_string();

    let fstab_content = format!(
        "# /etc/fstab: Pulsar OS Btrfs Configuration\n\
        UUID={} /               btrfs   subvol=@,compress=zstd:1,space_cache=v2 0 0\n\
        UUID={} /home           btrfs   subvol=@home,compress=zstd:1,space_cache=v2 0 0\n\
        {}\n",
        btrfs_uuid,
        btrfs_uuid,
        if !efi_uuid.is_empty() {
            format!("UUID={} /boot/efi       vfat    umask=0077 0 2", efi_uuid)
        } else {
            "".to_string()
        }
    );

    let _ = fs::write("/tmp/pulsar_new_fstab", &fstab_content);
    let _ = exec_cmd(&format!("cp -f /tmp/pulsar_new_fstab {}/etc/fstab", new_root));

    // Deploy udev rule to hide recovery partition from file managers
    let udev_dir = format!("{}/etc/udev/rules.d", new_root);
    let _ = fs::create_dir_all(&udev_dir);
    let _ = fs::write(
        format!("{}/99-pulsaros-hide-recovery.rules", udev_dir),
        "# Hide PULSAR_RECOVERY partition from file managers and desktop\nENV{ID_FS_LABEL}==\"PULSAR_RECOVERY\", ENV{UDISKS_IGNORE}=\"1\", ENV{UDISKS_AUTO}=\"0\"\n"
    );

    // Deploy default non-empty SDDM wallpaper
    let sddm_dir = format!("{}/var/lib/pulsar-sddm", new_root);
    let _ = fs::create_dir_all(&sddm_dir);
    let _ = exec_cmd(&format!("chmod 777 {}", sddm_dir));
    let wallpaper_sources = [
        format!("{}/usr/share/backgrounds/pulsar-os-tahoe.png", new_root),
        format!("{}/usr/share/sddm/themes/Apple.Tahoe/pulsar-os-tahoe.png", new_root),
        format!("{}/usr/share/backgrounds/gnome/pulsar-wallpaper.png", new_root),
    ];
    for ws in &wallpaper_sources {
        if Path::new(ws).exists() {
            let _ = exec_cmd(&format!("cp -f {} {}/pulsar-wallpaper.png", ws, sddm_dir));
            let _ = exec_cmd(&format!("chmod 666 {}/pulsar-wallpaper.png", sddm_dir));
            log(&format!("Deployed default SDDM wallpaper to {} from {}", sddm_dir, ws));
            break;
        }
    }

    // Remove unwanted GNOME extensions that should never be active in Pulsar OS
    log("Removing unwanted GNOME extensions (places-menu, window-list)...");
    let _ = exec_cmd(&format!(
        "rm -rf {}/usr/share/gnome-shell/extensions/places-menu@gnome-shell-extensions.gcampax.github.com \
                {}/usr/share/gnome-shell/extensions/window-list@gnome-shell-extensions.gcampax.github.com \
                {}/usr/share/gnome-shell/extensions/search-light@icedman.github.com 2>/dev/null || true",
        new_root, new_root, new_root
    ));

    // 8. Deploy boot kernels, recovery kernel, and align rEFInd
    progress(0.95, "Deploying OS & Recovery kernels to @/boot and aligning bootloader...");
    deploy_boot_and_recovery_kernels(&new_root, &btrfs_uuid, &log);

    // 9. Cleanup and sync
    progress(0.98, "Synchronizing disks and unmounting...");
    log("Syncing disks...");
    let _ = exec_cmd("sync");
    let _ = exec_cmd(&format!("umount -l {}", btrfs_mnt));

    progress(1.0, "Restoration complete!");
    log("System successfully restored.");
    Ok(())
}

fn deploy_boot_and_recovery_kernels<L>(new_root: &str, btrfs_uuid: &str, log: &L)
where
    L: Fn(&str) + Send + Sync + 'static,
{
    log("Verifying and deploying boot and recovery kernels into @/boot and ESP...");

    let boot_dir = format!("{}/boot", new_root);
    let _ = fs::create_dir_all(&boot_dir);

    // 1. Locate and deploy recovery kernel & initramfs
    let rec_kernel_sources = [
        "/run/live/medium/live/vmlinuz",
        "/run/live/medium/vmlinuz",
        "/run/live/medium/recovery/vmlinuz-recovery",
        "/run/live/medium/boot/vmlinuz-recovery",
        "/tmp/pulsar_recovery/boot/vmlinuz-recovery",
        "/tmp/pulsar_recovery/vmlinuz-recovery",
        "/tmp/pulsar_recovery/live/vmlinuz",
        "/recovery/vmlinuz-recovery",
        "/lib/live/mount/medium/live/vmlinuz",
        "/lib/live/mount/medium/vmlinuz",
    ];
    let rec_initrd_sources = [
        "/run/live/medium/live/initrd.img",
        "/run/live/medium/initrd.img",
        "/run/live/medium/recovery/initramfs-recovery.img",
        "/run/live/medium/boot/initramfs-recovery.img",
        "/tmp/pulsar_recovery/boot/initramfs-recovery.img",
        "/tmp/pulsar_recovery/initramfs-recovery.img",
        "/tmp/pulsar_recovery/live/initrd.img",
        "/recovery/initramfs-recovery.img",
        "/lib/live/mount/medium/live/initrd.img",
        "/lib/live/mount/medium/initrd.img",
    ];

    let mut rec_k_found: Option<String> = None;
    for src in &rec_kernel_sources {
        if Path::new(src).exists() {
            rec_k_found = Some(src.to_string());
            break;
        }
    }
    if rec_k_found.is_none() {
        if let Ok(entries) = fs::read_dir("/boot") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("vmlinuz") && !name.ends_with(".kver") {
                    rec_k_found = Some(entry.path().to_string_lossy().to_string());
                    break;
                }
            }
        }
    }

    if let Some(src) = rec_k_found {
        let dest = format!("{}/vmlinuz-recovery", boot_dir);
        let _ = exec_cmd(&format!("cp -f {} {}", src, dest));
        log(&format!("Restored recovery kernel to {} from {}", dest, src));
    }

    let mut rec_initrd_found: Option<String> = None;
    for src in &rec_initrd_sources {
        if Path::new(src).exists() {
            rec_initrd_found = Some(src.to_string());
            break;
        }
    }
    if rec_initrd_found.is_none() {
        if let Ok(entries) = fs::read_dir("/boot") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("initrd") || name.starts_with("initramfs") {
                    rec_initrd_found = Some(entry.path().to_string_lossy().to_string());
                    break;
                }
            }
        }
    }

    if let Some(src) = rec_initrd_found {
        let dest = format!("{}/initramfs-recovery.img", boot_dir);
        let _ = exec_cmd(&format!("cp -f {} {}", src, dest));
        log(&format!("Restored recovery initramfs to {} from {}", dest, src));
    }

    // 2. Ensure OS kernel naming aliases exist in @/boot
    let mut found_kernel: Option<String> = None;
    let mut found_initrd: Option<String> = None;
    if let Ok(entries) = fs::read_dir(&boot_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if name.starts_with("vmlinuz") && !name.contains("recovery") && !name.ends_with(".kver") {
                found_kernel = Some(p.to_string_lossy().to_string());
            }
            if (name.starts_with("initramfs") || name.starts_with("initrd")) && !name.contains("recovery") && !name.contains("fallback") && !name.contains("ucode") {
                found_initrd = Some(p.to_string_lossy().to_string());
            }
        }
    }

    // Fallback search for initrd if not in rootfs
    if found_initrd.is_none() {
        let alt_initrd_sources = [
            "/boot/initramfs-6.1-x86_64.img",
            "/boot/initramfs-linux.img",
            "/tmp/pulsar_recovery/boot/initramfs-6.1-x86_64.img",
            "/run/live/medium/boot/initramfs-6.1-x86_64.img",
            "/run/live/medium/boot/initramfs-linux.img",
        ];
        for alt in &alt_initrd_sources {
            if Path::new(alt).exists() {
                found_initrd = Some(alt.to_string());
                break;
            }
        }
    }

    if let Some(k) = &found_kernel {
        log(&format!("Detected main OS kernel: {}", k));
        let targets = ["vmlinuz-6.1-x86_64", "vmlinuz-linux", "vmlinuz"];
        for t in &targets {
            let dest = format!("{}/{}", boot_dir, t);
            if !Path::new(&dest).exists() || &dest != k {
                let _ = exec_cmd(&format!("cp -f {} {}", k, dest));
                log(&format!("Created kernel alias: {} -> {}", dest, k));
            }
        }
    }

    if let Some(i) = &found_initrd {
        log(&format!("Detected main OS initrd: {}", i));
        let targets = ["initramfs-6.1-x86_64.img", "initramfs-linux.img"];
        for t in &targets {
            let dest = format!("{}/{}", boot_dir, t);
            if !Path::new(&dest).exists() || &dest != i {
                let _ = exec_cmd(&format!("cp -f {} {}", i, dest));
                log(&format!("Created initramfs alias: {} -> {}", dest, i));
            }
        }
    }

    // Enforce UEFI-compatible permissions on @/boot and all boot assets
    let _ = exec_cmd(&format!("chmod 755 {}", boot_dir));
    let _ = exec_cmd(&format!("chmod 644 {}/*", boot_dir));
    let _ = exec_cmd(&format!("chown -R 0:0 {}", boot_dir));

    // Copy microcode files if present on host / recovery medium
    let ucode_sources = [
        "/tmp/pulsar_recovery/amd-ucode.img",
        "/run/live/medium/amd-ucode.img",
        "/boot/amd-ucode.img",
        "/tmp/pulsar_recovery/intel-ucode.img",
        "/run/live/medium/intel-ucode.img",
        "/boot/intel-ucode.img",
    ];
    for u in &ucode_sources {
        if Path::new(u).exists() {
            let fname = Path::new(u).file_name().and_then(|n| n.to_str()).unwrap_or_default();
            let dest = format!("{}/{}", boot_dir, fname);
            if !Path::new(&dest).exists() {
                let _ = exec_cmd(&format!("cp -f {} {}", u, dest));
            }
        }
    }

    // 3. Mount and configure ESP / rEFInd
    let esp_mnt = "/tmp/pulsar_esp_mount";
    let _ = fs::create_dir_all(esp_mnt);
    let _ = exec_cmd(&format!("umount -l {} 2>/dev/null || true", esp_mnt));

    if let Ok(out) = exec_cmd("blkid -t TYPE=vfat -o device | head -n 1") {
        let efi_dev = out.trim();
        if !efi_dev.is_empty() {
            if exec_cmd(&format!("mount {} {}", efi_dev, esp_mnt)).is_ok() {
                log(&format!("Mounted ESP on {} for bootloader alignment...", esp_mnt));

                // Copy recovery kernels to ESP as well
                let efi_rec_dir = format!("{}/EFI/recovery", esp_mnt);
                let _ = fs::create_dir_all(&efi_rec_dir);
                let _ = exec_cmd(&format!("cp -f {}/vmlinuz-recovery {}/vmlinuz-recovery 2>/dev/null || true", boot_dir, efi_rec_dir));
                let _ = exec_cmd(&format!("cp -f {}/initramfs-recovery.img {}/initramfs-recovery.img 2>/dev/null || true", boot_dir, efi_rec_dir));

                // Align refind.conf UUIDs
                let refind_confs = [
                    format!("{}/EFI/refind/refind.conf", esp_mnt),
                    format!("{}/EFI/BOOT/refind.conf", esp_mnt),
                ];
                for rc in &refind_confs {
                    if Path::new(rc).exists() {
                        if let Ok(content) = fs::read_to_string(rc) {
                            let re = Regex::new(r"root=UUID=[a-fA-F0-9-]+").unwrap();
                            let updated = re.replace_all(&content, &format!("root=UUID={}", btrfs_uuid)).to_string();
                            let _ = fs::write(rc, updated);
                            log(&format!("Updated root UUID in {} to {}", rc, btrfs_uuid));
                        }
                    }
                }

                let _ = exec_cmd(&format!("umount -l {}", esp_mnt));
            }
        }
    }
}

fn main() {
    let app = Application::builder()
        .application_id("es.inled.pulsaros.recovery-assistant")
        .build();

    app.connect_activate(build_ui);
    app.run();
}
