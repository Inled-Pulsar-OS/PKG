# Running on GNOME

Pulsar Store is a GTK4 + libadwaita app, so it follows GNOME's light/dark, accent color, animation, and high-contrast settings. It needs GTK 4.14 and libadwaita 1.5 or newer (Ubuntu 24.04, GNOME 46, Debian 13).

## How it works with GNOME Shell

- **Extensions:** after copying an extension into `~/.local/share/gnome-shell/extensions`, the store asks the Shell to enable it (`org.gnome.Shell.Extensions`) and then asks the Shell whether it did. GNOME Shell on Wayland only sees extensions that existed when it started, so a new one usually needs a log out and back in. The toast says which case applies: "installed and enabled", or "log out and back in to turn it on".
- **Version check:** if an extension lists `shell-version` and the running Shell is not in it, the install is refused with a clear message instead of installing something that cannot load.
- **Updates:** `pulsar-store-updated` sends a notification with the `desktop-entry` hint and a default action. GNOME groups it under Pulsar Store, and clicking it (or its button) opens the Updates page.

## Check it on a GNOME session

These steps have been written from the code and tested on another desktop only. Please run them once on GNOME:

1. `pulsar-store` opens; the window buttons are where GNOME puts them; dark mode follows the system setting.
2. `pulsar-store-updated --test-notification` shows a notification with an "Open Updates" button. Click it: it prints `action: open` or `action: default`.
3. Install a GNOME extension from the store. Check the toast, and whether the extension turns on after logging in again.
4. Install a Flatpak or native package: the GNOME authentication dialog should appear for native packages.
5. Narrow the window: the sidebar should collapse.
