import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import { NotchNux } from './src/notchnux.js';
import { ConfigStore } from './src/helpers/config.js';

export default class NotchNuxExtension extends Extension {
    enable() {
        console.log('NotchNux: Enabling PulsarOS build...');

        this._notches = [];
        this._monitorRebuildId = 0;
        this._config = new ConfigStore();
        this._currentDisplayMode = this._config.displayMonitor;

        // Wire DnD helper if available
        this._dbusConnection = Gio.bus_get_sync(Gio.BusType.SESSION, null);
        this._setupExtensionDbus();
        this._startDropHelper();

        this._syncDateMenu();

        // Watch config file for monitor and settings changes
        let configFile = Gio.File.new_for_path(this._config.path);
        try {
            this._configMonitor = configFile.monitor_file(Gio.FileMonitorFlags.NONE, null);
            this._configMonitorId = this._configMonitor.connect('changed', (_mon, _file, _otherFile, eventType) => {
                if (eventType === Gio.FileMonitorEvent.CHANGES_DONE_HINT || eventType === Gio.FileMonitorEvent.CREATED) {
                    this._config.reload();
                    this._syncDateMenu();
                    if (this._currentDisplayMode !== this._config.displayMonitor) {
                        this._currentDisplayMode = this._config.displayMonitor;
                        this._queueRebuild();
                    }
                }
            });
        } catch (_) {}

        this._buildNotches();

        this._monitorsChangedId =
            Main.layoutManager.connect('monitors-changed', () => {
                this._queueRebuild();
            });
    }

    _queueRebuild() {
        if (this._monitorRebuildId) return;
        this._monitorRebuildId =
            GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, () => {
                this._monitorRebuildId = 0;
                this._buildNotches();
                return GLib.SOURCE_REMOVE;
            });
    }

    _destroyNotches() {
        for (let notch of this._notches ?? []) {
            try {
                Main.layoutManager.removeChrome(notch);
            } catch (e) {}

            try {
                notch.destroy();
            } catch (e) {
                console.error('NotchNux: failed to destroy monitor instance', e);
            }
        }

        this._notches = [];
    }

    _buildNotches() {
        this._destroyNotches();

        let monitors = Main.layoutManager.monitors ?? [];
        let primary = Main.layoutManager.primaryIndex;
        let mode = this._config.displayMonitor;

        let chromeOptions = {
            trackFullscreen: false
        };

        if (mode === 'all') {
            for (let i = 0; i < monitors.length; i++) {
                let notch = new NotchNux(this, i, i === primary);
                this._notches.push(notch);
                Main.layoutManager.addTopChrome(notch, chromeOptions);
            }
        } else {
            let targetIdx = primary;
            let parsedIdx = parseInt(mode, 10);
            if (!isNaN(parsedIdx) && parsedIdx >= 0 && parsedIdx < monitors.length) {
                targetIdx = parsedIdx;
            }
            let notch = new NotchNux(this, targetIdx, true);
            this._notches.push(notch);
            Main.layoutManager.addTopChrome(notch, chromeOptions);
        }

        console.log(`NotchNux: ${this._notches.length} monitor notch(es) active (mode: ${mode})`);
    }

    _syncDateMenu() {
        try {
            let hideDate = this._config.isFeatureEnabled('hideShellDateMenu');
            if (Main.panel?.statusArea?.dateMenu) {
                if (hideDate)
                    Main.panel.statusArea.dateMenu.hide();
                else
                    Main.panel.statusArea.dateMenu.show();
            }
        } catch (_) {}
    }

    disable() {
        if (this._monitorRebuildId) {
            GLib.Source.remove(this._monitorRebuildId);
            this._monitorRebuildId = 0;
        }

        if (this._configMonitorId && this._configMonitor) {
            this._configMonitor.disconnect(this._configMonitorId);
            this._configMonitorId = 0;
        }
        if (this._configMonitor) {
            this._configMonitor.cancel();
            this._configMonitor = null;
        }

        if (this._monitorsChangedId) {
            Main.layoutManager.disconnect(this._monitorsChangedId);
            this._monitorsChangedId = 0;
        }

        this._destroyNotches();

        if (Main.panel?.statusArea?.dateMenu)
            Main.panel.statusArea.dateMenu.show();

        try {
            if (this._helperProxy) {
                try { this._helperProxy.call_sync('Exit', null, Gio.DBusCallFlags.NONE, -1, null); } catch (_) {}
            }
        } catch (_) {}
        try {
            if (this._helperBusId > 0 && this._dbusConnection) {
                this._dbusConnection.unregister_object(this._helperBusId);
            }
        } catch (_) {}
        this._helperBusId = 0;
        this._helperProxy = null;
        this._dbusConnection = null;

    _setupExtensionDbus() {
        try {
            let xml = '<node><interface name="org.gnome.Shell.Extensions.NotchNux"><method name="StageFiles"><arg type="as" direction="in" name="uris"/></method></interface></node>';
            let nodeInfo = Gio.DBusNodeInfo.new_for_xml(xml);
            this._dbusConnection.register_object('/org/gnome/Shell/Extensions/NotchNux', nodeInfo.interfaces[0], (conn, sender, path, iface, method, params, inv) => {
                if (method === 'StageFiles') {
                    let [uris] = params.unpack();
                    this._stageFilesFromHelper(uris);
                    inv.return_value(GLib.Variant('()'));
                } else {
                    inv.return_error_literal(Gio.dbus_error_quark(), Gio.DBUS_ERROR_UNKNOWN_METHOD, 'Unknown');
                }
            }, null, null);
        } catch (e) { console.warn('NotchNux: D-Bus reg failed', e); }
    }

    _stageFilesFromHelper(uris) {
        for (let n of (this._notches || [])) {
            try { if (n._shelf && n._shelf.addFiles) n._shelf.addFiles(uris); else if (n._shelf && n._shelf.addFile) { for (let u of uris) n._shelf.addFile(u); } } catch (_) {}
        }
        try {
            if (this._notches && this._notches[0]) {
                let nn = this._notches[0];
                if (nn._flashShareStatus) nn._flashShareStatus(`Staged ${uris.length} file(s) in Shelf`);
                nn._activeTab = 'shelf';
                if (!nn.isExpanded) nn.expand(); else if (nn._renderActiveTab) nn._renderActiveTab();
            }
        } catch (_) {}
    }

    _startDropHelper() {
        try {
            Gio.AppInfo.launch_default_for_uri('app://es.pulsaros.NotchNuxHelper', null);
        } catch (_) {
            try {
                let p = Gio.File.new_for_path('/usr/bin/notchnux-drop-helper');
                if (p.query_exists(null)) { let proc = Gio.Subprocess.new([p.get_path()], Gio.SubprocessFlags.NONE); proc.launch(null, null); }
            } catch (e) { console.warn('NotchNux: helper start failed', e); }
        }
        try {
            this._helperProxy = Gio.DBusProxy.new_for_bus_sync(Gio.BusType.SESSION, Gio.DBusProxyFlags.DO_NOT_AUTO_START, null, 'es.pulsaros.NotchNuxHelper', '/es/pulsaros/NotchNuxHelper', 'es.pulsaros.NotchNuxHelper', null);
        } catch (_) {}
    }
}
