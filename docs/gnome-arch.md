# GNOME Settings Architecture
agy --conversation=52ffb6e0-724d-4394-9c7b-cf418fe0ab81

## The Stack

```
+-----------------------------------------------+
|  gnome-control-center (UI)                    |
|  Each panel = plugin, talks backends          |
+----------------------+------------------------+
                       | D-Bus: org.gnome.Mutter.DisplayConfig
                       |        (monitors)
                       | GSettings API (everything else)
                       |
+----------------------+------------------------+
|  Two backends:                                |
|                                               |
|  1. MUTTER -- owns display/monitor config     |
|     MetaMonitorManager -> monitors.xml        |
|     Hotplug: kernel DRM -> udev -> mutter     |
|                                               |
|  2. DCONF -- owns all other settings          |
|     ~/.config/dconf/user (binary DB)          |
|     Layered system DBs from /etc/dconf/db/    |
|     Schema-enforced via .gschema.xml          |
+-----------------------------------------------+
```

## Key GNOME Components

| Component | Role |
|-----------|------|
| **mutter** | Wayland compositor / X11 WM. Owns display stack. Exposes `org.gnome.Mutter.DisplayConfig` D-Bus interface. Stores layout in `~/.config/monitors.xml`. |
| **gnome-shell** | Desktop shell. Hosts mutter, provides panel, overview, activities, window switching. |
| **gnome-settings-daemon (gsd)** | Session daemon translating settings changes into actions. Plugins for power, media-keys, xsettings, color. Watches GSettings and acts. |
| **gnome-control-center** | Settings UI. Each panel is a plugin. Display panel talks to mutter over D-Bus. |
| **GDM** | Login manager. Runs its own mutter/gnome-shell instance. Separate dconf profile and monitor config. |
| **dconf-service** | D-Bus activated daemon writing to binary dconf database. Stateless, crashes freely. |

## Settings Storage: dconf + GSettings

