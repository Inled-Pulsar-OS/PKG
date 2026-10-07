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
        this._helperBusId = 0;
        this._helperProc = null;
        this._pendingDropZonePos = null;
        this._pendingNotch = null;
        this._pendingCardTarget = null;
        this._zoneLastShow = 0;
        this._winCreatedId = 0;
        this._notchesReactiveOff = false;
        this._dragEndId = 0;
        this._externalDragActive = false;
        this._hoverCardOn = false;
        this._hoverCardLastShow = 0;
        this._stageMotionId = 0;
        this._config = new ConfigStore();
        this._currentDisplayMode = this._config.displayMonitor;

        // Wire DnD helper if available
        this._dbusConnection = Gio.bus_get_sync(Gio.BusType.SESSION, null);
        this._setupExtensionDbus();
        this._startDropHelper();

        // Restore the notch subtrees' reactivity and drop any pending
        // drag-out card target the moment an external drag ends.
        this._dragEndId = Main.xdndHandler?.connect?.('drag-end', () => {
            this._externalDragActive = false;
            this._hoverCardOn = false;
            this._pendingCardTarget = null;
            if (this._notchesReactiveOff) {
                this._notchesReactiveOff = false;
                this._setNotchesReactive(true);
            }
        }) ?? 0;

        // Move the helper's window the moment it exists (before its first
        // frame), so Mutter's own center placement never flashes on screen;
        // the retry timers in _showDropZone cover later re-maps.
        // Hover drag-out: while the shelf is expanded with staged files and
        // the pointer is over the notch, keep the companion's transparent
        // drag-source card materialized over the same notch rectangle, so
        // pressing any shelf row *is* grabbing the card (no re-grab). A
        // position-based stage monitor survives the card covering the shelf.
        this._stageMotionId = global.stage.connect('motion-event', (_s, ev) => {
            try { this._updateHoverCard(ev); } catch (e) {}
        });

        this._winCreatedId = global.display.connect('window-created', (_d, win) => {
            try {
                if (this._pendingDropZonePos &&
                    win.get_wm_class() === 'es.pulsaros.NotchNuxHelper')
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
        this._hoverCardOn = false;
        if (this._stageMotionId) {
            try { global.stage.disconnect(this._stageMotionId); } catch (_) {}
            this._stageMotionId = 0;
        }
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
            let xml = '<node><interface name="org.gnome.Shell.Extensions.NotchNux"><method name="StageFiles"><arg type="as" direction="in" name="uris"/></method></interface></node>';
            let nodeInfo = Gio.DBusNodeInfo.new_for_xml(xml);
            this._helperBusId = this._dbusConnection.register_object('/org/gnome/Shell/Extensions/NotchNux', nodeInfo.interfaces[0], (conn, sender, path, iface, method, params, inv) => {
                if (method === 'StageFiles') {
                    // unpack() es superficial: el hijo 'as' llega como Variant
                    let [urisVariant] = params.unpack();
                    let uris = (urisVariant && typeof urisVariant.deep_unpack === 'function')
                        ? urisVariant.deep_unpack() : urisVariant;
                    if (!Array.isArray(uris)) uris = uris ? [uris] : [];
                    this._stageFilesFromHelper(uris);
                    inv.return_value(new GLib.Variant('()', []));
                } else {
                    inv.return_error_literal(Gio.dbus_error_quark(), Gio.DBUS_ERROR_UNKNOWN_METHOD, 'Unknown');
                }
            }, null, null);
        } catch (e) { console.warn('NotchNux: D-Bus reg failed', e); }
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
        // deep_unpack(): unpack() deja los hijos como GLib.Variant, que sería
        // truthy aunque el nombre no tuviera dueño.
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

    // Warm the helper up at enable time: the window gets built but stays
    // unmapped, so the first drag does not pay the Python/GTK startup cost.
    _startDropHelper() {
        try {
            if (this._helperHasOwner()) return;
            this._spawnDropHelper(true);
        } catch (e) { console.warn('NotchNux: helper start failed', e); }
    }

    // Called by the notch's drag monitor while an external drag (source ===
    // Main.xdndHandler: files dragged out of Nautilus or any other app)
    // crosses the notch. Mutter hands the payload to whatever *client surface*
    // sits under the pointer, and the shell is never a drop target (see
    // acceptDrop in notchnux.js), so we map the helper's GTK drop zone
    // invisibly exactly where the notch is -- the "invisible drop rectangle at
    // the notch" the UI promises. For the pick to reach it, the whole notch
    // subtree is made non-reactive for the duration of the drag: with
    // CLUTTER_PICK_REACTIVE only individually reactive actors are recorded
    // (clutter_actor_should_pick), and shell chrome is not a surface, so any
    // reactive child of the expanded dashboard would refuse the drop.
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

        // The first activation maps a window Mutter places on its own (and a
        // cold spawn needs a few hundred ms to even exist), so retry the move
        // until the zone is on screen. Guards on _pendingDropZonePos keep this
        // inert after disable(). Re-applying the walk matters: expand() only
        // renders the dashboard rows *after* the first dragMotion, and any
        // reactive child born after the first walk would beat the zone in the
        // pick.
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

    // NEVER call_sync from the shell's main loop here: the helper answers
    // Activate by calling Gtk.present(), which needs Mutter to process its
    // Wayland requests — and Mutter is us, blocked waiting for that reply.
    // The result was a deadlock released only by the 2s timeout: every drag
    // event froze the shell (and the zone appeared seconds late). Measured
    // with the shell free, this roundtrip takes ~20ms, so fire and forget.
    _activateDropZone() {
        let conn;
        try {
            conn = this._dbusConnection ?? Gio.bus_get_sync(Gio.BusType.SESSION, null);
        } catch (e) {
            console.warn('NotchNux: no session bus', e);
            return;
        }
        try {
            conn.call('org.freedesktop.DBus', '/org/freedesktop/DBus',
                'org.freedesktop.DBus', 'NameHasOwner',
                new GLib.Variant('(s)', ['es.pulsaros.NotchNuxHelper']),
                new GLib.VariantType('(b)'), Gio.DBusCallFlags.NONE, 2000, null,
                (c, res) => {
                    let owned = false;
                    try { owned = c.call_finish(res).deep_unpack()[0]; } catch (_) { return; }
                    if (!this._dbusConnection) return; // extension disabled meanwhile
                    if (!owned) {
                        this._spawnDropHelper(false);
                        return;
                    }
                    try {
                        conn.call('es.pulsaros.NotchNuxHelper', '/es/pulsaros/NotchNuxHelper',
                            'org.freedesktop.Application', 'Activate',
                            new GLib.Variant('(a{sv})', [{}]), null,
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
        if (!this._pendingDropZonePos) return;
        let win = null;
        try {
            for (let a of global.get_window_actors()) {
                let w = a.metaWindow ?? a.get_meta_window?.();
                if (w && w.get_wm_class() === 'es.pulsaros.NotchNuxHelper') { win = w; break; }
            }
        } catch (_) {}
        if (win) this._applyDropZonePos(win);
    }

    _applyDropZonePos(win) {
        let pos = this._pendingDropZonePos;
        if (!pos || !win) return;
        try {
            let mon = (Main.layoutManager.monitors ?? []).find(m =>
                pos.x >= m.x && pos.x < m.x + m.width &&
                pos.y >= m.y && pos.y < m.y + m.height);
            if (!mon) return;

            let box;
            if (this._pendingCardTarget) {
                // Drag-out card: fixed size, under the pointer.
                box = { ...this._pendingCardTarget };
            } else {
                // Invisible drop rectangle: the notch widget's live bounds
                // (re-measured every pass, so the expand animation converges)
                // plus a small margin. get_transformed_* give floats.
                let nn = this._pendingNotch;
                if (!nn || typeof nn.get_transformed_position !== 'function')
                    return;
                box = this._notchZoneBox(nn);
            }

            let zx = Math.max(mon.x + 8, Math.min(box.x, mon.x + mon.width - box.w - 8));
            let zy = Math.max(mon.y + 8, Math.min(box.y, mon.y + mon.height - box.h - 8));
            // One call sizes AND places the window; the client honors it
            // (the helper window is resizable).
            win.move_resize_frame(global.get_current_time(), zx, zy, box.w, box.h);
            win.make_above();  // GTK4 lost keep_above, so keep the zone on top
        } catch (e) { console.warn('NotchNux: drop zone positioning failed', e); }
    }

    // The notch rectangle both the invisible drop zone and the transparent
    // drag-out card live in: the widget's live bounds plus a small margin.
    _notchZoneBox(notch) {
        let [nx, ny] = notch.get_transformed_position();
        let [nw, nh] = notch.get_transformed_size();
        return {
            x: Math.round(nx) - 40, y: Math.round(ny) - 40,
            w: Math.round(nw) + 80, h: Math.round(nh) + 80,
        };
    }

    // Called by the stage motion monitor: materialize the transparent
    // drag-out card while hovering an expanded shelf that has staged files,
    // hide it otherwise. External drop-drags take precedence.
    _updateHoverCard(ev) {
        if (this._externalDragActive) return;
        let [x, y] = ev?.get_coords?.() ?? [0, 0];
        let target = null;
        for (let n of this._notches ?? []) {
            try {
                if (!n?.isExpanded) continue;
                let [nx, ny] = n.get_transformed_position();
                let [nw, nh] = n.get_transformed_size();
                if (x >= nx && x < nx + nw && y >= ny && y < ny + nh) {
                    target = n;
                    break;
                }
            } catch (_) {}
        }
        if (!target) { this._hideHoverCard(); return; }
        let uris = target.getStagedFiles?.() ?? [];
        if (!uris || !uris.length) { this._hideHoverCard(); return; }
        let now = GLib.get_monotonic_time();
        if (now - this._hoverCardLastShow < 250 * 1000) return;
        this._hoverCardLastShow = now;
        this._showCompanionCard(uris, target, x, y);
    }

    // Map the helper's drag-source card over the notch (same rectangle as the
    // drop zone) and hand it every staged file, so a press on any shelf row
    // becomes a real Wayland drag of all of them.
    _showCompanionCard(uris, notch, x, y) {
        if (!this._dbusConnection || !uris?.length) return;
        this._pendingDropZonePos = { x, y };
        this._pendingCardTarget = this._notchZoneBox(notch);
        this._hoverCardOn = true;
        this._positionDropZone();
        for (let ms of [40, 160, 400]) {
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
                new GLib.Variant('(asa{sv})', [uris, {}]), null,
                Gio.DBusCallFlags.NONE, 3000, null,
                (c, res) => {
                    try { c.call_finish(res); }
                    catch (e) { console.warn('NotchNux: hover card Open failed', e); }
                });
        } catch (e) { console.warn('NotchNux: hover card failed', e); }
    }

    // Pointer left the shelf (or it collapsed/emptied): revert to the
    // invisible drop-zone mode of the same window.
    _hideHoverCard() {
        if (!this._hoverCardOn) return;
        this._hoverCardOn = false;
        this._pendingCardTarget = null;
        this._activateDropZone();
    }

    _startCompanionDrag(uris, x, y) {
        if (!this._dbusConnection) return;
        if (!uris || !uris.length) return;
        this._pendingDropZonePos = { x, y };
        this._pendingCardTarget = {
            x: Math.round(x - 130), y: Math.round(y - 16),
            w: 260, h: 64,
        };
        // If the helper window is already mapped (it usually is), place the
        // card before Open so it appears right under the pointer; otherwise
        // the retries below catch Open's present() mapping it.
        this._positionDropZone();
        for (let ms of [40, 160, 400]) {
            GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => {
                if (this._pendingCardTarget) this._positionDropZone();
                return GLib.SOURCE_REMOVE;
            });
        }
        // org.freedesktop.Application.Open maps the helper's drag-source card.
        // Fire and forget, never call_sync: the reply only arrives after the
        // helper's Gtk.present(), which needs Mutter to process Wayland
        // requests -- and Mutter is our own main loop.
        try {
            this._dbusConnection.call('es.pulsaros.NotchNuxHelper',
                '/es/pulsaros/NotchNuxHelper',
                'org.freedesktop.Application', 'Open',
                new GLib.Variant('(asa{sv})', [uris, {}]), null,
                Gio.DBusCallFlags.NONE, 3000, null,
                (c, res) => {
                    try { c.call_finish(res); }
                    catch (e) { console.warn('NotchNux: drag-out Open failed', e); }
                    this._pendingCardTarget = null;
                });
        } catch (e) {
            console.warn('NotchNux: drag-out failed', e);
            this._pendingCardTarget = null;
        }
    }

    // Make every shell actor of every notch non-reactive while an external
    // drop is possible. Reactivity is per-actor in Clutter (children of a
    // non-reactive parent are STILL picked -- clutter_actor_real_pick recurses
    // unconditionally), so the whole subtree must be walked. Restored on
    // drag-end.
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
