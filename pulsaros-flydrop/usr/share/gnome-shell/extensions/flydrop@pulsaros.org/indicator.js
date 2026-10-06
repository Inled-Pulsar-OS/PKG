import GObject from 'gi://GObject';
import St from 'gi://St';
import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';

import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const FLYDROP_BUS = 'es.pulsaros.FlyDrop';
const FLYDROP_PATH = '/es/pulsaros/FlyDrop';
const FLYDROP_IFACE = 'es.pulsaros.FlyDrop';

export const FlyDropIndicator = GObject.registerClass(
class FlyDropIndicator extends PanelMenu.Button {
    _init() {
        super._init(0.0, 'FlyDrop Indicator');

        // Only the icon in the top bar (clean AirDrop style, no text)
        let gicon = null;
        try {
            const ext = Extension.lookupByUUID('flydrop@pulsaros.org');
            const iconPath = ext ? `${ext.path}/icons/flydrop-symbolic.svg` : '/usr/share/icons/hicolor/scalable/apps/flydrop-symbolic.svg';
            const iconFile = Gio.File.new_for_path(iconPath);
            if (iconFile.query_exists(null)) {
                gicon = new Gio.FileIcon({ file: iconFile });
            }
        } catch (e) {
            console.log(`[FlyDrop] Error loading custom icon: ${e.message}`);
        }

        if (gicon) {
            this._icon = new St.Icon({
                gicon: gicon,
                style_class: 'system-status-icon',
                icon_size: 16,
            });
        } else {
            this._icon = new St.Icon({
                icon_name: 'flydrop-symbolic',
                style_class: 'system-status-icon',
                icon_size: 16,
            });
        }
        this.add_child(this._icon);

        // Build Popup Menu
        this._buildMenu();

        // Connect D-Bus
        this._initDBus();
    }

    _buildMenu() {
        // 1. Header Info (Device Name & Status)
        this._headerItem = new PopupMenu.PopupMenuItem('FlyDrop: Activo', {
            reactive: false,
        });
        this.menu.addMenuItem(this._headerItem);

        this._deviceInfoItem = new PopupMenu.PopupMenuItem('Visible como: Pulsar', {
            reactive: false,
        });
        this.menu.addMenuItem(this._deviceInfoItem);

        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        // 2. Direct Downloads / Auto Accept Switch
        this._autoAcceptSwitch = new PopupMenu.PopupSwitchMenuItem(
            'Aceptar descargas directas',
            false
        );
        this._autoAcceptSwitch.connect('toggled', (item, state) => {
            this._setDBusConfig('auto_accept', state);
        });
        this.menu.addMenuItem(this._autoAcceptSwitch);

        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        // 3. Action: Change Name / Settings
        const settingsItem = new PopupMenu.PopupMenuItem('Cambiar nombre del equipo...');
        settingsItem.connect('activate', () => {
            this._callDBusMethod('OpenSettingsDialog');
        });
        this.menu.addMenuItem(settingsItem);

        // 4. Action: Open Downloads Folder
        const downloadsItem = new PopupMenu.PopupMenuItem('Abrir carpeta de descargas');
        downloadsItem.connect('activate', () => {
            this._callDBusMethod('OpenDownloadsFolder');
        });
        this.menu.addMenuItem(downloadsItem);

        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        // 5. Discovered devices summary
        this._devicesSummaryItem = new PopupMenu.PopupMenuItem('Buscando dispositivos...', {
            reactive: false,
        });
        this.menu.addMenuItem(this._devicesSummaryItem);
    }

    _initDBus() {
        try {
            this._dbusProxy = new Gio.DBusProxy({
                g_bus_type: Gio.BusType.SESSION,
                g_name: FLYDROP_BUS,
                g_object_path: FLYDROP_PATH,
                g_interface_name: FLYDROP_IFACE,
            });

            this._dbusProxy.init_async(GLib.PRIORITY_DEFAULT, null, (proxy, res) => {
                try {
                    proxy.init_finish(res);
                    this._refreshState();
                    this._subscribeSignals();
                } catch (e) {
                    console.log(`[FlyDrop] DBus Proxy init error: ${e.message}`);
                }
            });
        } catch (e) {
            console.log(`[FlyDrop] Error connecting to DBus: ${e.message}`);
        }
    }

    _subscribeSignals() {
        if (!this._dbusProxy) return;

        this._dbusProxy.connect('g-signal', (proxy, sender, signalName, params) => {
            if (signalName === 'DeviceFound' || signalName === 'DeviceLost') {
                this._refreshDevices();
            }
        });

        // Periodic refresh of state
        this._refreshTimer = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 3, () => {
            this._refreshState();
            return GLib.SOURCE_CONTINUE;
        });
    }

    _refreshState() {
        if (!this._dbusProxy) return;

        this._dbusProxy.call(
            'GetConfig',
            null,
            Gio.DBusCallFlags.NONE,
            -1,
            null,
            (proxy, res) => {
                try {
                    const result = proxy.call_finish(res);
                    const configJson = result.deep_unpack()[0];
                    const cfg = JSON.parse(configJson);

                    if (cfg.alias) {
                        this._deviceInfoItem.label.text = `Visible como: ${cfg.alias}`;
                    }
                    if (cfg.auto_accept !== undefined) {
                        this._autoAcceptSwitch.setToggleState(Boolean(cfg.auto_accept));
                    }
                } catch (e) {}
            }
        );

        this._refreshDevices();
    }

    _refreshDevices() {
        if (!this._dbusProxy) return;

        this._dbusProxy.call(
            'GetDiscoveredDevices',
            null,
            Gio.DBusCallFlags.NONE,
            -1,
            null,
            (proxy, res) => {
                try {
                    const result = proxy.call_finish(res);
                    const devJson = result.deep_unpack()[0];
                    const devices = JSON.parse(devJson);
                    const count = devices.length;

                    if (count === 0) {
                        this._devicesSummaryItem.label.text = 'Sin dispositivos cercanos';
                    } else if (count === 1) {
                        this._devicesSummaryItem.label.text = `1 dispositivo: ${devices[0].alias}`;
                    } else {
                        this._devicesSummaryItem.label.text = `${count} dispositivos detectados`;
                    }
                } catch (e) {}
            }
        );
    }

    _setDBusConfig(key, value) {
        if (!this._dbusProxy) return;
        const valStr = JSON.stringify(value);
        this._dbusProxy.call(
            'SetConfig',
            new GLib.Variant('(ss)', [key, valStr]),
            Gio.DBusCallFlags.NONE,
            -1,
            null,
            null
        );
    }

    _callDBusMethod(methodName) {
        if (!this._dbusProxy) return;
        this._dbusProxy.call(
            methodName,
            null,
            Gio.DBusCallFlags.NONE,
            -1,
            null,
            null
        );
    }

    destroy() {
        if (this._refreshTimer) {
            GLib.source_remove(this._refreshTimer);
            this._refreshTimer = null;
        }
        super.destroy();
    }
});
