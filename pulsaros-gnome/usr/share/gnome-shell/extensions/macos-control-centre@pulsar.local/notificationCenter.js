import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Pango from 'gi://Pango';
import St from 'gi://St';
import Soup from 'gi://Soup?version=3.0';
import GWeather from 'gi://GWeather?version=4.0';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const NONE = Gio.DBusCallFlags.NONE;

// Marker Blur My Shell checks (see prepare-assets.sh) so it only skips the
// date menu while Liquid Glass is drawing its own glass on the cards.
const BLUR_EXCLUDE_CLASS = 'pulsar-blur-exclude';

const STRINGS = {
    es: {
        NOTIFICATIONS: 'NOTIFICACIONES',
        CLEAR_ALL: 'Borrar todo',
        EXPAND: 'Desplegar',
        COLLAPSE: 'Contraer',
        NO_NOTIFICATIONS: 'Sin notificaciones',
        MORE_NOTIFS: '+%d más... (Clic para desplegar)',
        WIDGETS: 'WIDGETS',
        TODAY: 'HOY',
        NO_EVENTS: 'Sin eventos hoy',
        SCHEDULED: 'Evento programado',
        TASKS: 'Recordatorios',
        ALL_DONE: 'Todo al día hoy',
        NO_TASKS: 'Sin tareas pendientes',
        PERFORMANCE: 'Rendimiento Pulsar',
        OPTIMAL: 'SISTEMA ÓPTIMO',
        CPU: 'CPU',
        MEMORY: 'MEMORIA',
        UPTIME: 'Tiempo activo',
        FREE: 'Libre',
        NOW: 'Ahora',
        UPDATED: 'Actualizado',
        MUSIC: 'Música',
        NO_MEDIA: 'Sin reproducción',
        BATTERY: 'Batería',
        CHARGING: 'Cargando',
        WORLD_CLOCK: 'Reloj Mundial',
        SAME_TIME: 'Misma hora',
        AHEAD: '+%dh',
        BEHIND: '-%dh',
        CLEAR_SKY: 'Despejado',
        FEW_CLOUDS: 'Poco nuboso',
        CLOUDY: 'Nublado',
        RAIN: 'Lluvia',
        SNOW: 'Nieve',
        THUNDERSTORM: 'Tormenta',
        FOG: 'Niebla',
        SYSTEM: 'Sistema',
        DEVICE: 'Dispositivo',
        MOUSE: 'Ratón',
        KEYBOARD: 'Teclado',
        HEADPHONES: 'Auriculares',
        CAL_DAYS: ['L', 'M', 'X', 'J', 'V', 'S', 'D'],
    },
    en: {
        NOTIFICATIONS: 'NOTIFICATIONS',
        CLEAR_ALL: 'Clear All',
        EXPAND: 'Expand',
        COLLAPSE: 'Collapse',
        NO_NOTIFICATIONS: 'No notifications',
        MORE_NOTIFS: '+%d more... (Click to expand)',
        WIDGETS: 'WIDGETS',
        TODAY: 'TODAY',
        NO_EVENTS: 'No events today',
        SCHEDULED: 'Scheduled event',
        TASKS: 'Reminders',
        ALL_DONE: 'All clear today',
        NO_TASKS: 'No tasks scheduled',
        PERFORMANCE: 'Pulsar Performance',
        OPTIMAL: 'SYSTEM OPTIMAL',
        CPU: 'CPU',
        MEMORY: 'MEMORY',
        UPTIME: 'Uptime',
        FREE: 'Free',
        NOW: 'Now',
        UPDATED: 'Updated',
        MUSIC: 'Music',
        NO_MEDIA: 'Not playing',
        BATTERY: 'Battery',
        CHARGING: 'Charging',
        WORLD_CLOCK: 'World Clock',
        SAME_TIME: 'Same time',
        AHEAD: '+%dh',
        BEHIND: '-%dh',
        CLEAR_SKY: 'Clear Sky',
        FEW_CLOUDS: 'Partly Cloudy',
        CLOUDY: 'Cloudy',
        RAIN: 'Rain',
        SNOW: 'Snow',
        THUNDERSTORM: 'Thunderstorm',
        FOG: 'Fog',
        SYSTEM: 'System',
        DEVICE: 'Device',
        MOUSE: 'Mouse',
        KEYBOARD: 'Keyboard',
        HEADPHONES: 'Headphones',
        CAL_DAYS: ['M', 'T', 'W', 'T', 'F', 'S', 'S'],
    }
};

function getLocale() {
    const langs = GLib.get_language_names();
    for (const l of langs) {
        if (l.startsWith('es')) return 'es';
    }
    return 'en';
}

function t(key, ...args) {
    const loc = getLocale();
    const dict = STRINGS[loc] || STRINGS.en;
    let str = dict[key] || STRINGS.en[key] || key;
    if (args.length > 0 && typeof str === 'string') {
        for (const arg of args) {
            str = str.replace(/%[ds]/, String(arg));
        }
    }
    return str;
}

function launch(cmd) {
    try {
        Gio.AppInfo.create_from_commandline(cmd, null, Gio.AppInfoCreateFlags.NONE).launch([], null);
    } catch (e) {
        console.warn(`macos-nc: failed to launch ${cmd}: ${e}`);
    }
}

// GJS aborts the whole shell when an exception escapes a GObject async
// callback ("terminated with uncatchable exception"). An extension callback
// that throws does not merely log: it takes the session down. Every DBus
// callback must therefore swallow its own errors, including those thrown by
// the fallback path, or a stale/disposed widget kills gnome-shell.
function guard(fn) {
    return (...args) => {
        try {
            return fn(...args);
        } catch (e) {
            logError(e, 'macos-nc: uncaught exception in async callback');
        }
    };
}

export class MacNotificationCenter {
    constructor(parentExtension) {
        this._parent = parentExtension;
        this._dateMenu = Main.panel.statusArea.dateMenu;
        this._ids = [];
        this._hiddenDateMenuChildren = [];
        this._notifsExpanded = false;
        this._soupSession = null;
        this._calendarProxy = null;
        this._calendarEvents = [];
        this._cards = [];
        this._mprisPlayers = [];
        this._activeMprisBus = null;
        this._lastCpuStats = null;
        this._pollTimer = 0;
        this._weatherData = null;
        this._locationName = 'A Coruña';
        this._lat = 43.3623;
        this._lon = -8.4115;
        // Remembers liquid-glass's date-menu sheet setting while the per-card
        // glass is active, so it can be restored afterwards.
        this._lgSheetOrig = null;
        this._lgToggles = false;
        this._blurExcluded = false;
    }

    enable() {
        this._destroyed = false;
        if (!this._dateMenu) {
            this._dateMenu = Main.panel.statusArea.dateMenu;
        }
        if (!this._dateMenu?.menu) {
            console.warn('macos-nc: Main.panel.statusArea.dateMenu.menu not available');
            return;
        }

        try {
            this._soupSession = new Soup.Session();
        } catch (e) {}

        this._applyTransparentDateMenu(true);
        this._initLocation();
        this._initCalendarServer();
        this._buildUI();

        // Hide default dateMenu widgets, preserving liquid-glass host
        const box = this._dateMenu.menu.box;
        box.get_children().forEach(child => {
            if (child !== this._root && !child.get_name()?.startsWith('liquid-glass') && child.visible) {
                child.visible = false;
                this._hiddenDateMenuChildren.push(child);
            }
        });
        box.add_child(this._root);

        // Connect menu open signal
        this._ids.push([
            this._dateMenu.menu,
            this._dateMenu.menu.connect('open-state-changed', guard((menu, isOpen) => {
                if (isOpen) {
                    this._dropDateMenuBlur();
                    GLib.timeout_add(GLib.PRIORITY_DEFAULT, 50, guard(() => {
                        this._dropDateMenuBlur();
                        return GLib.SOURCE_REMOVE;
                    }));
                    GLib.timeout_add(GLib.PRIORITY_DEFAULT, 250, guard(() => {
                        this._dropDateMenuBlur();
                        return GLib.SOURCE_REMOVE;
                    }));
                    this._updateScrollMaxHeight();
                    this._refreshAll();
                    this._startPoll();
                } else {
                    this._stopPoll();
                }
            })),
        ]);

        // Connect notifications model
        try {
            const notifSection = this._dateMenu._messageList?._notificationSection;
            if (notifSection?._model) {
                this._ids.push([
                    notifSection._model,
                    notifSection._model.connect('items-changed', guard(() => this._refreshNotifications())),
                ]);
            }
        } catch (e) {}

        // Listen for UPower devices
        try {
            const upower = Gio.DBus.system;
            this._ids.push([
                upower,
                upower.signal_subscribe(
                    'org.freedesktop.UPower',
                    'org.freedesktop.UPower',
                    'DeviceAdded',
                    null,
                    null,
                    Gio.DBusSignalFlags.NONE,
                    guard(() => this._refreshBattery())
                ),
            ]);
            this._ids.push([
                upower,
                upower.signal_subscribe(
                    'org.freedesktop.UPower',
                    'org.freedesktop.UPower',
                    'DeviceRemoved',
                    null,
                    null,
                    Gio.DBusSignalFlags.NONE,
                    guard(() => this._refreshBattery())
                ),
            ]);
        } catch (e) {}

        // Liquid Glass Integration
        try {
            this._lgSettings = new Gio.Settings({schema_id: 'org.gnome.shell.extensions.liquid-glass@thinkingcoding1231.gmail.com'});
            this._ids.push([this._lgSettings, this._lgSettings.connect('changed::enable-quick-settings-glass', guard(() => this._updateLiquidGlass()))]);
            this._ids.push([this._lgSettings, this._lgSettings.connect('changed::quick-settings-apply-to', guard(() => this._updateLiquidGlass()))]);
        } catch (e) {
            this._lgSettings = null;
        }
        this._updateLiquidGlass();

        try {
            this._refreshAll();
        } catch (e) {
            logError(e, 'macos-nc: initial refresh failed');
        }
    }

