import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import Clutter from 'gi://Clutter';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import { NotchNux } from './src/notchnux.js';
import { ConfigStore } from './src/helpers/config.js';

export default class NotchNuxExtension extends Extension {
    enable() {
        console.log('NotchNux: Enabling PulsarOS build...');

        this._notches = [];
        this._monitorRebuildId = 0;
        this._helperBusId = 0;
        this._helperProc = null;
        this._pendingDropZonePos = null;
        this._pendingNotch = null;
        this._pendingCardTarget = null;
        this._lastZoneBox = null;
        this._zoneLastShow = 0;
        this._winCreatedId = 0;
        this._notchesReactiveOff = false;
        this._dragEndId = 0;
        this._externalDragActive = false;
        this._companionDragActive = false;
        this._config = new ConfigStore();
        this._currentDisplayMode = this._config.displayMonitor;

        // Wire DnD helper if available
        this._dbusConnection = Gio.bus_get_sync(Gio.BusType.SESSION, null);
        this._setupExtensionDbus();
        this._startDropHelper();

        // Restore notch reactivity when an external drag ends
        this._dragEndId = Main.xdndHandler?.connect?.('drag-end', () => {
            this._externalDragActive = false;
            this._companionDragActive = false;
            if (this._notchesReactiveOff) {
                this._notchesReactiveOff = false;
                this._setNotchesReactive(true);
            }
        }) ?? 0;

        this._winCreatedId = global.display.connect('window-created', (_d, win) => {
            try {
                if ((this._pendingDropZonePos || this._pendingCardTarget) &&
                    (win.get_wm_class() === 'es.pulsaros.NotchNuxHelper' || win.get_gtk_application_id?.() === 'es.pulsaros.NotchNuxHelper' || String(win.get_wm_class() ?? '').includes('NotchNux')))
                    this._applyDropZonePos(win);
            } catch (_) {}
        });

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

        if (this._winCreatedId) {
            global.display.disconnect(this._winCreatedId);
            this._winCreatedId = 0;
        }

        if (this._dragEndId) {
            try { Main.xdndHandler?.disconnect?.(this._dragEndId); } catch (_) {}
            this._dragEndId = 0;
        }
        this._pendingNotch = null;
        this._pendingCardTarget = null;
        this._lastZoneBox = null;
        this._hideDragCards();
        if (this._notchesReactiveOff) {
            this._notchesReactiveOff = false;
            this._setNotchesReactive(true);
        }

        this._destroyNotches();

        if (Main.panel?.statusArea?.dateMenu)
            Main.panel.statusArea.dateMenu.show();

        try {
            if (this._helperProc) {
                this._helperProc.force_exit();
                this._helperProc = null;
            }
        } catch (_) {}
        this._pendingDropZonePos = null;
        this._zoneLastShow = 0;
        this._externalDragActive = false;
        this._companionDragActive = false;
        try {
            if (this._helperBusId > 0 && this._dbusConnection) {
                this._dbusConnection.unregister_object(this._helperBusId);
            }
        } catch (_) {}
        this._helperBusId = 0;
        this._dbusConnection = null;
    }