| Layer | What it does |
|-------|-------------|
| **GSettings** (GLib API) | Type-safe schema-enforced wrapper. Every key has a type, default, path defined in `.gschema.xml` files compiled by `glib-compile-schemas`. This is what apps call. |
| **dconf** (backend) | Raw key/value binary store. Optimized for reads (mmap'd hashtable, zero syscalls). Writes through `dconf-service` over D-Bus, fsync latency hidden by fast-path cache. |

### Schema Path Convention

Dotted IDs map to dconf paths. Schema `org.gnome.desktop.interface` lives at dconf path `/org/gnome/desktop/interface/`.

### Key Files

- `~/.config/dconf/user` -- per-user binary database (the actual storage)
- `/etc/dconf/db/<name>/` -- system-wide override databases (compiled from keyfiles)
- `/etc/dconf/profile/user` -- profile listing the database layers consulted in order
- `~/.config/monitors.xml` -- display configuration (separate from dconf)

### Write Flow

```
gsettings_set("org.gnome.desktop.interface", "gtk-theme", "'Adwaita'")
  -> GSettings API validates type against schema
    -> dconf backend calls dconf_engine_change_fast()
      -> local fast-path records change immediately (readable instantly)
        -> D-Bus call to dconf-service
          -> dconf-service renames new DB file over old one
            -> fsync() (up to 100ms latency)
              -> D-Bus signal notifies watchers of change
```

### Layering

dconf consults databases in profile order. User DB always wins. System databases (for admin policy/defaults) come from `/etc/dconf/db/`. Lock-down supported -- system admins can lock keys so users cannot override them.

### Key Schema Files

- `gsettings-desktop-schemas`: `org.gnome.desktop.interface`, `org.gnome.desktop.background`, `org.gnome.desktop.screensaver`, etc.
- `gnome-settings-daemon`: `org.gnome.settings-daemon.plugins.*`
- `mutter`: `org.gnome.mutter.*`

## Display/Monitor Management

**Key insight: mutter owns display configuration.** Not gnome-settings-daemon, not XRandR directly.

### Core Classes in Mutter

```
MetaMonitorManager (abstract)
  |
  +-- MetaMonitorManagerKms    (Wayland/native backend, KMS/DRM)
  +-- MetaMonitorManagerXrandr (X11 backend, XRandR)
  +-- MetaMonitorManagerDummy  (testing)
  |
  Provides: org.gnome.Mutter.DisplayConfig D-Bus service

MetaMonitorConfigManager
  |-- Manages MetaMonitorsConfig (in-memory representation)
  |-- Uses MetaMonitorConfigStore (reads/writes monitors.xml)

MetaMonitorConfigStore
  |-- Reads: ~/.config/monitors.xml  (user)
  |-- Reads: /etc/xdg/monitors.xml   (system-wide default)
  |-- Writes: ~/.config/monitors.xml  (when persistent config applied)
  |-- Format: XML with <monitors version="2"> root

MetaMonitor, MetaLogicalMonitor, MetaCrtc, MetaOutput, MetaGpu
  |-- Hardware abstraction objects
```

### monitors.xml Format

```xml
<monitors version="2">
  <configuration>
    <logicalmonitor>
      <x>0</x><y>0</y>
      <monitor>
        <connector>HDMI-1</connector>
        <vendor>ABC</vendor>
        <product>Display</product>
        <serial>123</serial>
        <mode>
          <width>1920</width>
          <height>1080</height>
          <rate>60.0</rate>
        </mode>
      </monitor>
      <scale>1</scale>
      <primary>yes</primary>
    </logicalmonitor>
  </configuration>
</monitors>
```

### Monitor Hotplug Detection (Wayland)

```
Kernel DRM subsystem
  -> udev uevent ("drm" subsystem, HOTPLUG property)
    -> GUdevClient signal "uevent"
      -> MetaUdev emits "hotplug" signal
        -> MetaMonitorManagerKms::on_udev_hotplug()
          -> handle_hotplug_event()
            -> meta_monitor_manager_read_current_state()  // re-read hardware
            -> meta_monitor_manager_on_hotplug()
              -> meta_monitor_manager_notify_monitors_changed()
                -> emits "monitors-changed" signal
                  -> D-Bus MonitorsChanged signal emitted
                    -> gnome-control-center reacts
```

**Hotplug flow in detail:**

1. Monitor plugged in. Kernel DRM driver emits uevent.
2. `gudev` picks it up in mutter's `MetaMonitorManagerKms`. Device must have `HOTPLUG` property.
3. Mutter calls `meta_monitor_manager_read_current_state()` -- re-enumerates all GPU connectors, modes, CRTCs.
4. `meta_monitor_manager_on_hotplug()` triggers `meta_monitor_manager_ensure_configured()`:
   - Tries stored config from `monitors.xml` (matching by connector/vendor/product/serial)
   - Falls back to current config if still works
   - Falls back to "suggested" config
   - Falls back to "linear" layout (side by side)
   - Falls back to single-monitor fallback
5. Config applied, `MonitorsChanged` D-Bus signal fires.
6. gsd-xsettings plugin reads new state, updates X11 properties (DPI, scaling).
7. gnome-control-center display panel receives signal, rebuilds UI.

**GPU hotplug** (e.g., eGPU): udev "add" events for DRM devices trigger `handle_gpu_hotplug()`, creating a new `MetaGpuKms` object.

## D-Bus Interfaces

### org.gnome.Mutter.DisplayConfig

**Bus name:** `org.gnome.Mutter.DisplayConfig`
**Object path:** `/org/gnome/Mutter/DisplayConfig`

This is THE central interface.

| Method | Purpose |
|--------|---------|
| `GetResources()` | Legacy: returns CRTCs, outputs, modes with serial. Older clients. |
| `GetCurrentState()` | Modern: returns serial, monitors list (connector/vendor/product/serial/modes with scales), logical monitors (position/scale/transform/primary/monitor-list), properties. |
| `ApplyMonitorsConfig(serial, method, logical_monitors, properties)` | Apply new layout. method: 0=verify, 1=temporary, 2=persistent. |
| `ApplyConfiguration(serial, persistent, crtcs, outputs)` | Legacy apply (older API). |
| `SetOutputCTM(serial, output, ctm)` | Set color transform matrix on output. |

**Properties:**

- `PowerSaveMode` (read/write)
- `PanelOrientationManaged` (read-only)
- `ApplyMonitorsConfigAllowed` (read-only, controlled by policy)

**Signal:**

- `MonitorsChanged` -- emitted when hardware config changes; clients should re-read via `GetCurrentState()`.

### org.gnome.SettingsDaemon.XRANDR (deprecated)

Formerly used by gnome-control-center. Methods: `ApplyConfiguration`, `VideoModeSwitch`, `Rotate`. Now deprecated -- mutter's DisplayConfig is the only supported path.

## Settings Flow: UI to Storage

### Display Setting Change (resolution, position, scale)

```
gnome-control-center (Display panel)
  -> CcDisplayConfigManagerDBus
    -> CcDisplayConfigDBus::config_apply()
      -> g_dbus_connection_call_sync("org.gnome.Mutter.DisplayConfig",
                                     "ApplyMonitorsConfig", ...)
        -> mutter's MetaMonitorManager handles the call
          -> meta_monitor_manager_apply_monitors_config()
            -> validates, applies to hardware
            -> if method == PERSISTENT:
                 meta_monitor_config_manager_set_current()
                   -> MetaMonitorConfigStore saves to ~/.config/monitors.xml
            -> emits MonitorsChanged signal
```

### General Desktop Setting (theme, font, etc.)

```
gnome-control-center (Appearance panel)
  -> g_settings_set("org.gnome.desktop.interface", "gtk-theme", "'Yaru'")
    -> GSettings API validates type
      -> dconf backend writes to ~/.config/dconf/user
        -> dconf-service commits via D-Bus
          -> gsd-xsettings plugin watches the key
            -> applies theme to running GTK apps
```

The **gsd-xsettings plugin** is a critical bridge: watches GSettings changes and applies them as X properties or GTK settings to the running session. On Wayland, many applied differently (through gnome-shell's own CSS theming).

## GDM's Relationship

GDM runs its own GNOME session (mutter + gnome-shell) as the `gdm` user.

- **Separate dconf profile:** `/etc/dconf/profile/gdm` with `system-db:gdm` and `file-db:/usr/share/gdm/greeter-dconf-defaults`
- **Separate monitor config:** `/etc/xdg/monitors.xml` (system-wide) or `~gdm/.config/monitors.xml` (per greeter user)
- **D-Bus:** GDM's session bus is separate from user session bus. Its mutter instance exposes its own `org.gnome.Mutter.DisplayConfig` on the system session bus.

The greeter's display config is independent of user session. To apply user display settings to GDM, copy `~/.config/monitors.xml` to the gdm user or `/etc/xdg/monitors.xml`.

## PulsarOS Modifications

### pulsaros-gnome

Default settings layer for Pulsar OS:

- `90_pulsaros.gschema.override` -- GSettings defaults (theme, extensions, keybindings, dock, blur, power)
- `etc/dconf/db/local.d/00-pulsaros-theme` -- dconf system DB (same + XKB swaps, Liquid Glass params, mutter overlay-key)
- `etc/dconf/db/local.d/locks/00-pulsaros-theme` -- Locks `button-layout` and `extension-version-validation`

### pulsaros-control-center

Replaces gnome-control-center with:

- macOS-style user banner at top (Apple ID & Accounts)
- Reordered sidebar (network first, system last)
- Custom squircle panel icons
- Heavily modified Background panel -> "Appearance" panel:
  - Light/Dark style toggle
  - macOS Keyboard Remap switch (XKB `ctrl:swap_lwin_lctl`)
  - macOS Fullscreen Spaces switch
  - Liquid Glass switch
  - Accent color picker
  - Live Wallpaper launcher
  - Spotlight vs Overview toggle
  - Cursor theme picker
- **Display panel is upstream mutter's** -- no PulsarOS override

### pulsaros-effects-settings

Python GTK3 app for toggling desktop visual effects:

- Two modes: "Blur my Shell (Recommended)" and "Liquid Glass (Glassmorphism)"
- Directly writes `org.gnome.shell enabled-extensions` strv
- Sets dock opacity, blur parameters via GSettings
- Chains multiple extension schema directories into custom `SettingsSchemaSource`

### What's NOT Customized

Monitor/display config uses upstream mutter. No PulsarOS override for:

- `monitors.xml` handling
- `org.gnome.Mutter.DisplayConfig` D-Bus interface
- Hotplug detection
- Multi-monitor layout logic

## Architecture Diagram

```
                     +-----------------+
                     | gnome-control-  |
                     | center (UI)     |
                     +--------+--------+
                              |
                     D-Bus: org.gnome.Mutter.DisplayConfig
                              |
                     +--------v--------+
                     |     mutter      |
                     | MetaMonitorManager
                     | MetaMonitorConfigManager
                     | MetaMonitorConfigStore
                     +---+--------+---+
                         |        |
          +--------------+        +--------------+
          |                                     |
  monitors.xml                        D-Bus signal
  (~/.config/)                        MonitorsChanged
          |                                     |
  +-------v-------+                  +----------v----------+
  | MetaMonitor    |                  | gnome-settings-     |
  | ConfigStore    |                  | daemon (gsd-        |
  | (XML parser/   |                  | xsettings, etc.)    |
  |  writer)       |                  +---------------------+
  +----------------+                            |
                                                v
                              GSettings / dconf (org.gnome.desktop.*)
                                                |
                                        +-------v-------+
                                        | dconf-service |
                                        | ~/.config/    |
                                        | dconf/user    |
                                        +---------------+

Hardware Detection:
  Kernel DRM -> udev -> GUdevClient -> MetaUdev -> MetaMonitorManagerKms
                                                          |
                                            handle_hotplug_event()
                                                          |
                                            meta_monitor_manager_on_hotplug()
```

---

## GNOME Shell Runtime Subsystems

### Notifications

#### Architecture

Three APIs exist. GNOME Shell is the **notification server** for all of them:

| API | Who uses it | How it reaches the shell |
|-----|-------------|--------------------------|
| **org.freedesktop.Notifications** (Fdo spec) | Any Linux app via libnotify or raw D-Bus | D-Bus `Notify()` method call |
| **Gio.Notification** (GLib/GIO) | GNOME/GTK apps | GIO sends to `org.freedesktop.Notifications` or the XDG Notification Portal |
| **XDG Desktop Notification Portal** (`org.freedesktop.portal.Notification`) | Flatpak/sandboxed apps | Portal daemon forwards to shell |

GNOME 46+ migrates toward the portal as the canonical path.

#### D-Bus Interface

GNOME Shell owns `org.freedesktop.Notifications` on the session bus via a separate proxy daemon:

- Bus name: `org.freedesktop.Notifications`
- Object path: `/org/freedesktop/Notifications`
- Interface: `org.freedesktop.Notifications`

| Method | Signature |
|--------|-----------|
| `Notify` | `(sasa{sv}) → u` -- app_name, replaces_id, app_icon, summary, body, actions, hints, expire_timeout |
| `CloseNotification` | `(u)` -- id |
| `GetCapabilities` | `→ as` -- supported features |
| `GetServerInformation` | `→ (ssss)` -- name, vendor, version, spec_version |

| Signal | Signature |
|--------|-----------|
| `NotificationClosed` | `(uu)` -- id, reason |
| `ActionInvoked` | `(us)` -- id, action_key |

#### Notification Flow

```
1. App calls Gio.Notification / libnotify / raw D-Bus Notify()
         |
         v
2. org.freedesktop.Notifications D-Bus name
   (owned by gnome-shell-notification-daemon process)
         |
         v
3. NotificationDaemon.js proxies to org.gnome.Shell
   js/ui/notificationDaemon.js
         |
         v
4. Shell creates MessageTray.Source + MessageTray.Notification
   js/ui/messageTray.js
         |
         +---> Banner popup (temporary overlay at screen top)
         |
         +---> Message Tray (notification center, revealed from bottom)
              Notifications persist until dismissed, expired, or source closed
```

#### Key Source Files

| File | Role |
|------|------|
| `js/ui/notificationDaemon.js` | `NotificationDaemon` -- D-Bus handler, creates `Source` objects per app |
| `js/ui/messageTray.js` | `MessageTray`, `Source`, `Notification`, `NotificationPolicy` -- notification center UI and data model |
| `js/ui/calendar.js` | `CalendarMessageList` -- notification list inside the date menu |
| `src/dbusServices/notifications/notificationDaemon.js` | Out-of-process proxy daemon |

#### Key Classes

- **`NotificationDaemon`** -- D-Bus interface handler, maps Fdo notifications to MessageTray sources
- **`MessageTray.Source`** -- groups notifications by app; owns icon, title, policy
- **`MessageTray.Notification`** -- single notification with title, body, actions, urgency
- **`NotificationPolicy`** / **`NotificationGenericPolicy`** -- controls banners, sounds, lock screen behavior
- **`Urgency`** -- `LOW(0)`, `NORMAL(1)`, `HIGH(2)`, `CRITICAL(3)`

#### Persistence

Fdo spec advertises `"persistence"` capability. GNOME Shell keeps dismissed notifications in the Message Tray until the source is destroyed. `Gio.Notification` notifications can survive app exit only when routed through the shell's private API.

---

### Calendar / Date Menu

#### Architecture

Calendar events load **out-of-process** because Evolution Data Server (EDS) libraries can block and are not thread-safe. A dedicated `gnome-shell-calendar-server` C binary talks to EDS and exposes a private D-Bus interface.

#### Data Flow

```
Evolution Data Server (e-cal-client, e-source-registry)
         |
         v
gnome-shell-calendar-server (C binary)
   bus name: org.gnome.Shell.CalendarServer
   object path: /org/gnome/Shell/CalendarServer
   interface: org.gnome.Shell.CalendarServer
         |
         v
gnome-shell JS process
   js/ui/dateMenu.js -- DateMenuButton
   js/ui/calendar.js -- Calendar, DBusEventSource
```

#### D-Bus Interface: org.gnome.Shell.CalendarServer

| Method | Args | Purpose |
|--------|------|---------|
| `SetTimeRange` | `(xxb)` since, until, force_reload | Request events in a time window |

| Signal | Args | Purpose |
|--------|------|---------|
| `EventsAdded` | `a(ssbxxa{sv})` events | Event tuple: (id, summary, all_day, start_time, end_time, extras) |
| `EventsRemoved` | `as` ids | Removed event IDs |
| `ClientDisappeared` | `s` source_uid | EDS source went away |

| Property | Type | Purpose |
|----------|------|---------|
| `HasCalendars` | `b` | Whether any calendar sources exist |
| `Since` / `Until` | `x` | Current time range bounds |

#### Calendar Server Internals

- `gnome-shell-calendar-server.c` -- GApplication, owns the bus name
- `calendar-sources.c` -- wraps `ESourceRegistry`, manages `ECalClient` connections per source
- Creates `ECalClientView` live queries with iCal RDATE/expansion
- Emits `EventsAddedOrUpdated` / `EventsRemoved` when EDS view fires
- Handles timezone via `e_cal_system_timezone_get_location()`
- Integrates `EReminderWatcher` for alarm/reminder notifications

#### Key Source Files

| File | Role |
|------|------|
| `src/calendar-server/gnome-shell-calendar-server.c` | Out-of-process calendar server |
| `src/calendar-server/calendar-sources.c` | EDS source registry + ECalClient management |
| `src/calendar-server/reminder-watcher.c` | Calendar reminder/alarm handling |
| `js/ui/dateMenu.js` | `DateMenuButton` -- clock+calendar dropdown in top bar |
| `js/ui/calendar.js` | `Calendar` widget, `DBusEventSource`, `EventsSection`, `CalendarMessageList` |

#### Key Classes

- **`DateMenuButton`** -- `PanelMenu.Button` subclass; owns clock display, calendar widget, events section, world clocks, weather
- **`Calendar`** -- monthly calendar grid widget
- **`DBusEventSource`** -- JS proxy to `org.gnome.Shell.CalendarServer`; calls `SetTimeRange`, listens for events
- **`EventsSection`** -- shows upcoming events below the calendar
- **`WorldClocksSection`**, **`WeatherSection`** -- additional displays in the date dropdown

---

### Top Bar / System Status

#### Panel Layout

```
+---------------------------------------------------------------+
| [leftBox]              [centerBox]           [rightBox]       |
| Activities            Date/Time/Clock        QuickSettings    |
| workspace indicators  MessagesIndicator      indicators       |
|                                            (wifi, vol, bat)  |
+---------------------------------------------------------------+
```

#### Panel Item Registry

```js
// js/ui/panel.js
const PANEL_ITEM_IMPLEMENTATIONS = {
    'activities': ActivitiesButton,
    'quickSettings': QuickSettings,
    'dateMenu': DateMenuButton,
    'a11y': ATIndicator,
    'keyboard': InputSourceIndicator,
    'dwellClick': DwellClickIndicator,
    'screenRecording': ScreenRecordingIndicator,
    'screenSharing': ScreenSharingIndicator,
};
```

#### QuickSettings Menu

```
QuickSettings (PanelMenu.Button)
  +-- indicators box (wifi icon, volume icon, battery icon...)
  +-- QuickSettingsItems grid (toggles):
  |     +-- Wi-Fi toggle
  |     +-- Bluetooth toggle
  |     +-- Night Light toggle
  |     +-- Dark Style toggle
  |     +-- Do Not Disturb toggle
  +-- Sliders:
  |     +-- Volume slider (QuickSlider)
  |     +-- Brightness slider
  |     +-- Microphone slider
  +-- SystemItem (bottom row):
        +-- Settings button
        +-- Lock button
        +-- Power Off button (Restart, Log Out, Switch User sub-menu)
```

#### D-Bus Services Consumed by Status Indicators

| Indicator | D-Bus Service | Bus Name |
|-----------|---------------|----------|
| Network (Wi-Fi, wired) | NetworkManager | `org.freedesktop.NetworkManager` |
| Volume | PulseAudio / PipeWire | `org.pulseaudio.Server` (via Gvc/MixerControl) |
| Bluetooth | BlueZ | `org.bluez` |
| Battery/Power | UPower | `org.freedesktop.UPower` |
| System (shutdown/lock) | systemd-logind | `org.freedesktop.login1` |
| Screen Sharing | Mutter ScreenCast | PipeWire-based screencast |
| Remote Access | GNOME RemoteDesktop | RDP/VNC |

#### Key Source Files

| File | Role |
|------|------|
| `js/ui/panel.js` | `Panel` -- top bar widget, `ActivitiesButton`, panel layout |
| `js/ui/panelMenu.js` | `PanelMenu.Button` -- base class for all panel indicators |
| `js/ui/quickSettings.js` | `QuickSettingsMenu`, `SystemIndicator`, `QuickSettingsItem`, `QuickSlider` |
| `js/ui/status/system.js` | `ShutdownItem`, `LockItem`, `SettingsItem` |
| `js/ui/status/network.js` | Network indicator, NMApplet |
| `js/ui/status/volume.js` | Volume indicator |
| `js/ui/status/power.js` | Battery indicator, UPower proxy |
| `js/ui/status/bluetooth.js` | Bluetooth indicator |
| `js/ui/status/keyboard.js` | Input source indicator |
| `js/ui/status/accessibility.js` | Accessibility indicators |
| `js/ui/status/remoteAccess.js` | Screen sharing/recording indicators |

#### Extension API for Quick Settings

Extensions add indicators via:
```js
Main.panel.statusArea.quickSettings.addExternalIndicator(this._indicator);
```
Where `_indicator` is a `SystemIndicator` subclass with `quickSettingsItems` array.

---

### Shell Extensions

#### Loading Mechanism

1. **Discovery**: `ExtensionManager` scans `$XDG_DATA_HOME/gnome-shell/extensions/` (user) and `$XDG_DATA_DIRS/gnome-shell/extensions/` (system) for directories with `metadata.json`.
2. **Registration**: Each extension gets an entry in `ExtensionManager._extensions[uuid]`.
3. **Enable/Disable**: `org.gnome.shell` GSettings key `enabled-extensions` (type `as`) lists enabled UUIDs. `ExtensionManager` watches this key.
4. **Loading**: `extension.js` imported as ES module. Must export a subclass of `Extension` (or legacy `init()`/`enable()`/`disable()` functions).

#### Extension Lifecycle

```
constructor(metadata)   <- once, when module is loaded
enable()                <- when extension is enabled
disable()               <- when extension is disabled
```

#### D-Bus Interface

| Method | Return | Purpose |
|--------|--------|---------|
| `ListExtensions` | `a{sa{sv}}` | All discovered extensions |
| `GetExtensionInfo` | `a{sv}` | Extension metadata |
| `EnableExtension` | `b` | Enable by UUID |
| `DisableExtension` | `b` | Disable by UUID |
| `LaunchExtensionPrefs` | | Open prefs window |
| `InstallExtension` | | Install from zip |
| `UninstallExtension` | | Remove extension |
| `ReloadExtension` | | Reload from disk |
| `CheckForUpdates` | | Check extensions.gnome.org |

| Signal | Purpose |
|--------|---------|
| `ExtensionStateChanged` | `(s, a{sv})` -- UUID, state info |

#### Settings Storage

Extensions use GSettings backed by dconf:

- Schema ID: `org.gnome.shell.extensions.<uuid>`
- Schema path: `/org/gnome/shell/extensions/<uuid>/`
- Schema files: `schemas/org.gnome.shell.extensions.<uuid>.gschema.xml`
- Compiled with `glib-compile-schemas` into `schemas/gschemas.compiled`
- `settings-schema` key in `metadata.json` enables `this.getSettings()` auto-resolution

#### Key Source Files

| File | Role |
|------|------|
| `js/ui/extensionSystem.js` | `ExtensionManager` -- discovery, loading, enable/disable |
| `js/ui/shellDBus.js` | `GnomeShellExtensions` -- D-Bus interface |
| `js/misc/extensionUtils.js` | `ExtensionFinder`, `getSettings()`, extension type constants |
| `js/ui/extensionDownloader.js` | Download/install from extensions.gnome.org |

---

### App Grid / Overview

#### Overview State Machine

```js
var ControlsState = {
    HIDDEN: 0,          // desktop, no overview
    WINDOW_PICKER: 1,   // activities overview -- workspaces visible
    APP_GRID: 2,        // application grid visible
};
```

Transitions animated via `OverviewAdjustment` (an `St.Adjustment`) interpolating between states.

#### Actor Hierarchy

```
Overview (js/ui/overview.js)
  +-- ControlsManager (js/ui/overviewControls.js)
       +-- SearchEntry (St.Entry, "Type to search")
       +-- AppDisplay (js/ui/appDisplay.js)
       |    +-- St.ScrollView
       |    |    +-- AppGrid (IconGrid subclass)
       |    |         +-- AppIcon items, FolderIcon items
       |    +-- PageIndicators
       +-- Dash (js/ui/dash.js)
       |    +-- AppFavorites
       |    +-- DashItemContainer items
       +-- SearchController
       +-- ThumbnailsBox (workspace thumbnails sidebar)
       +-- WorkspacesDisplay (js/ui/workspacesView.js)
            +-- Workspace views
```

#### ControlsManagerLayout

The `ControlsManagerLayout` (a `Clutter.BoxLayout`) computes positions based on `ControlsState`:

- **HIDDEN**: Everything offscreen
- **WINDOW_PICKER**: Search at top, workspace thumbnails on left, workspaces in center, dash at bottom, app display hidden
- **APP_GRID**: Search at top, app grid fills remaining space (workspaces shrink), dash at bottom

Layout respects the **work area** (excludes panels, docks) for proper sizing.

#### App Grid Internals

- **`AppDisplay`** extends `BaseAppView` -- paginated icon grid, folder dialogs, drag-and-drop
- **`AppGrid`** extends `IconGrid` -- `IconGridLayout` manages page/column/row computation
- **`PageManager`** -- reads/writes `global.settings` key `app-picker-layout` (GVariant `a{sa{sv}}`)
- **`AppIcon`** -- individual app launcher icon
- **`FolderIcon`** -- represents a folder; opens `FolderDialog` overlay
- App ordering: saved positions from `app-picker-layout` take priority; unplaced apps sort alphabetically
- DnD: apps dragged between pages and into/out of folders. `SwipeTracker` handles touchpad page switching.

#### Key Source Files

| File | Role |
|------|------|
| `js/ui/overview.js` | `Overview` -- top-level widget, show/hide/toggle logic |
| `js/ui/overviewControls.js` | `ControlsManager`, `ControlsManagerLayout`, `OverviewAdjustment` |
| `js/ui/appDisplay.js` | `AppDisplay`, `AppGrid`, `AppIcon`, `FolderIcon`, `FolderDialog`, `PageManager` |
| `js/ui/iconGrid.js` | `IconGrid`, `IconGridLayout` -- paginated grid layout |
| `js/ui/dash.js` | `Dash` -- favorites dock at bottom of overview |
| `js/ui/appFavorites.js` | `AppFavorites` -- reads/writes `org.gnome.shell` key `favorite-apps` |
| `js/ui/workspacesView.js` | `WorkspacesDisplay` -- workspace thumbnails and primary workspace |
| `js/ui/workspaceThumbnail.js` | `ThumbnailsBox` -- workspace thumbnail sidebar |
| `js/ui/pageIndicators.js` | `PageIndicators` -- dots showing current page |
| `js/ui/search.js` | `SearchResultsView` -- search providers integration |

---

## D-Bus Interfaces Summary

| Subsystem | Bus Name | Object Path | Interface |
|-----------|----------|-------------|-----------|
| Display Config | `org.gnome.Mutter.DisplayConfig` | `/org/gnome/Mutter/DisplayConfig` | `org.gnome.Mutter.DisplayConfig` |
| Notifications | `org.freedesktop.Notifications` | `/org/freedesktop/Notifications` | `org.freedesktop.Notifications` |
| Calendar | `org.gnome.Shell.CalendarServer` | `/org/gnome/Shell/CalendarServer` | `org.gnome.Shell.CalendarServer` |
| Shell Extensions | `org.gnome.Shell` | `/org/gnome/Shell` | `org.gnome.Shell.Extensions` |
| Network | `org.freedesktop.NetworkManager` | `/org/freedesktop/NetworkManager` | Various NM interfaces |
| Power/Battery | `org.freedesktop.UPower` | `/org/freedesktop/UPower` | `org.freedesktop.UPower.Device` |
| Bluetooth | `org.bluez` | Varies | `org.bluez.Adapter1`, `org.bluez.Device1` |
| Login/Session | `org.freedesktop.login1` | `/org/freedesktop/login1` | `org.freedesktop.login1.Manager` |
| AppIndicator | `org.kde.StatusNotifierWatcher` | `/StatusNotifierWatcher` | `org.kde.StatusNotifierWatcher` |

---

## Additional GNOME Subsystems

### Power Management

#### Architecture

```
+-----------------------------------------------+
| GNOME Settings / Shell / Control Center        |  session bus clients
+-----------------------------------------------+
| gnome-settings-daemon / gsd-power             |  session bus: org.gnome.SettingsDaemon.Power
|   gsd-power-manager.c (core logic)            |
|   gsd-backlight.c (display/kbd brightness)    |
+-----------------------------------------------+
| power-profiles-daemon                          |  system bus: net.hadess.PowerProfiles
+-----------------------------------------------+
| UPower                                        |  system bus: org.freedesktop.UPower
|   /org/freedesktop/UPower                     |
|   /org/freedesktop/UPower/devices/*           |
|   /org/freedesktop/UPower/KbdBacklight        |
+-----------------------------------------------+
| systemd-logind                                |  system bus: org.freedesktop.login1
|   HandleLidSwitch, Inhibit                    |
+-----------------------------------------------+
| Kernel (sysfs, udev, /dev/input/*)            |
+-----------------------------------------------+
```

#### Key D-Bus Interfaces

**org.freedesktop.UPower** (system bus):

| Method/Property | Purpose |
|-----------------|---------|
| `EnumerateDevices()` | List all power devices |
| `OnBattery` (prop) | Whether running on battery |
| `LidIsClosed` (prop) | Lid switch state |

**org.freedesktop.UPower.Device** (per-device):

| Property | Purpose |
|----------|---------|
| `Type` | 1=line-power, 2=battery, 6=keyboard, etc. |
| `State` | 0=unknown, 1=charging, 2=discharging, 4=full |
| `Percentage` | Battery percentage |
| `TimeToEmpty` / `TimeToFull` | Estimated seconds |
| `ChargeStartThreshold` / `ChargeEndThreshold` | Battery charge limits |
| `ChargeThresholdSupported` | Whether thresholds are supported |

**org.freedesktop.UPower.KbdBacklight** (system bus):

| Method | Purpose |
|--------|---------|
| `GetMaxBrightness()` | Maximum keyboard backlight level |
| `GetBrightness()` | Current level |
| `SetBrightness(value)` | Set level |

**net.hadess.PowerProfiles** (system bus):

| Property | Purpose |
|----------|---------|
| `ActiveProfile` | "power-saver", "balanced", or "performance" |
| `Profiles` | Available profiles with degradation info |

#### Suspend/Hibernate Flow

```
1. Idle timeout fires (gnome_idle_monitor_add_idle_watch)
   -> idle_set_mode(GSD_POWER_IDLE_MODE_SLEEP)

2. do_power_action_type() reads setting:
   - On battery: sleep-inactive-battery-type
   - On AC: sleep-inactive-ac-type

3. For suspend: gnome_session_shutdown_type("Suspend")
   -> org.freedesktop.login1.Manager.Suspend via systemd-logind

4. Inhibitors: gsd-power acquires logind inhibitor locks
   (delay type for sleep/idle) to prevent premature suspend
```

#### Lid Switch Flow

```
1. systemd-logind detects lid switch via /dev/input or ACPI
   -> emits LidClosed signal

2. gsd-power monitors UPower's LidIsClosed property
   OR receives logind's PrepareForSleep signal

3. gsd-power checks gsettings:
   - lid-close-ac-action (default: "suspend")
   - lid-close-battery-action (default: "suspend")

4. External monitor check: if multiple displays attached,
   gsd-power acquires inhibitor lock "Multiple displays attached"
   -> blocks logind's HandleLidSwitch

5. Action dispatched via do_power_action_type()
```

#### Idle Dimming Flow

```
idle_configure() in gsd-power-manager.c:
  1. Checks idle-dim setting and idle-delay timeout
  2. Sets up gnome_idle_monitor_add_idle_watch for DIM
  3. On DIM trigger:
     - Reads idle-brightness percentage
     - display_backlight_dim(): saves current, sets dim level
     - kbd_backlight_dim(): dims keyboard
  4. Sets up BLANK watch (next stage)
  5. On BLANK: backlight_disable(), toggle kbd off
  6. Sets up SLEEP watch (final stage)
  7. On SLEEP: do_power_action_type()
  8. On NORMAL (user active): restore saved brightness
```

#### Key GSettings Keys

| Schema | Key | Purpose |
|--------|-----|---------|
| `org.gnome.settings-daemon.plugins.power` | `sleep-inactive-ac-type` | Action on AC idle (suspend/hibernate/nothing) |
| `org.gnome.settings-daemon.plugins.power` | `sleep-inactive-battery-type` | Action on battery idle |
| `org.gnome.settings-daemon.plugins.power` | `lid-close-ac-action` | Lid close on AC |
| `org.gnome.settings-daemon.plugins.power` | `lid-close-battery-action` | Lid close on battery |
| `org.gnome.settings-daemon.plugins.power` | `idle-dim` | Whether to dim display before blank |
| `org.gnome.settings-daemon.plugins.power` | `idle-brightness` | Dim brightness percentage |

#### Key Source Files

| File | Role |
|------|------|
| `plugins/power/gsd-power-manager.c` | Core: idle handling, lid switch, suspend/hibernate, battery warnings |
| `plugins/power/gsd-backlight.c` | Display backlight abstraction |
| `power-profiles-daemon/src/ppd-main.c` | Power profiles daemon |
| `upower/src/up-main.c` | UPower daemon |
| `upower/src/up-device.c` | UPower device implementation |
| `upower/src/up-kbd-backlight.c` | Keyboard backlight LED interface |

---

### Screen Locking

#### Architecture

Since GNOME 3.5.5 (2012), screen locking is handled **natively by GNOME Shell**. The standalone `gnome-screensaver` is deprecated.

```
+-----------------------------------------------+
| Triggers                                       |
| - Ctrl+Alt+L (gsd-media-keys)                 |
| - System menu Lock button                      |
| - Idle timeout (gsd-power -> screenShield)     |
+-----------------------------------------------+
| GNOME Shell                                    |
|   js/ui/screenShield.js (lock screen UI)       |
|   org.gnome.ScreenSaver D-Bus interface        |
+-----------------------------------------------+
| GDM (for unlock authentication)                |
|   org.gnome.DisplayManager D-Bus (system bus)  |
|   PAM authentication                           |
+-----------------------------------------------+
| systemd-logind                                 |
|   org.freedesktop.login1.Session.Lock/Unlock   |
+-----------------------------------------------+
```

#### D-Bus Interfaces

**org.gnome.ScreenSaver** (session bus, gnome-shell):

| Method | Purpose |
|--------|---------|
| `Lock()` | Immediately lock screen |
| `GetActive()` | Whether screensaver is active |
| `GetActiveTime()` | Seconds since activation |
| `SetActive(active)` | Enable/disable screensaver |

| Signal | Purpose |
|--------|---------|
| `ActiveChanged(boolean)` | `true` = locked, `false` = unlocked |

**org.freedesktop.ScreenSaver** (freedesktop standard, also implemented by gnome-shell):

| Method | Purpose |
|--------|---------|
| `Lock()` | Lock screen |
| `SimulateUserActivity()` | Prevent screensaver activation |

#### Lock Flow

```
1. Idle detected by gnome_idle_monitor
   OR manual lock via Ctrl+Alt+L / system menu

2. screenShield.deactivate() called
   -> Creates lock screen group with clock, notification box
   -> Slides curtain overlay down (1.2s animation)
   -> Grabs keyboard/pointer input
   -> Emits ActiveChanged(true)

3. User presses key/moves mouse -> unlock dialog appears
   -> PAM authentication via org.gnome.DisplayManager (GDM)
   -> On success: curtain slides up
   -> Emits ActiveChanged(false)
```

#### Key GSettings Keys (org.gnome.desktop.screensaver)

| Key | Default | Purpose |
|-----|---------|---------|
| `idle-delay` | 300 | Seconds before blanking |
| `lock-enabled` | true | Lock when screen blanks |
| `lock-delay` | 0 | Seconds after blanking before lock |
| `show-notifications` | true | Show notifications on lock screen |

#### Key Source Files

| File | Role |
|------|------|
| `js/ui/screenShield.js` | Core lock screen: curtain, clock, notifications, unlock dialog |
| `js/ui/shellDBus.js` | Registers org.gnome.ScreenSaver D-Bus interface |
| `src/shell-screen-saver-dbus.c` | C D-Bus skeleton |
| `plugins/media-keys/gsd-media-keys-manager.c` | Ctrl+Alt+L shortcut handler |
| `gdm/daemon/gdm-manager.c` | GDM session management, PAM verification |

---

### Screenshots / Recording

#### Architecture

```
+-----------------------------------------------+
| Applications                                   |
|   gnome-screenshot (legacy)                    |
|   Sandboxed apps (Flatpak)                     |
+-----------------------------------------------+
| XDG Desktop Portal (session bus)               |
|   org.freedesktop.portal.Screenshot            |
|   org.freedesktop.portal.ScreenCast            |
+-----------------------------------------------+
| xdg-desktop-portal-gnome (backend)             |
|   Bridges portal to gnome-shell APIs           |
+-----------------------------------------------+
| GNOME Shell (session bus)                      |
|   org.gnome.Shell.Screenshot                   |
|   org.gnome.Shell.Screencast                   |
+-----------------------------------------------+
| PipeWire                                       |
|   pw_stream, pipewiresrc (GStreamer)           |
+-----------------------------------------------+
```

#### org.gnome.Shell.Screenshot (session bus)

| Method | Purpose |
|--------|---------|
| `Screenshot(flash, filename)` | Full screen capture |
| `ScreenshotWindow(frame, cursor, flash, filename)` | Window capture |
| `ScreenshotArea(x, y, w, h, flash, filename)` | Area capture |
| `InteractiveScreenshot()` | Interactive overlay mode |
| `PickColor()` | Color picker |
| `SelectArea()` | User selects region |

#### org.freedesktop.portal.Screenshot (session bus)

| Method | Purpose |
|--------|---------|
| `Screenshot(parent_window, options)` | Capture with options (modal, interactive, target) |
| `PickColor(parent_window, options)` | Color picker |

Options target bitmask: 1=Screen, 2=Window, 4=Area, 8=ActiveWindow

#### org.freedesktop.portal.ScreenCast (session bus)

| Method | Purpose |
|--------|---------|
| `CreateSession(options)` | Create screencast session |
| `SelectSources(session, options)` | Select monitors/windows, cursor mode |
| `Start(session, parent_window, options)` | Start capture, returns PipeWire node IDs |
| `OpenPipeWireRemote(session, options)` | Returns PipeWire fd for stream connection |

Options: `types(u)` (1=MONITOR, 2=WINDOW), `cursor_mode(u)` (1=Hidden, 2=Embedded, 4=Metadata), `persist_mode(u)`

#### PipeWire Screencast Flow

```
1. App calls CreateSession()
2. App calls SelectSources(types=MONITOR, cursor_mode=EMBEDDED)
3. App calls Start()
   -> Portal shows source selection dialog
   -> Mutter creates PipeWire stream node
   -> Returns: streams = [{node_id, properties: {position, size}}]
4. App calls OpenPipeWireRemote()
   -> Returns fd -> app connects: pw_context_connect_fd(fd)
   -> Frames arrive as DMA-BUF or video format buffers
5. App closes Session -> PipeWire stream destroyed
```

#### Screen Recording Indicator

When recording active (Ctrl+Shift+Alt+R or D-Bus):
1. `Shell.Recorder` creates GStreamer pipeline: `pipewiresrc -> vp8enc -> webmmux -> filesink`
2. Red circle indicator with elapsed time appears in top bar
3. Clicking indicator stops recording
4. Default max: 30s (configurable via `org.gnome.settings-daemon.plugins.media-keys max-screencast-length`)

#### Key Source Files

| File | Role |
|------|------|
| `js/ui/screenshot.js` | Interactive screenshot overlay, Print Screen handler |
| `src/shell-screenshot.c` | C: pixel capture via `stage_paint_to_buffer()` |
| `src/shell-recorder.c` | C: screen recording, GStreamer pipeline |
| `js/dbusServices/screencast/screencastService.js` | Screencast D-Bus service |
| `xdg-desktop-portal-gnome/src/screenshot.c` | Portal screenshot backend |
| `xdg-desktop-portal-gnome/src/screencast.c` | Portal screencast backend |

---

### Input Methods

#### Overview

GNOME integrates IBus (Intelligent Input Bus) as its default input method framework. The system provides a unified UI for keyboard layouts (XKB) and input methods (IBus engines) via "input sources."

#### GSettings Schema (org.gnome.desktop.input-sources)

| Key | Type | Purpose |
|-----|------|---------|
| `sources` | `a(ss)` | Input sources: `("xkb","us")` or `("ibus","anthy")` |
| `mru-sources` | `a(ss)` | Most recently used sources |
| `xkb-options` | `as` | XKB options (e.g. `compose:ralt`) |
| `per-window` | `b` | Different sources per window |
| `show-all-sources` | `b` | Show all installed sources |

#### Data Flow (Wayland -- Composer Mode)

```
User types -> GTK4 app -> Wayland text-input protocol -> Mutter
  -> org.freedesktop.IBus.InputContext D-Bus -> ibus-daemon
  -> IBus engine (e.g. ibus-libpinyin) processes input
  -> commit signal back through Mutter -> GTK4 app
```

#### Data Flow (X11)

```
User types -> GTK app -> GtkIMContext -> IBus (GTK_IM_MODULE=ibus)
  -> ibus-daemon -> engine -> commit signal -> GTK app
```

#### Input Source Switching

- Triggered via `Super+Space` (default) or `org.gnome.desktop.wm.keybindings switch-input-source`
- `gsd-keyboard-manager.c` watches `current` key changes
- For IBus: calls `org.freedesktop.IBus.InputContext.SetEngine()`
- For XKB: applies new layout via XKB API

#### fcitx5 Support

Users set `GTK_IM_MODULE=fcitx`, `XMODIFIERS=@im=fcitx5`, `QT_IM_MODULE=fcitx5`. GNOME's sources list doesn't have a native "fcitx" type -- users typically use fcitx5's IBus compatibility layer or manage it externally.

#### Key Source Files

| File | Role |
|------|------|
| `plugins/keyboard/gsd-keyboard-manager.c` | Applies XKB config, switches IBus engines |
| `src/wayland/meta-wayland-text-input.c` | Wayland text-input protocol handling |
| `src/wayland/meta-wayland-input-method.c` | Wayland input-method protocol |
| `bus/inputcontext.c` | IBus InputContext D-Bus implementation |

---

### Accessibility

#### Architecture (Two-Layer Bus)

GNOME accessibility uses a **separate D-Bus bus** (not the session bus) for the chatty AT-SPI protocol.

```
Session Bus:                    Accessibility Bus (separate daemon):
  org.a11y.Bus                     org.a11y.atspi.Registry
     |                                 |
  GetAddress() returns address  -->  Registry.RegisterEvent()
                                     org.a11y.atspi.Accessible.*
                                     org.a11y.atspi.Action.*
                                     org.a11y.atspi.Event.*
```

The accessibility bus runs at `/run/user/UID/at-spi/bus_0`.

#### D-Bus Interfaces

| Interface | Bus | Purpose |
|-----------|-----|---------|
| `org.a11y.Bus` | Session | Entry point: `GetAddress()` |
| `org.a11y.Status` | Session | `IsEnabled`, `ScreenReaderEnabled` properties |
| `org.a11y.atspi.Registry` | A11y Bus | Event registration |
| `org.a11y.atspi.Accessible` | A11y Bus | Base: `Name`, `Description`, `Role`, `Children` |
| `org.a11y.atspi.Action` | A11y Bus | `GetNActions()`, `DoAction()` |
| `org.a11y.atspi.Event` | A11y Bus | Focus, state-changed, window events |

#### GSettings Schemas

**org.gnome.desktop.a11y.applications:**

| Key | Purpose |
|-----|---------|
| `screen-reader-enabled` | Launches Orca |
| `screen-magnifier-enabled` | Launches gnome-mag |

**org.gnome.desktop.a11y.keyboard:**

| Key | Purpose |
|-----|---------|
| `slowkeys-delay` | Hold time before key registers (ms) |
| `bouncekeys-delay` | Ignore period after keypress (ms) |
| `stickykeys-enable` | Modifier sequences without holding |

**org.gnome.desktop.a11y.interface:**

| Key | Purpose |
|-----|---------|
| `high-contrast` | High contrast theme |
| `large-text` | Large text mode |
| `text-scaling-factor` | Fine-grained text scaling |

#### Data Flow

```
Application (GTK3) -> ATK API -> atk-adaptor (in-process)
  -> D-Bus -> accessibility bus -> libatspi -> pyatspi2 -> Orca
  -> Speech Dispatcher / liblouis (braille) -> User

Application (GTK4) -> D-Bus directly (no ATK intermediary)
  -> accessibility bus -> libatspi -> Orca
```

#### Key Source Files

| File | Role |
|------|------|
| `plugins/a11y-settings/gsd-a11y-settings-manager.c` | Enables toolkit-accessibility when a11y active |
| `plugins/a11y-keyboard/` | Applies slow/bounce/sticky keys to XKB |
| `registryd/` | `at-spi2-registryd` -- app registry, event routing |
| `bus/at-spi-bus-launcher.c` | Launches accessibility bus |
| `atk-adaptor/` | Translates ATK API to AT-SPI D-Bus |
| `src/orca/` | Orca screen reader |

---

### Color Management

#### Architecture

Color management spans three layers: **colord** (system daemon, device/profile registry), **gsd-color** (night light tracking), and **Mutter** (owns color device management and profile application).

#### D-Bus Interfaces (colord, system bus)

| Interface | Purpose |
|-----------|---------|
| `org.freedesktop.ColorManager` | Top-level: `GetDevices()`, `GetProfiles()` |
| `org.freedesktop.ColorManager.Device` | Per-device: `GetProfiles()`, `SetProfile()` |
| `org.freedesktop.ColorManager.Profile` | Per-profile: `GetFilename()`, `GetMetadata()` |
| `org.freedesktop.ColorManager.Sensor` | Calibration device |

#### Persistent Storage

| Path | Purpose |
|------|---------|
| `/var/lib/colord/mapping.db` | Device-to-profile mappings |
| `/var/lib/colord/storage.db` | Profile metadata |
| `/var/lib/colord/icc/` | System-wide ICC profiles |
| `~/.local/share/icc/` | Per-user ICC profiles |

#### Data Flow

```
Monitor connected -> Mutter (MetaMonitorManager) detects via KMS/udev
  -> MetaColorManager creates MetaColorDevice
  -> MetaColorDevice creates colord Device (D-Bus)
  -> MetaColorStore resolves ICC profile:
      1. Check colord for assigned profile
      2. Check ~/.local/share/icc/ for user profiles
      3. Generate from EDID as fallback
  -> MetaColorDevice applies profile:
      - X11: set _ICC_PROFILE atom, load VCGT gamma LUT
      - Wayland: apply color transform matrix to compositor
  -> Night light: mutter reads temperature from gsd-color D-Bus
```

#### Mutter Color Classes

- **`MetaColorManager`** -- singleton, creates `MetaColorDevice` per monitor
- **`MetaColorManagerX11`** -- X11: sets `_ICC_PROFILE` atoms, loads VCGT
- **`MetaColorDevice`** -- per-monitor, creates colord device, applies profile
- **`MetaColorProfile`** -- ICC profile abstraction (EDID-generated, standard, calibration)
- **`MetaColorStore`** -- caches profiles, watches `~/.local/share/icc/`

#### gsd-color Current Role

After mutter integration, `gsd-color` is reduced to:
- Tracking night light state (on/off, temperature)
- Exposing via `org.gnome.SettingsDaemon.Power.Screen` D-Bus
- Mutter reads this and applies warm color temperature filter

#### Key Source Files

| File | Role |
|------|------|
| `plugins/color/gsd-color-manager.c` | Night light tracking |
| `src/backends/meta-color-manager.c` | Main color management object |
| `src/backends/meta-color-manager-x11.c` | X11 ICC atom, VCGT |
| `src/backends/meta-color-device.c` | Per-monitor colord device |
| `src/backends/meta-color-profile.c` | ICC profile abstraction |
| `src/backends/meta-color-store.c` | Profile cache, watches user ICC dir |

---

### Window Management

#### Overview

Mutter is both a Wayland compositor and window manager library. It handles keybindings, workspaces, tiling, focus, and compositing.

#### Keybinding Schemas

| Schema | Purpose |
|--------|---------|
| `org.gnome.desktop.wm.keybindings` | Shared WM keybindings (legacy metacity port) |
| `org.gnome.mutter.keybindings` | Mutter-specific keybindings |
| `org.gnome.mutter.wayland.keybindings` | Wayland-specific keybindings |
| `org.gnome.shell.keybindings` | Shell keybindings (override wm.keybindings when both define same action) |

#### Keybinding Data Flow

```
Input event (libinput)
  -> Clutter event processing
    -> process_event() in keybindings.c
      -> lookup in key_bindings hash table (combo + layout index)
        -> invoke_handler() -> MetaKeyHandler.func()
          -> handle_switch_to_workspace(), handle_toggle_tiled(), etc.
```

#### Window Tiling

**MetaTileMode:**
```
META_TILE_NONE, META_TILE_LEFT, META_TILE_RIGHT, META_TILE_MAXIMIZED
```

**MetaTileZone (constraint-based, WIP):**
```
MAXIMIZED_HORZ, MAXIMIZED_VERT, MAXIMIZED,
W (left), E (right), N (top), S (bottom),
NW, NE, SW, SE (quarters)
```

**Tiling flow:**
1. Keybinding triggers `handle_toggle_tiled()`
2. Sets `window->tile_monitor_number`
3. Calls `meta_window_tile(window, mode)`
4. Computes target rectangle via `get_tile_zone_area()`
5. Constraint engine applies tile zone constraints

**Tile matching:** `meta_window_compute_tile_match()` finds complementary window (LEFT<->RIGHT). Resizing one auto-resizes the sibling.

#### Mutter Plugin API

Plugins are shared libraries. `GnomeShellPlugin` is the concrete implementation.

Plugin virtual methods:
```
start, minimize, unminimize, size_changed, size_change, map, destroy,
switch_workspace, show_tile_preview, hide_tile_preview,
show_window_menu, keybinding_filter, confirm_display_change,
create_close_dialog, locate_pointer
```

Each effect must call `meta_plugin_*_completed()` when done.

#### Key Source Files

| File | Role |
|------|------|
| `src/core/keybindings.c` | Keybinding manager, grab/ungrab, dispatch |
| `src/core/meta-accel-parse.c` | Accelerator string parser |
| `src/core/display.c` | `MetaDisplay` -- central singleton |
| `src/core/window.c` | `MetaWindow` -- tiling, maximize, minimize |
| `src/core/constraints.c` | Constraint engine (tile zones, placement) |
| `src/core/place.c` | Window placement algorithm |
| `src/meta/meta-workspace-manager.h` | Workspace management API |
| `src/meta/meta-plugin.h` | Plugin abstract class |
| `src/gnome-shell-plugin.c` | GNOME Shell's plugin implementation |

---

### Wayland / Portals

#### Architecture

The portal system provides sandboxed apps access to desktop features through D-Bus.

```
+-----------------------------------------------+
| Sandboxed Apps (Flatpak/snap)                  |
+-----------------------------------------------+
| xdg-desktop-portal (frontend)                  |
|   Bus: org.freedesktop.portal.Desktop          |
|   Routes to backend based on XDG_CURRENT_DESKTOP|
+-----------------------------------------------+
| xdg-desktop-portal-gnome (backend)             |
|   Implements org.freedesktop.impl.portal.*     |
|   Bridges to GNOME-specific D-Bus APIs         |
+-----------------------------------------------+
| GNOME Shell / Mutter / other daemons           |
+-----------------------------------------------+
```

#### Key Portal Interfaces

| Interface | Purpose |
|-----------|---------|
| `org.freedesktop.portal.FileChooser` | File open/save dialogs |
| `org.freedesktop.portal.ScreenCast` | Screen capture via PipeWire |
| `org.freedesktop.portal.RemoteDesktop` | Remote desktop (input + capture) |
| `org.freedesktop.portal.Screenshot` | Screenshot capture |
| `org.freedesktop.portal.Settings` | Desktop settings (font, theme) |
| `org.freedesktop.portal.Notification` | Desktop notifications |
| `org.freedesktop.portal.Inhibit` | Inhibit screensaver/logout |
| `org.freedesktop.portal.Background` | Background activity management |
| `org.freedesktop.portal.Wallpaper` | Wallpaper setting |
| `org.freedesktop.portal.Clipboard` | Clipboard access |
| `org.freedesktop.portal.InputCapture` | Input capture for remote desktop |

#### Backend Selection

1. Backend declares in `/usr/share/xdg-desktop-portal/portals/gnome.portal`:
   ```ini
   [portal]
   Interfaces=org.freedesktop.impl.portal.FileChooser;...
   UseIn=gnome
   ```

2. `xdg-desktop-portal` reads `$XDG_CURRENT_DESKTOP`
3. For each interface, selects backend via `.portal` files
4. Frontend creates proxy to backend's `org.freedesktop.impl.portal.*`

#### Portal vs Direct D-Bus

| Aspect | Portal | Direct D-Bus |
|--------|--------|--------------|
| Sandboxing | Works in Flatpak/snap | Only outside sandbox |
| UI | Native desktop dialogs | No UI, direct call |
| Security | Permission store, user consent | Trusts caller |
| File access | FUSE filesystem / document IDs | Direct paths |
| Portability | Desktop-agnostic | Desktop-specific |

#### Key Source Files

| File | Role |
|------|------|
| `src/xdg-desktop-portal.c` | Frontend: owns bus name, routes to backends |
| `src/portal-impl.c` | Backend discovery |
| `src/request.c` | Request lifecycle |
| `src/file-chooser.c` | FileChooser frontend |
| `src/screen-cast.c` | ScreenCast frontend |
| `xdg-desktop-portal-gnome/src/filechooser.c` | GTK4 file chooser dialog |
| `xdg-desktop-portal-gnome/src/screencast.c` | Screencast via gnome-shell |
| `xdg-desktop-portal-gnome/src/screenshot.c` | Screenshot via gnome-shell |
| `xdg-desktop-portal-gnome/src/remotedesktop.c` | Remote desktop |

---

### Search Providers

#### Architecture

GNOME Shell's overview search uses a **search provider** architecture. Text input is dispatched to registered providers, which return results asynchronously.

#### D-Bus Interface: org.gnome.Shell.SearchProvider2

| Method | Args | Purpose |
|--------|------|---------|
| `GetInitialResultSet` | `(as) terms` | First search results |
| `GetSubsearchResultSet` | `(as) previous, (as) terms` | Refined results |
| `GetResultMetas` | `(as) identifiers` | Result metadata (name, icon, description) |
| `ActivateResult` | `(s) id, (as) terms, (u) timestamp` | Open selected result |
| `LaunchSearch` | `(as) terms, (u) timestamp` | Open full search view |

#### Registration (Application Providers)

`.ini` file in `$(datadir)/gnome-shell/search-providers/`:

```ini
[Shell Search Provider]
DesktopId=org.gnome.Nautilus.desktop
BusName=org.gnome.Nautilus
ObjectPath=/org/gnome/Nautilus/SearchProvider
Version=2
```

#### Registration (Extension Providers)

```javascript
class MySearchProvider {
    getInitialResultSet(terms, cancellation) { /* return string[] */ }
    getSubsearchResultSet(previousResults, terms, cancellation) { /* ... */ }
    getResultMetas(ids, cancellation) { /* return ResultMeta[] */ }
    activateResult(id, terms, timestamp) { /* open result */ }
}
Main.overview.searchController.addProvider(provider);
```

#### ResultMeta Properties

| Property | Type | Purpose |
|----------|------|---------|
| `id` | `s` | Unique identifier |
| `name` | `s` | Display name |
| `icon` / `gicon` / `icon-data` | various | App icon |
| `description` | `s` | Optional description |
| `clipboardText` | `s` | Text for clipboard on activation |

#### Built-in Providers

1. **Application Search** -- `.desktop` apps, Flatpak apps (always active)
2. **Calculator** -- arithmetic expressions
3. **Terminal commands** -- suggests CLI commands
4. **Settings** -- GNOME Settings panels
5. **File search** -- via Localsearch/Tracker (TinySPARQL)

#### Data Flow

```
User types "foo" in overview search
  -> SearchController dispatches to all providers:
      1. AppSearchProvider: fuzzy match apps
      2. External providers (D-Bus): GetInitialResultSet(["foo"])
      3. Extension providers (in-process)
  -> GetResultMetas for top results
  -> SearchResults renders grouped results (max 3 per provider)
  -> Click result: ActivateResult()
  -> Click provider icon: LaunchSearch()
```

#### Localsearch (Tracker) Integration

- **TinySPARQL** (`tsparql`): RDF triple store, SQLite backend
- **Localsearch**: Miner daemon, crawls filesystem, extracts metadata
- **D-Bus**: `org.freedesktop.Tracker3.Miner` -- pause, resume, status
- gnome-shell queries Localsearch via TinySPARQL (built into shell's C code)

#### Key Source Files

| File | Role |
|------|------|
| `js/ui/search.js` | `SearchController`, `SearchProviderManager` |
| `js/ui/searchResults.js` | Renders search results, groups by provider |
| `src/app-search-provider.c` | Built-in app/flatpak search |
| `src/shell-search-provider.c` | D-Bus proxy to external SearchProvider2 |
| `src/shell-app-system.c` | App registry |