    _isLiquidGlassActive() {
        if (!this._lgSettings) return false;
        try {
            const active = this._lgSettings.get_boolean('enable-quick-settings-glass') ?? true;
            const mode = this._lgSettings.get_int('quick-settings-apply-to');
            return active && mode === 1;
        } catch (e) {
            return false;
        }
    }

    _updateLiquidGlass() {
        if (this._destroyed) return;
        const togglesMode = this._isLiquidGlassActive();

        try {
            if (this._lgSettings) {
                const sheet = this._lgSettings.get_boolean('enable-menu-glass');
                if (togglesMode) {
                    if (this._lgSheetOrig === null)
                        this._lgSheetOrig = sheet;
                    if (sheet)
                        this._lgSettings.set_boolean('enable-menu-glass', false);
                } else if (this._lgSheetOrig !== null) {
                    this._lgSettings.set_boolean('enable-menu-glass', this._lgSheetOrig);
                    this._lgSheetOrig = null;
                }
            }
        } catch (e) {}

        if (this._root) {
            if (togglesMode) {
                this._root.add_style_class_name('mac-lg-toggles');
            } else {
                this._root.remove_style_class_name('mac-lg-toggles');
            }
        }

        this._lgToggles = togglesMode;
        this._syncDateMenuSurface();
    }

    disable() {
        this._destroyed = true;
        this._stopPoll();

        // Restore liquid-glass date-menu sheet if it was forced off while active.
        try {
            if (this._lgSettings && this._lgSheetOrig !== null) {
                if ((this._lgSettings.get_boolean('enable-menu-glass') ?? false) !== this._lgSheetOrig)
                    this._lgSettings.set_boolean('enable-menu-glass', this._lgSheetOrig);
                this._lgSheetOrig = null;
            }
        } catch (e) {}

        for (const [obj, id] of this._ids) {
            try {
                if (obj && typeof obj.disconnect === 'function') obj.disconnect(id);
                else if (obj && typeof obj.signal_unsubscribe === 'function') obj.signal_unsubscribe(id);
            } catch (e) {}
        }
        this._ids = [];

        if (this._root) {
            this._root.destroy();
            this._root = null;
        }

        // Restore original dateMenu widgets
        for (const child of this._hiddenDateMenuChildren) {
            try {
                if (child) child.visible = true;
            } catch (e) {}
        }
        this._hiddenDateMenuChildren = [];

        this._applyTransparentDateMenu(false);
    }

    _applyTransparentDateMenu(enable) {
        const menu = this._dateMenu?.menu;
        if (!menu) return;
        const cls = 'mac-nc-datemenu';

        for (const actor of [menu.actor, menu.box, menu._boxPointer, menu._boxPointer?.bin]) {
            if (!actor) continue;
            if (enable)
                actor.add_style_class_name(cls);
            else
                actor.remove_style_class_name(cls);
        }

        // In Liquid Glass mode the cards draw their own glass, so the shell
        // date-menu surface is dropped along with Blur My Shell's blur. In
        // normal mode the classes stay so Blur My Shell blurs the panel.
        if (enable) {
            this._syncDateMenuSurface();
        } else if (menu.box) {
            menu.box.add_style_class_name('datemenu-popover');
            menu.box.add_style_class_name('popup-menu-content');
            for (const actor of [menu.actor, menu.box, menu._boxPointer, menu._boxPointer?.bin]) {
                if (actor)
                    actor.remove_style_class_name(BLUR_EXCLUDE_CLASS);
            }
        }
    }

    _dropDateMenuBlur() {
        const menu = this._dateMenu?.menu;
        if (!menu) return;

        // Let Blur My Shell destroy the surface it created for this menu (it
        // keeps one surface per target actor). Only needed while Liquid Glass
        // draws the cards itself; in normal mode the blur stays.
        try {
            const bms = Main.extensionManager.lookup('blur-my-shell@aunetx')?.stateObj;
            const popup = bms?._popup;
            if (popup?.surfaces && typeof popup.destroy_blur === 'function') {
                for (const target of [...popup.surfaces.keys()]) {
                    if (target === menu.box || target === menu.actor || this._isInsideDateMenu(target))
                        popup.destroy_blur(target);
                }
            }
        } catch (e) {}

        // Fallback: hide any blur surface stacked directly below the panel.
        try {
            const actor = menu.actor;
            const parent = actor?.get_parent();
            if (parent) {
                const children = parent.get_children();
                const idx = children.indexOf(actor);
                if (idx > 0) {
                    const below = children[idx - 1];
                    const classes = below?.get_style_class_name?.() ?? '';
                    const name = below?.name ?? '';
                    if (classes.includes('bms') || name.includes('bms')) {
                        below.visible = false;
                        below.opacity = 0;
                    }
                }
            }
        } catch (e) {}
    }

    _isInsideDateMenu(actor) {
        const root = this._dateMenu?.menu?.actor;
        for (let a = actor; a; a = a.get_parent()) {
            if (a === root)
                return true;
        }
        return false;
    }

    /*
     * Keeps the date-menu shell surface in sync with the Liquid Glass state.
     * While Liquid Glass draws per-card glass the shell background is dropped
     * and the menu is marked so Blur My Shell skips it; otherwise the classes
     * stay and Blur My Shell blurs the panel behind the cards.
     */
    _syncDateMenuSurface() {
        const menu = this._dateMenu?.menu;
        if (!menu) return;

        if (menu.box) {
            menu.box.remove_style_class_name('datemenu-popover');
            menu.box.remove_style_class_name('popup-menu-content');
        }

        for (const actor of [menu.actor, menu.box, menu._boxPointer, menu._boxPointer?.bin, this._root]) {
            if (!actor) continue;
            actor.add_style_class_name(BLUR_EXCLUDE_CLASS);
        }

        this._dropDateMenuBlur();
        this._blurExcluded = true;
    }

    _restoreDateMenuBlur() {
        try {
            const bms = Main.extensionManager.lookup('blur-my-shell@aunetx')?.stateObj;
            bms?._popup?.reset?.();
        } catch (e) {}
    }

    _updateScrollMaxHeight() {
        if (!this._scroll) return;
        const monitor = Main.layoutManager.primaryMonitor || { height: 900 };
        const maxH = Math.max(480, Math.min(740, monitor.height - 80));
        this._scroll.style = `max-height: ${maxH}px;`;
    }

