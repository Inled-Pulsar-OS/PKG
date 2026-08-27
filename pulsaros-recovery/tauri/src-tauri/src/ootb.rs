use std::fs;
use std::path::Path;
use std::process::Command;

fn exec_cmd(cmd: &str) -> Result<String, String> {
    let out = Command::new("sudo")
        .args(["-n", "sh", "-c", cmd])
        .output()
        .map_err(|e| format!("Failed to execute '{}': {}", cmd, e))?;

    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    if !out.status.success() {
        return Err(format!(
            "Command '{}' failed ({}): {}",
            cmd,
            out.status.code().unwrap_or(-1),
            stderr
        ));
    }

    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn write_temp_and_move(content: &str, dest: &str) -> Result<(), String> {
    let tmp = format!("{}.tmp", dest);
    fs::write(&tmp, content).map_err(|e| format!("Failed to write {}: {}", tmp, e))?;
    fs::rename(&tmp, dest).map_err(|e| format!("Failed to rename {} to {}: {}", tmp, dest, e))?;
    Ok(())
}

fn get_existing_groups() -> Vec<String> {
    fs::read_to_string("/etc/group")
        .map(|content| {
            content
                .lines()
                .filter_map(|line| line.split(':').next())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}

pub struct OotbConfig {
    pub fullname: String,
    pub username: String,
    pub password: String,
    pub language: String,
    pub keymap: String,
    pub timezone: String,
    pub avatar_path: Option<String>,
}

pub fn run_setup(
    config: &OotbConfig,
    log_fn: &dyn Fn(String),
    progress_fn: &dyn Fn(f64, String),
) -> Result<(), String> {
    let locale_full = format!("{}.UTF-8", config.language);

    // ── Locale ──
    progress_fn(0.05, "Configuring locale...".into());
    log_fn(format!("Setting locale to {}", locale_full));

    if let Ok(content) = fs::read_to_string("/etc/locale.gen") {
        let mut found = false;
        let new_lines: Vec<String> = content
            .lines()
            .map(|line| {
                let stripped = line.trim();
                let cand = stripped.strip_prefix('#').unwrap_or(stripped).trim();
                if !cand.is_empty()
                    && cand.split_whitespace().next() == Some(locale_full.as_str())
                {
                    found = true;
                    format!("{} UTF-8", locale_full)
                } else {
                    line.to_string()
                }
            })
            .collect();

        let mut lines = new_lines;
        if !found {
            lines.push(format!("{} UTF-8", locale_full));
        }
        let _ = write_temp_and_move(&(lines.join("\n") + "\n"), "/etc/locale.gen");
    }

    let _ = exec_cmd("locale-gen 2>/dev/null || true");

    let res = exec_cmd(&format!("localectl set-locale LANG={}", locale_full));
    if res.is_err() {
        if Path::new("/etc/debian_version").exists() {
            let _ = write_temp_and_move(
                &format!("LANG=\"{}\"\n", locale_full),
                "/etc/default/locale",
            );
            let _ = exec_cmd("chown root:root /etc/default/locale 2>/dev/null || true");
            let _ = exec_cmd("chmod 644 /etc/default/locale 2>/dev/null || true");
        } else {
            let _ = write_temp_and_move(&format!("LANG={}\n", locale_full), "/etc/locale.conf");
            let _ = exec_cmd("chown root:root /etc/locale.conf 2>/dev/null || true");
            let _ = exec_cmd("chmod 644 /etc/locale.conf 2>/dev/null || true");
        }
    }

    // ── Keyboard ──
    progress_fn(0.10, "Configuring keyboard...".into());
    log_fn(format!("Setting keyboard layout to {}", config.keymap));

    if Path::new("/etc/debian_version").exists() {
        let _ = exec_cmd(
            "apt-get install -y console-setup keyboard-configuration kbd 2>/dev/null || true",
        );
    }

    let res = exec_cmd(&format!("localectl set-keymap {}", config.keymap));
    if res.is_err() {
        log_fn("Fallback: writing /etc/default/keyboard directly".into());
        let kb_content = format!(
            "XKBMODEL=\"pc105\"\nXKBLAYOUT=\"{}\"\nXKBVARIANT=\"\"\nXKBOPTIONS=\"\"\nBACKSPACE=\"guess\"\n",
            config.keymap
        );
        let _ = write_temp_and_move(&kb_content, "/etc/default/keyboard");
    }

    // ── Timezone ──
    progress_fn(0.15, "Configuring timezone...".into());
    log_fn(format!("Setting timezone to {}", config.timezone));

    let res = exec_cmd(&format!("timedatectl set-timezone {}", config.timezone));
    if res.is_err() {
        log_fn("Fallback: manual timezone symlink".into());
        let _ = exec_cmd(&format!(
            "ln -sf /usr/share/zoneinfo/{} /etc/localtime 2>/dev/null || true",
            config.timezone
        ));
        let _ = write_temp_and_move(&format!("{}\n", config.timezone), "/etc/timezone");
        let _ = exec_cmd("chown root:root /etc/timezone 2>/dev/null || true");
        let _ = exec_cmd("chmod 644 /etc/timezone 2>/dev/null || true");
    }

    // ── Lock live user ──
    progress_fn(0.20, "Locking live user account...".into());
    log_fn("Locking live user account...".into());
    let _ = exec_cmd("usermod -L -s /usr/sbin/nologin live 2>/dev/null || true");

    // ── Create real user ──
    progress_fn(0.25, format!("Creating user '{}'...", config.username));
    log_fn(format!("Creating user '{}' via useradd...", config.username));

    let user_home = format!("/home/{}", config.username);
    let existing_groups = get_existing_groups();
    let desired_groups = ["sudo", "wheel", "audio", "video", "plugdev", "docker"];
    let extra_groups: Vec<&str> = desired_groups
        .iter()
        .filter(|g| existing_groups.contains(&g.to_string()))
        .copied()
        .collect();
    let groups_str = extra_groups.join(",");

    exec_cmd(&format!(
        "useradd -m -d {} -s /bin/bash -G {} -c '{}' {}",
        user_home, groups_str, config.fullname, config.username
    ))?;

    let passwd_check = exec_cmd(&format!("grep ^{}: /etc/passwd", config.username))?;
    if !passwd_check.contains(&format!("{}:", config.username)) {
        return Err(format!(
            "User '{}' not found in /etc/passwd after useradd",
            config.username
        ));
    }
    if !Path::new(&user_home).is_dir() {
        return Err(format!(
            "Home directory {} does not exist after useradd",
            user_home
        ));
    }

    // ── Grant sudo ──
    progress_fn(0.30, "Granting sudo access...".into());
    let sudoers_file = format!("/etc/sudoers.d/pulsaros-user-{}", config.username);
    let _ = write_temp_and_move(
        &format!("{} ALL=(ALL:ALL) ALL\n", config.username),
        &sudoers_file,
    );
    let _ = exec_cmd(&format!("chown root:root {}", sudoers_file));
    let _ = exec_cmd(&format!("chmod 0440 {}", sudoers_file));
    log_fn(format!(
        "Granted sudo to '{}' via {}",
        config.username, sudoers_file
    ));

    // ── Update /etc/hosts ──
    progress_fn(0.35, "Updating /etc/hosts...".into());
    if let Ok(content) = fs::read_to_string("/etc/hosts") {
        if !content.contains("pulsaros") {
            let new_lines: Vec<String> = content
                .lines()
                .map(|line| {
                    if line.starts_with("127.0.0.1") {
                        format!("{} pulsaros", line)
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            let _ = write_temp_and_move(&(new_lines.join("\n") + "\n"), "/etc/hosts");
            log_fn("Updated /etc/hosts with 'pulsaros' alias".into());
        }
    }

    // ── Set passwords ──
    progress_fn(0.40, "Setting passwords...".into());

    let user_pw_cmd = format!(
        "echo '{}:{}' | chpasswd",
        config.username, config.password
    );
    let out = Command::new("sudo")
        .args(["-n", "sh", "-c", &user_pw_cmd])
        .output()
        .map_err(|e| format!("Failed to set user password: {}", e))?;
    if !out.status.success() {
        return Err(format!(
            "Failed to set user password: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    log_fn("User password set successfully".into());

    let root_pw_cmd = format!("echo 'root:{}' | chpasswd", config.password);
    let out = Command::new("sudo")
        .args(["-n", "sh", "-c", &root_pw_cmd])
        .output()
        .map_err(|e| format!("Failed to set root password: {}", e))?;
    if !out.status.success() {
        return Err(format!(
            "Failed to set root password: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    log_fn("Root password set successfully".into());

    // ── Avatar ──
    progress_fn(0.45, "Configuring avatar...".into());

    let mut as_icon_dest = String::new();
    if let Some(path) = &config.avatar_path {
        if !path.is_empty() && Path::new(path).exists() {
            if Path::new(&user_home).is_dir() {
                let face_dest = format!("{}/.face", user_home);
                let _ = exec_cmd(&format!("cp -f {} {}", path, face_dest));
                let _ = exec_cmd(&format!(
                    "chown {}:{} {}",
                    config.username, config.username, face_dest
                ));
            }

            let as_icons_dir = "/var/lib/AccountsService/icons";
            let _ = exec_cmd(&format!("mkdir -p {}", as_icons_dir));
            as_icon_dest = format!("{}/{}", as_icons_dir, config.username);
            let _ = exec_cmd(&format!("cp -f {} {}", path, as_icon_dest));
            let _ = exec_cmd(&format!("chown root:root {}", as_icon_dest));
        }
    }

    // ── AccountsService ──
    progress_fn(0.50, "Configuring AccountsService...".into());

    let as_user_file = format!("/var/lib/AccountsService/users/{}", config.username);
    let mut as_content = format!(
        "[User]\nLanguage={}\nSession=gnome\nXSession=gnome\nSystemAccount=false\n",
        config.language
    );
    if !as_icon_dest.is_empty() {
        as_content.push_str(&format!("Icon={}\n", as_icon_dest));
    }

    let _ = exec_cmd("mkdir -p /var/lib/AccountsService/users");
    let _ = write_temp_and_move(&as_content, &as_user_file);
    let _ = exec_cmd(&format!("chown root:root {}", as_user_file));
    let _ = exec_cmd(&format!("chmod 600 {}", as_user_file));
    log_fn(format!(
        "AccountsService config written to {}",
        as_user_file
    ));

    let _ = exec_cmd("systemctl restart accounts-daemon.service 2>/dev/null || true");

    // ── GNOME keymap ──
    progress_fn(0.55, "Configuring GNOME keymap...".into());
    let _ = exec_cmd(&format!(
        "sudo -u {} dbus-run-session gsettings set org.gnome.desktop.input-sources sources \"[('xkb', '{}')]\" 2>/dev/null || true",
        config.username, config.keymap
    ));

    // ── macOS keybindings ──
    progress_fn(0.60, "Configuring macOS keybindings...".into());
    log_fn("Setting macOS keybindings...".into());

    let mac_settings: Vec<(&str, &str, &str)> = vec![
        ("org.gnome.desktop.input-sources", "xkb-options", "['ctrl:swap_lwin_lctl', 'ctrl:swap_rwin_rctl']"),
        ("org.gnome.mutter", "overlay-key", "'Super_R'"),
        ("org.gnome.desktop.wm.keybindings", "minimize", "['<Primary>m']"),
        ("org.gnome.desktop.wm.keybindings", "show-desktop", "['<Primary>d']"),
        ("org.gnome.desktop.wm.keybindings", "switch-applications", "['<Primary>Tab']"),
        ("org.gnome.desktop.wm.keybindings", "switch-applications-backward", "['<Primary><Shift>Tab']"),
        ("org.gnome.desktop.wm.keybindings", "switch-group", "['<Primary>grave']"),
        ("org.gnome.desktop.wm.keybindings", "switch-group-backward", "['<Primary><Shift>grave']"),
        ("org.gnome.mutter.keybindings", "toggle-tiled-left", "[]"),
        ("org.gnome.mutter.keybindings", "toggle-tiled-right", "[]"),
        ("org.gnome.desktop.wm.keybindings", "switch-to-workspace-left", "['<Super>Left']"),
        ("org.gnome.desktop.wm.keybindings", "switch-to-workspace-right", "['<Super>Right']"),
        ("org.gnome.shell.keybindings", "toggle-overview", "['LaunchA']"),
        ("org.gnome.shell.keybindings", "toggle-application-view", "['LaunchB']"),
        ("org.gnome.shell.keybindings", "toggle-message-tray", "[]"),
        ("org.gnome.shell.keybindings", "screenshot", "['<Primary><Shift>numbersign']"),
        ("org.gnome.shell.keybindings", "show-screenshot-ui", "['Print', '<Shift><Control>dollar', '<Shift><Super>4', '<Shift><Super>5']"),
        ("org.gnome.shell.keybindings", "screenshot-window", "['<Shift><Control>percent']"),
        ("org.gnome.settings-daemon.plugins.media-keys", "screensaver", "['<Super>l', '<Control>l']"),
        ("org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings", "['/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/', '/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom1/']"),
    ];

    for (schema, key, value) in &mac_settings {
        let _ = exec_cmd(&format!(
            "sudo -u {} dbus-run-session gsettings set {} {} \"{}\" 2>/dev/null || true",
            config.username, schema, key, value
        ));
    }

    let spotlight_ctrl = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/";
    for (key, value) in &[
        ("name", "'Spotlight'"),
        ("command", "'pulsaros-spotlight'"),
        ("binding", "'<Ctrl>space'"),
    ] {
        let _ = exec_cmd(&format!(
            "sudo -u {} dbus-run-session gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{} {} \"{}\" 2>/dev/null || true",
            config.username, spotlight_ctrl, key, value
        ));
    }

    let spotlight_super = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom1/";
    for (key, value) in &[
        ("name", "'Spotlight'"),
        ("command", "'pulsaros-spotlight'"),
        ("binding", "'<Super>space'"),
    ] {
        let _ = exec_cmd(&format!(
            "sudo -u {} dbus-run-session gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{} {} \"{}\" 2>/dev/null || true",
            config.username, spotlight_super, key, value
        ));
    }

    progress_fn(1.0, "Setup complete!".into());
    log_fn("OOTB setup completed successfully.".into());

    Ok(())
}

pub fn run_final_cleanup(username: &str, log_fn: &dyn Fn(String)) -> Result<(), String> {
    log_fn("Cleaning up temporary sudoers for live user...".into());
    let _ = fs::remove_file("/etc/sudoers.d/pulsar-ootb-live");

    let _ = exec_cmd("rm -f /etc/sddm.conf /etc/sddm.conf.d/live /etc/lightdm/lightdm.conf.d/* 2>/dev/null || true");

    let _ = exec_cmd("mkdir -p /etc/sddm.conf.d");
    let autologin_conf = format!(
        "[Autologin]\nUser={}\nSession=gnome\nRelogin=false\n",
        username
    );
    let _ = fs::write("/etc/sddm.conf.d/autologin.conf", &autologin_conf);
    let _ = exec_cmd("chmod 644 /etc/sddm.conf.d/autologin.conf");
    log_fn(format!("SDDM autologin configured for '{}'", username));

    if Path::new("/etc/pulsar-need-setup").exists() {
        let _ = fs::remove_file("/etc/pulsar-need-setup");
        log_fn("Removed /etc/pulsar-need-setup".into());
    }

    let _ = fs::write("/etc/pulsar-need-cleanup", username);
    log_fn("Created /etc/pulsar-need-cleanup".into());

    let _ = exec_cmd("systemctl disable pulsar-ootb.service 2>/dev/null || true");
    log_fn("Disabled pulsar-ootb.service".into());

    log_fn("Restarting SDDM to pick up new user...".into());
    let _ = exec_cmd("systemctl restart sddm 2>/dev/null || true");

    log_fn("Setup complete. Rebooting in 3 seconds...".into());

    let _ = Command::new("sh")
        .args(["-c", "sleep 3 && systemctl reboot"])
        .spawn();

    Ok(())
}
