import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import Clutter from 'gi://Clutter';
import * as St from 'gi://St';
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

        // Restore notch reactivity when an external drag ends (unless shelf companion card is active)
        this._dragEndId = Main.xdndHandler?.connect?.('drag-end', () => {
            this._externalDragActive = false;
            this._companionDragActive = false;
            if (this._notchesReactiveOff && !this._pendingCardTarget) {
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
            trackFullscreen: false,
            affectsStruts: false
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
            let xml = '<node><interface name="org.gnome.Shell.Extensions.NotchNux"><method name="StageFiles"><arg type="as" direction="in" name="uris"/></method><method name="CompanionDragState"><arg type="b" direction="in" name="active"/></method><method name="ForwardClick"><arg type="i" direction="in" name="x"/><arg type="i" direction="in" name="y"/><arg type="d" direction="in" name="button"/></method><method name="ForwardScroll"><arg type="d" direction="in" name="dx"/><arg type="d" direction="in" name="dy"/></method><method name="PointerLeave"/><method name="DropZoneClosed"/></interface></node>';
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
                } else if (method === 'PointerLeave') {
                    for (let n of (this._notches || [])) {
                        if (n && n.isExpanded && !n._anyOwnedMenuOpen?.()) {
                            n.collapse?.();
                        }
                    }
                    inv.return_value(new GLib.Variant('()', []));
                } else if (method === 'DropZoneClosed') {
                    this._externalDragActive = false;
                    this._pendingDropZonePos = null;
                    if (this._notchesReactiveOff && !this._pendingCardTarget) {
                        this._notchesReactiveOff = false;
                        this._setNotchesReactive(true);
                    }
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

        // Hit-test inside the notch chrome instead of picking on the stage:
        // the helper window sits over the notch and a stage pick can return
        // its own (handler-less) window actor, so the forwarded click was
        // silently dropped. The notch actors are the only thing the user can
        // be clicking on.
        let actor = null;
        for (const notch of this._notches ?? []) {
            const inner = this._pickInActor(notch, rootX, rootY);
            if (inner)
                actor = inner;
        }
        if (!actor)
            return;

        let target = actor;
        while (target && target !== global.stage) {
            if (target instanceof St.Button) {
                if (target.can_focus) target.grab_key_focus?.();
                if (typeof target.clicked === 'function') target.clicked(0);
                else target.emit('clicked', 0);
                return;
            }
            if (typeof target.clicked === 'function') {
                if (target.can_focus) target.grab_key_focus?.();
                target.clicked(0);
                return;
            }
            if (target._delegate && typeof target._delegate.activate === 'function') {
                target._delegate.activate();
                return;
            }
            target = target.get_parent?.();
        }
    }

    // Deepest visible actor of the subtree containing (x, y); later children
    // win, matching Clutter's paint order.
    _pickInActor(root, x, y) {
        const [rx, ry] = root.get_transformed_position();
        const [rw, rh] = root.get_transformed_size();
        if (!(rw > 0 && rh > 0) || x < rx || y < ry || x > rx + rw || y > ry + rh)
            return null;
        let best = root;
        for (const child of root.get_children?.() ?? []) {
            if (!child.visible)
                continue;
            const inner = this._pickInActor(child, x, y);
            if (inner)
                best = inner;
        }
        return best;
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
        this._externalDragActive = false;
        this._pendingDropZonePos = null;
        if (this._notchesReactiveOff && !this._pendingCardTarget) {
            this._notchesReactiveOff = false;
            this._setNotchesReactive(true);
        }
        this._hideDragCards();

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
        if (this._zoneLastShow && now - this._zoneLastShow < 250 * 1000) return;
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
        for (let ms of [50, 150, 300, 600]) {
            GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => {
                if (this._pendingDropZonePos && this._externalDragActive) {
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
            conn.call('es.pulsaros.NotchNuxHelper', '/es/pulsaros/NotchNuxHelper',
                'org.freedesktop.Application', 'Open',
                new GLib.Variant('(asa{sv})', [[uri], {}]), null,
                Gio.DBusCallFlags.NONE, 2000, null,
                (c, res) => {
                    try { c.call_finish(res); }
                    catch (_) {
                        this._spawnDropHelper(false);
                    }
                });
        } catch (_) {
            this._spawnDropHelper(false);
        }
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
        let nw = Math.max(640, notch.get_width());
        let nh = Math.max(350, notch.get_height());
        let mon = (Main.layoutManager.monitors ?? []).find(m =>
            nx + 100 >= m.x && nx + 100 < m.x + m.width);
        let cx = mon ? mon.x + Math.floor((mon.width - nw) / 2) : nx;
        let cy = mon ? mon.y : ny;
        return {
            x: Math.round(cx) - 20, y: Math.round(cy) - 10,
            w: Math.round(nw) + 40, h: Math.round(nh) + 40,
        };
    }

    _shelfCardBox(notch) {
        if (!notch || !notch.isExpanded) return null;
        try {
            let targetActor = notch._shelfScroll ?? notch._contentContainer;
            if (targetActor && typeof targetActor.get_transformed_position === 'function') {
                let [ax, ay] = targetActor.get_transformed_position();
                let [aw, ah] = targetActor.get_transformed_size();
                if (aw > 50 && ah > 30) {
                    return {
                        x: Math.round(ax),
                        y: Math.round(ay),
                        w: Math.round(aw),
                        h: Math.round(ah),
                    };
                }
            }
            let box = this._notchZoneBox(notch);
            return {
                x: box.x,
                y: box.y + 48,
                w: box.w,
                h: Math.max(150, box.h - 48),
            };
        } catch (_) {}
        return null;
    }

    _setShelfCompanionReactivity(notch, active) {
        if (!notch) return;
        if (active) {
            notch.reactive = false;
            if (notch._surface) notch._surface.reactive = false;
            if (notch._dashboard) notch._dashboard.reactive = false;
            if (notch._contentContainer) notch._contentContainer.reactive = false;
            if (notch._shelfScroll) notch._shelfScroll.reactive = false;
            if (notch._tabStrip) notch._tabStrip.reactive = true;
            if (notch._headerWrap) notch._headerWrap.reactive = true;
            if (notch._powerButton) notch._powerButton.reactive = true;
            if (notch._settingsButton) notch._settingsButton.reactive = true;
        } else {
            notch.reactive = true;
            if (notch._surface) notch._surface.reactive = true;
            if (notch._dashboard) notch._dashboard.reactive = true;
            if (notch._contentContainer) notch._contentContainer.reactive = true;
            if (notch._shelfScroll) notch._shelfScroll.reactive = true;
        }
    }

    _showCompanionCard(uris, notch) {
        if (!this._dbusConnection || !uris || !uris.length || !notch) return;
        if (!this._helperHasOwner()) {
            this._spawnDropHelper(false);
        }
        let box = this._shelfCardBox(notch) ?? this._notchZoneBox(notch);
        this._pendingCardTarget = box;
        this._lastZoneBox = box;

        this._setShelfCompanionReactivity(notch, true);

        this._positionDropZone();
        for (let ms of [40, 120, 250, 500]) {
            GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => {
                if (this._pendingCardTarget && !this._externalDragActive) {
                    let newBox = this._shelfCardBox(notch);
                    if (newBox) {
                        this._pendingCardTarget = newBox;
                        this._lastZoneBox = newBox;
                    }
                    this._positionDropZone();
                }
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
        for (let n of this._notches ?? []) {
            this._setShelfCompanionReactivity(n, false);
        }
        if (this._notchesReactiveOff && !this._externalDragActive) {
            this._notchesReactiveOff = false;
            this._setNotchesReactive(true);
        }
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
