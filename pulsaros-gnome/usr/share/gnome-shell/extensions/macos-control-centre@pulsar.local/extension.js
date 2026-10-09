import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Pango from 'gi://Pango';
import St from 'gi://St';
import Soup from 'gi://Soup?version=3.0';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Switch} from 'resource:///org/gnome/shell/ui/popupMenu.js';
import {Slider} from 'resource:///org/gnome/shell/ui/slider.js';
import * as Volume from 'resource:///org/gnome/shell/ui/status/volume.js';

const NONE = Gio.DBusCallFlags.NONE;

let ICON_DIR = '';
function symbol(name) {
    try {
        return Gio.icon_new_for_string(`${ICON_DIR}/${name}-symbolic.svg`);
    } catch (e) {
        return Gio.ThemedIcon.new_with_default_fallbacks('image-missing');
    }
}

const BL_DIR = '/sys/class/backlight';
function findBacklight() {
    try {
        const en = Gio.File.new_for_path(BL_DIR).enumerate_children(
            'standard::name', Gio.FileQueryInfoFlags.NONE, null);
        const names = [];
        let info;
        while ((info = en.next_file(null)))
            names.push(info.get_name());
        en.close(null);
        return names[0] ?? null;
    } catch (e) {
        return null;
    }
}

function readInt(path) {
    try {
        const [, bytes] = GLib.file_get_contents(path);
        return parseInt(new TextDecoder().decode(bytes).trim());
    } catch (e) {
        return null;
    }
}

function setBacklight(name, value) {
    try {
        Gio.DBus.system.call('org.freedesktop.login1', '/org/freedesktop/login1/session/auto',
            'org.freedesktop.login1.Session', 'SetBrightness',
            new GLib.Variant('(ssu)', ['backlight', name, value]), null,
            Gio.DBusCallFlags.NONE, -1, null, (c, r) => {
                try {
                    c.call_finish(r);
                } catch (e) {}
            });
    } catch (e) {}
}

function later(ms, fn) {
    return GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => {
        try {
            fn();
        } catch (e) {}
        return GLib.SOURCE_REMOVE;
    });
}

function propGet(bus, name, path, iface, prop, cb) {
    try {
        bus.call(name, path, 'org.freedesktop.DBus.Properties', 'Get',
            new GLib.Variant('(ss)', [iface, prop]), GLib.VariantType.new('(v)'),
            NONE, -1, null, (c, r) => {
                try {
                    cb(c.call_finish(r).get_child_value(0).get_variant().unpack());
                } catch (e) {
                    cb(null);
                }
            });
    } catch (e) {
        cb(null);
    }
}

function propSet(bus, name, path, iface, prop, variant) {
    try {
        bus.call(name, path, 'org.freedesktop.DBus.Properties', 'Set',
            new GLib.Variant('(ssv)', [iface, prop, variant]), null,
            NONE, -1, null, (c, r) => {
                try {
                    c.call_finish(r);
                } catch (e) {}
            });
    } catch (e) {}
}

const MPRIS_PREFIX = 'org.mpris.MediaPlayer2.';
const MPRIS_PATH = '/org/mpris/MediaPlayer2';
const PLAYER_IFACE = 'org.mpris.MediaPlayer2.Player';

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

function icon(name, cls = '') {
    const props = {
        style_class: `mac-icon ${cls}`,
        x_align: Clutter.ActorAlign.CENTER,
        y_align: Clutter.ActorAlign.CENTER,
    };
    if (name.startsWith('mc:'))
        props.gicon = symbol(name.slice(3));
    else
        props.icon_name = name;
    return new St.Icon(props);
}

function wideTile(iconName, title, subtitle, extraClass, onToggle, onMenu) {
    const tile = new St.BoxLayout({
        style_class: `mac-tile ${extraClass}`,
        reactive: true,
        can_focus: true,
        y_align: Clutter.ActorAlign.CENTER,
    });
    
    const bubble = new St.Button({
        style_class: 'mac-bubble',
        y_align: Clutter.ActorAlign.CENTER,
        can_focus: true,
        reactive: true,
        child: icon(iconName),
    });
    bubble.connect('clicked', () => onToggle?.());

    const menuBtn = new St.Button({
        style_class: 'mac-tile-content-btn',
        y_align: Clutter.ActorAlign.CENTER,
        x_expand: true,
        can_focus: true,
        reactive: true,
    });
    const contentRow = new St.BoxLayout({
        y_align: Clutter.ActorAlign.CENTER,
        x_expand: true,
        style: 'spacing: 4px;',
    });

    const texts = new St.BoxLayout({vertical: true, y_align: Clutter.ActorAlign.CENTER, x_expand: true});
    const t = new St.Label({text: title, style_class: 'mac-title'});
    const s = new St.Label({text: subtitle, style_class: 'mac-sub'});
    for (const l of [t, s])
        l.clutter_text.ellipsize = Pango.EllipsizeMode.END;
    texts.add_child(t);
    texts.add_child(s);
    contentRow.add_child(texts);

    if (onMenu) {
        const chev = new St.Icon({
            icon_name: 'go-next-symbolic',
            style_class: 'mac-small mac-chevron',
            y_align: Clutter.ActorAlign.CENTER,
            x_align: Clutter.ActorAlign.END,
        });
        contentRow.add_child(chev);
        menuBtn.connect('clicked', () => onMenu?.());
    }

    menuBtn.set_child(contentRow);

    tile.add_child(bubble);
    tile.add_child(menuBtn);
    
    tile._sub = s;
    tile._title = t;
    tile._bubble = bubble;
    return tile;
}

