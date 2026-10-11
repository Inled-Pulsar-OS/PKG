/* macOS Notification Center GNOME Shell extension for Pulsar OS
 * High-performance Liquid Glass Notification Center & Widgets drawer for GNOME Shell 48 & 51.
 * Integrated with real Month Calendar Grid, GNOME Weather location,
 * Liquid Glass shader integration, MPRIS album art, UPower connected devices, and English/Spanish i18n support.
 */
import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Pango from 'gi://Pango';
import St from 'gi://St';
import Soup from 'gi://Soup?version=3.0';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const NONE = Gio.DBusCallFlags.NONE;
const MPRIS_PREFIX = 'org.mpris.MediaPlayer2.';
const MPRIS_PATH = '/org/mpris/MediaPlayer2';
const PLAYER_IFACE = 'org.mpris.MediaPlayer2.Player';
const LG_UUID = 'liquid-glass@thinkingcoding1231.gmail.com';

/* i18n strings dictionary */
const STRINGS = {
    es: {
        NOTIFICATIONS: 'NOTIFICACIONES',
        CLEAR_ALL: 'Borrar todo',
        EXPAND: 'Desplegar',
        COLLAPSE: 'Contraer',
        NO_NOTIFICATIONS: 'Sin notificaciones pendientes',
        MORE_NOTIFS: '+%d notificaciones más... (Toca para desplegar)',
        WIDGETS: 'WIDGETS',
        TODAY: 'HOY',
        NO_EVENTS: 'Sin eventos hoy',
        SCHEDULED: 'Evento programado',
        TASKS: 'Tareas',
        ALL_DONE: 'Todo al día hoy',
        NO_TASKS: 'Sin tareas pendientes',
        PERFORMANCE: 'Rendimiento Pulsar',
        OPTIMAL: 'SISTEMA ÓPTIMO',
        CPU: 'CPU',
        MEMORY: 'MEMORIA',
        UPTIME: 'TIEMPO ACTIVO',
        FREE: 'Libre',
        NOW: 'Ahora',
        UPDATED: 'Actualizado',
        MUSIC: 'Música',
        NO_MEDIA: 'Sin reproducción',
        BATTERY: 'Batería',
        CHARGING: 'Cargando',
        WORLD_CLOCK: 'Reloj mundial',
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
        CAL_DAYS: ['L', 'M', 'X', 'J', 'V', 'S', 'D'],
    },
    en: {
        NOTIFICATIONS: 'NOTIFICATIONS',
        CLEAR_ALL: 'Clear All',
        EXPAND: 'Expand',
        COLLAPSE: 'Collapse',
        NO_NOTIFICATIONS: 'No notifications',
        MORE_NOTIFS: '+%d more notifications... (Click to expand)',
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
        UPTIME: 'UPTIME',
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

const CALENDAR_SERVER_XML = `
<node>
  <interface name="org.gnome.Shell.CalendarServer">
    <method name="SetTimeRange">
      <arg type="x" direction="in" name="since"/>
      <arg type="x" direction="in" name="until"/>
      <arg type="b" direction="in" name="force_reload"/>
    </method>
    <signal name="EventsAddedOrUpdated">
      <arg type="a(ssxxa{sv})" name="events"/>
    </signal>
    <signal name="EventsRemoved">
      <arg type="as" name="ids"/>
    </signal>
  </interface>
</node>`;

let ICON_DIR = '';

function symbol(name) {
    try {
        return Gio.icon_new_for_string(`${ICON_DIR}/${name}-symbolic.svg`);
    } catch (e) {
        return Gio.ThemedIcon.new_with_default_fallbacks('image-missing');
    }
}

function icon(name, cls = '', size = 16) {
    const props = {
        style_class: `mac-nc-icon ${cls}`,
        x_align: Clutter.ActorAlign.CENTER,
        y_align: Clutter.ActorAlign.CENTER,
        icon_size: size,
    };
    if (name.startsWith('nc:'))
        props.gicon = symbol(name.slice(3));
    else
        props.icon_name = name;
    return new St.Icon(props);
}

function launch(cmd) {
    try {
        Gio.AppInfo.create_from_commandline(cmd, null, Gio.AppInfoCreateFlags.NONE).launch([], null);
    } catch (e) {
        console.warn(`macos-nc: failed to launch ${cmd}: ${e}`);
    }
}

function later(ms, fn) {
    return GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => {
        try {
            fn();
        } catch (e) {}
        return GLib.SOURCE_REMOVE;
    });
}

function findPlayer(cb) {
    try {
        Gio.DBus.session.call('org.freedesktop.DBus', '/org/freedesktop/DBus',
            'org.freedesktop.DBus', 'ListNames', null, GLib.VariantType.new('(as)'),
            NONE, -1, null, (c, r) => {
                try {
                    const names = c.call_finish(r).deepUnpack()[0];
                    cb(names.find(n => n.startsWith(MPRIS_PREFIX)) ?? null);
                } catch (e) {
                    cb(null);
                }
            });
    } catch (e) {
        cb(null);
    }
}

export default class MacOSNotificationCenter extends Extension {
    enable() {
        ICON_DIR = `${this.path}/icons`;
        this._dateMenu = Main.panel.statusArea.dateMenu;
        this._ids = [];
        this._subs = [];
        this._hiddenChildren = [];
        this._cards = [];
        this._pollId = 0;
        this._weatherPollId = 0;
        this._soupSession = new Soup.Session();
        this._soupSession.timeout = 10;
        this._weatherData = null;
        this._mprisPlayer = null;
        this._mprisPlaying = false;
        this._notifsExpanded = false;
        this._calendarEvents = [];
        this._calendarProxy = null;
        this._lastCpuTotal = 0;
        this._lastCpuIdle = 0;

        try {
            this._ifaceSettings = new Gio.Settings({schema_id: 'org.gnome.desktop.interface'});
        } catch (e) {
            this._ifaceSettings = null;
        }

        this._applyTransparentContainer(true);
        this._initCalendarProxy();
        this._buildUI();

        if (this._dateMenu?.menu?.box) {
            const box = this._dateMenu.menu.box;
            box.get_children().forEach(child => {
                if (child !== this._root && child.visible) {
                    child.visible = false;
                    this._hiddenChildren.push(child);
                }
            });
            box.add_child(this._root);
        }

        try {
            if (this._dateMenu?.menu?._boxPointer) {
                this._dateMenu.menu._boxPointer.setSourceAlignment(0.85);
            }
        } catch (e) {}

        if (this._dateMenu?.menu) {
            this._ids.push([this._dateMenu.menu, this._dateMenu.menu.connect('open-state-changed', (_m, open) => {
                if (open) {
                    this._applyTransparentContainer(true);
                    this._queryTodayEvents();
                    this._refresh();
                    this._updateLiquidGlass();
                    this._startPolling();
                } else {
                    this._stopPolling();
                }
            })]);
        }

        this._subs = [
            Gio.DBus.session.signal_subscribe(null, 'org.freedesktop.DBus.Properties',
                'PropertiesChanged', MPRIS_PATH, null, Gio.DBusSignalFlags.NONE,
                () => this._refreshMedia()),
            Gio.DBus.session.signal_subscribe('org.freedesktop.DBus', 'org.freedesktop.DBus',
                'NameOwnerChanged', null, null, Gio.DBusSignalFlags.NONE,
                () => later(200, () => this._refreshMedia())),
        ];

        this._fetchWeather();
        this._weatherPollId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 900, () => {
            this._fetchWeather();
            return GLib.SOURCE_CONTINUE;
        });

        this._refresh();
        this._updateLiquidGlass();
    }

    disable() {
        this._stopPolling();

        if (this._weatherPollId) {
            GLib.source_remove(this._weatherPollId);
            this._weatherPollId = 0;
        }

        if (this._soupSession) {
            try {
                this._soupSession.abort();
            } catch (e) {}
            this._soupSession = null;
        }

        for (const id of this._subs ?? []) {
            try {
                Gio.DBus.session.signal_unsubscribe(id);
            } catch (e) {}
        }
        this._subs = [];

        for (const [obj, id] of this._ids ?? []) {
            try {
                obj.disconnect(id);
            } catch (e) {}
        }
        this._ids = [];

        this._applyTransparentContainer(false);

        if (this._root) {
            try {
                this._dateMenu?.menu?.box?.remove_child(this._root);
                this._root.destroy();
            } catch (e) {}
            this._root = null;
        }

        for (const child of this._hiddenChildren ?? []) {
            try {
                child.visible = true;
            } catch (e) {}
        }
        this._hiddenChildren = [];

        this._dateMenu = null;
        this._calendarProxy = null;
    }

    _applyTransparentContainer(enable) {
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
    }

    _updateLiquidGlass() {
        let lg = null;
        try {
            lg = Main.extensionManager?.lookup?.(LG_UUID)?.stateObj ?? null;
        } catch (e) {}
        const active = !!lg;

        for (const card of this._cards ?? []) {
            if (!card) continue;
            if (active)
                card.add_style_class_name('quick-toggle');
            else
                card.remove_style_class_name('quick-toggle');
        }
    }

    _initCalendarProxy() {
        try {
            const ProxyClass = Gio.DBusProxy.makeProxyWrapper(CALENDAR_SERVER_XML);
            new ProxyClass(
                Gio.DBus.session,
                'org.gnome.Shell.CalendarServer',
                '/org/gnome/Shell/CalendarServer',
                (proxy, error) => {
                    if (error) return;
                    this._calendarProxy = proxy;
                    this._calendarProxy.connectSignal('EventsAddedOrUpdated', (_p, _s, [events]) => {
                        this._calendarEvents = events || [];
                        this._refreshCalendar();
                    });
                    this._calendarProxy.connectSignal('EventsRemoved', () => {
                        this._refreshCalendar();
                    });
                    this._queryTodayEvents();
                }
            );
        } catch (e) {}
    }

    _queryTodayEvents() {
        if (!this._calendarProxy) return;
        try {
            const now = GLib.DateTime.new_now_local();
            const startOfDay = GLib.DateTime.new_local(now.get_year(), now.get_month(), now.get_day_of_month(), 0, 0, 0);
            const endOfDay = GLib.DateTime.new_local(now.get_year(), now.get_month(), now.get_day_of_month(), 23, 59, 59);

            this._calendarProxy.SetTimeRangeRemote(
                startOfDay.to_unix(),
                endOfDay.to_unix(),
                true
            );
        } catch (e) {}
    }

    _refresh() {
        this._refreshCalendar();
        this._refreshNotifications();
        this._refreshWorldClocks();
        this._refreshMedia();
        this._refreshWeatherUI();
        this._refreshBatteryAndSystem();
    }

    _startPolling() {
        if (this._pollId) return;
        this._pollId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 1500, () => {
            try {
                if (!this._dateMenu?.menu?.isOpen) {
                    this._pollId = 0;
                    return GLib.SOURCE_REMOVE;
                }
                this._updateLiveClocks();
                this._refreshMedia();
                this._refreshBatteryAndSystem();
            } catch (e) {}
            return GLib.SOURCE_CONTINUE;
        });
    }

    _stopPolling() {
        if (this._pollId) {
            GLib.source_remove(this._pollId);
            this._pollId = 0;
        }
    }

    /* Build Entire UI Layout */
    _buildUI() {
        this._cards = [];

        this._root = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-sidebar',
            reactive: true,
            x_expand: true,
        });

        const scroll = new St.ScrollView({
            hscrollbar_policy: St.PolicyType.NEVER,
            vscrollbar_policy: St.PolicyType.AUTOMATIC,
            style_class: 'mac-nc-scroll',
            x_expand: true,
            y_expand: true,
        });

        const content = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-content',
            x_expand: true,
        });

        // 1. Notifications Section
        this._notifSection = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-section',
            x_expand: true,
        });

        const notifHeader = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-notif-header',
            x_expand: true,
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
        this._expandBtn.connect('clicked', () => {
            this._notifsExpanded = !this._notifsExpanded;
            this._expandBtn.label = this._notifsExpanded ? t('COLLAPSE') : t('EXPAND');
            this._refreshNotifications();
        });
        notifHeader.add_child(this._expandBtn);

        this._clearBtn = new St.Button({
            label: t('CLEAR_ALL'),
            style_class: 'mac-nc-clear-btn',
            can_focus: true,
            visible: false,
        });
        this._clearBtn.connect('clicked', () => this._clearAllNotifications());
        notifHeader.add_child(this._clearBtn);

        this._notifSection.add_child(notifHeader);

        this._notifList = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-section',
            x_expand: true,
        });
        this._notifSection.add_child(this._notifList);
        content.add_child(this._notifSection);

        // Section Title: Widgets
        const widgetHeaderBox = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-section-header',
            x_expand: true,
        });
        widgetHeaderBox.add_child(new St.Label({
            text: t('WIDGETS'),
            style_class: 'mac-nc-section-title',
            x_expand: true,
        }));
        content.add_child(widgetHeaderBox);

        // 2. Widgets Grid Section
        const widgetsBox = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-widgets',
            x_expand: true,
        });

        // Row 1: Real Month Calendar (Square) + Tasks (Square)
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
        scroll.set_child(content);
        this._root.add_child(scroll);
    }

    /* 1. Notifications UI */
    _refreshNotifications() {
        if (!this._notifList) return;
        this._notifList.destroy_all_children();

        const notifModel = Main.panel.statusArea.dateMenu?._messageList?._notificationSection?._model
            || Main.messageTray?._notificationQueue;
        let notifs = [];

        try {
            if (notifModel?.get_n_items) {
                const count = notifModel.get_n_items();
                for (let i = 0; i < count; i++) {
                    const item = notifModel.get_item(i);
                    if (item) notifs.push(item);
                }
            } else if (Main.panel.statusArea.dateMenu?._messageList?._notificationSection?._messages) {
                notifs = Main.panel.statusArea.dateMenu._messageList._notificationSection._messages;
            }
        } catch (e) {}

        const total = notifs.length;
        this._notifTitle.text = `${t('NOTIFICATIONS')} (${total})`;
        this._clearBtn.visible = (total > 0);

        if (total === 0) {
            this._expandBtn.visible = false;
            const emptyCard = new St.BoxLayout({
                orientation: Clutter.Orientation.HORIZONTAL,
                style_class: 'mac-nc-card mac-nc-empty-card',
                x_expand: true,
                x_align: Clutter.ActorAlign.CENTER,
            });
            emptyCard.add_child(icon('nc:messages', 'mac-nc-empty-icon', 16));
            emptyCard.add_child(new St.Label({
                text: t('NO_NOTIFICATIONS'),
                style_class: 'mac-nc-empty-notif',
            }));
            this._notifList.add_child(emptyCard);
            return;
        }

        const maxVisible = this._notifsExpanded ? total : Math.min(3, total);
        this._expandBtn.visible = (total > 3);

        for (let i = 0; i < maxVisible; i++) {
            const notif = notifs[i];
            const card = this._buildNotificationCard(notif);
            this._notifList.add_child(card);
        }

        if (!this._notifsExpanded && total > 3) {
            const remaining = total - 3;
            const moreBtn = new St.Button({
                style_class: 'mac-nc-more-notifs-btn',
                can_focus: true,
                reactive: true,
                x_expand: true,
            });
            const moreBox = new St.BoxLayout({
                orientation: Clutter.Orientation.HORIZONTAL,
                x_align: Clutter.ActorAlign.CENTER,
            });
            moreBox.add_child(new St.Label({
                text: t('MORE_NOTIFS', remaining),
                style_class: 'mac-nc-more-label',
            }));
            moreBtn.set_child(moreBox);
            moreBtn.connect('clicked', () => {
                this._notifsExpanded = true;
                this._expandBtn.label = t('COLLAPSE');
                this._refreshNotifications();
            });
            this._notifList.add_child(moreBtn);
        }
    }

    _buildNotificationCard(notification) {
        const card = new St.Button({
            style_class: 'mac-nc-card mac-nc-notif-card',
            can_focus: true,
            reactive: true,
            x_expand: true,
        });

        const box = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            x_expand: true,
        });

        const topRow = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-notif-top',
            x_expand: true,
        });

        let gicon = notification.gicon || notification.icon;
        if (!gicon && notification.source?.icon) gicon = notification.source.icon;

        const appIcon = new St.Icon({
            gicon: gicon || Gio.ThemedIcon.new_with_default_fallbacks('dialog-information-symbolic'),
            style_class: 'mac-nc-notif-icon',
            icon_size: 16,
        });
        topRow.add_child(appIcon);

        const appTitle = notification.source?.title || notification.source?.name || 'Sistema';
        const appLabel = new St.Label({
            text: appTitle,
            style_class: 'mac-nc-notif-app',
            x_expand: true,
        });
        topRow.add_child(appLabel);

        const timeLabel = new St.Label({
            text: t('NOW'),
            style_class: 'mac-nc-notif-time',
        });
        topRow.add_child(timeLabel);

        const closeBtn = new St.Button({
            style_class: 'mac-nc-notif-close',
            can_focus: true,
            reactive: true,
        });
        closeBtn.set_child(icon('window-close-symbolic', 'mac-nc-mini-icon', 10));
        closeBtn.connect('clicked', () => {
            try {
                if (typeof notification.destroy === 'function') notification.destroy();
                else if (typeof notification.close === 'function') notification.close();
            } catch (e) {}
            this._refreshNotifications();
        });
        topRow.add_child(closeBtn);
        box.add_child(topRow);

        const titleText = notification.title || notification.summary || '';
        if (titleText) {
            const title = new St.Label({
                text: titleText,
                style_class: 'mac-nc-notif-title',
            });
            title.clutter_text.ellipsize = Pango.EllipsizeMode.END;
            box.add_child(title);
        }

        const bodyText = notification.bannerBodyText || notification.body || '';
        if (bodyText) {
            const body = new St.Label({
                text: bodyText,
                style_class: 'mac-nc-notif-body',
            });
            body.clutter_text.line_wrap = true;
            body.clutter_text.ellipsize = Pango.EllipsizeMode.END;
            box.add_child(body);
        }

        card.set_child(box);
        card.connect('clicked', () => {
            try {
                notification.activate();
            } catch (e) {}
            this._dateMenu?.menu?.close();
        });
        return card;
    }

    _clearAllNotifications() {
        try {
            const notifSection = Main.panel.statusArea.dateMenu?._messageList?._notificationSection;
            if (notifSection?.clear) notifSection.clear();
            else if (notifSection?._model?.clear) notifSection._model.clear();
        } catch (e) {}
        this._refreshNotifications();
    }

    /* 2. Calendar Widget (Real Interactive Month Grid) */
    _buildCalendarCard() {
        const card = new St.Button({
            style_class: 'mac-nc-card mac-nc-square mac-nc-calendar-card',
            can_focus: true,
            reactive: true,
            x_expand: true,
        });

        const box = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-card-inner',
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
                const cell = new St.Bin({style_class: 'mac-nc-cal-cell'});
                cell.set_child(new St.Label({
                    text: String(daysInPrevMonth - i),
                    style_class: 'mac-nc-cal-day-label mac-nc-cal-other-month',
                }));
                rowBox.add_child(cell);
            }

            // Current month days
            for (let d = 1; d <= daysInMonth; d++) {
                const isToday = (d === todayDay);
                const cell = new St.Bin({style_class: 'mac-nc-cal-cell'});

                if (isToday) {
                    const bubble = new St.Bin({
                        style_class: 'mac-nc-cal-today-bubble',
                        x_align: Clutter.ActorAlign.CENTER,
                        y_align: Clutter.ActorAlign.CENTER,
                    });
                    bubble.set_child(new St.Label({
                        text: String(d),
                        style_class: 'mac-nc-cal-day-label',
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
                const cell = new St.Bin({style_class: 'mac-nc-cal-cell'});
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
                this._calEventText.text = summary;
            } else {
                this._calEventText.text = t('NO_EVENTS');
            }
        }
    }

    /* 3. Tasks Widget (Square) */
    _buildRemindersCard() {
        const card = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-card mac-nc-square mac-nc-reminders-card',
            reactive: true,
            x_expand: true,
        });

        const header = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-card-header',
        });
        header.add_child(icon('object-select-symbolic', 'mac-nc-reminders-icon-bg', 12));
        header.add_child(new St.Label({
            text: t('TASKS'),
            style_class: 'mac-nc-card-title',
        }));
        card.add_child(header);

        const spacer = new St.Widget({y_expand: true});
        card.add_child(spacer);

        const list = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-reminders-list',
        });

        const row = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-card-header',
        });
        row.add_child(icon('emblem-ok-symbolic', 'mac-nc-check-icon', 14));
        row.add_child(new St.Label({
            text: t('ALL_DONE'),
            style_class: 'mac-nc-reminders-status',
        }));
        list.add_child(row);

        list.add_child(new St.Label({
            text: t('NO_TASKS'),
            style_class: 'mac-nc-reminders-sub',
        }));
        card.add_child(list);
        return card;
    }

    /* 4. Performance Widget (Wide Card) */
    _buildSystemActivityCard() {
        const card = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-card mac-nc-wide mac-nc-stocks-card',
            reactive: true,
            x_expand: true,
        });

        const header = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-card-header',
            x_expand: true,
        });
        header.add_child(icon('utilities-system-monitor-symbolic', 'mac-nc-stocks-icon', 14));
        header.add_child(new St.Label({
            text: t('PERFORMANCE'),
            style_class: 'mac-nc-card-title',
            x_expand: true,
        }));
        header.add_child(new St.Label({
            text: t('OPTIMAL'),
            style_class: 'mac-nc-stocks-status',
        }));
        card.add_child(header);

        const row = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-stocks-row',
            x_expand: true,
        });

        // CPU
        const cpuCol = new St.BoxLayout({orientation: Clutter.Orientation.VERTICAL, style_class: 'mac-nc-stock-col', x_expand: true});
        cpuCol.add_child(new St.Label({text: t('CPU'), style_class: 'mac-nc-stock-sym'}));
        this._cpuVal = new St.Label({text: '12%', style_class: 'mac-nc-stock-price'});
        cpuCol.add_child(this._cpuVal);
        cpuCol.add_child(new St.Label({text: 'Normal', style_class: 'mac-nc-stock-delta positive'}));
        row.add_child(cpuCol);

        // Memory
        const memCol = new St.BoxLayout({orientation: Clutter.Orientation.VERTICAL, style_class: 'mac-nc-stock-col', x_expand: true});
        memCol.add_child(new St.Label({text: t('MEMORY'), style_class: 'mac-nc-stock-sym'}));
        this._memVal = new St.Label({text: '4.2 GB', style_class: 'mac-nc-stock-price'});
        memCol.add_child(this._memVal);
        memCol.add_child(new St.Label({text: t('FREE'), style_class: 'mac-nc-stock-delta positive'}));
        row.add_child(memCol);

        // Uptime
        const upCol = new St.BoxLayout({orientation: Clutter.Orientation.VERTICAL, style_class: 'mac-nc-stock-col', x_expand: true});
        upCol.add_child(new St.Label({text: t('UPTIME'), style_class: 'mac-nc-stock-sym'}));
        this._uptimeVal = new St.Label({text: '2h 15m', style_class: 'mac-nc-stock-price'});
        upCol.add_child(this._uptimeVal);
        upCol.add_child(new St.Label({text: 'Pulsar OS', style_class: 'mac-nc-stock-delta positive'}));
        row.add_child(upCol);

        card.add_child(row);
        return card;
    }

    _refreshBatteryAndSystem() {
        // Read CPU
        try {
            const [ok, contents] = GLib.file_get_contents('/proc/stat');
            if (ok) {
                const line = new TextDecoder().decode(contents).split('\n')[0];
                const parts = line.split(/\s+/).slice(1).map(Number);
                const idle = parts[3] + (parts[4] || 0);
                const total = parts.reduce((a, b) => a + b, 0);

                if (this._lastCpuTotal > 0) {
                    const diffTotal = total - this._lastCpuTotal;
                    const diffIdle = idle - this._lastCpuIdle;
                    if (diffTotal > 0) {
                        const usage = Math.round(100 * (1 - diffIdle / diffTotal));
                        if (this._cpuVal) this._cpuVal.text = `${usage}%`;
                    }
                }
                this._lastCpuTotal = total;
                this._lastCpuIdle = idle;
            }
        } catch (e) {}

        // Read Memory
        try {
            const [ok, contents] = GLib.file_get_contents('/proc/meminfo');
            if (ok) {
                const text = new TextDecoder().decode(contents);
                const availMatch = text.match(/MemAvailable:\s+(\d+)\s+kB/);
                if (availMatch && this._memVal) {
                    const availGb = (parseInt(availMatch[1], 10) / (1024 * 1024)).toFixed(1);
                    this._memVal.text = `${availGb} GB`;
                }
            }
        } catch (e) {}

        // Read Uptime
        try {
            const [ok, contents] = GLib.file_get_contents('/proc/uptime');
            if (ok) {
                const seconds = Math.floor(parseFloat(new TextDecoder().decode(contents).split(' ')[0]));
                const hours = Math.floor(seconds / 3600);
                const mins = Math.floor((seconds % 3600) / 60);
                if (this._uptimeVal) this._uptimeVal.text = `${hours}h ${mins}m`;
            }
        } catch (e) {}

        this._queryBatteryDevices();
    }

    _queryBatteryDevices() {
        try {
            Gio.DBus.system.call('org.freedesktop.UPower', '/org/freedesktop/UPower',
                'org.freedesktop.UPower', 'EnumerateDevices', null,
                GLib.VariantType.new('(ao)'), NONE, -1, null, (conn, res) => {
                    try {
                        const paths = conn.call_finish(res).deepUnpack()[0];
                        const devices = [];

                        let pending = paths.length;
                        if (pending === 0) {
                            this._updateBatteryUI(devices);
                            return;
                        }

                        for (const path of paths) {
                            conn.call('org.freedesktop.UPower', path,
                                'org.freedesktop.DBus.Properties', 'GetAll',
                                GLib.Variant.new('(s)', ['org.freedesktop.UPower.Device']),
                                GLib.VariantType.new('(a{sv})'), NONE, -1, null, (c, r) => {
                                    pending--;
                                    try {
                                        const props = c.call_finish(r).deepUnpack()[0];
                                        const type = props['Type']?.deepUnpack() ?? 0;
                                        const percentage = Math.round(props['Percentage']?.deepUnpack() ?? 0);
                                        const state = props['State']?.deepUnpack() ?? 0;
                                        const model = props['Model']?.deepUnpack() ?? '';
                                        const isPresent = props['IsPresent']?.deepUnpack() ?? true;

                                        if (isPresent && (type === 2 || type === 5 || type === 6 || type === 9)) {
                                            let iconName = 'battery-good-symbolic';
                                            let name = model || t('BATTERY');
                                            if (type === 5) { iconName = 'input-mouse-symbolic'; name = model || 'Mouse'; }
                                            else if (type === 6) { iconName = 'input-keyboard-symbolic'; name = model || 'Keyboard'; }
                                            else if (type === 9) { iconName = 'audio-headphones-symbolic'; name = model || 'Headset'; }
                                            else if (state === 1) { iconName = 'battery-good-charging-symbolic'; }

                                            devices.push({name, percentage, iconName});
                                        }
                                    } catch (e) {}

                                    if (pending === 0) {
                                        this._updateBatteryUI(devices);
                                    }
                                });
                        }
                    } catch (e) {
                        this._updateBatteryUI([]);
                    }
                });
        } catch (e) {
            this._updateBatteryUI([]);
        }
    }

    _updateBatteryUI(devices) {
        if (!this._batteryList) return;
        this._batteryList.destroy_all_children();

        if (devices.length === 0) {
            const row = new St.BoxLayout({orientation: Clutter.Orientation.HORIZONTAL, style_class: 'mac-nc-battery-row', x_expand: true});
            row.add_child(icon('ac-adapter-symbolic', 'mac-nc-device-icon', 14));
            row.add_child(new St.Label({text: 'Alimentación CA', style_class: 'mac-nc-device-name', x_expand: true}));
            row.add_child(new St.Label({text: '100%', style_class: 'mac-nc-device-level'}));
            this._batteryList.add_child(row);
            return;
        }

        for (const dev of devices) {
            const row = new St.BoxLayout({orientation: Clutter.Orientation.HORIZONTAL, style_class: 'mac-nc-battery-row', x_expand: true});
            row.add_child(icon(dev.iconName, 'mac-nc-device-icon', 14));
            const devLabel = new St.Label({text: dev.name, style_class: 'mac-nc-device-name', x_expand: true});
            devLabel.clutter_text.ellipsize = Pango.EllipsizeMode.END;
            row.add_child(devLabel);
            row.add_child(new St.Label({text: `${dev.percentage}%`, style_class: 'mac-nc-device-level'}));
            this._batteryList.add_child(row);
        }
    }

    /* 5. Weather Widget (Wide Card) */
    _buildWeatherCard() {
        const card = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-card mac-nc-wide mac-nc-weather-card',
            reactive: true,
            x_expand: true,
        });

        const topRow = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            x_expand: true,
        });

        const cityBox = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            x_expand: true,
        });
        this._weatherCity = new St.Label({text: 'A Coruña', style_class: 'mac-nc-weather-city'});
        this._weatherCond = new St.Label({text: `${t('CLEAR_SKY')} • ${t('UPDATED')}`, style_class: 'mac-nc-weather-condition'});
        cityBox.add_child(this._weatherCity);
        cityBox.add_child(this._weatherCond);
        topRow.add_child(cityBox);

        this._weatherTemp = new St.Label({text: '19°', style_class: 'mac-nc-weather-temp'});
        topRow.add_child(this._weatherTemp);

        this._weatherIcon = icon('weather-clear-symbolic', 'mac-nc-weather-big-icon', 28);
        topRow.add_child(this._weatherIcon);
        card.add_child(topRow);

        this._forecastRow = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-weather-forecast-row',
            x_expand: true,
        });
        card.add_child(this._forecastRow);
        return card;
    }

    _getConfiguredLocation() {
        let name = 'A Coruña';
        let lat = 43.36;
        let lon = -8.41;

        try {
            const settings = new Gio.Settings({schema_id: 'org.gnome.Weather'});
            const locs = settings.get_value('locations').deepUnpack();
            if (locs && locs.length > 0) {
                const locTuple = locs[0];
                const inner = locTuple[1];
                if (inner && inner[0]) {
                    name = inner[0];
                    const coords = inner[3];
                    if (coords && coords.length > 0) {
                        lat = (coords[0][0] * 180 / Math.PI);
                        lon = (coords[0][1] * 180 / Math.PI);
                    }
                }
            }
        } catch (e) {
            try {
                const s2 = new Gio.Settings({schema_id: 'org.gnome.shell.weather'});
                const locs = s2.get_value('locations').deepUnpack();
                if (locs && locs.length > 0) {
                    const inner = locs[0][1];
                    if (inner && inner[0]) {
                        name = inner[0];
                        const coords = inner[3];
                        if (coords && coords.length > 0) {
                            lat = (coords[0][0] * 180 / Math.PI);
                            lon = (coords[0][1] * 180 / Math.PI);
                        }
                    }
                }
            } catch (e2) {}
        }
        return {name, lat: lat.toFixed(2), lon: lon.toFixed(2)};
    }

    _fetchWeather() {
        if (!this._soupSession) return;
        const loc = this._getConfiguredLocation();
        const url = `https://api.open-meteo.com/v1/forecast?latitude=${loc.lat}&longitude=${loc.lon}&current=temperature_2m,weather_code&hourly=temperature_2m,weather_code&forecast_days=1&timezone=auto`;

        const msg = Soup.Message.new('GET', url);
        this._soupSession.send_and_read_async(msg, GLib.PRIORITY_DEFAULT, null, (_s, res) => {
            try {
                const bytes = this._soupSession.send_and_read_finish(res);
                if (msg.get_status() === 200 && bytes) {
                    const data = JSON.parse(new TextDecoder().decode(bytes.get_data()));
                    data.cityName = loc.name;
                    this._weatherData = data;
                    this._refreshWeatherUI();
                }
            } catch (e) {}
        });
    }

    _getWeatherIconAndName(code) {
        if (code === 0) return {icon: 'weather-clear-symbolic', name: t('CLEAR_SKY')};
        if (code >= 1 && code <= 3) return {icon: 'weather-few-clouds-symbolic', name: t('FEW_CLOUDS')};
        if (code >= 45 && code <= 48) return {icon: 'weather-fog-symbolic', name: t('FOG')};
        if (code >= 51 && code <= 67) return {icon: 'weather-showers-symbolic', name: t('RAIN')};
        if (code >= 71 && code <= 77) return {icon: 'weather-snow-symbolic', name: t('SNOW')};
        if (code >= 80 && code <= 82) return {icon: 'weather-showers-symbolic', name: t('RAIN')};
        if (code >= 95) return {icon: 'weather-storm-symbolic', name: t('THUNDERSTORM')};
        return {icon: 'weather-overcast-symbolic', name: t('CLOUDY')};
    }

    _refreshWeatherUI() {
        if (!this._weatherData || !this._weatherCity) return;
        const current = this._weatherData.current;
        const cityName = this._weatherData.cityName || 'A Coruña';
        const code = current.weather_code ?? 0;
        const info = this._getWeatherIconAndName(code);

        this._weatherCity.text = cityName;
        this._weatherCond.text = `${info.name} • ${t('UPDATED')}`;
        this._weatherTemp.text = `${Math.round(current.temperature_2m)}°`;
        this._weatherIcon.icon_name = info.icon;

        if (this._forecastRow && this._weatherData.hourly) {
            this._forecastRow.destroy_all_children();
            const hourly = this._weatherData.hourly;
            const nowHour = new Date().getHours();
            const offsets = [0, 2, 4, 6, 8];

            for (const offset of offsets) {
                const targetHour = (nowHour + offset) % 24;
                const temp = Math.round(hourly.temperature_2m[targetHour] || current.temperature_2m);
                const hCode = hourly.weather_code[targetHour] || 0;
                const hInfo = this._getWeatherIconAndName(hCode);

                const pill = new St.BoxLayout({
                    orientation: Clutter.Orientation.VERTICAL,
                    style_class: 'mac-nc-forecast-pill',
                    x_expand: true,
                });
                pill.add_child(new St.Label({
                    text: offset === 0 ? t('NOW') : `+${offset}h`,
                    style_class: 'mac-nc-forecast-time',
                }));
                pill.add_child(icon(hInfo.icon, 'mac-nc-forecast-icon', 14));
                pill.add_child(new St.Label({
                    text: `${temp}°`,
                    style_class: 'mac-nc-forecast-temp',
                }));
                this._forecastRow.add_child(pill);
            }
        }
    }

    /* 6. Music Widget (Square) with Real Cover Art */
    _buildMusicCard() {
        const card = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-card mac-nc-square mac-nc-podcasts-card',
            reactive: true,
            x_expand: true,
        });

        const header = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-card-header',
        });
        header.add_child(icon('audio-x-generic-symbolic', 'mac-nc-mini-white', 12));
        header.add_child(new St.Label({
            text: t('MUSIC'),
            style_class: 'mac-nc-card-title-white',
        }));
        card.add_child(header);

        const spacer = new St.Widget({y_expand: true});
        card.add_child(spacer);

        const bottom = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-podcasts-bottom',
            x_expand: true,
        });

        this._musicArt = new St.Bin({
            style_class: 'mac-nc-cover-art',
            child: icon('audio-x-generic-symbolic', 'mac-nc-mini-white', 20),
        });
        bottom.add_child(this._musicArt);

        const trackInfo = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            x_expand: true,
        });
        this._musicTitle = new St.Label({
            text: 'Pulsar Music',
            style_class: 'mac-nc-podcast-title',
        });
        this._musicTitle.clutter_text.ellipsize = Pango.EllipsizeMode.END;
        this._musicArtist = new St.Label({
            text: t('NO_MEDIA'),
            style_class: 'mac-nc-podcast-artist',
        });
        this._musicArtist.clutter_text.ellipsize = Pango.EllipsizeMode.END;
        trackInfo.add_child(this._musicTitle);
        trackInfo.add_child(this._musicArtist);
        bottom.add_child(trackInfo);

        this._playBtn = new St.Button({
            style_class: 'mac-nc-podcast-play-btn',
            can_focus: true,
            reactive: true,
        });
        this._playIcon = icon('media-playback-start-symbolic', 'mac-nc-play-icon', 12);
        this._playBtn.set_child(this._playIcon);
        this._playBtn.connect('clicked', () => this._togglePlayback());
        bottom.add_child(this._playBtn);

        card.add_child(bottom);
        return card;
    }

    _refreshMedia() {
        findPlayer(busName => {
            if (!busName) {
                this._mprisPlayer = null;
                if (this._musicTitle) this._musicTitle.text = 'Pulsar Music';
                if (this._musicArtist) this._musicArtist.text = t('NO_MEDIA');
                if (this._playIcon) this._playIcon.icon_name = 'media-playback-start-symbolic';
                if (this._musicArt) this._musicArt.set_child(icon('audio-x-generic-symbolic', 'mac-nc-mini-white', 20));
                return;
            }
            this._mprisPlayer = busName;

            Gio.DBus.session.call(busName, MPRIS_PATH, 'org.freedesktop.DBus.Properties',
                'GetAll', GLib.Variant.new('(s)', [PLAYER_IFACE]),
                GLib.VariantType.new('(a{sv})'), NONE, -1, null, (c, res) => {
                    try {
                        const props = c.call_finish(res).deepUnpack()[0];
                        const status = props['PlaybackStatus']?.deepUnpack() ?? 'Stopped';
                        const meta = props['Metadata']?.deepUnpack() ?? {};
                        const title = meta['xesam:title']?.deepUnpack() ?? 'Pulsar Music';
                        const artist = meta['xesam:artist']?.deepUnpack()?.[0] ?? meta['xesam:albumArtist']?.deepUnpack()?.[0] ?? 'Pulsar OS';
                        const artUrl = meta['mpris:artUrl']?.deepUnpack() ?? '';

                        this._mprisPlaying = (status === 'Playing');
                        if (this._musicTitle) this._musicTitle.text = title;
                        if (this._musicArtist) this._musicArtist.text = artist;
                        if (this._playIcon) {
                            this._playIcon.icon_name = this._mprisPlaying ? 'media-playback-pause-symbolic' : 'media-playback-start-symbolic';
                        }

                        if (artUrl && this._musicArt) {
                            try {
                                const file = Gio.File.new_for_uri(artUrl);
                                const gicon = new Gio.FileIcon({file});
                                this._musicArt.set_child(new St.Icon({
                                    gicon: gicon,
                                    icon_size: 38,
                                    style_class: 'mac-nc-cover-art',
                                }));
                            } catch (e) {
                                this._musicArt.set_child(icon('audio-x-generic-symbolic', 'mac-nc-mini-white', 20));
                            }
                        }
                    } catch (e) {}
                });
        });
    }

    _togglePlayback() {
        if (!this._mprisPlayer) {
            launch('rhythmbox || spotify || amberol || elisa');
            return;
        }
        Gio.DBus.session.call(this._mprisPlayer, MPRIS_PATH, PLAYER_IFACE, 'PlayPause',
            null, null, NONE, -1, null, () => this._refreshMedia());
    }

    /* 7. Battery Widget (Square) */
    _buildBatteryCard() {
        const card = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-card mac-nc-square mac-nc-battery-card',
            reactive: true,
            x_expand: true,
        });

        const header = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-card-header',
        });
        header.add_child(icon('battery-good-symbolic', 'mac-nc-battery-icon', 14));
        header.add_child(new St.Label({
            text: t('BATTERY'),
            style_class: 'mac-nc-card-title',
        }));
        card.add_child(header);

        this._batteryList = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-battery-list',
            x_expand: true,
            y_expand: true,
        });
        card.add_child(this._batteryList);
        return card;
    }

    /* 8. World Clock (Wide Card) */
    _buildWorldClockCard() {
        const card = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'mac-nc-card mac-nc-wide mac-nc-worldclock-card',
            reactive: true,
            x_expand: true,
        });

        const header = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-card-header',
        });
        header.add_child(icon('preferences-system-time-symbolic', 'mac-nc-worldclock-icon', 14));
        header.add_child(new St.Label({
            text: t('WORLD_CLOCK'),
            style_class: 'mac-nc-card-title',
        }));
        card.add_child(header);

        this._clocksRow = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            style_class: 'mac-nc-clocks-row',
            x_expand: true,
        });
        card.add_child(this._clocksRow);
        return card;
    }

    _refreshWorldClocks() {
        if (!this._clocksRow) return;
        this._clocksRow.destroy_all_children();

        const cities = [
            {name: 'Cupertino', tz: 'America/Los_Angeles'},
            {name: 'Tokio', tz: 'Asia/Tokyo'},
            {name: 'Sídney', tz: 'Australia/Sydney'},
            {name: 'París', tz: 'Europe/Paris'},
        ];

        this._clockLabels = [];
        const localNow = GLib.DateTime.new_now_local();

        for (const city of cities) {
            const tz = GLib.TimeZone.new_identifier(city.tz) || GLib.TimeZone.new_utc();
            const cityTime = localNow.to_timezone(tz);

            const col = new St.BoxLayout({
                orientation: Clutter.Orientation.VERTICAL,
                style_class: 'mac-nc-clock-col',
                x_expand: true,
            });

            col.add_child(new St.Label({
                text: city.name,
                style_class: 'mac-nc-clock-city',
            }));

            const diffHours = Math.round((cityTime.to_unix() - localNow.to_unix()) / 3600);
            let diffText = t('SAME_TIME');
            if (diffHours > 0) diffText = t('AHEAD', diffHours);
            else if (diffHours < 0) diffText = t('BEHIND', Math.abs(diffHours));

            col.add_child(new St.Label({
                text: diffText,
                style_class: 'mac-nc-clock-offset',
            }));

            const timeLabel = new St.Label({
                text: cityTime.format('%H:%M') || '12:00',
                style_class: 'mac-nc-clock-time',
            });
            this._clockLabels.push({label: timeLabel, tz});
            col.add_child(timeLabel);

            this._clocksRow.add_child(col);
        }
    }

    _updateLiveClocks() {
        if (!this._clockLabels) return;
        const localNow = GLib.DateTime.new_now_local();
        for (const item of this._clockLabels) {
            const cityTime = localNow.to_timezone(item.tz);
            item.label.text = cityTime.format('%H:%M') || '12:00';
        }
    }
}
