use std::cell::RefCell;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::Command;
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
    background-color: #1c1c1e !important;
    color: #ffffff !important;
}
window, .root-container, * {
    font-family: 'Inter', 'SF Pro Display', -apple-system, sans-serif;
}
.apple-box {
    background-color: #2c2c2e !important;
    border: 1px solid rgba(255, 255, 255, 0.12) !important;
    border-radius: 18px !important;
    padding: 28px !important;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.8) !important;
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
    color: #98989d !important;
    margin-bottom: 16px !important;
}
/* Force Pure Dark ListBox & Rows (completely eliminates light theme override) */
list, listview, listbox {
    background-color: #202022 !important;
    background-image: none !important;
    border: 1px solid rgba(255, 255, 255, 0.08) !important;
    border-radius: 12px !important;
    padding: 0 !important;
    margin: 0 !important;
}
listbox > row, listboxrow, row {
    background-color: transparent !important;
    background-image: none !important;
    border: none !important;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06) !important;
    margin: 0 !important;
    padding: 0 !important;
    color: #ffffff !important;
}
listbox > row:last-child, listboxrow:last-child {
    border-bottom: none !important;
}
listbox > row:hover, listboxrow:hover {
    background-color: rgba(255, 255, 255, 0.07) !important;
}
listbox > row:selected, listboxrow:selected, listbox > row:selected:focus, listboxrow:selected:focus {
    background-color: rgba(255, 255, 255, 0.12) !important;
    color: #ffffff !important;
}
.utility-row-box {
    padding: 12px 16px !important;
}
.utility-title-lbl {
    font-size: 15px !important;
    font-weight: 600 !important;
    color: #ffffff !important;
}
.utility-desc-lbl {
    font-size: 13px !important;
    color: #98989d !important;
}
listbox > row:selected .utility-title-lbl,
listboxrow:selected .utility-title-lbl {
    color: #ffffff !important;
}
listbox > row:selected .utility-desc-lbl,
listboxrow:selected .utility-desc-lbl {
    color: #e5e5ea !important;
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
    disk_path: String,
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

fn exec_cmd(cmd: &str) -> Result<String, String> {
    log_msg(&format!("Running: {}", cmd));
    let out = Command::new("sh")
        .arg("-c")
        .arg(cmd)
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
    if let Ok(out) = Command::new("lsblk").args(&["-P", "-o", "NAME,LABEL,UUID,FSTYPE,SIZE,PKNAME"]).output() {
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
                    disk_path,
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

fn detect_local_squashfs() -> Option<String> {
    let candidates = [
        "/recovery/images/pulsaros-base.squashfs",
        "/run/archiso/bootmnt/live/x86_64/airootfs.sfs",
        "/run/archiso/bootmnt/live/filesystem.squashfs",
        "/run/live/medium/live/filesystem.squashfs",
        "/lib/live/mount/medium/live/filesystem.squashfs",
        "/run/archiso/airootfs.sfs",
    ];
    for p in &candidates {
        if Path::new(p).exists() {
            return Some(p.to_string());
        }
    }
    None
}

fn create_icon_widget(file_path: &str, fallback_icon_name: &str, size: i32) -> Image {
    if Path::new(file_path).exists() {
        let img = Image::from_file(file_path);
        img.set_pixel_size(size);
        img
    } else {
        let img = Image::from_icon_name(fallback_icon_name);
        img.set_pixel_size(size);
        img
    }
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Pulsar OS Recovery")
        .default_width(760)
        .default_height(590)
        .resizable(true)
        .build();

    let style_mgr = libadwaita::StyleManager::default();
    style_mgr.set_color_scheme(libadwaita::ColorScheme::ForceDark);

    let provider = CssProvider::new();
    provider.load_from_data(APP_CSS);
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );
    }


    let center_box = CenterBox::new();
    center_box.add_css_class("root-container");
    center_box.set_hexpand(true);
    center_box.set_vexpand(true);

    let card_box = GtkBox::new(Orientation::Vertical, 0);
    card_box.add_css_class("apple-box");
    card_box.set_size_request(560, 450);
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
    listbox.set_selection_mode(SelectionMode::Single);

    let add_row = |id: &str, title: &str, desc: &str, icon_file: &str, icon_fallback: &str| {
        let row = ListBoxRow::new();
        let hbox = GtkBox::new(Orientation::Horizontal, 18);
        hbox.add_css_class("utility-row-box");

        let icon = create_icon_widget(icon_file, icon_fallback, 52);
        hbox.append(&icon);

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

        hbox.append(&vbox);
        row.set_child(Some(&hbox));
        unsafe {
            row.set_data("action_id", id.to_string());
        }
        listbox.append(&row);
    };

    add_row(
        "backup",
        "Restore from Time Machine",
        "If you have a backup of your system that you want to restore.",
        "/usr/share/pulsaros-recovery/timemachine.png",
        "org.gnome.DejaDup",
    );
    add_row(
        "reinstall",
        "Reinstall Pulsar OS",
        "Install a fresh copy of Pulsar OS while keeping your home files intact.",
        "/usr/share/pulsaros-recovery/reinstall.png",
        "system-software-install",
    );
    add_row(
        "internet",
        "Pulsar Internet Recovery",
        "Download latest recovery image from GitHub Releases and restore core system.",
        "/usr/share/pulsaros-recovery/safari.png",
        "preferences-system-network",
    );
    add_row(
        "disk",
        "Disk Utility",
        "Repair, inspect, or manage disk partitions with GParted.",
        "/usr/share/pulsaros-recovery/diskutility.png",
        "gparted",
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

    let target_icon = Image::from_icon_name("drive-harddisk");
    target_icon.set_pixel_size(64);
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

    let prog_icon = Image::from_icon_name("system-software-install");
    prog_icon.set_pixel_size(72);
    prog_box.append(&prog_icon);

    let prog_title = Label::new(Some("Restoring Pulsar OS..."));
    prog_title.add_css_class("welcome-title");
    prog_box.append(&prog_title);

    let prog_desc = Label::new(Some("Preparing disk and restoring root subvolume (@)..."));
    prog_desc.add_css_class("progress-text");
    prog_box.append(&prog_desc);

    let pbar = ProgressBar::new();
    pbar.add_css_class("progress-bar-thin");
    pbar.set_size_request(380, -1);
    prog_box.append(&pbar);

    let scrolled_log = ScrolledWindow::new();
    scrolled_log.set_size_request(440, 140);
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

    let done_icon = Image::from_icon_name("emblem-default");
    done_icon.set_pixel_size(72);
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
        let _ = Command::new("systemctl").arg("reboot").spawn();
    });
    done_box.append(&btn_reboot);

    stack.add_named(&done_box, Some("complete"));

    // ─────────────────────────────────────────────────────────────
    // 5. Error Screen
    // ─────────────────────────────────────────────────────────────
    let err_box = GtkBox::new(Orientation::Vertical, 14);
    err_box.set_valign(Align::Center);
    err_box.set_halign(Align::Center);

    let err_icon = Image::from_icon_name("dialog-error");
    err_icon.set_pixel_size(72);
    err_box.append(&err_icon);

    let err_title = Label::new(Some("Restoration Failed"));
    err_title.add_css_class("welcome-title");
    err_box.append(&err_title);

    let err_desc = Label::new(Some("An error occurred during system restoration. Check the logs below for details."));
    err_desc.add_css_class("welcome-subtitle");
    err_box.append(&err_desc);

    let btn_err_back = Button::with_label("Back to Utilities");
    btn_err_back.add_css_class("secondary-action");
    btn_err_back.connect_clicked(clone!(@weak stack => move |_| {
        stack.set_visible_child_name("utilities");
    }));
    err_box.append(&btn_err_back);

    stack.add_named(&err_box, Some("error"));

    // ─────────────────────────────────────────────────────────────
    // Callbacks & Connections
    // ─────────────────────────────────────────────────────────────
    listbox.connect_row_selected(clone!(@weak btn_util_continue, @strong selected_action => move |_, row| {
        if let Some(r) = row {
            unsafe {
                if let Some(id) = r.data::<String>("action_id") {
                    *selected_action.borrow_mut() = Some(id.as_ref().clone());
                    btn_util_continue.set_sensitive(true);
                }
            }
        }
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
            "backup" => {
                let _ = Command::new("timeshift-launcher")
                    .spawn()
                    .or_else(|_| Command::new("pkexec").arg("timeshift-gtk").spawn())
                    .or_else(|_| Command::new("timeshift-gtk").spawn())
                    .or_else(|_| Command::new("deja-dup").arg("--restore").spawn())
                    .or_else(|_| Command::new("deja-dup").spawn());
            }
            "disk" => {
                let _ = Command::new("gparted")
                    .spawn()
                    .or_else(|_| Command::new("pkexec").arg("gparted").spawn())
                    .or_else(|_| Command::new("gnome-disks").spawn())
                    .or_else(|_| Command::new("pkexec").arg("gnome-disks").spawn());
            }
            "terminal" => {
                let _ = Command::new("alacritty")
                    .spawn()
                    .or_else(|_| Command::new("gnome-terminal").spawn())
                    .or_else(|_| Command::new("xterm").spawn());
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
                        let disk_icon = Image::from_icon_name("drive-harddisk");
                        disk_icon.set_pixel_size(44);
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

        glib::timeout_add_local(std::time::Duration::from_millis(60), move || {
            while let Ok(msg) = receiver.try_recv() {
                match msg {
                    RecoveryUpdate::Progress(fraction, text) => {
                        pbar_c.set_fraction(fraction);
                        desc_c.set_label(&text);
                    }
                    RecoveryUpdate::Log(line) => {
                        let mut end = buffer.end_iter();
                        buffer.insert(&mut end, &format!("{}\n", line));
                    }
                    RecoveryUpdate::Finished(res) => {
                        match res {
                            Ok(_) => {
                                stack_c.set_visible_child_name("complete");
                            }
                            Err(e) => {
                                log_msg(&format!("Restoration error: {}", e));
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

    // 2. Backup existing user accounts from old @ subvolume
    progress(0.20, "Preserving user accounts and identities...");
    log("Backing up /etc/passwd, /etc/shadow, /etc/group for users with UID >= 1000...");
    let old_root = format!("{}/@", btrfs_mnt);
    let mut preserved_passwd = Vec::new();
    let mut preserved_shadow = Vec::new();
    let mut preserved_group = Vec::new();
    let mut preserved_gshadow = Vec::new();

    if Path::new(&old_root).exists() {
        if let Ok(file) = File::open(format!("{}/etc/passwd", old_root)) {
            for line in BufReader::new(file).lines().flatten() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 3 {
                    if let Ok(uid) = parts[2].parse::<u32>() {
                        if uid >= 1000 && uid < 65534 {
                            preserved_passwd.push(line);
                        }
                    }
                }
            }
        }
        if let Ok(file) = File::open(format!("{}/etc/shadow", old_root)) {
            for line in BufReader::new(file).lines().flatten() {
                let uname = line.split(':').next().unwrap_or_default();
                if preserved_passwd.iter().any(|p| p.starts_with(&format!("{}:", uname))) {
                    preserved_shadow.push(line);
                }
            }
        }
        if let Ok(file) = File::open(format!("{}/etc/group", old_root)) {
            for line in BufReader::new(file).lines().flatten() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 3 {
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
                if preserved_group.iter().any(|g| g.starts_with(&format!("{}:", gname))) {
                    preserved_gshadow.push(line);
                }
            }
        }
        log(&format!("Preserved {} user account(s) and credentials.", preserved_passwd.len()));
    }

    // 3. Resolve SquashFS source
    let squashfs_path = match mode {
        RecoveryMode::Local => {
            progress(0.30, "Locating local recovery image...");
            log("Searching for local recovery squashfs...");
            detect_local_squashfs().ok_or_else(|| {
                "No local recovery SquashFS found in /recovery/images or live media. Please use Internet Recovery.".to_string()
            })?
        }
        RecoveryMode::Internet(url) => {
            progress(0.25, "Downloading Pulsar OS image from GitHub Releases...");
            log(&format!("Downloading clean image from: {}", url));
            let dl_path = "/tmp/pulsaros-remote-recovery.squashfs";
            let dl_cmd = format!("curl -L -C - --retry 3 -o {} {}", dl_path, url);
            exec_cmd(&dl_cmd)?;
            dl_path.to_string()
        }
    };

    // 4. Wipe and recreate @ subvolume
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
    exec_cmd(&format!("unsquashfs -f -d {}/@ {}", btrfs_mnt, squashfs_path))?;

    // 6. Re-inject preserved users
    progress(0.85, "Re-injecting user credentials and settings...");
    log("Restoring user accounts into clean /etc...");
    let new_root = format!("{}/@", btrfs_mnt);
    if !preserved_passwd.is_empty() {
        if let Ok(mut f) = OpenOptions::new().append(true).open(format!("{}/etc/passwd", new_root)) {
            for l in &preserved_passwd {
                let _ = writeln!(f, "{}", l);
            }
        }
        if let Ok(mut f) = OpenOptions::new().append(true).open(format!("{}/etc/shadow", new_root)) {
            for l in &preserved_shadow {
                let _ = writeln!(f, "{}", l);
            }
        }
        if let Ok(mut f) = OpenOptions::new().append(true).open(format!("{}/etc/group", new_root)) {
            for l in &preserved_group {
                let _ = writeln!(f, "{}", l);
            }
        }
        if let Ok(mut f) = OpenOptions::new().append(true).open(format!("{}/etc/gshadow", new_root)) {
            for l in &preserved_gshadow {
                let _ = writeln!(f, "{}", l);
            }
        }
    }

    // 7. Regenerate clean /etc/fstab with correct UUID
    progress(0.92, "Configuring file systems and boot mounts...");
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

    let _ = fs::write(format!("{}/etc/fstab", new_root), fstab_content);

    // 8. Cleanup and sync
    progress(0.98, "Synchronizing disks and unmounting...");
    log("Syncing disks...");
    let _ = exec_cmd("sync");
    let _ = exec_cmd(&format!("umount -l {}", btrfs_mnt));

    progress(1.0, "Restoration complete!");
    log("System successfully restored.");
    Ok(())
}

fn main() {
    let app = Application::builder()
        .application_id("es.inled.pulsaros.recovery-assistant")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

