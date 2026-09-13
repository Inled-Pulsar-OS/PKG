import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';

import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';

const SELECTION_CLIPBOARD = 0; // MetaSelectionType.SELECTION_CLIPBOARD

// D-Bus interface of the island daemon (pulsar-island.service).
const ISLAND_DBUS_NAME = 'org.pulsaros.Island';
const ISLAND_DBUS_PATH = '/org/pulsaros/Island';
const ISLAND_DBUS_IFACE = `
<node>
  <interface name='org.pulsaros.Island'>
    <method name='ShowItem'>
      <arg type='s' name='kind' direction='in'/>
      <arg type='s' name='payload' direction='in'/>
    </method>
    <method name='Ping'>
      <arg type='s' name='pong' direction='out'/>
    </method>
  </interface>
</node>`;

const DBUS_PROXY_TIMEOUT_MS = 3000;

const IslandDBusClient = GObject.registerClass(
class IslandDBusClient extends GObject.Object {
    _init() {
        super._init();
        this._proxy = null;
        this._nodeInfo = Gio.DBusNodeInfo.new_for_xml(ISLAND_DBUS_IFACE);
        this._watchId = Gio.bus_watch_name(
            Gio.BusType.SESSION, ISLAND_DBUS_NAME, Gio.BusNameWatcherFlags.NONE,
            this._onNameAppeared.bind(this), this._onNameVanished.bind(this));
    }

    _onNameAppeared(connection, name) {
        this._proxy = new Gio.DBusProxy({
            g_connection: connection,
            g_interface_name: this._nodeInfo.interfaces[0].name,
            g_name: ISLAND_DBUS_NAME,
            g_object_path: ISLAND_DBUS_PATH,
            g_flags: Gio.DBusProxyFlags.DO_NOT_AUTO_START,
        });
        this._proxy.init_async(GLib.PRIORITY_DEFAULT, null, (_p, res) => {
            try {
                this._proxy.init_finish(res);
            } catch (e) {
                console.warn(`pulsar-island: proxy init failed: ${e.message}`);
                this._proxy = null;
            }
        });
    }

    _onNameVanished(_connection, _name) {
        this._proxy = null;
    }

    showItem(kind, payload) {
        if (!this._proxy)
            return;
        this._proxy.call(
            'ShowItem',
            new GLib.Variant('(ss)', [kind, payload]),
            Gio.DBusCallFlags.NONE, DBUS_PROXY_TIMEOUT_MS, null,
            (_proxy, res) => {
                try {
                    this._proxy.call_finish(res);
                } catch (e) {
                    console.warn(`pulsar-island: ShowItem failed: ${e.message}`);
                }
            });
    }

    destroy() {
        if (this._watchId) {
            Gio.bus_unwatch_name(this._watchId);
            this._watchId = 0;
        }
        this._proxy = null;
    }
});

// Monitors MetaSelection ownership changes on the clipboard. This works on
// both X11 and Wayland because it goes through Mutter's selection machinery.
const ClipboardMonitor = GObject.registerClass(
class ClipboardMonitor extends GObject.Object {
    _init(onItem, minSize) {
        super._init();
        this._onItem = onItem;
        this._minSize = minSize;
        this._selection = global.display.get_selection();
        this._ownerChangedId = this._selection.connect('owner-changed',
            (selection, selectionType, selectionSource) => this._ownerChanged(selectionType, selectionSource));
    }

    _ownerChanged(selectionType, selectionSource) {
        if (selectionType !== SELECTION_CLIPBOARD || !selectionSource)
            return;

        const cb = St.Clipboard.get_default();
        cb.get_content(St.ClipboardType.CLIPBOARD, (clipboard, content) => {
            if (content && content.get_mime_types().some(m => m.startsWith('image/'))) {
                this._readImageToTemp(content);
                return;
            }
            cb.get_text(St.ClipboardType.CLIPBOARD, (_c, text) => {
                if (text && text.trim().length >= this._minSize)
                    this._onItem('text', text);
            });
        });
    }

    _readImageToTemp(content) {
        const mimeTypes = content.get_mime_types();
        const mime = mimeTypes.find(m => m.startsWith('image/')) ?? 'image/png';
        const ext = mime === 'image/jpeg' ? 'jpg' : mime === 'image/gif' ? 'gif' : 'png';

        content.read_async(0, -1, null, (src, res) => {
            try {
                const stream = content.read_finish(res);
                const path = GLib.build_filenamev(
                    [GLib.get_tmp_dir(), `pulsar-island-${Date.now()}.${ext}`]);
                const outFile = Gio.File.new_for_path(path);
                stream.splice_async(outFile,
                    Gio.OutputStreamSpliceFlags.CLOSE_SOURCE | Gio.OutputStreamSpliceFlags.CLOSE_TARGET,
                    Gio.PRIORITY_DEFAULT, null, (_s, r) => {
                        try {
                            stream.splice_finish(r);
                            this._onItem('image', path);
                        } catch (e) {
                            console.warn(`pulsar-island: image read failed: ${e.message}`);
                        }
                    });
            } catch (e) {
                console.warn(`pulsar-island: image read failed: ${e.message}`);
            }
        });
    }

    destroy() {
        if (this._ownerChangedId) {
            this._selection.disconnect(this._ownerChangedId);
            this._ownerChangedId = 0;
        }
    }
});