    _setupExtensionDbus() {
        try {
            let xml = '<node><interface name="org.gnome.Shell.Extensions.NotchNux"><method name="StageFiles"><arg type="as" direction="in" name="uris"/></method><method name="CompanionDragState"><arg type="b" direction="in" name="active"/></method><method name="ForwardClick"><arg type="i" direction="in" name="x"/><arg type="i" direction="in" name="y"/><arg type="d" direction="in" name="button"/></method><method name="ForwardScroll"><arg type="d" direction="in" name="dx"/><arg type="d" direction="in" name="dy"/></method></interface></node>';
            let nodeInfo = Gio.DBusNodeInfo.new_for_xml(xml);
            this._helperBusId = this._dbusConnection.register_object('/org/gnome/Shell/Extensions/NotchNux', nodeInfo.interfaces[0], (conn, sender, path, iface, method, params, inv) => {
                if (method === 'StageFiles') {
                    let [urisVariant] = params.unpack();
                    let uris = (urisVariant && typeof urisVariant.deep_unpack === 'function')
                        ? urisVariant.deep_unpack() : urisVariant;
                    if (!Array.isArray(uris)) uris = uris ? [uris] : [];
                    this._stageFilesFromHelper(uris);
                    inv.return_value(new GLib.Variant('()', []));
                } else if (method === 'CompanionDragState') {
                    try {
                        let [active] = params.unpack();
                        this._companionDragActive = !!active;
                    } catch (_) {}
                    inv.return_value(new GLib.Variant('()', []));
                } else if (method === 'ForwardClick') {
                    try {
                        let [x, y, btn] = params.unpack();
                        this._handleForwardClick(x, y, btn);
                    } catch (e) { console.warn('NotchNux: ForwardClick error', e); }
                    inv.return_value(new GLib.Variant('()', []));
                } else if (method === 'ForwardScroll') {
                    try {
                        let [dx, dy] = params.unpack();
                        this._handleForwardScroll(dx, dy);
                    } catch (e) { console.warn('NotchNux: ForwardScroll error', e); }
                    inv.return_value(new GLib.Variant('()', []));
                } else {
                    inv.return_error_literal(Gio.dbus_error_quark(), Gio.DBUS_ERROR_UNKNOWN_METHOD, 'Unknown');
                }
            }, null, null);
        } catch (e) { console.warn('NotchNux: D-Bus reg failed', e); }
    }

    _handleForwardClick(winX, winY, button) {
        let box = this._lastZoneBox;
        let rootX = box ? box.x + winX : winX;
        let rootY = box ? box.y + winY : winY;

        let actor = global.stage.get_actor_at_pos(Clutter.PickMode.ALL, rootX, rootY);
        if (!actor) return;

        let target = actor;
        while (target && target !== global.stage) {
            if (typeof target.clicked === 'function') {
                if (target.can_focus) target.grab_key_focus?.();
                target.clicked(0);
                return;
            }
            if (typeof target.emit === 'function' && target.reactive) {
                if (target.can_focus) target.grab_key_focus?.();
                target.emit('clicked', 0);
                return;
            }
            if (target._delegate && typeof target._delegate.activate === 'function') {
                target._delegate.activate();
                return;
            }
            target = target.get_parent?.();
        }
    }

    _handleForwardScroll(dx, dy) {
        for (let notch of this._notches ?? []) {
            if (notch && notch._shelfScroll && notch._shelfScroll.get_vscroll_bar) {
                try {
                    let adj = notch._shelfScroll.get_vscroll_bar().get_adjustment();
                    if (adj) {
                        adj.value = Math.max(adj.lower, Math.min(adj.upper - adj.page_size, adj.value + dy * 35));
                    }
                } catch (_) {}
            }
        }
    }

    _stageFilesFromHelper(uris) {
        let staged = 0;
        for (let n of (this._notches || [])) {
            try {
                if (n._shelf?.addFile) {
                    for (let u of uris) { if (n._shelf.addFile(u)) staged++; }
                }
            } catch (e) { console.error('NotchNux: StageFiles failed', e); }
        }
        console.log(`NotchNux: StageFiles: received ${uris.length} uri(s), staged ${staged}`);
        try {
            if (this._notches && this._notches[0]) {
                let nn = this._notches[0];
                if (nn._flashShareStatus) nn._flashShareStatus(`Staged ${staged} file(s) in Shelf`);
                nn._activeTab = 'shelf';
                if (!nn.isExpanded) nn.expand(); else if (nn._renderActiveTab) nn._renderActiveTab();
            }
        } catch (_) {}
    }

