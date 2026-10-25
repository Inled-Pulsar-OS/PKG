//! GNOME system-commands provider. Commands run as argv (never a shell string).
use std::collections::HashMap;
use std::process::Command as Proc;

/// Checked when the probe's stdout contains `contains` (trimmed, case-insensitive).
#[derive(Clone, Debug, PartialEq)]
pub struct StateProbe {
    pub argv: Vec<String>,
    pub contains: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    pub id: String,
    pub title: String,
    pub breadcrumb: String,
    pub keywords: Vec<String>,
    pub icon: String,
    pub argv: Vec<String>,
    pub confirm: bool,
    pub state: Option<StateProbe>,
}

/// Runs argv, returns stdout on success. Injectable for tests.
pub type Runner = dyn Fn(&[String]) -> Option<String>;

pub fn system_runner(argv: &[String]) -> Option<String> {
    let (cmd, args) = argv.split_first()?;
    let out = Proc::new(cmd).args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn probe(cmd: &Command, runner: &Runner) -> Option<bool> {
    let p = cmd.state.as_ref()?;
    let out = runner(&p.argv)?;
    Some(out.trim().to_lowercase().contains(&p.contains.to_lowercase()))
}

/// `id -> checked` for every command that has a probe and whose probe ran.
pub fn probe_states(cmds: &[Command], runner: &Runner) -> HashMap<String, bool> {
    cmds.iter().filter_map(|c| probe(c, runner).map(|b| (c.id.clone(), b))).collect()
}

fn v(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

struct B(Vec<Command>);

impl B {
    fn add(&mut self, id: &str, title: &str, crumb: &str, kw: &str, icon: &str, argv: &[&str]) -> usize {
        self.0.push(Command {
            id: format!("gnome:{id}"),
            title: title.into(),
            breadcrumb: crumb.into(),
            keywords: kw.split_whitespace().map(String::from).collect(),
            icon: icon.into(),
            argv: v(argv),
            confirm: false,
            state: None,
        });
        self.0.len() - 1
    }
    fn panel(&mut self, id: &str, title: &str, panel: &str, kw: &str, icon: &str) {
        self.add(&format!("settings.{id}"), title, &format!("Settings > {title}"), kw, icon, &["gnome-control-center", panel]);
    }
    #[allow(clippy::too_many_arguments)]
    fn gset(&mut self, id: &str, title: &str, crumb: &str, kw: &str, icon: &str, schema: &str, key: &str, val: &str) {
        let i = self.add(id, title, crumb, kw, icon, &["gsettings", "set", schema, key, val]);
        self.0[i].state = Some(StateProbe { argv: v(&["gsettings", "get", schema, key]), contains: val.into() });
    }
    fn confirm(&mut self, i: usize) {
        self.0[i].confirm = true;
    }
}

const IFACE: &str = "org.gnome.desktop.interface";
const COLOR: &str = "org.gnome.settings-daemon.plugins.color";

/// All built-in commands (60+).
pub fn builtin() -> Vec<Command> {
    let mut b = B(Vec::new());

    // Settings panels
    for (id, title, panel, kw, icon) in [
        ("wifi", "Wi-Fi", "wifi", "wireless network internet", "network-wireless-symbolic"),
        ("network", "Network", "network", "ethernet vpn proxy internet", "network-workgroup-symbolic"),
        ("bluetooth", "Bluetooth", "bluetooth", "devices pair wireless", "bluetooth-symbolic"),
        ("display", "Displays", "display", "monitor resolution scale refresh night", "video-display-symbolic"),
        ("sound", "Sound", "sound", "audio volume output input speaker microphone", "audio-volume-high-symbolic"),
        ("power", "Power", "power", "battery energy saver suspend", "battery-symbolic"),
        ("appearance", "Appearance", "background", "theme dark light accent wallpaper background", "preferences-desktop-appearance-symbolic"),
        ("notifications", "Notifications", "notifications", "alerts banners do not disturb", "preferences-system-notifications-symbolic"),
        ("search", "Search", "search", "search locations indexing", "preferences-system-search-symbolic"),
        ("multitasking", "Multitasking", "multitasking", "hot corner workspaces windows", "view-grid-symbolic"),
        ("applications", "Applications", "applications", "default apps permissions", "view-app-grid-symbolic"),
        ("privacy", "Privacy", "privacy", "location camera microphone screen lock history", "preferences-system-privacy-symbolic"),
        ("online-accounts", "Online Accounts", "online-accounts", "google nextcloud cloud login", "goa-panel-symbolic"),
        ("sharing", "Sharing", "sharing", "remote desktop file sharing media hostname", "emblem-shared-symbolic"),
        ("mouse", "Mouse & Touchpad", "mouse", "pointer scroll tap click speed", "input-mouse-symbolic"),
        ("keyboard", "Keyboard", "keyboard", "shortcuts layout input sources typing", "input-keyboard-symbolic"),
        ("printers", "Printers", "printers", "print scanner", "printer-symbolic"),
        ("removable-media", "Removable Media", "removable-media", "usb autorun cd dvd", "media-removable-symbolic"),
        ("color", "Color", "color", "icc profile calibration", "preferences-color-symbolic"),
        ("region", "Region & Language", "region", "locale formats language", "preferences-desktop-locale-symbolic"),
        ("accessibility", "Accessibility", "universal-access", "a11y zoom screen reader text size contrast", "preferences-desktop-accessibility-symbolic"),
        ("users", "Users", "system", "accounts password login", "system-users-symbolic"),
        ("datetime", "Date & Time", "datetime", "clock timezone", "preferences-system-time-symbolic"),
        ("about", "About", "about", "system info version hostname device", "help-about-symbolic"),
        ("wacom", "Wacom Tablet", "wacom", "pen stylus graphics", "input-tablet-symbolic"),
    ] {
        b.panel(id, title, panel, kw, icon);
    }

    // Appearance toggles with live state
    b.gset("appearance.dark", "Dark Style", "Appearance > Dark Style", "dark theme night mode color scheme", "weather-clear-night-symbolic", IFACE, "color-scheme", "'prefer-dark'");
    b.gset("appearance.light", "Light Style", "Appearance > Light Style", "light theme day mode color scheme default", "weather-clear-symbolic", IFACE, "color-scheme", "'default'");
    b.gset("night-light.on", "Turn On Night Light", "Displays > Night Light > On", "night light warm blue filter eye", "night-light-symbolic", COLOR, "night-light-enabled", "true");
    b.gset("night-light.off", "Turn Off Night Light", "Displays > Night Light > Off", "night light warm blue filter eye", "night-light-symbolic", COLOR, "night-light-enabled", "false");
    b.gset("dnd.on", "Turn On Do Not Disturb", "Notifications > Do Not Disturb > On", "dnd focus silence mute banners", "notifications-disabled-symbolic", "org.gnome.desktop.notifications", "show-banners", "false");
    b.gset("dnd.off", "Turn Off Do Not Disturb", "Notifications > Do Not Disturb > Off", "dnd focus silence banners", "preferences-system-notifications-symbolic", "org.gnome.desktop.notifications", "show-banners", "true");
    b.gset("animations.on", "Enable Animations", "Accessibility > Animations > On", "motion effects", "preferences-desktop-accessibility-symbolic", IFACE, "enable-animations", "true");
    b.gset("animations.off", "Disable Animations", "Accessibility > Animations > Off", "motion effects reduce", "preferences-desktop-accessibility-symbolic", IFACE, "enable-animations", "false");
    b.gset("clock.seconds", "Show Clock Seconds", "Date & Time > Clock > Seconds", "clock time", "preferences-system-time-symbolic", IFACE, "clock-show-seconds", "true");
    b.gset("clock.weekday", "Show Clock Weekday", "Date & Time > Clock > Weekday", "clock day", "preferences-system-time-symbolic", IFACE, "clock-show-weekday", "true");
    b.gset("battery.percentage", "Show Battery Percentage", "Power > Battery Percentage", "battery status bar", "battery-symbolic", IFACE, "show-battery-percentage", "true");
    b.gset("hotcorner.on", "Enable Hot Corner", "Multitasking > Hot Corner > On", "overview activities corner", "view-grid-symbolic", IFACE, "enable-hot-corners", "true");
    b.gset("hotcorner.off", "Disable Hot Corner", "Multitasking > Hot Corner > Off", "overview activities corner", "view-grid-symbolic", IFACE, "enable-hot-corners", "false");
    b.gset("touchpad.tap.on", "Enable Tap to Click", "Mouse & Touchpad > Tap to Click > On", "touchpad tap", "input-touchpad-symbolic", "org.gnome.desktop.peripherals.touchpad", "tap-to-click", "true");
    b.gset("touchpad.tap.off", "Disable Tap to Click", "Mouse & Touchpad > Tap to Click > Off", "touchpad tap", "input-touchpad-symbolic", "org.gnome.desktop.peripherals.touchpad", "tap-to-click", "false");
    b.gset("touchpad.natural.on", "Enable Natural Scrolling", "Mouse & Touchpad > Natural Scrolling > On", "touchpad scroll reverse", "input-touchpad-symbolic", "org.gnome.desktop.peripherals.touchpad", "natural-scroll", "true");
    b.gset("touchpad.natural.off", "Disable Natural Scrolling", "Mouse & Touchpad > Natural Scrolling > Off", "touchpad scroll reverse", "input-touchpad-symbolic", "org.gnome.desktop.peripherals.touchpad", "natural-scroll", "false");
    b.gset("dock.autohide.on", "Enable Dock Auto-hide", "Dock > Auto-hide > On", "dash dock hide panel", "view-app-grid-symbolic", "org.gnome.shell.extensions.dash-to-dock", "autohide", "true");
    b.gset("dock.autohide.off", "Disable Dock Auto-hide", "Dock > Auto-hide > Off", "dash dock hide panel", "view-app-grid-symbolic", "org.gnome.shell.extensions.dash-to-dock", "autohide", "false");

    // Radios (nmcli / bluetoothctl)
    let i = b.add("wifi.on", "Turn On Wi-Fi", "Network > Wi-Fi > On", "wifi wireless radio enable", "network-wireless-symbolic", &["nmcli", "radio", "wifi", "on"]);
    b.0[i].state = Some(StateProbe { argv: v(&["nmcli", "radio", "wifi"]), contains: "enabled".into() });
    let i = b.add("wifi.off", "Turn Off Wi-Fi", "Network > Wi-Fi > Off", "wifi wireless radio disable airplane", "network-wireless-disabled-symbolic", &["nmcli", "radio", "wifi", "off"]);
    b.0[i].state = Some(StateProbe { argv: v(&["nmcli", "radio", "wifi"]), contains: "disabled".into() });
    let i = b.add("bluetooth.on", "Turn On Bluetooth", "Bluetooth > On", "bluetooth radio enable", "bluetooth-active-symbolic", &["bluetoothctl", "power", "on"]);
    b.0[i].state = Some(StateProbe { argv: v(&["bluetoothctl", "show"]), contains: "Powered: yes".into() });
    let i = b.add("bluetooth.off", "Turn Off Bluetooth", "Bluetooth > Off", "bluetooth radio disable airplane", "bluetooth-disabled-symbolic", &["bluetoothctl", "power", "off"]);
    b.0[i].state = Some(StateProbe { argv: v(&["bluetoothctl", "show"]), contains: "Powered: no".into() });
    b.add("network.airplane.on", "Turn On Airplane Mode", "Network > Airplane Mode > On", "flight radio wifi bluetooth off", "airplane-mode-symbolic", &["nmcli", "radio", "all", "off"]);
    b.add("network.airplane.off", "Turn Off Airplane Mode", "Network > Airplane Mode > Off", "flight radio wifi bluetooth on", "airplane-mode-symbolic", &["nmcli", "radio", "all", "on"]);

    // Audio
    b.add("sound.mute", "Mute Volume", "Sound > Output > Mute", "audio silence speaker", "audio-volume-muted-symbolic", &["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "1"]);
    b.add("sound.unmute", "Unmute Volume", "Sound > Output > Unmute", "audio speaker", "audio-volume-high-symbolic", &["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "0"]);
    b.add("sound.mic.mute", "Mute Microphone", "Sound > Input > Mute", "audio mic silence", "microphone-disabled-symbolic", &["wpctl", "set-mute", "@DEFAULT_AUDIO_SOURCE@", "1"]);
    b.add("sound.mic.unmute", "Unmute Microphone", "Sound > Input > Unmute", "audio mic", "audio-input-microphone-symbolic", &["wpctl", "set-mute", "@DEFAULT_AUDIO_SOURCE@", "0"]);
    b.add("sound.volume.up", "Volume Up", "Sound > Output > Volume Up", "audio louder", "audio-volume-high-symbolic", &["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "10%+"]);
    b.add("sound.volume.down", "Volume Down", "Sound > Output > Volume Down", "audio quieter", "audio-volume-low-symbolic", &["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "10%-"]);

    // Power / session
    b.add("session.lock", "Lock Screen", "System > Lock Screen", "lock secure away", "system-lock-screen-symbolic", &["loginctl", "lock-session"]);
    b.add("session.suspend", "Sleep", "System > Sleep", "suspend standby", "weather-clear-night-symbolic", &["systemctl", "suspend"]);
    let i = b.add("session.hibernate", "Hibernate", "System > Hibernate", "suspend disk", "system-suspend-hibernate-symbolic", &["systemctl", "hibernate"]);
    b.confirm(i);
    let i = b.add("session.restart", "Restart", "System > Restart", "reboot", "system-reboot-symbolic", &["systemctl", "reboot"]);
    b.confirm(i);
    let i = b.add("session.shutdown", "Shut Down", "System > Shut Down", "poweroff power off turn off", "system-shutdown-symbolic", &["systemctl", "poweroff"]);
    b.confirm(i);
    let i = b.add("session.logout", "Log Out", "System > Log Out", "sign out logoff session", "system-log-out-symbolic", &["gnome-session-quit", "--logout", "--no-prompt"]);
    b.confirm(i);
    b.add("session.switch-user", "Switch User", "System > Switch User", "login account", "system-users-symbolic", &["gdmflexiserver"]);

    // Files / trash / screenshots
    let i = b.add("trash.empty", "Empty Trash", "Files > Trash > Empty", "delete bin rubbish clean", "user-trash-full-symbolic", &["gio", "trash", "--empty"]);
    b.confirm(i);
    b.add("trash.open", "Open Trash", "Files > Trash", "bin deleted", "user-trash-symbolic", &["nautilus", "trash:///"]);
    b.add("files.home", "Open Home Folder", "Files > Home", "nautilus files folder", "user-home-symbolic", &["nautilus", "--new-window"]);
    b.add("files.downloads", "Open Downloads", "Files > Downloads", "nautilus folder", "folder-download-symbolic", &["xdg-open", "Downloads"]);
    b.add("screenshot.ui", "Take Screenshot", "Screenshot > Interactive", "capture screen snip", "applets-screenshooter-symbolic", &["gnome-screenshot", "--interactive"]);
    b.add("screenshot.screen", "Screenshot Entire Screen", "Screenshot > Screen", "capture screen full", "applets-screenshooter-symbolic", &["gnome-screenshot"]);
    b.add("screenshot.window", "Screenshot Window", "Screenshot > Window", "capture window", "applets-screenshooter-symbolic", &["gnome-screenshot", "--window"]);
    b.add("screenshot.area", "Screenshot Area", "Screenshot > Area", "capture selection region", "applets-screenshooter-symbolic", &["gnome-screenshot", "--area"]);

    // Apps / tools
    b.add("tool.system-monitor", "System Monitor", "Tools > System Monitor", "processes cpu memory task manager", "utilities-system-monitor-symbolic", &["gnome-system-monitor"]);
    b.add("tool.extensions", "Extensions", "Tools > Extensions", "gnome shell add-ons", "application-x-addon-symbolic", &["gnome-extensions-app"]);
    b.add("tool.tweaks", "Tweaks", "Tools > Tweaks", "gnome customize", "preferences-other-symbolic", &["gnome-tweaks"]);
    b.add("tool.terminal", "Open Terminal", "Tools > Terminal", "console shell command line", "utilities-terminal-symbolic", &["gnome-terminal"]);
    b.add("tool.software", "Software", "Tools > Software", "app store install packages", "system-software-install-symbolic", &["gnome-software"]);
    b.add("tool.disks", "Disks", "Tools > Disks", "partition format drive usb", "drive-harddisk-symbolic", &["gnome-disks"]);
    b.add("tool.logs", "Logs", "Tools > Logs", "journal system messages", "text-x-generic-symbolic", &["gnome-logs"]);
    b.add("shell.restart-nautilus", "Restart Files", "Tools > Restart Files", "nautilus reload quit", "system-file-manager-symbolic", &["nautilus", "-q"]);
    b.add("shell.overview", "Show Overview", "Shell > Overview", "activities windows workspaces", "view-grid-symbolic", &["busctl", "--user", "call", "org.gnome.Shell", "/org/gnome/Shell", "org.gnome.Shell", "Eval", "s", "Main.overview.toggle()"]);

    b.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn at_least_60_commands() {
        assert!(builtin().len() >= 60, "got {}", builtin().len());
    }

    #[test]
    fn ids_unique_and_prefixed() {
        let c = builtin();
        let ids: HashSet<_> = c.iter().map(|c| c.id.clone()).collect();
        assert_eq!(ids.len(), c.len());
        assert!(c.iter().all(|c| c.id.starts_with("gnome:")));
    }

    #[test]
    fn fields_complete_and_argv_not_shell() {
        for c in builtin() {
            assert!(!c.title.is_empty() && !c.breadcrumb.is_empty() && !c.icon.is_empty(), "{}", c.id);
            assert!(!c.keywords.is_empty(), "{}", c.id);
            assert!(!c.argv.is_empty(), "{}", c.id);
            assert!(!["sh", "bash", "zsh"].contains(&c.argv[0].as_str()), "{}", c.id);
        }
    }

    #[test]
    fn confirm_flags() {
        let c = builtin();
        let confirmed: HashSet<_> = c.iter().filter(|c| c.confirm).map(|c| c.title.as_str()).collect();
        for t in ["Shut Down", "Restart", "Log Out", "Empty Trash"] {
            assert!(confirmed.contains(t), "{t}");
        }
        assert!(!c.iter().find(|c| c.title == "Lock Screen").unwrap().confirm);
    }

    #[test]
    fn dark_style_state_probe() {
        let c = builtin();
        let dark = c.iter().find(|c| c.id == "gnome:appearance.dark").unwrap();
        assert_eq!(dark.breadcrumb, "Appearance > Dark Style");
        let on: &Runner = &|a: &[String]| {
            assert_eq!(a[0], "gsettings");
            Some("'prefer-dark'\n".to_string())
        };
        assert_eq!(probe(dark, on), Some(true));
        let off: &Runner = &|_: &[String]| Some("'default'\n".to_string());
        assert_eq!(probe(dark, off), Some(false));
        let fail: &Runner = &|_: &[String]| None;
        assert_eq!(probe(dark, fail), None);
    }

    #[test]
    fn no_probe_for_plain_command() {
        let c = builtin();
        let lock = c.iter().find(|c| c.id == "gnome:session.lock").unwrap();
        assert_eq!(probe(lock, &|_: &[String]| Some("x".into())), None);
    }

    #[test]
    fn probe_states_map() {
        let c = builtin();
        let r: &Runner = &|a: &[String]| match a.first().map(|s| s.as_str()) {
            Some("nmcli") => Some("enabled\n".into()),
            Some("gsettings") => Some("'prefer-dark'".into()),
            _ => None,
        };
        let m = probe_states(&c, r);
        assert!(m["gnome:wifi.on"]);
        assert!(!m["gnome:wifi.off"]);
        assert!(m["gnome:appearance.dark"]);
        assert!(!m["gnome:appearance.light"]);
        assert!(!m.contains_key("gnome:bluetooth.on"));
    }

    #[test]
    fn system_runner_works_and_fails_cleanly() {
        assert_eq!(system_runner(&v(&["echo", "hi"])).unwrap().trim(), "hi");
        assert!(system_runner(&v(&["definitely-not-a-binary-xyz"])).is_none());
        assert!(system_runner(&[]).is_none());
    }

    #[test]
    fn dark_is_findable_by_ranker() {
        use crate::core::ranker::{rank, Item};
        let items: Vec<Item> = builtin()
            .iter()
            .map(|c| Item { id: c.id.clone(), title: c.title.clone(), keywords: c.keywords.clone(), alias: None })
            .collect();
        let r = rank("dark", &items);
        assert_eq!(r[0].0.id, "gnome:appearance.dark");
    }
}