function circleButton(iconName, extraClass, onClick) {
    const btn = new St.Button({
        style_class: `mac-circle ${extraClass}`,
        can_focus: true,
        reactive: true,
        child: icon(iconName),
    });
    btn.connect('clicked', () => onClick?.());
    return btn;
}

function createAppleSwitch(initialState, onToggle) {
    // Native GNOME/adwaita switch (styled by the active shell theme).
    const sw = new Switch(!!initialState);
    sw._syncing = false;

    sw.updateState = function(state) {
        sw._syncing = true;
        try {
            sw.state = !!state;
        } finally {
            sw._syncing = false;
        }
    };

    sw.connect('notify::state', () => {
        if (sw._syncing)
            return;
        onToggle?.(sw.state);
    });

    return sw;
}

function setOn(widget, on) {
    if (!widget) return;
    const active = !!on;
    try {
        if ('checked' in widget)
            widget.checked = active;
    } catch (e) {}
    if (active) {
        widget.add_style_pseudo_class('checked');
        widget.add_style_class_name('active');
    } else {
        widget.remove_style_pseudo_class('checked');
        widget.remove_style_class_name('active');
    }
}

function launch(cmd) {
    try {
        Gio.AppInfo.create_from_commandline(cmd, null, Gio.AppInfoCreateFlags.NONE).launch([], null);
    } catch (e) {
        console.warn(`macos-cc: failed to launch ${cmd}: ${e}`);
    }
}

function sliderTile(title, lowIcon, highIcon, onTrail) {
    const box = new St.BoxLayout({vertical: true, style_class: 'mac-slider-tile'});
    box.add_child(new St.Label({text: title, style_class: 'mac-title'}));
    const row = new St.BoxLayout({y_align: Clutter.ActorAlign.CENTER, style_class: 'mac-slider-row'});
    const slider = new Slider(0.5);
    slider.x_expand = true;
    slider.y_align = Clutter.ActorAlign.CENTER;
    row.add_child(icon(lowIcon, 'mac-small'));
    row.add_child(slider);
    row.add_child(icon(highIcon, 'mac-small'));
    if (onTrail) {
        const trail = new St.Button({
            style_class: 'mac-airplay',
            can_focus: true,
            reactive: true,
            child: icon('mc:airplay', 'mac-small'),
        });
        trail.connect('clicked', () => onTrail());
        row.add_child(trail);
    }
    box.add_child(row);
    box.slider = slider;
    return box;
}

export default class MacControlCentre extends Extension {
    enable() {
        ICON_DIR = `${this.path}/icons`;
        this._qs = Main.panel.statusArea.quickSettings;
        this._ids = [];
        this._artUrl = null;
        this._building = false;
        this._hiddenChildren = [];

        try {
            this._ifaceSettings = new Gio.Settings({schema_id: 'org.gnome.desktop.interface'});
        } catch (e) {
            this._ifaceSettings = null;
        }

        try {
            this._notifSettings = new Gio.Settings({schema_id: 'org.gnome.desktop.notifications'});
        } catch (e) {
            this._notifSettings = null;
        }

        try {
            this._colorSettings = new Gio.Settings({schema_id: 'org.gnome.settings-daemon.plugins.color'});
        } catch (e) {
            this._colorSettings = null;
        }

        this._applyTransparentContainer(true);

        this._root = new St.BoxLayout({vertical: true});
        this._buildMainPanel();
        this._buildWifiPanel();
        this._buildBluetoothPanel();

        this._root.add_child(this._mainPanel);
        this._root.add_child(this._wifiPanel);
        this._root.add_child(this._btPanel);
        this._showView('main');

        const box = this._qs.menu.box;
        box.get_children().forEach(child => {
            if (child !== this._root && child.visible) {
                child.visible = false;
                this._hiddenChildren.push(child);
            }
        });

        box.add_child(this._root);

        this._updateThemeContrast();
        if (this._ifaceSettings) {
            this._ids.push([this._ifaceSettings, this._ifaceSettings.connect('changed::color-scheme', () => {
                this._updateThemeContrast();
                this._refresh();
            })]);
            this._ids.push([this._ifaceSettings, this._ifaceSettings.connect('changed::gtk-theme', () => {
                this._updateThemeContrast();
            })]);
        }

        this._ids.push([this._qs.menu, this._qs.menu.connect('open-state-changed', (_m, open) => {
            if (open) {
                this._applyTransparentContainer(true);
                this._showView('main');
                this._updateThemeContrast();
                this._refresh();
                later(250, () => this._dropQuickSettingsBlur());
            }
        })]);

        this._subs = [
            Gio.DBus.session.signal_subscribe(null, 'org.freedesktop.DBus.Properties',
                'PropertiesChanged', MPRIS_PATH, null, Gio.DBusSignalFlags.NONE,
                () => this._refreshMedia()),
            Gio.DBus.session.signal_subscribe('org.freedesktop.DBus', 'org.freedesktop.DBus',
                'NameOwnerChanged', null, null, Gio.DBusSignalFlags.NONE,
                () => later(200, () => this._refreshMedia())),
        ];
        this._refresh();
    }