    _startPoll() {
        this._stopPoll();
        this._pollTimer = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 2, guard(() => {
            if (this._dateMenu?.menu?.isOpen) {
                this._refreshSystemActivity();
                this._refreshMusic();
                this._refreshWorldClock();
                return GLib.SOURCE_CONTINUE;
            }
            return GLib.SOURCE_REMOVE;
        }));
    }

    _stopPoll() {
        if (this._pollTimer) {
            GLib.Source.remove(this._pollTimer);
            this._pollTimer = 0;
        }
    }

    _refreshAll() {
        this._refreshNotifications();
        this._refreshCalendar();
        this._refreshReminders();
        this._refreshSystemActivity();
        this._refreshWeather();
        this._refreshMusic();
        this._refreshBattery();
        this._refreshWorldClock();
    }

    _buildUI() {
        this._root = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-sidebar',
            x_expand: false,
            y_expand: true,
        });

        this._scroll = new St.ScrollView({
            style_class: 'mac-nc-scroll',
            hscrollbar_policy: St.PolicyType.NEVER,
            vscrollbar_policy: St.PolicyType.AUTOMATIC,
            x_expand: true,
            y_expand: true,
        });
        this._updateScrollMaxHeight();

        const content = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-content',
            x_expand: true,
        });

        // 1. Notification Stack
        this._notifSection = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-section mac-nc-notifications-section',
            x_expand: true,
        });

        const notifHeader = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-section-header mac-nc-notif-header',
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        });

        this._notifTitle = new St.Label({
            text: `${t('NOTIFICATIONS')} (0)`,
            style_class: 'mac-nc-section-title',
            x_expand: true,
        });
        notifHeader.add_child(this._notifTitle);

        this._expandBtn = new St.Button({
            label: t('EXPAND'),
            style_class: 'mac-nc-clear-btn',
            can_focus: true,
            visible: false,
        });
        this._expandBtn.connect('clicked', guard(() => {
            this._notifsExpanded = !this._notifsExpanded;
            this._expandBtn.label = this._notifsExpanded ? t('COLLAPSE') : t('EXPAND');
            this._refreshNotifications();
        }));
        notifHeader.add_child(this._expandBtn);

        this._clearBtn = new St.Button({
            label: t('CLEAR_ALL'),
            style_class: 'mac-nc-clear-btn',
            can_focus: true,
            visible: false,
        });
        this._clearBtn.connect('clicked', guard(() => this._clearAllNotifications()));
        notifHeader.add_child(this._clearBtn);

        this._notifSection.add_child(notifHeader);

        this._notifList = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-notif-list',
            x_expand: true,
        });
        this._notifSection.add_child(this._notifList);
        content.add_child(this._notifSection);

        // 2. Widgets Section
        const widgetsHeader = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-section-header',
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        });
        widgetsHeader.add_child(new St.Label({
            text: t('WIDGETS'),
            style_class: 'mac-nc-section-title',
            x_expand: true,
        }));
        content.add_child(widgetsHeader);

        const widgetsBox = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-widgets',
            x_expand: true,
        });

        this._cards = [];

        // Row 1: Calendar (Square) + Reminders (Square)
        const row1 = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-row',
            x_expand: true,
        });
        this._calendarCard = this._buildCalendarCard();
        this._remindersCard = this._buildRemindersCard();
        this._cards.push(this._calendarCard, this._remindersCard);
        row1.add_child(this._calendarCard);
        row1.add_child(this._remindersCard);
        widgetsBox.add_child(row1);

        // Row 2: Performance (Wide Card)
        this._systemCard = this._buildSystemActivityCard();
        this._cards.push(this._systemCard);
        widgetsBox.add_child(this._systemCard);

        // Row 3: Weather (Wide Card)
        this._weatherCard = this._buildWeatherCard();
        this._cards.push(this._weatherCard);
        widgetsBox.add_child(this._weatherCard);

        // Row 4: Music (Square) + Real Battery (Square)
        const row4 = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-row',
            x_expand: true,
        });
        this._musicCard = this._buildMusicCard();
        this._batteryCard = this._buildBatteryCard();
        this._cards.push(this._musicCard, this._batteryCard);
        row4.add_child(this._musicCard);
        row4.add_child(this._batteryCard);
        widgetsBox.add_child(row4);

        // Row 5: World Clock (Wide Card)
        this._worldClockCard = this._buildWorldClockCard();
        this._cards.push(this._worldClockCard);
        widgetsBox.add_child(this._worldClockCard);

        content.add_child(widgetsBox);
        this._scroll.set_child(content);
        this._root.add_child(this._scroll);
    }

    /* 1. Notifications UI */
    _refreshNotifications() {
        if (!this._notifList) return;
        this._notifList.destroy_all_children();

        const notifModel = this._dateMenu?._messageList?._notificationSection?._model
            || Main.messageTray?._notificationQueue;
        let notifs = [];

        try {
            if (notifModel?.get_n_items) {
                const count = notifModel.get_n_items();
                for (let i = 0; i < count; i++) {
                    const item = notifModel.get_item(i);
                    if (item) notifs.push(item);
                }
            } else if (Array.isArray(notifModel)) {
                notifs = notifModel;
            }
        } catch (e) {}

        const total = notifs.length;
        if (this._notifTitle) {
            this._notifTitle.text = `${t('NOTIFICATIONS')} (${total})`;
        }

        if (total === 0) {
            if (this._clearBtn) this._clearBtn.visible = false;
            if (this._expandBtn) this._expandBtn.visible = false;

            const emptyCard = new St.BoxLayout({
                orientation: Clutter.Orientation.HORIZONTAL,
                style_class: 'mac-nc-card quick-toggle mac-nc-empty-card',
                x_expand: true,
                y_align: Clutter.ActorAlign.CENTER,
            });
            emptyCard.add_child(new St.Icon({
                icon_name: 'notifications-disabled-symbolic',
                style_class: 'mac-nc-empty-icon',
            }));
            emptyCard.add_child(new St.Label({
                text: t('NO_NOTIFICATIONS'),
                style_class: 'mac-nc-empty-notif',
                y_align: Clutter.ActorAlign.CENTER,
            }));
            this._notifList.add_child(emptyCard);
            return;
        }

        if (this._clearBtn) this._clearBtn.visible = true;
        if (this._expandBtn) this._expandBtn.visible = (total > 2);

        // Hard cap on live cards. A very full notification list would otherwise
        // allocate - and re-allocate on every items-changed - hundreds of actors,
        // which is what saturates the shell when the centre is opened while it is
        // already loaded. Collapsed shows two; expanded shows at most MAX_CARDS.
        const BASE_LIMIT = 2;
        const MAX_CARDS = 40;
        const limit = this._notifsExpanded ? MAX_CARDS : BASE_LIMIT;
        const displayNotifs = notifs.slice(0, limit);

        for (const notif of displayNotifs) {
            const card = this._buildNotificationCard(notif);
            this._notifList.add_child(card);
        }

        if (total > displayNotifs.length) {
            const remaining = total - displayNotifs.length;
            const moreBtn = new St.Button({
                style_class: 'mac-nc-more-notifs-btn',
                can_focus: true,
                x_expand: true,
            });
            moreBtn.set_child(new St.Label({
                text: t('MORE_NOTIFS', remaining),
                style_class: 'mac-nc-more-label',
            }));
            moreBtn.connect('clicked', guard(() => {
                if (!this._notifsExpanded) {
                    this._notifsExpanded = true;
                    if (this._expandBtn) this._expandBtn.label = t('COLLAPSE');
                }
                this._refreshNotifications();
            }));
            this._notifList.add_child(moreBtn);
        }
    }

    _buildNotificationCard(notif) {
        const card = new St.Button({
            style_class: 'mac-nc-card quick-toggle mac-nc-notif-card',
            can_focus: true,
            reactive: true,
            clip_to_allocation: true,
            x_expand: true,
        });

        const box = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-card-inner',
            clip_to_allocation: true,
            x_expand: true,
        });

        const topRow = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-notif-top',
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        });

        let appIcon = notif.gicon || notif.icon;
        let iconWidget;
        if (appIcon) {
            iconWidget = new St.Icon({
                gicon: appIcon,
                style_class: 'mac-nc-notif-icon',
            });
        } else {
            iconWidget = new St.Icon({
                icon_name: 'dialog-information-symbolic',
                style_class: 'mac-nc-notif-icon',
            });
        }
        topRow.add_child(iconWidget);

        const appName = notif.source?.title || notif.source?.name || notif.title || t('SYSTEM');
        topRow.add_child(new St.Label({
            text: appName,
            style_class: 'mac-nc-notif-app',
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        }));

        topRow.add_child(new St.Label({
            text: t('NOW'),
            style_class: 'mac-nc-notif-time',
            y_align: Clutter.ActorAlign.CENTER,
        }));

        const closeBtn = new St.Button({
            style_class: 'mac-nc-notif-close',
            can_focus: true,
            child: new St.Icon({
                icon_name: 'window-close-symbolic',
                style_class: 'mac-nc-mini-icon',
            }),
        });
        closeBtn.connect('clicked', guard(() => {
            try {
                if (typeof notif.destroy === 'function') notif.destroy();
                else if (typeof notif.close === 'function') notif.close();
            } catch (e) {}
            this._refreshNotifications();
        }));
        topRow.add_child(closeBtn);
        box.add_child(topRow);

        // notif.body is a string; notif.banner is a Clutter actor and must never
        // be handed to St.Label as text (it would be coerced into garbage and,
        // in the worst case, throw inside a shell callback).
        const titleText = typeof notif.title === 'string' ? notif.title : '';
        const bodyText = typeof notif.body === 'string'
            ? notif.body
            : (typeof notif.banner === 'string' ? notif.banner : '');

        if (titleText) {
            box.add_child(new St.Label({
                text: titleText,
                style_class: 'mac-nc-notif-title',
            }));
        }

        if (bodyText) {
            const bodyLabel = new St.Label({
                text: bodyText,
                style_class: 'mac-nc-notif-body',
            });
            bodyLabel.clutter_text.line_wrap = true;
            bodyLabel.clutter_text.ellipsize = Pango.EllipsizeMode.END;
            box.add_child(bodyLabel);
        }

        card.set_child(box);
        card.connect('clicked', guard(() => {
            try {
                if (typeof notif.activate === 'function') notif.activate();
            } catch (e) {}
            this._dateMenu?.menu?.close();
        }));

        return card;
    }

    _clearAllNotifications() {
        try {
            const notifSection = this._dateMenu?._messageList?._notificationSection;
            if (notifSection?._model) {
                notifSection._model.clear();
            } else if (notifSection?.clear) {
                notifSection.clear();
            }
        } catch (e) {}
        this._refreshNotifications();
    }

    /* 2. Calendar Widget */
    _initCalendarServer() {
        Gio.DBus.session.call(
            'org.gnome.Shell.CalendarServer',
            '/org/gnome/Shell/CalendarServer',
            'org.freedesktop.DBus.Properties',
            'GetAll',
            new GLib.Variant('(s)', ['org.gnome.Shell.CalendarServer']),
            null,
            NONE,
            -1,
            null,
            () => {}
        );

        Gio.DBusProxy.new(
            Gio.DBus.session,
            Gio.DBusProxyFlags.NONE,
            null,
            'org.gnome.Shell.CalendarServer',
            '/org/gnome/Shell/CalendarServer',
            'org.gnome.Shell.CalendarServer',
            null,
            guard((initable, result) => {
                try {
                    if (this._destroyed) return;
                    this._calendarProxy = Gio.DBusProxy.new_finish(result);
                    this._loadCalendarEvents();
                    this._calendarProxy.connect('g-signal', guard((proxy, sender, signal, params) => {
                        if (this._destroyed || !this._root?.get_stage()) return;
                        if (signal === 'EventsAddedOrUpdated' || signal === 'EventsRemoved') {
                            this._loadCalendarEvents();
                        }
                    }));
                } catch (e) {}
            })
        );
    }

    _loadCalendarEvents() {
        if (!this._calendarProxy || this._destroyed) return;
        const now = GLib.DateTime.new_now_local();
        const startOfDay = GLib.DateTime.new_local(now.get_year(), now.get_month(), now.get_day_of_month(), 0, 0, 0);
        const endOfDay = startOfDay.add_days(1);

        try {
            this._calendarProxy.SetTimeRangeRemote(
                startOfDay.to_unix(),
                endOfDay.to_unix(),
                true,
                guard((result, exc) => {
                    if (exc || this._destroyed || !this._root?.get_stage()) return;
                    this._refreshCalendar();
                })
            );
        } catch (e) {}
    }

    _buildCalendarCard() {
        const card = new St.Button({
            style_class: 'mac-nc-card quick-toggle mac-nc-square mac-nc-calendar-card',
            can_focus: true,
            reactive: true,
            clip_to_allocation: true,
            x_expand: true,
        });

        const box = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-card-inner',
            clip_to_allocation: true,
            x_expand: true,
            y_expand: true,
        });

        this._calMonthHeader = new St.Label({
            text: 'OCTUBRE 2026',
            style_class: 'mac-nc-cal-month-title',
        });
        box.add_child(this._calMonthHeader);

        // Day of week labels (L M X J V S D)
        const headerRow = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-cal-row',
            x_expand: true,
            x_align: Clutter.ActorAlign.CENTER,
        });
        const dayHeaders = t('CAL_DAYS');
        for (const h of dayHeaders) {
            headerRow.add_child(new St.Label({
                text: h,
                style_class: 'mac-nc-cal-header-day',
            }));
        }
        box.add_child(headerRow);

        // Month Grid Container
        this._calGrid = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-cal-grid',
            x_expand: true,
            x_align: Clutter.ActorAlign.CENTER,
        });
        box.add_child(this._calGrid);

        this._calEventText = new St.Label({
            text: t('NO_EVENTS'),
            style_class: 'mac-nc-cal-event',
        });
        this._calEventText.clutter_text.ellipsize = Pango.EllipsizeMode.END;
        box.add_child(this._calEventText);

        card.set_child(box);
        card.connect('clicked', () => {
            this._dateMenu?.menu?.close();
            launch('gnome-calendar || xdg-open calendar:');
        });
        return card;
    }

    _refreshCalendar() {
        const now = GLib.DateTime.new_now_local();
        const year = now.get_year();
        const month = now.get_month(); // 1-12
        const todayDay = now.get_day_of_month();

        if (this._calMonthHeader) {
            const monthName = now.format('%B %Y') || 'CALENDARIO';
            this._calMonthHeader.text = monthName.toUpperCase();
        }

        if (this._calGrid) {
            this._calGrid.destroy_all_children();

            const firstDay = new Date(year, month - 1, 1).getDay(); // 0 is Sun
            const startCol = (firstDay + 6) % 7; // Mon is 0
            const daysInMonth = new Date(year, month, 0).getDate();
            const daysInPrevMonth = new Date(year, month - 1, 0).getDate();

            let rowBox = new St.BoxLayout({
                orientation: Clutter.Orientation.HORIZONTAL,
                style_class: 'mac-nc-cal-row',
                x_expand: true,
                x_align: Clutter.ActorAlign.CENTER,
            });

            // Prev month trailing
            for (let i = startCol - 1; i >= 0; i--) {
                const cell = new St.Bin({
                    style_class: 'mac-nc-cal-cell',
                    x_align: Clutter.ActorAlign.CENTER,
                    y_align: Clutter.ActorAlign.CENTER,
                });
                cell.set_child(new St.Label({
                    text: String(daysInPrevMonth - i),
                    style_class: 'mac-nc-cal-day-label mac-nc-cal-other-month',
                }));
                rowBox.add_child(cell);
            }

            // Current month days
            for (let d = 1; d <= daysInMonth; d++) {
                const isToday = (d === todayDay);
                const cell = new St.Bin({
                    style_class: 'mac-nc-cal-cell',
                    x_align: Clutter.ActorAlign.CENTER,
                    y_align: Clutter.ActorAlign.CENTER,
                });

                if (isToday) {
                    const bubble = new St.Bin({
                        style_class: 'mac-nc-cal-today-bubble',
                        x_align: Clutter.ActorAlign.CENTER,
                        y_align: Clutter.ActorAlign.CENTER,
                    });
                    bubble.set_child(new St.Label({
                        text: String(d),
                        style_class: 'mac-nc-cal-day-label mac-nc-cal-today-text',
                    }));
                    cell.set_child(bubble);
                } else {
                    cell.set_child(new St.Label({
                        text: String(d),
                        style_class: 'mac-nc-cal-day-label',
                    }));
                }

                rowBox.add_child(cell);

                if (rowBox.get_n_children() === 7) {
                    this._calGrid.add_child(rowBox);
                    rowBox = new St.BoxLayout({
                        orientation: Clutter.Orientation.HORIZONTAL,
                        style_class: 'mac-nc-cal-row',
                        x_expand: true,
                        x_align: Clutter.ActorAlign.CENTER,
                    });
                }
            }

            // Next month leading
            let nextD = 1;
            while (rowBox.get_n_children() > 0 && rowBox.get_n_children() < 7) {
                const cell = new St.Bin({
                    style_class: 'mac-nc-cal-cell',
                    x_align: Clutter.ActorAlign.CENTER,
                    y_align: Clutter.ActorAlign.CENTER,
                });
                cell.set_child(new St.Label({
                    text: String(nextD++),
                    style_class: 'mac-nc-cal-day-label mac-nc-cal-other-month',
                }));
                rowBox.add_child(cell);
            }
            if (rowBox.get_n_children() > 0) {
                this._calGrid.add_child(rowBox);
            }
        }

        if (this._calEventText) {
            if (this._calendarEvents && this._calendarEvents.length > 0) {
                const first = this._calendarEvents[0];
                const summary = first[1] || t('SCHEDULED');
                this._calEventText.text = `• ${summary}`;
            } else {
                this._calEventText.text = t('NO_EVENTS');
            }
        }
    }

    /* 3. Tasks / Reminders Widget */
    _buildRemindersCard() {
        const card = new St.Button({
            style_class: 'mac-nc-card quick-toggle mac-nc-square mac-nc-reminders-card',
            can_focus: true,
            reactive: true,
            clip_to_allocation: true,
            x_expand: true,
        });

        const box = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-card-inner',
            clip_to_allocation: true,
            x_expand: true,
            y_expand: true,
        });

        const header = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-card-header',
            y_align: Clutter.ActorAlign.CENTER,
        });

        const iconBg = new St.Bin({
            style_class: 'mac-nc-reminders-icon-bg',
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });
        iconBg.set_child(new St.Icon({
            icon_name: 'view-list-symbolic',
            style_class: 'mac-nc-mini-white',
        }));
        header.add_child(iconBg);

        header.add_child(new St.Label({
            text: t('TASKS'),
            style_class: 'mac-nc-card-title',
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        }));
        box.add_child(header);

        this._remindersList = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-reminders-list',
            x_expand: true,
            y_expand: true,
        });
        box.add_child(this._remindersList);

        card.set_child(box);
        card.connect('clicked', () => {
            this._dateMenu?.menu?.close();
            launch('gnome-todo || endeavour || xdg-open todo:');
        });
        return card;
    }

    _refreshReminders() {
        if (!this._remindersList) return;
        this._remindersList.destroy_all_children();

        const empty = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-reminders-item',
            y_align: Clutter.ActorAlign.CENTER,
            y_expand: true,
        });
        empty.add_child(new St.Icon({
            icon_name: 'object-select-symbolic',
            style_class: 'mac-nc-check-icon',
        }));
        empty.add_child(new St.Label({
            text: t('ALL_DONE'),
            style_class: 'mac-nc-reminders-text-done',
            y_align: Clutter.ActorAlign.CENTER,
        }));
        this._remindersList.add_child(empty);
    }

    /* 4. Performance Widget */
    _buildSystemActivityCard() {
        const card = new St.Button({
            style_class: 'mac-nc-card quick-toggle mac-nc-wide mac-nc-perf-card',
            can_focus: true,
            reactive: true,
            clip_to_allocation: true,
            x_expand: true,
        });

        const box = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-card-inner',
            clip_to_allocation: true,
            x_expand: true,
        });

        const header = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-card-header',
            y_align: Clutter.ActorAlign.CENTER,
        });

        const iconBg = new St.Bin({
            style_class: 'mac-nc-perf-icon-bg',
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });
        iconBg.set_child(new St.Icon({
            icon_name: 'utilities-system-monitor-symbolic',
            style_class: 'mac-nc-mini-white',
        }));
        header.add_child(iconBg);

        header.add_child(new St.Label({
            text: t('PERFORMANCE'),
            style_class: 'mac-nc-card-title',
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        }));

        this._perfStatusLabel = new St.Label({
            text: t('OPTIMAL'),
            style_class: 'mac-nc-perf-status',
            y_align: Clutter.ActorAlign.CENTER,
        });
        header.add_child(this._perfStatusLabel);
        box.add_child(header);

        // Stats Row (CPU & RAM)
        const statsRow = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-perf-row',
            x_expand: true,
        });

        // CPU
        const cpuBox = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-perf-col',
            x_expand: true,
        });
        const cpuTop = new St.BoxLayout({orientation: Clutter.Orientation.HORIZONTAL, x_expand: true});
        cpuTop.add_child(new St.Label({text: t('CPU'), style_class: 'mac-nc-stat-label', x_expand: true}));
        this._cpuValLabel = new St.Label({text: '0%', style_class: 'mac-nc-stat-val'});
        cpuTop.add_child(this._cpuValLabel);
        cpuBox.add_child(cpuTop);

        this._cpuBar = new St.DrawingArea({
            style_class: 'mac-nc-progress-bar',
            height: 4,
            x_expand: true,
        });
        this._cpuBar.percent = 0;
        this._cpuBar.connect('repaint', (area) => this._drawBar(area, '#30d158'));
        cpuBox.add_child(this._cpuBar);
        statsRow.add_child(cpuBox);

        // RAM
        const ramBox = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-perf-col',
            x_expand: true,
        });
        const ramTop = new St.BoxLayout({orientation: Clutter.Orientation.HORIZONTAL, x_expand: true});
        ramTop.add_child(new St.Label({text: t('MEMORY'), style_class: 'mac-nc-stat-label', x_expand: true}));
        this._ramValLabel = new St.Label({text: '0 GB', style_class: 'mac-nc-stat-val'});
        ramTop.add_child(this._ramValLabel);
        ramBox.add_child(ramTop);

        this._ramBar = new St.DrawingArea({
            style_class: 'mac-nc-progress-bar',
            height: 4,
            x_expand: true,
        });
        this._ramBar.percent = 0;
        this._ramBar.connect('repaint', (area) => this._drawBar(area, '#0a84ff'));
        ramBox.add_child(this._ramBar);
        statsRow.add_child(ramBox);

        box.add_child(statsRow);

        // Uptime Footer
        const footer = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-perf-footer',
            x_expand: true,
        });
        footer.add_child(new St.Label({text: `${t('UPTIME')}: `, style_class: 'mac-nc-uptime-label'}));
        this._uptimeLabel = new St.Label({text: '0h 0m', style_class: 'mac-nc-uptime-val', x_expand: true});
        footer.add_child(this._uptimeLabel);
        box.add_child(footer);

        card.set_child(box);
        card.connect('clicked', () => {
            this._dateMenu?.menu?.close();
            launch('gnome-system-monitor || htop');
        });
        return card;
    }

    _drawBar(area, colorHex) {
        // Painting before the actor has a valid (non-NaN, non-zero) allocation
        // feeds NaN into cairo/Clutter and trips
        // "clutter_actor_set_allocation_internal: assertion '!isnan (box->x1)'".
        // Repeated under memory pressure this can take the shell down, so bail
        // out instead of drawing garbage. Never let a paint error escape either:
        // an exception in a shell callback aborts the whole session.
        if (!area || this._destroyed || !this._root?.get_stage?.() || !area.get_stage?.()) return;
        let cr = null;
        try {
            const [w, h] = area.get_surface_size();
            if (!Number.isFinite(w) || !Number.isFinite(h) || w <= 0 || h <= 0)
                return;

            cr = area.get_context();
            if (!cr)
                return;

            const radius = Math.max(0, h / 2);

            cr.setSourceRGBA(1, 1, 1, 0.15);
            this._roundedRect(cr, 0, 0, w, h, radius);
            cr.fill();

            const pct = Math.max(0, Math.min(100, Number(area.percent) || 0));
            const fillW = Math.max(h, (w * pct) / 100);

            if (colorHex === '#30d158') cr.setSourceRGBA(0.18, 0.82, 0.35, 0.95);
            else if (colorHex === '#0a84ff') cr.setSourceRGBA(0.04, 0.52, 1.0, 0.95);
            else cr.setSourceRGBA(1, 0.62, 0.04, 0.95);

            this._roundedRect(cr, 0, 0, fillW, h, radius);
            cr.fill();
        } catch (e) {
            logError(e, 'macos-nc: _drawBar failed');
        } finally {
            try { if (cr) cr.$dispose(); } catch (e) {}
        }
    }

    _roundedRect(cr, x, y, w, h, r) {
        cr.newSubPath();
        cr.arc(x + r, y + r, r, Math.PI, 1.5 * Math.PI);
        cr.arc(x + w - r, y + r, r, 1.5 * Math.PI, 2 * Math.PI);
        cr.arc(x + w - r, y + h - r, r, 0, 0.5 * Math.PI);
        cr.arc(x + r, y + h - r, r, 0.5 * Math.PI, Math.PI);
        cr.closePath();
    }

    _refreshSystemActivity() {
        // CPU
        try {
            const [, bytes] = GLib.file_get_contents('/proc/stat');
            const lines = new TextDecoder().decode(bytes).split('\n');
            const cpuLine = lines.find(l => l.startsWith('cpu '));
            if (cpuLine) {
                const parts = cpuLine.trim().split(/\s+/).slice(1).map(Number);
                const idle = parts[3] + (parts[4] || 0);
                const total = parts.reduce((a, b) => a + b, 0);

                if (this._lastCpuStats) {
                    const idleDelta = idle - this._lastCpuStats.idle;
                    const totalDelta = total - this._lastCpuStats.total;
                    if (totalDelta > 0) {
                        const usage = Math.round(((totalDelta - idleDelta) / totalDelta) * 100);
                        if (this._cpuValLabel) this._cpuValLabel.text = `${usage}%`;
                        if (this._cpuBar) {
                            this._cpuBar.percent = usage;
                            this._cpuBar.queue_repaint();
                        }
                    }
                }
                this._lastCpuStats = { idle, total };
            }
        } catch (e) {}

        // Memory
        try {
            const [, bytes] = GLib.file_get_contents('/proc/meminfo');
            const str = new TextDecoder().decode(bytes);
            const totalMatch = str.match(/MemTotal:\s+(\d+)/);
            const availMatch = str.match(/MemAvailable:\s+(\d+)/);

            if (totalMatch && availMatch) {
                const totalKb = parseInt(totalMatch[1], 10);
                const availKb = parseInt(availMatch[1], 10);
                const usedKb = totalKb - availKb;
                const percent = Math.round((usedKb / totalKb) * 100);
                const usedGb = (usedKb / (1024 * 1024)).toFixed(1);
                const totalGb = (totalKb / (1024 * 1024)).toFixed(0);

                if (this._ramValLabel) this._ramValLabel.text = `${usedGb} / ${totalGb} GB`;
                if (this._ramBar) {
                    this._ramBar.percent = percent;
                    this._ramBar.queue_repaint();
                }
            }
        } catch (e) {}

        // Uptime
        try {
            const [, bytes] = GLib.file_get_contents('/proc/uptime');
            const upSec = parseFloat(new TextDecoder().decode(bytes).split(' ')[0]);
            const hours = Math.floor(upSec / 3600);
            const mins = Math.floor((upSec % 3600) / 60);
            if (this._uptimeLabel) this._uptimeLabel.text = `${hours}h ${mins}m`;
        } catch (e) {}
    }

    /* 5. Weather Widget (GNOME Weather live sync) */
    _initLocation() {
        this._locationName = 'La Coruña';
        this._lat = 43.3667;
        this._lon = -8.3833;

        try {
            const weatherSettings = new Gio.Settings({schema_id: 'org.gnome.Weather'});
            const locsVal = weatherSettings.get_value('locations');
            if (locsVal && locsVal.n_children() > 0) {
                const child = locsVal.get_child_value(0);
                const world = GWeather.Location.get_world();
                const loc = world.deserialize(child.get_variant());
                if (loc) {
                    this._locationName = loc.get_name() || 'La Coruña';
                    const [lat, lon] = loc.get_coords();
                    if (lat !== undefined && lon !== undefined) {
                        this._lat = lat;
                        this._lon = lon;
                    }
                }
            }
        } catch (e) {
            try {
                const shellWeather = new Gio.Settings({schema_id: 'org.gnome.shell.weather'});
                const locsVal = shellWeather.get_value('locations');
                if (locsVal && locsVal.n_children() > 0) {
                    const child = locsVal.get_child_value(0);
                    const world = GWeather.Location.get_world();
                    const loc = world.deserialize(child.get_variant());
                    if (loc) {
                        this._locationName = loc.get_name() || 'La Coruña';
                        const [lat, lon] = loc.get_coords();
                        if (lat !== undefined && lon !== undefined) {
                            this._lat = lat;
                            this._lon = lon;
                        }
                    }
                }
            } catch (err) {}
        }
    }

    _buildWeatherCard() {
        const card = new St.Button({
            style_class: 'mac-nc-card quick-toggle mac-nc-wide mac-nc-weather-card',
            can_focus: true,
            reactive: true,
            clip_to_allocation: true,
            x_expand: true,
        });

        const box = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-card-inner',
            clip_to_allocation: true,
            x_expand: true,
        });

        const top = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-weather-top',
            clip_to_allocation: true,
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        });

        const locCol = new St.BoxLayout({orientation: Clutter.Orientation.VERTICAL, x_expand: true, clip_to_allocation: true});
        this._weatherLocLabel = new St.Label({text: this._locationName, style_class: 'mac-nc-weather-city'});
        this._weatherDescLabel = new St.Label({text: t('CLOUDY'), style_class: 'mac-nc-weather-desc'});
        locCol.add_child(this._weatherLocLabel);
        locCol.add_child(this._weatherDescLabel);
        top.add_child(locCol);

        this._weatherIcon = new St.Icon({
            icon_name: 'weather-overcast-symbolic',
            style_class: 'mac-nc-weather-icon',
        });
        top.add_child(this._weatherIcon);

        this._weatherTempLabel = new St.Label({
            text: '--°',
            style_class: 'mac-nc-weather-temp',
            y_align: Clutter.ActorAlign.CENTER,
        });
        top.add_child(this._weatherTempLabel);
        box.add_child(top);

        // Hourly pills container
        this._weatherHourlyBox = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-weather-hourly',
            clip_to_allocation: true,
            x_expand: true,
        });
        box.add_child(this._weatherHourlyBox);

        card.set_child(box);
        card.connect('clicked', () => {
            this._dateMenu?.menu?.close();
            launch('gnome-weather || xdg-open https://weather.com');
        });
        return card;
    }

    _refreshWeather() {
        if (this._destroyed || !this._soupSession || !this._root?.get_stage?.()) return;

        const url = `https://api.open-meteo.com/v1/forecast?latitude=${this._lat.toFixed(4)}&longitude=${this._lon.toFixed(4)}&current=temperature_2m,weather_code&hourly=temperature_2m,weather_code&timezone=auto&forecast_days=1`;
        const msg = Soup.Message.new('GET', url);

        this._soupSession.send_and_read_async(msg, GLib.PRIORITY_DEFAULT, null, guard((session, res) => {
            if (this._destroyed || !this._root?.get_stage?.()) return;
            try {
                const bytes = session.send_and_read_finish(res);
                if (!bytes) return;
                const data = JSON.parse(new TextDecoder().decode(bytes.get_data()));
                this._weatherData = data;
                this._updateWeatherUI();
            } catch (e) {}
        }));
    }

    _getWeatherInfo(code) {
        if (code === 0) return { name: t('CLEAR_SKY'), icon: 'weather-clear-symbolic' };
        if (code === 1 || code === 2) return { name: t('FEW_CLOUDS'), icon: 'weather-few-clouds-symbolic' };
        if (code === 3) return { name: t('CLOUDY'), icon: 'weather-overcast-symbolic' };
        if (code >= 51 && code <= 67) return { name: t('RAIN'), icon: 'weather-showers-symbolic' };
        if (code >= 71 && code <= 77) return { name: t('SNOW'), icon: 'weather-snow-symbolic' };
        if (code >= 80 && code <= 82) return { name: t('RAIN'), icon: 'weather-showers-scattered-symbolic' };
        if (code >= 95) return { name: t('THUNDERSTORM'), icon: 'weather-storm-symbolic' };
        if (code === 45 || code === 48) return { name: t('FOG'), icon: 'weather-fog-symbolic' };
        return { name: t('CLEAR_SKY'), icon: 'weather-clear-symbolic' };
    }

    _updateWeatherUI() {
        if (this._destroyed || !this._root?.get_stage?.() || !this._weatherData || !this._weatherHourlyBox) return;

        if (this._weatherLocLabel) this._weatherLocLabel.text = this._locationName;

        const current = this._weatherData.current;
        if (current) {
            const temp = Math.round(current.temperature_2m);
            const info = this._getWeatherInfo(current.weather_code);

            if (this._weatherTempLabel) this._weatherTempLabel.text = `${temp}°`;
            if (this._weatherDescLabel) this._weatherDescLabel.text = info.name;
            if (this._weatherIcon) this._weatherIcon.icon_name = info.icon;
        }

        const hourly = this._weatherData.hourly;
        if (hourly && this._weatherHourlyBox) {
            this._weatherHourlyBox.destroy_all_children();

            const currentHour = new Date().getHours();
            for (let i = 0; i < 5; i++) {
                const hIdx = (currentHour + i * 2) % 24;
                const temp = Math.round(hourly.temperature_2m[hIdx] || 0);
                const code = hourly.weather_code[hIdx] || 0;
                const info = this._getWeatherInfo(code);

                const pill = new St.BoxLayout({
                    orientation: Clutter.Orientation.VERTICAL,
                    style_class: 'mac-nc-weather-pill',
                    clip_to_allocation: true,
                    x_expand: true,
                    x_align: Clutter.ActorAlign.CENTER,
                });

                pill.add_child(new St.Label({
                    text: i === 0 ? t('NOW') : `${hIdx}:00`,
                    style_class: 'mac-nc-pill-time',
                }));
                pill.add_child(new St.Icon({
                    icon_name: info.icon,
                    style_class: 'mac-nc-pill-icon',
                }));
                pill.add_child(new St.Label({
                    text: `${temp}°`,
                    style_class: 'mac-nc-pill-temp',
                }));
                this._weatherHourlyBox.add_child(pill);
            }
            this._weatherCard?.queue_relayout();
            this._scroll?.queue_relayout();
        }
    }

    /* 6. Music Widget (with real cover art) */
    _buildMusicCard() {
        const card = new St.Button({
            style_class: 'mac-nc-card quick-toggle mac-nc-square mac-nc-music-card',
            can_focus: true,
            reactive: true,
            clip_to_allocation: true,
            x_expand: true,
        });

        const box = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-card-inner',
            clip_to_allocation: true,
            x_expand: true,
            y_expand: true,
        });

        const top = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-card-header',
            clip_to_allocation: true,
            y_align: Clutter.ActorAlign.CENTER,
        });
        const iconBg = new St.Bin({
            style_class: 'mac-nc-music-icon-bg',
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });
        iconBg.set_child(new St.Icon({
            icon_name: 'audio-x-generic-symbolic',
            style_class: 'mac-nc-mini-white',
        }));
        top.add_child(iconBg);

        top.add_child(new St.Label({
            text: t('MUSIC'),
            style_class: 'mac-nc-card-title',
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        }));
        box.add_child(top);

        // Center Album Art & Info
        const midRow = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-music-mid',
            clip_to_allocation: true,
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        });

        this._musicArtBin = new St.Bin({
            style_class: 'mac-nc-music-art-bin',
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._musicArtIcon = new St.Icon({
            icon_name: 'emblem-music-symbolic',
            style_class: 'mac-nc-music-fallback-art',
        });
        this._musicArtBin.set_child(this._musicArtIcon);
        midRow.add_child(this._musicArtBin);

        const infoCol = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-music-info',
            clip_to_allocation: true,
            x_expand: true,
        });
        this._musicTitle = new St.Label({
            text: t('NO_MEDIA'),
            style_class: 'mac-nc-music-title',
        });
        this._musicTitle.clutter_text.ellipsize = Pango.EllipsizeMode.END;
        infoCol.add_child(this._musicTitle);

        this._musicArtist = new St.Label({
            text: 'Pulsar OS',
            style_class: 'mac-nc-music-artist',
        });
        this._musicArtist.clutter_text.ellipsize = Pango.EllipsizeMode.END;
        infoCol.add_child(this._musicArtist);
        midRow.add_child(infoCol);
        box.add_child(midRow);

        // Media controls row
        const controls = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-music-controls',
            clip_to_allocation: true,
            x_expand: true,
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });

        const prevBtn = new St.Button({
            style_class: 'mac-nc-media-btn',
            can_focus: true,
            child: new St.Icon({icon_name: 'media-skip-backward-symbolic', style_class: 'mac-nc-media-icon'}),
        });
        prevBtn.connect('clicked', () => this._mprisCmd('Previous'));
        controls.add_child(prevBtn);

        this._playPauseBtn = new St.Button({
            style_class: 'mac-nc-media-btn mac-nc-play-btn',
            can_focus: true,
            child: new St.Icon({icon_name: 'media-playback-start-symbolic', style_class: 'mac-nc-play-icon'}),
        });
        this._playPauseBtn.connect('clicked', () => this._mprisCmd('PlayPause'));
        controls.add_child(this._playPauseBtn);

        const nextBtn = new St.Button({
            style_class: 'mac-nc-media-btn',
            can_focus: true,
            child: new St.Icon({icon_name: 'media-skip-forward-symbolic', style_class: 'mac-nc-media-icon'}),
        });
        nextBtn.connect('clicked', () => this._mprisCmd('Next'));
        controls.add_child(nextBtn);

        box.add_child(controls);

        card.set_child(box);
        return card;
    }

    _mprisCmd(cmd) {
        if (!this._activeMprisBus) return;
        try {
            Gio.DBus.session.call(
                this._activeMprisBus,
                '/org/mpris/MediaPlayer2',
                'org.mpris.MediaPlayer2.Player',
                cmd,
                null,
                null,
                NONE,
                -1,
                null,
                guard(() => this._refreshMusic())
            );
        } catch (e) {}
    }

    _refreshMusic() {
        Gio.DBus.session.call(
            'org.freedesktop.DBus',
            '/org/freedesktop/DBus',
            'org.freedesktop.DBus',
            'ListNames',
            null,
            new GLib.VariantType('(as)'),
            NONE,
            -1,
            null,
            guard((c, res) => {
                try {
                    const [names] = c.call_finish(res).deepUnpack();
                    const players = names.filter(n => n.startsWith('org.mpris.MediaPlayer2.'));
                    if (players.length > 0) {
                        this._activeMprisBus = players[0];
                        this._queryMpris(players[0]);
                    } else {
                        this._activeMprisBus = null;
                        this._setNoMedia();
                    }
                } catch (e) {
                    this._setNoMedia();
                }
            })
        );
    }

    _queryMpris(busName) {
        Gio.DBus.session.call(
            busName,
            '/org/mpris/MediaPlayer2',
            'org.freedesktop.DBus.Properties',
            'GetAll',
            new GLib.Variant('(s)', ['org.mpris.MediaPlayer2.Player']),
            new GLib.VariantType('(a{sv})'),
            NONE,
            -1,
            null,
            guard((c, res) => {
                try {
                    const [props] = c.call_finish(res).deepUnpack();
                    const status = props['PlaybackStatus'] ? props['PlaybackStatus'].unpack() : 'Stopped';
                    const meta = props['Metadata'] ? props['Metadata'].unpack() : {};

                    let title = t('NO_MEDIA');
                    let artist = 'Pulsar OS';
                    let artUrl = null;

                    if (meta['xesam:title']) title = meta['xesam:title'].unpack();
                    if (meta['xesam:artist']) {
                        const a = meta['xesam:artist'].unpack();
                        artist = Array.isArray(a) ? a.join(', ') : String(a);
                    }
                    if (meta['mpris:artUrl']) {
                        artUrl = meta['mpris:artUrl'].unpack();
                    }

                    if (this._musicTitle) this._musicTitle.text = title;
                    if (this._musicArtist) this._musicArtist.text = artist;

                    if (this._playPauseBtn) {
                        const isPlaying = (status === 'Playing');
                        const iconName = isPlaying ? 'media-playback-pause-symbolic' : 'media-playback-start-symbolic';
                        this._playPauseBtn.set_child(new St.Icon({
                            icon_name: iconName,
                            style_class: 'mac-nc-play-icon',
                        }));
                    }

                    // Album Art
                    if (artUrl && artUrl !== this._currentArtUrl) {
                        this._currentArtUrl = artUrl;
                        this._loadCoverArt(artUrl);
                    } else if (!artUrl) {
                        this._currentArtUrl = null;
                        if (this._musicArtBin) {
                            this._musicArtBin.set_child(new St.Icon({
                                icon_name: 'emblem-music-symbolic',
                                style_class: 'mac-nc-music-fallback-art',
                            }));
                        }
                    }
                } catch (e) {
                    this._setNoMedia();
                }
            })
        );
    }

    _loadCoverArt(url) {
        if (!this._musicArtBin) return;
        try {
            let file;
            if (url.startsWith('file://')) {
                file = Gio.File.new_for_uri(url);
            } else if (url.startsWith('/')) {
                file = Gio.File.new_for_path(url);
            }
            if (file && file.query_exists(null)) {
                const gicon = new Gio.FileIcon({file});
                this._musicArtBin.set_child(new St.Icon({
                    gicon: gicon,
                    style_class: 'mac-nc-music-art-image',
                }));
            }
        } catch (e) {}
    }

    _setNoMedia() {
        // Widgets are destroyed and rebuilt whenever the panel is rebuilt; a
        // stale reference here must never throw, because this runs from an
        // async callback where an exception is fatal to the whole shell.
        try {
            if (this._musicTitle) this._musicTitle.text = t('NO_MEDIA');
        } catch (e) {}
        try {
            if (this._musicArtist) this._musicArtist.text = 'Pulsar OS';
        } catch (e) {}
        try {
            if (this._playPauseBtn) {
                this._playPauseBtn.set_child(new St.Icon({
                    icon_name: 'media-playback-start-symbolic',
                    style_class: 'mac-nc-play-icon',
                }));
            }
        } catch (e) {}
        try {
            if (this._musicArtBin) {
                this._musicArtBin.set_child(new St.Icon({
                    icon_name: 'emblem-music-symbolic',
                    style_class: 'mac-nc-music-fallback-art',
                }));
            }
        } catch (e) {}
    }

    /* 7. Battery Widget (Enumerates all connected UPower devices) */
    _buildBatteryCard() {
        const card = new St.Button({
            style_class: 'mac-nc-card quick-toggle mac-nc-square mac-nc-battery-card',
            can_focus: true,
            reactive: true,
            clip_to_allocation: true,
            x_expand: true,
        });

        const box = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-card-inner',
            clip_to_allocation: true,
            x_expand: true,
            y_expand: true,
        });

        const header = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-card-header',
            clip_to_allocation: true,
            y_align: Clutter.ActorAlign.CENTER,
        });

        const iconBg = new St.Bin({
            style_class: 'mac-nc-battery-icon-bg',
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });
        iconBg.set_child(new St.Icon({
            icon_name: 'battery-level-100-charged-symbolic',
            style_class: 'mac-nc-mini-white',
        }));
        header.add_child(iconBg);

        header.add_child(new St.Label({
            text: t('BATTERY'),
            style_class: 'mac-nc-card-title',
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        }));
        box.add_child(header);

        this._batteryDeviceList = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-battery-list',
            clip_to_allocation: true,
            x_expand: true,
            y_expand: true,
        });
        box.add_child(this._batteryDeviceList);

        card.set_child(box);
        card.connect('clicked', () => {
            this._dateMenu?.menu?.close();
            launch('gnome-control-center power');
        });
        return card;
    }

    _refreshBattery() {
        if (!this._batteryDeviceList) return;

        Gio.DBus.system.call(
            'org.freedesktop.UPower',
            '/org/freedesktop/UPower',
            'org.freedesktop.UPower',
            'EnumerateDevices',
            null,
            new GLib.VariantType('(ao)'),
            NONE,
            -1,
            null,
            guard((c, res) => {
                try {
                    const [devices] = c.call_finish(res).deepUnpack();
                    this._queryBatteryDevices(devices);
                } catch (e) {
                    this._renderFallbackBattery();
                }
            })
        );
    }

    _queryBatteryDevices(devicePaths) {
        let queried = 0;
        const validDevices = [];

        if (!devicePaths || devicePaths.length === 0) {
            this._renderFallbackBattery();
            return;
        }

        for (const path of devicePaths) {
            Gio.DBus.system.call(
                'org.freedesktop.UPower',
                path,
                'org.freedesktop.DBus.Properties',
                'GetAll',
                new GLib.Variant('(s)', ['org.freedesktop.UPower.Device']),
                new GLib.VariantType('(a{sv})'),
                NONE,
                -1,
                null,
                guard((c, res) => {
                    queried++;
                    try {
                        const [props] = c.call_finish(res).deepUnpack();
                        const type = props['Type'] ? props['Type'].unpack() : 0;
                        const percentage = props['Percentage'] ? Math.round(props['Percentage'].unpack()) : 0;
                        const state = props['State'] ? props['State'].unpack() : 0;
                        const model = props['Model'] ? props['Model'].unpack() : '';
                        const isPresent = props['IsPresent'] ? props['IsPresent'].unpack() : true;

                        // Type 2 = Battery, 3 = UPS, 4 = Monitor, 5 = Mouse, 6 = Keyboard, 7 = PDA, 8 = Phone, 11 = Headphones
                        if (isPresent && type > 1) {
                            validDevices.push({ type, percentage, state, model });
                        }
                    } catch (e) {}

                    if (queried === devicePaths.length) {
                        this._renderBatteryDevices(validDevices);
                    }
                })
            );
        }
    }

    _renderBatteryDevices(devices) {
        if (!this._batteryDeviceList) return;
        this._batteryDeviceList.destroy_all_children();

        if (devices.length === 0) {
            this._renderFallbackBattery();
            return;
        }

        for (const dev of devices.slice(0, 3)) {
            const row = new St.BoxLayout({
                orientation: Clutter.Orientation.HORIZONTAL,
                style_class: 'mac-nc-battery-row',
                clip_to_allocation: true,
                x_expand: true,
                y_align: Clutter.ActorAlign.CENTER,
            });

            let iconName = 'battery-symbolic';
            let devName = t('DEVICE');

            if (dev.type === 2) {
                devName = t('BATTERY');
                iconName = dev.state === 1 ? 'battery-charging-symbolic' : 'battery-level-80-symbolic';
            } else if (dev.type === 5) {
                devName = dev.model || t('MOUSE');
                iconName = 'input-mouse-symbolic';
            } else if (dev.type === 6) {
                devName = dev.model || t('KEYBOARD');
                iconName = 'input-keyboard-symbolic';
            } else if (dev.type === 11 || dev.type === 8) {
                devName = dev.model || t('HEADPHONES');
                iconName = 'audio-headphones-symbolic';
            }

            row.add_child(new St.Icon({
                icon_name: iconName,
                style_class: 'mac-nc-battery-item-icon',
            }));

            const nameLabel = new St.Label({
                text: devName,
                style_class: 'mac-nc-battery-item-name',
                x_expand: true,
                y_align: Clutter.ActorAlign.CENTER,
            });
            nameLabel.clutter_text.ellipsize = Pango.EllipsizeMode.END;
            row.add_child(nameLabel);

            const pctLabel = new St.Label({
                text: `${dev.percentage}%`,
                style_class: 'mac-nc-battery-item-pct',
                y_align: Clutter.ActorAlign.CENTER,
            });
            row.add_child(pctLabel);

            this._batteryDeviceList.add_child(row);
        }
    }

    _renderFallbackBattery() {
        if (!this._batteryDeviceList) return;
        this._batteryDeviceList.destroy_all_children();

        const row = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-battery-row',
            clip_to_allocation: true,
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        });
        row.add_child(new St.Icon({
            icon_name: 'battery-level-100-charged-symbolic',
            style_class: 'mac-nc-battery-item-icon',
        }));
        const nameLabel = new St.Label({
            text: t('BATTERY'),
            style_class: 'mac-nc-battery-item-name',
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        });
        nameLabel.clutter_text.ellipsize = Pango.EllipsizeMode.END;
        row.add_child(nameLabel);

        const pctLabel = new St.Label({
            text: '100%',
            style_class: 'mac-nc-battery-item-pct',
            y_align: Clutter.ActorAlign.CENTER,
        });
        row.add_child(pctLabel);
        this._batteryDeviceList.add_child(row);
    }

    /* 8. World Clock Widget */
    _buildWorldClockCard() {
        const card = new St.Button({
            style_class: 'mac-nc-card quick-toggle mac-nc-wide mac-nc-world-card',
            can_focus: true,
            reactive: true,
            clip_to_allocation: true,
            x_expand: true,
        });

        const box = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-card-inner',
            clip_to_allocation: true,
            x_expand: true,
        });

        const header = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-card-header',
            clip_to_allocation: true,
            y_align: Clutter.ActorAlign.CENTER,
        });

        const iconBg = new St.Bin({
            style_class: 'mac-nc-world-icon-bg',
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });
        iconBg.set_child(new St.Icon({
            icon_name: 'preferences-system-time-symbolic',
            style_class: 'mac-nc-mini-white',
        }));
        header.add_child(iconBg);

        header.add_child(new St.Label({
            text: t('WORLD_CLOCK'),
            style_class: 'mac-nc-card-title',
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        }));
        box.add_child(header);

        this._clockGrid = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-world-grid',
            clip_to_allocation: true,
            x_expand: true,
        });

        this._cityClocks = [
            { name: 'Cupertino', tz: 'America/Los_Angeles', offset: -9 },
            { name: 'Tokio', tz: 'Asia/Tokyo', offset: +7 },
            { name: 'Sídney', tz: 'Australia/Sydney', offset: +9 },
            { name: 'París', tz: 'Europe/Paris', offset: 0 },
        ];

        this._cityWidgets = [];
        for (const city of this._cityClocks) {
            const col = new St.BoxLayout({
                orientation: Clutter.Orientation.VERTICAL,
                style_class: 'mac-nc-world-col',
                x_expand: true,
                x_align: Clutter.ActorAlign.CENTER,
            });

            col.add_child(new St.Label({
                text: city.name,
                style_class: 'mac-nc-city-name',
            }));

            const timeLabel = new St.Label({
                text: '--:--',
                style_class: 'mac-nc-city-time',
            });
            col.add_child(timeLabel);

            let diffText = t('SAME_TIME');
            if (city.offset > 0) diffText = t('AHEAD', city.offset);
            else if (city.offset < 0) diffText = t('BEHIND', Math.abs(city.offset));

            col.add_child(new St.Label({
                text: diffText,
                style_class: 'mac-nc-city-diff',
            }));

            this._clockGrid.add_child(col);
            this._cityWidgets.push({ city, timeLabel });
        }

        box.add_child(this._clockGrid);

        card.set_child(box);
        card.connect('clicked', () => {
            this._dateMenu?.menu?.close();
            launch('gnome-clocks');
        });
        return card;
    }

    _refreshWorldClock() {
        if (!this._cityWidgets || this._destroyed || !this._root?.get_stage?.()) return;

        const now = GLib.DateTime.new_now_local();

        for (const { city, timeLabel } of this._cityWidgets) {
            let cityTime = now;
            try {
                const tz = GLib.TimeZone.new_identifier(city.tz);
                if (tz) cityTime = GLib.DateTime.new_now(tz);
            } catch (e) {}

            const timeStr = cityTime.format('%H:%M') || '12:00';
            if (timeLabel) timeLabel.text = timeStr;
        }
    }
}