// Watches the GNOME screenshot folder so a freshly taken screenshot is
// offered on the island.
const ScreenshotMonitor = GObject.registerClass(
class ScreenshotMonitor extends GObject.Object {
    _init(onItem) {
        super._init();
        this._onItem = onItem;

        const dir = Gio.File.new_for_path(
            GLib.build_filenamev([GLib.get_user_special_dir(GLib.UserDirectory.DIRECTORY_PICTURES), 'Screenshots']));
        try {
            this._monitor = dir.monitor_directory(Gio.FileMonitorFlags.NONE, null);
            this._monitor.connect('changed', (_m, file, _other, eventType) => {
                if (eventType !== Gio.FileMonitorEvent.CREATED || !file)
                    return;
                const name = file.get_basename().toLowerCase();
                if (!/\.(png|jpe?g)$/.test(name))
                    return;
                GLib.timeout_add(GLib.PRIORITY_DEFAULT, 700, () => {
                    this._onItem('image', file.get_path());
                    return GLib.SOURCE_REMOVE;
                });
            });
        } catch (e) {
            console.warn(`pulsar-island: screenshot monitor unavailable: ${e.message}`);
        }
    }

    destroy() {
        this._monitor?.cancel();
        this._monitor = null;
    }
});

export default class PulsarIslandExtension extends Extension {
    enable() {
        this._settings = this.getSettings();
        this._dbus = new IslandDBusClient();
        this._clipboardMonitor = null;
        this._screenshotMonitor = null;
        this._settingsId = this._settings.connect('changed', () => this._syncMonitors());
        this._syncMonitors();
    }

    _syncMonitors() {
        const clipboardEnabled = this._settings.get_boolean('clipboard-enabled');
        const screenshotEnabled = this._settings.get_boolean('screenshot-enabled');
        const minSize = this._settings.get_int('clipboard-min-size');

        if (clipboardEnabled && !this._clipboardMonitor) {
            this._clipboardMonitor = new ClipboardMonitor(
                (kind, payload) => this._dbus.showItem(kind, payload), minSize);
        } else if (!clipboardEnabled && this._clipboardMonitor) {
            this._clipboardMonitor.destroy();
            this._clipboardMonitor = null;
        }

        if (screenshotEnabled && !this._screenshotMonitor) {
            this._screenshotMonitor = new ScreenshotMonitor(
                (kind, payload) => this._dbus.showItem(kind, payload));
        } else if (!screenshotEnabled && this._screenshotMonitor) {
            this._screenshotMonitor.destroy();
            this._screenshotMonitor = null;
        }
    }

    disable() {
        if (this._settingsId) {
            this._settings.disconnect(this._settingsId);
            this._settingsId = 0;
        }
        this._clipboardMonitor?.destroy();
        this._clipboardMonitor = null;
        this._screenshotMonitor?.destroy();
        this._screenshotMonitor = null;
        this._dbus?.destroy();
        this._dbus = null;
        this._settings = null;
    }
}
