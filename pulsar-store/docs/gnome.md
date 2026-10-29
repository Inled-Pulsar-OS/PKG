# Running on GNOME

Pulsar Store is a GTK4 + libadwaita app, so it follows GNOME's light/dark, accent color, animation, and high-contrast settings. It needs GTK 4.14 and libadwaita 1.5 or newer (Ubuntu 24.04, GNOME 46, Debian 13).

## How it works with GNOME Shell

- **Extensions:** after copying an extension into `~/.local/share/gnome-shell/extensions`, the store asks the Shell to enable it (`org.gnome.Shell.Extensions`) and then asks the Shell whether it did. GNOME Shell on Wayland only sees extensions that existed when it started, so a new one usually needs a log out and back in. The toast says which case applies: "installed and enabled", or "log out and back in to turn it on".
- **Version check:** if an extension lists `shell-version` and the running Shell is not in it, the install is refused with a clear message instead of installing something that cannot load.
- **Updates:** `pulsar-store-updated` sends a standard `org.freedesktop.Notifications` notification, which GNOME Shell implements. It carries the `desktop-entry` hint (`es.inled.PulsarStore`), so GNOME groups it under Pulsar Store, shows the app icon, and lists it in Settings > Notifications. It has a default action (click the notification) and an "Open Updates" button. A later check replaces the same notification instead of stacking new ones.
- **Opening the page:** both actions run `xdg-open pulsar://page/updates`. The desktop's link handler then starts or focuses Pulsar Store, so the app is not a child of the background service (systemd would stop it with the service).

## Check it on a GNOME session

The notification code was written against the freedesktop notification spec and GNOME Shell's behavior; it has only been run against another desktop's notification daemon, not GNOME Shell. The link-opening path (`pulsar://page/updates`) was run for real. Please run them once on GNOME:

1. `pulsar-store` opens; the window buttons are where GNOME puts them; dark mode follows the system setting.
2. `pulsar-store-updated --test-notification` shows a notification with an "Open Updates" button. Click it: it prints `action: open` or `action: default`.
3. Install a GNOME extension from the store. Check the toast, and whether the extension turns on after logging in again.
4. Install a Flatpak or native package: the GNOME authentication dialog should appear for native packages.
5. Narrow the window: the sidebar should collapse.
