
// Favorite apps helpers.
// English: Reading/writing /org/gnome/shell/favorite-apps through the dconf
//          D-Bus "ca.desrt.dconf.Writer" interface (Read/Write methods) is used
//          by Pulsar OS's own distro builds, but newer dconf releases (e.g.
//          Arch's) dropped those two methods, so pin/unpin silently did nothing.
//          Shelling out to the `gsettings` CLI works on every GNOME/dconf.
// Español: Leer/escribir /org/gnome/shell/favorite-apps vía el D-Bus
//          "ca.desrt.dconf.Writer" (métodos Read/Write) funcionaba en los ISO,
//          pero las versiones nuevas de dconf (p.ej. la de Arch) eliminaron esos
//          métodos y el pin/unpin no hacía nada. Usar `gsettings` funciona en
//          cualquier GNOME/dconf.
pub fn get_favorites() -> Vec<String> {
    let out = match std::process::Command::new("gsettings")
        .args(["get", "org.gnome.shell", "favorite-apps"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    if !out.status.success() {
        return Vec::new();
    }
    let Ok(text) = String::from_utf8(out.stdout) else {
        return Vec::new();
    };

    let clean = text.trim();
    let clean = clean.strip_prefix("@as ").unwrap_or(clean);
    let clean = clean.strip_prefix('[').unwrap_or(clean);
    let clean = clean.strip_suffix(']').unwrap_or(clean);

    let mut list = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for c in clean.chars() {
        if c == '\'' || c == '"' {
            in_quotes = !in_quotes;
            if !in_quotes && !current.is_empty() {
                list.push(std::mem::take(&mut current));
            }
        } else if in_quotes {
            current.push(c);
        }
    }
    list
}

pub fn set_favorites(favs: &[String]) {
    let formatted = format!("[{}]", favs.iter().map(|s| format!("'{}'", s)).collect::<Vec<_>>().join(", "));

    let _ = std::process::Command::new("gsettings")
        .args(["set", "org.gnome.shell", "favorite-apps", &formatted])
        .output();
}