    _helperHasOwner() {
        let conn = this._dbusConnection ?? Gio.bus_get_sync(Gio.BusType.SESSION, null);
        let [hasOwner] = conn.call_sync('org.freedesktop.DBus', '/org/freedesktop/DBus',
            'org.freedesktop.DBus', 'NameHasOwner',
            new GLib.Variant('(s)', ['es.pulsaros.NotchNuxHelper']),
            new GLib.VariantType('(b)'), Gio.DBusCallFlags.NONE, 2000, null).deep_unpack();
        return !!hasOwner;
    }

    _spawnDropHelper(hidden) {
        let p = Gio.File.new_for_path('/usr/bin/notchnux-drop-helper');
        if (!p.query_exists(null)) return false;
        let args = [p.get_path(), '--invisible'];
        if (hidden) args.push('--hidden');
        this._helperProc = Gio.Subprocess.new(args, Gio.SubprocessFlags.NONE);
        return true;
    }

    _startDropHelper() {
        try {
            if (this._helperHasOwner()) return;
            this._spawnDropHelper(true);
        } catch (e) { console.warn('NotchNux: helper start failed', e); }
    }

    _showDropZone(x, y, notch) {
        this._externalDragActive = true;
        let now = GLib.get_monotonic_time();
        if (this._zoneLastShow && now - this._zoneLastShow < 700 * 1000) return;
        this._zoneLastShow = now;
        this._pendingDropZonePos = { x, y };
        this._pendingNotch = notch ?? null;
        this._pendingCardTarget = null;

        if (!this._notchesReactiveOff) {
            this._notchesReactiveOff = true;
            this._setNotchesReactive(false);
        }

        this._activateDropZone();

        this._positionDropZone();
        for (let ms of [60, 240, 500, 900]) {
            GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => {
                if (this._pendingDropZonePos) {
                    this._positionDropZone();
                    if (this._notchesReactiveOff) this._setNotchesReactive(false);
                }
                return GLib.SOURCE_REMOVE;
            });
        }
    }

    _activateDropZone() {
        let conn;
        try {
            conn = this._dbusConnection ?? Gio.bus_get_sync(Gio.BusType.SESSION, null);
        } catch (e) {
            console.warn('NotchNux: no session bus', e);
            return;
        }
        let box = this._pendingNotch ? this._notchZoneBox(this._pendingNotch) : null;
        let uri = box ? `notchnux://zone?w=${box.w}&h=${box.h}` : 'notchnux://zone';
        try {
            conn.call('org.freedesktop.DBus', '/org/freedesktop/DBus',
                'org.freedesktop.DBus', 'NameHasOwner',
                new GLib.Variant('(s)', ['es.pulsaros.NotchNuxHelper']),
                new GLib.VariantType('(b)'), Gio.DBusCallFlags.NONE, 2000, null,
                (c, res) => {
                    let owned = false;
                    try { owned = c.call_finish(res).deep_unpack()[0]; } catch (_) { return; }
                    if (!this._dbusConnection) return;
                    if (!owned) {
                        this._spawnDropHelper(false);
                        return;
                    }
                    try {
                        conn.call('es.pulsaros.NotchNuxHelper', '/es/pulsaros/NotchNuxHelper',
                            'org.freedesktop.Application', 'Open',
                            new GLib.Variant('(asa{sv})', [[uri], {}]), null,
                            Gio.DBusCallFlags.NONE, 3000, null,
                            (c2, res2) => {
                                try { c2.call_finish(res2); }
                                catch (e) { console.warn('NotchNux: drop zone activation failed', e); }
                            });
                    } catch (e) { console.warn('NotchNux: drop zone activation failed', e); }
                });
        } catch (e) { console.warn('NotchNux: drop zone activation failed', e); }
    }

    _positionDropZone() {
        if (!this._pendingDropZonePos && !this._pendingCardTarget) return;
        let win = null;
        try {
            for (let a of global.get_window_actors()) {
                let w = a.metaWindow ?? a.get_meta_window?.();
                if (w && (w.get_wm_class() === 'es.pulsaros.NotchNuxHelper' || w.get_gtk_application_id?.() === 'es.pulsaros.NotchNuxHelper' || String(w.get_wm_class() ?? '').includes('NotchNux'))) {
                    win = w;
                    break;
                }
            }
        } catch (_) {}
        if (win) this._applyDropZonePos(win);
    }

    _applyDropZonePos(win) {
        if (!win) return;
        try {
            let pos = this._pendingDropZonePos;
            let box, mon;
            if (this._pendingCardTarget) {
                box = { ...this._pendingCardTarget };
                mon = (Main.layoutManager.monitors ?? []).find(m =>
                    box.x + box.w / 2 >= m.x && box.x + box.w / 2 < m.x + m.width);
            } else {
                if (!pos) return;
                mon = (Main.layoutManager.monitors ?? []).find(m =>
                    pos.x >= m.x && pos.x < m.x + m.width &&
                    pos.y >= m.y && pos.y < m.y + m.height);
                let nn = this._pendingNotch;
                if (!nn || typeof nn.get_transformed_position !== 'function')
                    return;
                box = this._notchZoneBox(nn);
            }
            if (!mon) return;

            let zx = Math.max(mon.x + 8, Math.min(box.x, mon.x + mon.width - box.w - 8));
            let zy = Math.max(mon.y + 8, Math.min(box.y, mon.y + mon.height - box.h - 8));
            this._lastZoneBox = { x: zx, y: zy, w: box.w, h: box.h };
            win.move_resize_frame(global.get_current_time(), zx, zy, box.w, box.h);
            win.make_above();
        } catch (e) { console.warn('NotchNux: drop zone positioning failed', e); }
    }

    _notchZoneBox(notch) {
        let [nx, ny] = notch.get_transformed_position();
        let [nw, nh] = notch.get_transformed_size();
        return {
            x: Math.round(nx) - 40, y: Math.round(ny) - 40,
            w: Math.round(nw) + 80, h: Math.round(nh) + 80,
        };
    }

    _showCompanionCard(uris, notch) {
        if (!this._dbusConnection || !uris || !uris.length || !notch) return;
        if (!this._helperHasOwner()) {
            this._spawnDropHelper(false);
        }
        let box = this._notchZoneBox(notch);
        this._pendingCardTarget = box;
        this._lastZoneBox = box;
        this._positionDropZone();
        for (let ms of [40, 150, 350, 700]) {
            GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => {
                if (this._pendingCardTarget && !this._externalDragActive)
                    this._positionDropZone();
                return GLib.SOURCE_REMOVE;
            });
        }
        try {
            this._dbusConnection.call('es.pulsaros.NotchNuxHelper',
                '/es/pulsaros/NotchNuxHelper',
                'org.freedesktop.Application', 'Open',
                new GLib.Variant('(asa{sv})',
                    [[`notchnux://card?w=${box.w}&h=${box.h}`, ...uris], {}]), null,
                Gio.DBusCallFlags.NONE, 3000, null,
                (c, res) => {
                    try { c.call_finish(res); } catch (_) {}
                });
        } catch (_) {}
    }

    _hideDragCards() {
        this._pendingCardTarget = null;
        if (!this._dbusConnection) return;
        try {
            this._dbusConnection.call('es.pulsaros.NotchNuxHelper',
                '/es/pulsaros/NotchNuxHelper',
                'org.freedesktop.Application', 'Open',
                new GLib.Variant('(asa{sv})', [['notchnux://hide'], {}]), null,
                Gio.DBusCallFlags.NONE, 3000, null,
                (c, res) => {
                    try { c.call_finish(res); }
                    catch (_) {}
                });
        } catch (_) {}
    }

    _setNotchesReactive(reactive) {
        for (let n of this._notches ?? []) {
            try { this._setSubtreeReactive(n, reactive); }
            catch (e) { console.warn('NotchNux: set subtree reactive failed', e); }
        }
    }

    _setSubtreeReactive(actor, reactive) {
        if (!actor) return;
        actor.set_reactive?.(reactive);
        for (let c of actor.get_children?.() ?? [])
            this._setSubtreeReactive(c, reactive);
    }
}