    _applyTransparentContainer(enable) {
        const menu = this._qs?.menu;
        if (!menu) return;
        const cls = 'mac-cc-quick-settings';

        for (const actor of [menu.actor, menu.box, menu._boxPointer, menu._boxPointer?.bin]) {
            if (!actor) continue;
            if (enable)
                actor.add_style_class_name(cls);
            else
                actor.remove_style_class_name(cls);
        }

        // The panel draws its own background, so drop the shell's Quick
        // Settings surface. Removing these classes also keeps Blur My Shell
        // from finding the menu as a blur target.
        if (menu.box) {
            if (enable) {
                menu.box.remove_style_class_name('quick-settings');
                menu.box.remove_style_class_name('popup-menu-content');
            } else {
                menu.box.add_style_class_name('quick-settings');
                menu.box.add_style_class_name('popup-menu-content');
            }
        }

        if (enable)
            this._dropQuickSettingsBlur();
    }

    _dropQuickSettingsBlur() {
        const menu = this._qs?.menu;
        if (!menu) return;

        // Preferred: let Blur My Shell destroy the surface it created for
        // this menu (it keeps one surface per target actor).
        try {
            const bms = Main.extensionManager.lookup('blur-my-shell@aunetx')?.stateObj;
            const popup = bms?._popup;
            if (popup?.surfaces && typeof popup.destroy_blur === 'function') {
                for (const target of [...popup.surfaces.keys()]) {
                    if (target === menu.box || this._isInsideQuickSettings(target))
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

    _isInsideQuickSettings(actor) {
        const root = this._qs?.menu?.actor;
        for (let a = actor; a; a = a.get_parent()) {
            if (a === root)
                return true;
        }
        return false;
    }

    disable() {
        this._applyTransparentContainer(false);

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

        if (this._root) {
            try {
                this._qs?.menu?.box?.remove_child(this._root);
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

        this._mainPanel = null;
        this._wifiPanel = null;
        this._btPanel = null;
        this._wifiList = null;
        this._btList = null;
        this._ifaceSettings = this._notifSettings = this._colorSettings = null;
        this._mixer = null;
    }

    _updateThemeContrast() {
        if (!this._root) return;
        const isDark = this._ifaceSettings?.get_string('color-scheme') === 'prefer-dark';
        const addCls = isDark ? 'mac-dark-theme' : 'mac-light-theme';
        const remCls = isDark ? 'mac-light-theme' : 'mac-dark-theme';

        this._root.remove_style_class_name(remCls);
        this._root.add_style_class_name(addCls);

        for (const panel of [this._mainPanel, this._wifiPanel, this._btPanel]) {
            if (!panel) continue;
            try {
                panel.remove_style_class_name(remCls);
                panel.add_style_class_name(addCls);
            } catch (e) {}
        }
    }

    _showView(viewName) {
        if (!this._root) return;
        if (this._mainPanel) this._mainPanel.visible = (viewName === 'main');
        if (this._wifiPanel) this._wifiPanel.visible = (viewName === 'wifi');
        if (this._btPanel) this._btPanel.visible = (viewName === 'bt');

        if (viewName === 'wifi')
            this._refreshWifiList();
        else if (viewName === 'bt')
            this._refreshBluetoothList();
    }

    _buildMainPanel() {
        const panel = new St.BoxLayout({vertical: true, style_class: 'mac-cc'});
        this._mainPanel = panel;

        /* Top 2 Columns */
        const top = new St.BoxLayout({style_class: 'mac-row'});
        const left = new St.BoxLayout({vertical: true, style_class: 'mac-col'});
        const right = new St.BoxLayout({vertical: true, style_class: 'mac-col'});

        this._wifi = wideTile('mc:wifi', 'Wi-Fi', 'Off', '', 
            () => this._toggleWifi(), 
            () => this._showView('wifi')
        );
        this._bt = wideTile('mc:bluetooth', 'Bluetooth', 'Off', '', 
            () => this._toggleBt(), 
            () => this._showView('bt')
        );
        this._night = wideTile('mc:sun-horizon', 'Night Light', 'Off', '', 
            () => this._toggleNight(), 
            () => { this._qs.menu.close(); launch('gnome-control-center display'); }
        );
        left.add_child(this._wifi);
        left.add_child(this._bt);
        left.add_child(this._night);

        right.add_child(this._buildMedia());
        const pair = new St.BoxLayout({style_class: 'mac-row'});
        pair.add_child(circleButton('mc:stage', '', () => {
            this._qs.menu.close();
            Main.overview.toggle();
        }));
        pair.add_child(circleButton('mc:mirror', '', () => {
            this._qs.menu.close();
            launch('gnome-control-center display');
        }));
        right.add_child(pair);

        top.add_child(left);
        top.add_child(right);
        panel.add_child(top);

        /* Row: Dark Mode, Screenshot, Do Not Disturb */
        const row4 = new St.BoxLayout({style_class: 'mac-row'});
        const circles = new St.BoxLayout({style_class: 'mac-row'});
        
        this._dark = circleButton('mc:contrast', '', () => this._toggleDark());
        circles.add_child(this._dark);
        circles.add_child(circleButton('mc:screenshot', '', () => {
            this._qs.menu.close();
            GLib.timeout_add(GLib.PRIORITY_DEFAULT, 250, () => {
                Main.screenshotUI.open();
                return GLib.SOURCE_REMOVE;
            });
        }));
        this._dnd = wideTile('mc:moon', 'Do Not Disturb', 'Off', '', 
            () => this._toggleDnd(), 
            () => { this._qs.menu.close(); launch('gnome-control-center notifications'); }
        );
        row4.add_child(circles);
        row4.add_child(this._dnd);
        panel.add_child(row4);

        /* Sliders */
        this._display = sliderTile('Display', 'mc:sun-min', 'mc:sun-max');
        this._sound = sliderTile('Sound', 'mc:speaker-low', 'mc:speaker-high', () => {
            this._qs.menu.close();
            launch('gnome-control-center sound');
        });
        panel.add_child(this._display);
        panel.add_child(this._sound);

        this._display.slider.connect('notify::value', () => {
            if (this._building)
                return;
            const v = this._display.slider.value;
            const bl = findBacklight();
            const max = bl ? readInt(`${BL_DIR}/${bl}/max_brightness`) : null;
            if (bl && max) {
                setBacklight(bl, Math.max(1, Math.round(v * max)));
                return;
            }
            propSet(Gio.DBus.session, 'org.gnome.SettingsDaemon.Power',
                '/org/gnome/SettingsDaemon/Power',
                'org.gnome.SettingsDaemon.Power.Screen', 'Brightness',
                new GLib.Variant('i', Math.round(v * 100)));
        });
        this._sound.slider.connect('notify::value', () => {
            if (this._building)
                return;
            try {
                const sink = this._mixer.get_default_sink();
                sink.volume = this._sound.slider.value * this._mixer.get_vol_max_norm();
                sink.push_volume();
            } catch (e) {
                console.warn(`macos-cc: volume: ${e}`);
            }
        });
    }

    _buildMedia() {
        const tile = new St.BoxLayout({
            vertical: true,
            style_class: 'mac-media',
            x_expand: true,
        });

        this._art = new St.Widget({
            style_class: 'mac-media-art',
            x_align: Clutter.ActorAlign.START,
            y_align: Clutter.ActorAlign.START,
        });

        this._mediaTitle = new St.Label({
            text: 'Not Playing',
            style_class: 'mac-title mac-media-title',
            x_align: Clutter.ActorAlign.START,
        });
        this._mediaTitle.clutter_text.ellipsize = Pango.EllipsizeMode.END;

        const controls = new St.BoxLayout({
            style_class: 'mac-media-controls',
            x_align: Clutter.ActorAlign.CENTER,
            x_expand: true,
            y_expand: true,
            y_align: Clutter.ActorAlign.END,
        });

        const mk = (name, method) => {
            const b = new St.Button({
                style_class: 'mac-media-button',
                child: new St.Icon({gicon: symbol(name), style_class: 'mac-media-icon'}),
                can_focus: true,
            });
            b.connect('clicked', () => findPlayer(p => {
                if (!p)
                    return;
                Gio.DBus.session.call(p, MPRIS_PATH, PLAYER_IFACE, method, null, null,
                    NONE, -1, null, () => later(150, () => this._refreshMedia()));
            }));
            return b;
        };

        this._playBtn = mk('play', 'PlayPause');
        controls.add_child(mk('rewind', 'Previous'));
        controls.add_child(this._playBtn);
        controls.add_child(mk('forward', 'Next'));

        tile.add_child(this._art);
        tile.add_child(this._mediaTitle);
        tile.add_child(controls);
        return tile;
    }

    _setArt(url) {
        if (url === this._artUrl)
            return;
        this._artUrl = url;
        if (!this._art) return;
        if (!url) {
            this._art.set_style(null);
            return;
        }
        const apply = path => {
            if (this._artUrl !== url || !this._mainPanel || !this._art)
                return;
            this._art.set_style(`background-image: url("file://${path}"); background-size: 44px 44px;`);
        };
        if (url.startsWith('file://')) {
            apply(decodeURIComponent(url.slice(7)));
        } else if (url.startsWith('http')) {
            const sum = GLib.compute_checksum_for_string(GLib.ChecksumType.MD5, url, -1);
            const cache = GLib.build_filenamev([GLib.get_user_cache_dir(), `macos-cc-${sum}.jpg`]);
            if (GLib.file_test(cache, GLib.FileTest.EXISTS)) {
                apply(cache);
                return;
            }
            try {
                const session = new Soup.Session();
                const msg = Soup.Message.new('GET', url);
                session.send_and_read_async(msg, GLib.PRIORITY_DEFAULT, null, (sess, res) => {
                    try {
                        const bytes = sess.send_and_read_finish(res);
                        if (msg.get_status() === Soup.Status.OK) {
                            GLib.file_set_contents(cache, bytes.get_data());
                            apply(cache);
                        }
                    } catch (e) {
                        console.warn(`macos-cc: album art download failed: ${e}`);
                    }
                });
            } catch (e) {
                console.warn(`macos-cc: album art: ${e}`);
            }
        }
    }

    /* Subviews: Wi-Fi */
    _buildWifiPanel() {
        const p = new St.BoxLayout({vertical: true, style_class: 'mac-subview'});
        this._wifiPanel = p;

        const header = new St.BoxLayout({y_align: Clutter.ActorAlign.CENTER, style_class: 'mac-sub-header'});
        const backBtn = new St.Button({
            style_class: 'mac-back-btn',
            child: new St.Icon({icon_name: 'go-previous-symbolic', style_class: 'mac-icon'}),
            can_focus: true,
        });
        backBtn.connect('clicked', () => this._showView('main'));
        header.add_child(backBtn);

        const title = new St.Label({text: 'Wi-Fi', style_class: 'mac-sub-title', x_expand: true});
        header.add_child(title);

        this._wifiSwitch = createAppleSwitch(false, on => {
            this._setWifiEnabled(on);
        });
        header.add_child(this._wifiSwitch);
        p.add_child(header);

        const card = new St.BoxLayout({vertical: true, style_class: 'mac-sub-card'});
        card.add_child(new St.Label({text: 'Known & Available Networks', style_class: 'mac-sub-section-title'}));

        this._wifiScroll = new St.ScrollView({
            hscrollbar_policy: St.PolicyType.NEVER,
            vscrollbar_policy: St.PolicyType.AUTOMATIC,
            style_class: 'mac-scroll-area',
            x_expand: true,
            y_expand: true,
        });
        this._wifiList = new St.BoxLayout({vertical: true, style: 'spacing: 4px;', x_expand: true});
        this._wifiScroll.set_child(this._wifiList);
        card.add_child(this._wifiScroll);
        p.add_child(card);

        const settingsBtn = new St.Button({
            style_class: 'mac-bottom-action-btn',
            label: 'Wi-Fi Settings...',
            can_focus: true,
        });
        settingsBtn.connect('clicked', () => {
            this._qs.menu.close();
            launch('gnome-control-center wifi');
        });
        p.add_child(settingsBtn);
    }

    _setWifiEnabled(enable) {
        propSet(Gio.DBus.system, 'org.freedesktop.NetworkManager', '/org/freedesktop/NetworkManager',
            'org.freedesktop.NetworkManager', 'WirelessEnabled', new GLib.Variant('b', enable));
        later(350, () => {
            this._refreshWifiList();
            this._refresh();
        });
    }

    _refreshWifiList() {
        if (!this._wifiList || !this._wifiSwitch) return;

        // Show immediate scanning indicator
        this._wifiList.remove_all_children();
        const scanBox = new St.BoxLayout({y_align: Clutter.ActorAlign.CENTER, style: 'spacing: 8px; padding: 14px;'});
        scanBox.add_child(new St.Icon({icon_name: 'view-refresh-symbolic', style_class: 'mac-icon'}));
        scanBox.add_child(new St.Label({text: 'Scanning for networks...', style_class: 'mac-sub'}));
        this._wifiList.add_child(scanBox);

        // Check Wi-Fi state
        propGet(Gio.DBus.system, 'org.freedesktop.NetworkManager', '/org/freedesktop/NetworkManager',
            'org.freedesktop.NetworkManager', 'WirelessEnabled', on => {
                if (this._wifiSwitch)
                    this._wifiSwitch.updateState(!!on);
            });

        try {
            const proc = Gio.Subprocess.new(
                ['nmcli', '-t', '-f', 'IN-USE,SSID,SIGNAL,SECURITY', 'dev', 'wifi', 'list'],
                Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_SILENCE
            );
            proc.communicate_utf8_async(null, null, (p, res) => {
                try {
                    const [, stdout] = p.communicate_utf8_finish(res);
                    if (!this._wifiList || !this._root) return;
                    this._wifiList.remove_all_children();

                    const lines = (stdout || '').split('\n').filter(Boolean);
                    const seen = new Map();

                    for (const line of lines) {
                        const parts = line.split(/(?<!\\):/);
                        if (parts.length < 4) continue;
                        const inUse = parts[0].includes('*');
                        const ssid = parts[1].replace(/\\:/g, ':').trim();
                        const signal = parseInt(parts[2]) || 0;
                        const security = parts[3].trim();

                        if (!ssid || ssid === '--') continue;
                        const existing = seen.get(ssid);
                        if (!existing || (!existing.inUse && inUse) || (!existing.inUse && signal > existing.signal)) {
                            seen.set(ssid, { ssid, signal, security, inUse });
                        }
                    }

                    const networks = Array.from(seen.values());

                    // Sort connected first, then signal strength desc
                    networks.sort((a, b) => {
                        if (a.inUse) return -1;
                        if (b.inUse) return 1;
                        return b.signal - a.signal;
                    });

                    for (const net of networks) {
                        const item = new St.Button({
                            style_class: `mac-item-btn ${net.inUse ? 'active' : ''}`,
                            can_focus: true,
                            x_expand: true,
                        });
                        const row = new St.BoxLayout({y_align: Clutter.ActorAlign.CENTER, style: 'spacing: 10px;', x_expand: true});

                        const iconCircle = new St.Bin({
                            style_class: `mac-item-icon-circle ${net.inUse ? 'connected' : ''}`,
                            x_align: Clutter.ActorAlign.CENTER,
                            y_align: Clutter.ActorAlign.CENTER,
                        });
                        iconCircle.set_child(icon('mc:wifi', 'mac-small'));
                        row.add_child(iconCircle);

                        const nameLabel = new St.Label({
                            text: net.ssid,
                            style_class: 'mac-item-name',
                            x_expand: true,
                            y_align: Clutter.ActorAlign.CENTER,
                        });
                        nameLabel.clutter_text.ellipsize = Pango.EllipsizeMode.END;
                        row.add_child(nameLabel);

                        if (net.security && net.security.length > 0 && net.security !== '--') {
                            row.add_child(new St.Icon({
                                icon_name: 'channel-secure-symbolic',
                                style_class: 'mac-small',
                                opacity: 180,
                                y_align: Clutter.ActorAlign.CENTER,
                            }));
                        }

                        if (net.inUse) {
                            row.add_child(new St.Icon({
                                icon_name: 'emblem-ok-symbolic',
                                style_class: 'mac-small',
                                style: 'color: #007AFF;',
                                y_align: Clutter.ActorAlign.CENTER,
                            }));
                        }

                        item.set_child(row);
                        item.connect('clicked', () => {
                            if (net.inUse) return;
                            launch(`nmcli device wifi connect "${net.ssid}"`);
                            later(1500, () => this._refreshWifiList());
                        });
                        this._wifiList.add_child(item);
                    }

                    if (networks.length === 0) {
                        this._wifiList.add_child(new St.Label({
                            text: 'No networks found',
                            style_class: 'mac-sub',
                            style: 'padding: 16px; text-align: center;',
                        }));
                    }
                } catch (e) {
                    console.warn(`macos-cc: wifi parse error: ${e}`);
                }
            });
        } catch (e) {
            console.warn(`macos-cc: wifi subprocess error: ${e}`);
        }
    }

    /* Subviews: Bluetooth */
    _buildBluetoothPanel() {
        const p = new St.BoxLayout({vertical: true, style_class: 'mac-subview'});
        this._btPanel = p;

        const header = new St.BoxLayout({y_align: Clutter.ActorAlign.CENTER, style_class: 'mac-sub-header'});
        const backBtn = new St.Button({
            style_class: 'mac-back-btn',
            child: new St.Icon({icon_name: 'go-previous-symbolic', style_class: 'mac-icon'}),
            can_focus: true,
        });
        backBtn.connect('clicked', () => this._showView('main'));
        header.add_child(backBtn);

        const title = new St.Label({text: 'Bluetooth', style_class: 'mac-sub-title', x_expand: true});
        header.add_child(title);

        this._btSwitch = createAppleSwitch(false, on => {
            this._setBtEnabled(on);
        });
        header.add_child(this._btSwitch);
        p.add_child(header);

        const card = new St.BoxLayout({vertical: true, style_class: 'mac-sub-card'});
        card.add_child(new St.Label({text: 'Devices', style_class: 'mac-sub-section-title'}));

        this._btScroll = new St.ScrollView({
            hscrollbar_policy: St.PolicyType.NEVER,
            vscrollbar_policy: St.PolicyType.AUTOMATIC,
            style_class: 'mac-scroll-area',
            x_expand: true,
            y_expand: true,
        });
        this._btList = new St.BoxLayout({vertical: true, style: 'spacing: 4px;', x_expand: true});
        this._btScroll.set_child(this._btList);
        card.add_child(this._btScroll);
        p.add_child(card);

        const settingsBtn = new St.Button({
            style_class: 'mac-bottom-action-btn',
            label: 'Bluetooth Settings...',
            can_focus: true,
        });
        settingsBtn.connect('clicked', () => {
            this._qs.menu.close();
            launch('gnome-control-center bluetooth');
        });
        p.add_child(settingsBtn);
    }

    _setBtEnabled(enable) {
        const args = ['org.bluez', '/org/bluez/hci0', 'org.bluez.Adapter1', 'Powered'];
        propSet(Gio.DBus.system, ...args, new GLib.Variant('b', enable));
        later(350, () => {
            this._refreshBluetoothList();
            this._refresh();
        });
    }

    _refreshBluetoothList() {
        if (!this._btList || !this._btSwitch) return;

        // Show searching indicator
        this._btList.remove_all_children();
        const scanBox = new St.BoxLayout({y_align: Clutter.ActorAlign.CENTER, style: 'spacing: 8px; padding: 14px;'});
        scanBox.add_child(new St.Icon({icon_name: 'view-refresh-symbolic', style_class: 'mac-icon'}));
        scanBox.add_child(new St.Label({text: 'Searching for devices...', style_class: 'mac-sub'}));
        this._btList.add_child(scanBox);

        // Check powered state
        const args = ['org.bluez', '/org/bluez/hci0', 'org.bluez.Adapter1', 'Powered'];
        propGet(Gio.DBus.system, ...args.slice(0, 3), args[3], on => {
            if (this._btSwitch)
                this._btSwitch.updateState(!!on);
        });

        // Trigger background discovery
        launch('bluetoothctl --timeout 4 scan on');

        // Query Bluez ObjectManager directly for all Bluetooth devices
        try {
            Gio.DBus.system.call(
                'org.bluez', '/', 'org.freedesktop.DBus.ObjectManager', 'GetManagedObjects',
                null, GLib.VariantType.new('(a{oa{sa{sv}}})'),
                NONE, -1, null, (c, r) => {
                    try {
                        const res = c.call_finish(r).get_child_value(0).deepUnpack();
                        if (!this._btList || !this._root) return;
                        this._btList.remove_all_children();

                        const devices = [];
                        for (const [path, ifaces] of Object.entries(res)) {
                            const dev = ifaces['org.bluez.Device1'];
                            if (!dev) continue;

                            const name = dev['Alias']?.unpack() || dev['Name']?.unpack() || dev['Address']?.unpack() || 'Device';
                            const connected = dev['Connected']?.unpack() ?? false;
                            const mac = dev['Address']?.unpack() || '';
                            devices.push({ name, connected, mac, path });
                        }

                        // Sort connected first
                        devices.sort((a, b) => {
                            if (a.connected) return -1;
                            if (b.connected) return 1;
                            return a.name.localeCompare(b.name);
                        });

                        for (const d of devices) {
                            const item = new St.Button({
                                style_class: `mac-item-btn ${d.connected ? 'active' : ''}`,
                                can_focus: true,
                                x_expand: true,
                            });
                            const row = new St.BoxLayout({y_align: Clutter.ActorAlign.CENTER, style: 'spacing: 10px;', x_expand: true});

                            const iconCircle = new St.Bin({
                                style_class: `mac-item-icon-circle ${d.connected ? 'connected' : ''}`,
                                x_align: Clutter.ActorAlign.CENTER,
                                y_align: Clutter.ActorAlign.CENTER,
                            });
                            iconCircle.set_child(icon('mc:bluetooth', 'mac-small'));
                            row.add_child(iconCircle);

                            const nameLabel = new St.Label({
                                text: d.name,
                                style_class: 'mac-item-name',
                                x_expand: true,
                                y_align: Clutter.ActorAlign.CENTER,
                            });
                            nameLabel.clutter_text.ellipsize = Pango.EllipsizeMode.END;
                            row.add_child(nameLabel);

                            if (d.connected) {
                                row.add_child(new St.Label({
                                    text: 'Connected',
                                    style_class: 'mac-sub',
                                    style: 'color: #007AFF; font-weight: 600;',
                                    y_align: Clutter.ActorAlign.CENTER,
                                }));
                            }

                            item.set_child(row);
                            item.connect('clicked', () => {
                                if (d.connected) {
                                    launch(`bluetoothctl disconnect ${d.mac}`);
                                } else {
                                    launch(`bluetoothctl connect ${d.mac}`);
                                }
                                later(1500, () => this._refreshBluetoothList());
                            });
                            this._btList.add_child(item);
                        }

                        if (devices.length === 0) {
                            this._btList.add_child(new St.Label({
                                text: 'No devices found',
                                style_class: 'mac-sub',
                                style: 'padding: 16px; text-align: center;',
                            }));
                        }
                    } catch (e) {
                        console.warn(`macos-cc: bluetooth DBus error: ${e}`);
                    }
                }
            );
        } catch (e) {
            console.warn(`macos-cc: bluetooth call error: ${e}`);
        }
    }

    /* Actions */
    _toggleWifi() {
        propGet(Gio.DBus.system, 'org.freedesktop.NetworkManager', '/org/freedesktop/NetworkManager',
            'org.freedesktop.NetworkManager', 'WirelessEnabled', v => {
                if (v === null)
                    return;
                this._setWifiEnabled(!v);
            });
    }

    _toggleBt() {
        const args = ['org.bluez', '/org/bluez/hci0', 'org.bluez.Adapter1', 'Powered'];
        propGet(Gio.DBus.system, ...args.slice(0, 3), args[3], v => {
            if (v === null)
                return;
            this._setBtEnabled(!v);
        });
    }

    _toggleNight() {
        if (!this._colorSettings)
            return;
        try {
            this._colorSettings.set_boolean('night-light-enabled',
                !this._colorSettings.get_boolean('night-light-enabled'));
            this._refresh();
        } catch (e) {}
    }

    _toggleDark() {
        if (!this._ifaceSettings) return;
        try {
            const currentDark = this._ifaceSettings.get_string('color-scheme') === 'prefer-dark';
            const nextDark = !currentDark;
            const newScheme = nextDark ? 'prefer-dark' : 'default';
            this._ifaceSettings.set_string('color-scheme', newScheme);

            try {
                this._ifaceSettings.set_string('gtk-theme', nextDark ? 'MacTahoe-Dark' : 'MacTahoe-Light');
                this._ifaceSettings.set_string('icon-theme', nextDark ? 'MacTahoe-blue-dark' : 'MacTahoe-blue-light');
            } catch (e) {}

            try {
                const userTheme = new Gio.Settings({schema_id: 'org.gnome.shell.extensions.user-theme'});
                userTheme.set_string('name', nextDark ? 'MacTahoe-Dark' : 'MacTahoe-Light');
            } catch (e) {}

            this._updateThemeContrast();
            this._refresh();
        } catch (e) {}
    }

    _toggleDnd() {
        if (!this._notifSettings) return;
        try {
            this._notifSettings.set_boolean('show-banners', !this._notifSettings.get_boolean('show-banners'));
            this._refresh();
        } catch (e) {}
    }

    /* State Synchronization */
    _refresh() {
        this._building = true;

        // Wi-Fi Status & Subtitle (SSID name or Off)
        propGet(Gio.DBus.system, 'org.freedesktop.NetworkManager', '/org/freedesktop/NetworkManager',
            'org.freedesktop.NetworkManager', 'WirelessEnabled', on => {
                const isEnabled = !!on;
                if (this._wifi?._bubble) {
                    setOn(this._wifi._bubble, isEnabled);
                }
                if (!isEnabled) {
                    if (this._wifi?._sub) this._wifi._sub.text = 'Off';
                } else {
                    try {
                        const proc = Gio.Subprocess.new(
                            ['nmcli', '-t', '-f', 'IN-USE,SSID', 'dev', 'wifi'],
                            Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_SILENCE
                        );
                        proc.communicate_utf8_async(null, null, (p, res) => {
                            try {
                                const [, stdout] = p.communicate_utf8_finish(res);
                                const lines = (stdout || '').split('\n');
                                let activeSSID = null;
                                for (const line of lines) {
                                    if (line.startsWith('*:')) {
                                        activeSSID = line.slice(2).trim();
                                        break;
                                    }
                                }
                                if (this._wifi?._sub) {
                                    this._wifi._sub.text = activeSSID || 'On';
                                }
                            } catch (e) {
                                if (this._wifi?._sub) this._wifi._sub.text = 'On';
                            }
                        });
                    } catch (e) {
                        if (this._wifi?._sub) this._wifi._sub.text = 'On';
                    }
                }
            });

        // Bluetooth Status & Subtitle (Connected Device Name or Off)
        propGet(Gio.DBus.system, 'org.bluez', '/org/bluez/hci0', 'org.bluez.Adapter1', 'Powered', on => {
            const isEnabled = !!on;
            if (this._bt?._bubble) {
                setOn(this._bt._bubble, isEnabled);
            }
            if (!isEnabled) {
                if (this._bt?._sub) this._bt._sub.text = 'Off';
            } else {
                try {
                    Gio.DBus.system.call(
                        'org.bluez', '/', 'org.freedesktop.DBus.ObjectManager', 'GetManagedObjects',
                        null, GLib.VariantType.new('(a{oa{sa{sv}}})'),
                        NONE, -1, null, (c, r) => {
                            try {
                                const res = c.call_finish(r).get_child_value(0).deepUnpack();
                                let connectedName = null;
                                for (const [, ifaces] of Object.entries(res)) {
                                    const dev = ifaces['org.bluez.Device1'];
                                    if (dev && dev['Connected']?.unpack()) {
                                        connectedName = dev['Alias']?.unpack() || dev['Name']?.unpack() || 'Connected';
                                        break;
                                    }
                                }
                                if (this._bt?._sub) {
                                    this._bt._sub.text = connectedName || 'On';
                                }
                            } catch (e) {
                                if (this._bt?._sub) this._bt._sub.text = 'On';
                            }
                        }
                    );
                } catch (e) {
                    if (this._bt?._sub) this._bt._sub.text = 'On';
                }
            }
        });

        const night = this._colorSettings?.get_boolean('night-light-enabled') ?? false;
        if (this._night?._bubble) {
            setOn(this._night._bubble, night);
            this._night._sub.text = night ? 'On' : 'Off';
        }

        const dnd = this._notifSettings ? !this._notifSettings.get_boolean('show-banners') : false;
        if (this._dnd?._bubble) {
            setOn(this._dnd._bubble, dnd);
            this._dnd._sub.text = dnd ? 'On' : 'Off';
        }
        
        const dark = this._ifaceSettings?.get_string('color-scheme') === 'prefer-dark';
        if (this._dark) {
            setOn(this._dark, dark);
        }

        const bl = findBacklight();
        const blMax = bl ? readInt(`${BL_DIR}/${bl}/max_brightness`) : null;
        const blCur = bl ? readInt(`${BL_DIR}/${bl}/brightness`) : null;
        if (bl && blMax && blCur !== null) {
            if (this._display?.slider)
                this._display.slider.value = Math.min(1, blCur / blMax);
        } else {
            propGet(Gio.DBus.session, 'org.gnome.SettingsDaemon.Power', '/org/gnome/SettingsDaemon/Power',
                'org.gnome.SettingsDaemon.Power.Screen', 'Brightness', b => {
                    this._building = true;
                    if (b !== null && b >= 0 && this._display?.slider)
                        this._display.slider.value = b / 100;
                    this._building = false;
                });
        }

        try {
            this._mixer ??= Volume.getMixerControl();
            const sink = this._mixer.get_default_sink();
            if (sink && this._sound?.slider)
                this._sound.slider.value = Math.min(1, sink.volume / this._mixer.get_vol_max_norm());
        } catch (e) {
            console.warn(`macos-cc: mixer unavailable: ${e}`);
        }

        this._building = false;
        this._refreshMedia();
    }

    _refreshMedia() {
        findPlayer(name => {
            if (!this._mainPanel || !this._mediaTitle || !this._playBtn?.child)
                return;
            if (!name) {
                this._mediaTitle.text = 'Not Playing';
                this._playBtn.child.gicon = symbol('play');
                this._setArt(null);
                return;
            }
            const get = prop => new Promise(res => Gio.DBus.session.call(
                name, MPRIS_PATH, 'org.freedesktop.DBus.Properties', 'Get',
                new GLib.Variant('(ss)', [PLAYER_IFACE, prop]), GLib.VariantType.new('(v)'),
                NONE, -1, null, (c, r) => {
                    try {
                        res(c.call_finish(r).get_child_value(0).get_variant());
                    } catch (e) {
                        res(null);
                    }
                }));
            Promise.all([get('PlaybackStatus'), get('Metadata')]).then(([st, md]) => {
                if (!this._mainPanel || !this._mediaTitle || !this._playBtn?.child)
                    return;
                const playing = st?.get_string()[0] === 'Playing';
                this._playBtn.child.gicon = symbol(playing ? 'pause' : 'play');
                try {
                    const m = md.deepUnpack();
                    this._mediaTitle.text = m['xesam:title']?.deepUnpack() || 'Not Playing';
                    this._setArt(m['mpris:artUrl']?.deepUnpack() ?? null);
                } catch (e) {
                    this._mediaTitle.text = 'Not Playing';
                    this._setArt(null);
                }
            });
        });
    }
}
