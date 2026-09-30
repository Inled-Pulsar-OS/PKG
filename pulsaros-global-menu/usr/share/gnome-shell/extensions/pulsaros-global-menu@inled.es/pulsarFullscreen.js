import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import { getPointerWatcher } from 'resource:///org/gnome/shell/ui/pointerWatcher.js';

const IGNORED_APPS = [
    'Plank', 'Conky', 'gjs', 'Gjs', 'gjs-console',
    'ding', 'DING', 'org.gnome.Shell.Extensions.DING',
    'sayri', 'Sayri', 'es.inled.sayri', 'sayri-overlay', 'sayri-indicator',
    'pulsaros-spotlight', 'es.inled.PulsarStore'
];

export class MacOSFullscreenManager {
    constructor(extension) {
        this._extension = extension;
        this._windowSignals = new Map();
        this._spaceWindows = new Map();
        this._enabled = false;
        this._panelVisible = true;
        this._hideTimeoutId = 0;
        this._pointerWatcher = null;
        this._pointerWatch = null;
        this._wsChangedId = 0;
        this._windowCreatedId = 0;
        this._stageResizeId = 0;
        this._topTrigger = null;
        this._refreshingGeometry = false;

        this._setupSettings();
        this._setupSignals();
    }

    // ─── Utilities ─────────────────────────────────────────────────────────────

    _hasOpenMenu() {
        try {
            if (Main.overview?.visible) return true;
            if (Main.panel?.menuManager?.activeMenu) return true;
            if (Main.panel?.statusArea) {
                for (let k in Main.panel.statusArea) {
                    let item = Main.panel.statusArea[k];
                    if (item?.menu?.isOpen) return true;
                }
            }
        } catch (e) {}
        return false;
    }

    _isIgnoredWindow(win) {
        if (!win) return true;
        try {
            if (win.is_override_redirect?.()) return true;
            if (win.is_skip_taskbar?.()) return true;
            if (win.is_on_all_workspaces?.()) return true;
            let type = win.get_window_type?.() ?? Meta.WindowType.NORMAL;
            if (type !== Meta.WindowType.NORMAL) return true;
            if (win.get_transient_for?.() !== null) return true;

            let wmClass = win.get_wm_class?.() ?? '';
            let appId  = win.get_gtk_application_id?.() ?? '';
            let title  = win.get_title?.() ?? '';
            let sand   = win.get_sandboxed_app_id?.() ?? '';
            let cmdline = '';
            let pid = win.get_pid?.() ?? 0;
            if (pid > 0) {
                try {
                    let [ok, buf] = GLib.file_get_contents(`/proc/${pid}/cmdline`);
                    if (ok) cmdline = new TextDecoder().decode(buf).replace(/\0/g, ' ');
                } catch (_) {}
            }
            let check = `${wmClass} ${appId} ${title} ${sand} ${cmdline}`.toLowerCase();
            for (let ign of IGNORED_APPS)
                if (check.includes(ign.toLowerCase())) return true;
        } catch (e) { return true; }
        return false;
    }

    _isSpaceWorkspace(ws) {
        if (!this._enabled || !ws) return false;
        if (this._spaceWindows.size === 0) return false;

        for (let [win] of this._spaceWindows) {
            try {
                if (win && !win.unmanaged && !win.minimized && win.get_workspace() === ws) {
                    let isMax = (win.maximized_horizontally && win.maximized_vertically) || (win.is_fullscreen && win.is_fullscreen());
                    if (isMax) return true;
                }
            } catch (_) {}
        }
        return false;
    }

    _isCurrentWorkspaceFullscreenSpace() {
        if (!this._enabled) return false;
        return this._isSpaceWorkspace(global.workspace_manager?.get_active_workspace());
    }

    // ─── Settings ──────────────────────────────────────────────────────────────

    _setupSettings() {
        try {
            let schema = 'org.gnome.shell.extensions.pulsaros-global-menu';
            if (Gio.SettingsSchemaSource.get_default()?.lookup(schema, true)) {
                this._settings = new Gio.Settings({ schema_id: schema });
                this._enabled = this._settings.get_boolean('macos-fullscreen-spaces');
                this._settings.connect('changed::macos-fullscreen-spaces', () => {
                    this._enabled = this._settings.get_boolean('macos-fullscreen-spaces');
                    this._syncPanelForCurrentWorkspace();
                });
            }
        } catch (_) {}
    }

    // ─── Signal Setup ──────────────────────────────────────────────────────────

    _setupSignals() {
        this._windowCreatedId = global.display.connect('window-created', (_d, win) => {
            this._trackWindow(win);
        });
        for (let actor of global.get_window_actors())
            this._trackWindow(actor.meta_window);

        this._wsChangedId = global.workspace_manager.connect('active-workspace-changed', () => {
            this._syncPanelForCurrentWorkspace();
        });

        // PointerWatcher for reliable edge detection
        try {
            this._pointerWatcher = getPointerWatcher();
            this._pointerWatch = this._pointerWatcher.addWatch(50, (x, y) => this._onPointerMoved(x, y));
        } catch (_) {}

        this._syncPanelForCurrentWorkspace();
    }

    _trackWindow(win) {
        if (this._isIgnoredWindow(win)) return;
        let id = win.get_id?.();
        if (!id || this._windowSignals.has(id)) return;

        let sigs = [
            win.connect('notify::maximized-horizontally', () => this._onMaximizeChanged(win)),
            win.connect('notify::maximized-vertically',   () => this._onMaximizeChanged(win)),
            win.connect('notify::minimized',              () => this._syncPanelForCurrentWorkspace()),
            win.connect('workspace-changed',              () => this._syncPanelForCurrentWorkspace()),
            win.connect('unmanaged',                      () => this._untrackWindow(win)),
        ];
        this._windowSignals.set(id, { win, sigs });
    }

    // ─── Workspace / Maximize Logic ────────────────────────────────────────────

    _onMaximizeChanged(win) {
        if (!this._enabled || win._pulsarLock || this._isIgnoredWindow(win)) return;

        let isMax     = win.maximized_horizontally && win.maximized_vertically;
        let isTracked = this._spaceWindows.has(win);

        if (isMax && !isTracked) {
            win._pulsarLock = true;
            try {
                let wsm   = global.workspace_manager;
                let curWs = wsm.get_active_workspace();
                let origI = curWs.index();

                let newWs = wsm.append_new_workspace(false, global.get_current_time());
                let dest  = origI + 1;
                if (dest < wsm.n_workspaces && wsm.reorder_workspace)
                    wsm.reorder_workspace(newWs, dest);

                this._spaceWindows.set(win, { origWsIndex: origI });
                win.change_workspace(newWs);
                newWs.activate_with_focus(win, global.get_current_time());
            } catch (e) {
                console.error('[MacOSFullscreen] move workspace:', e);
            } finally {
                win._pulsarLock = false;
            }
            this._syncPanelForCurrentWorkspace();

        } else if (!isMax && isTracked) {
            this._restoreWindow(win);
            this._syncPanelForCurrentWorkspace();
        }
    }

    _restoreWindow(win) {
        let saved = this._spaceWindows.get(win);
        if (!saved) return;

        win._pulsarLock = true;
        try {
            this._spaceWindows.delete(win);

            let wsm = global.workspace_manager;
            let targetWs = wsm.get_workspace_by_index(
                Math.min(saved.origWsIndex, wsm.n_workspaces - 1));

            if (win.maximized_horizontally || win.maximized_vertically)
                win.unmaximize(Meta.MaximizeFlags.BOTH);

            if (targetWs) {
                win.change_workspace(targetWs);
                targetWs.activate_with_focus(win, global.get_current_time());
            }
        } catch (e) {
            console.error('[MacOSFullscreen] restore window:', e);
        } finally {
            win._pulsarLock = false;
            this._syncPanelForCurrentWorkspace();
        }
    }

    // ─── Struts ────────────────────────────────────────────────────────────────

    /**
     * Toggle affectsStruts on the panelBox Chrome entry WITHOUT removing it
     * (removing it from chrome breaks GNOME Shell's panel lifecycle).
     * After toggling, we must force maximized windows to recalculate their geometry
     * by briefly unmaximizing + re-maximizing them with _pulsarLock held.
     */
    _setPanelStruts(affectsStruts) {
        try {
            let chrome = Main.layoutManager._chrome;
            let entry = chrome?._trackedActors?.find(a => a.actor === Main.layoutManager.panelBox);
            if (entry && entry.affectsStruts !== affectsStruts) {
                entry.affectsStruts = affectsStruts;
                // trackFullscreen must be false so panel reacts normally
                entry.trackFullscreen = false;
                Main.layoutManager._queueUpdateRegions?.();
            }
        } catch (e) {
            console.error('[MacOSFullscreen] struts:', e);
        }
    }

    _forceWindowsRefreshGeometry(ws) {
        // Guard against re-entry
        if (this._refreshingGeometry) return;
        this._refreshingGeometry = true;

        // Use GLib.PRIORITY_DEFAULT so this runs after _updateRegions (which uses
        // Meta.later_add BEFORE_REDRAW). By this point struts=0 in Mutter.
        GLib.idle_add(GLib.PRIORITY_DEFAULT, () => {
            try {
                for (let [win] of this._spaceWindows) {
                    try {
                        if (win.unmanaged || win.get_workspace() !== ws) continue;
                        if (!win.maximized_horizontally || !win.maximized_vertically) continue;

                        let actor = win.get_compositor_private();
                        win._pulsarLock = true;

                        // Hide actor so GNOME Shell skips unmaximize/maximize animations
                        if (actor) actor.hide();
                        try {
                            if (typeof Meta.MaximizeFlags !== 'undefined' && win.unmaximize.length > 0) {
                                win.unmaximize(Meta.MaximizeFlags.BOTH);
                            } else {
                                win.unmaximize();
                            }
                            if (typeof Meta.MaximizeFlags !== 'undefined' && win.maximize.length > 0) {
                                win.maximize(Meta.MaximizeFlags.BOTH);
                            } else {
                                win.maximize();
                            }
                        } finally {
                            if (actor) actor.show();
                            // Release lock right away – notify:: signals fire synchronously
                            // during unmaximize/maximize, while the lock was still held
                            win._pulsarLock = false;
                        }
                    } catch (_) {}
                }
            } finally {
                this._refreshingGeometry = false;
            }
            return GLib.SOURCE_REMOVE;
        });
    }

    // ─── Panel show/hide ───────────────────────────────────────────────────────

    _showPanel(animated) {
        if (this._hideTimeoutId) {
            GLib.source_remove(this._hideTimeoutId);
            this._hideTimeoutId = 0;
        }
        this._panelVisible = true;

        let panelBox = Main.layoutManager.panelBox;
        let panelH   = panelBox?.height || 36;

        Main.panel.remove_all_transitions();
        Main.panel.translation_y = 0;
        Main.panel.reactive = true;
        Main.panel.opacity  = 255;

        if (!panelBox) return;

        if (animated) {
            // Slide in from off-screen: start at y=-panelH then ease to y=0
            // affectsStruts is still false (space mode), so showing doesn't add struts
            panelBox.remove_all_transitions();
            panelBox.ease({
                y: 0,
                duration: 200,
                mode: Clutter.AnimationMode.EASE_OUT_QUAD,
            });
        } else {
            // Normal workspace: restore y=0 so get_transformed_position() is on-screen → struts restored
            panelBox.remove_all_transitions();
            panelBox.set_y(0);
            // Synchronously update regions so struts take effect immediately
            try { Main.layoutManager._updateRegions?.(); } catch (_) {}
            Main.layoutManager._queueUpdateRegions?.();
        }
    }

    _hidePanel(animated) {
        if (this._hasOpenMenu()) return;
        this._panelVisible = false;

        let panelBox = Main.layoutManager.panelBox;
        let panelH   = panelBox?.height || 36;

        Main.panel.remove_all_transitions();
        Main.panel.translation_y = 0;

        if (!panelBox) return;

        if (animated) {
            // Slide panel upward by animating its actual y position.
            // When y = -panelH, get_transformed_position() returns (0, -panelH)
            // → _updateRegions computes strut rect at (0, -panelH, w, panelH)
            // → Mutter clips to screen: effective top strut = 0
            panelBox.remove_all_transitions();
            panelBox.ease({
                y: -panelH,
                duration: 200,
                mode: Clutter.AnimationMode.EASE_OUT_QUAD,
            });
        } else {
            // Move panelBox off-screen immediately.
            // y=-panelH → get_transformed_position returns off-screen → struts=0
            panelBox.remove_all_transitions();
            panelBox.set_y(-panelH);
        }
    }

    // ─── Master sync ───────────────────────────────────────────────────────────

    _syncPanelForCurrentWorkspace() {
        if (this._hideTimeoutId) {
            GLib.source_remove(this._hideTimeoutId);
            this._hideTimeoutId = 0;
        }

        if (!this._enabled) {
            this._setPanelStruts(true);
            this._showPanel(false);
            return;
        }

        let activeWs = global.workspace_manager?.get_active_workspace();
        let isSpace  = this._isSpaceWorkspace(activeWs);

        if (isSpace) {
            // 1. Set affectsStruts=false (belt-and-suspenders)
            this._setPanelStruts(false);
            // 2. Move panelBox to y=-panelH so get_transformed_position() is off-screen
            //    → _updateRegions computes struts=0 regardless of affectsStruts
            this._hidePanel(false);
            try { Main.layoutManager._updateRegions?.(); } catch (_) {}
            Main.layoutManager._queueUpdateRegions?.();
            // 4. Now that Mutter has struts=0, force windows to recalculate their maximized rect
            this._forceWindowsRefreshGeometry(activeWs);
        } else {
            // Restore struts first, then show panel (order matters for window repositioning)
            this._setPanelStruts(true);
            this._showPanel(false);
        }
    }

    // ─── Pointer hover ─────────────────────────────────────────────────────────

    _onPointerMoved(x, y) {
        if (!this._enabled) return;
        if (!this._isCurrentWorkspaceFullscreenSpace()) {
            if (!this._panelVisible || (Main.layoutManager.panelBox && Main.layoutManager.panelBox.y !== 0)) {
                this._setPanelStruts(true);
                this._showPanel(false);
            }
            return;
        }

        let panelH = (Main.panel.height || 36) + 16;

        if (y <= 4) {
            // Cursor at top edge – show panel
            if (this._hideTimeoutId) {
                GLib.source_remove(this._hideTimeoutId);
                this._hideTimeoutId = 0;
            }
            if (!this._panelVisible) this._showPanel(true);
        } else if (y < panelH) {
            // Cursor still inside panel area – cancel any pending hide
            if (this._hideTimeoutId) {
                GLib.source_remove(this._hideTimeoutId);
                this._hideTimeoutId = 0;
            }
        } else {
            // Cursor below panel – schedule hide
            if (this._panelVisible && !this._hasOpenMenu() && !this._hideTimeoutId) {
                this._hideTimeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 400, () => {
                    this._hideTimeoutId = 0;
                    if (this._isCurrentWorkspaceFullscreenSpace() && !this._hasOpenMenu())
                        this._hidePanel(true);
                    return GLib.SOURCE_REMOVE;
                });
            }
        }
    }

    // ─── Cleanup ───────────────────────────────────────────────────────────────

    _untrackWindow(win) {
        let id = win.get_id?.();
        if (id && this._windowSignals.has(id)) {
            let { sigs } = this._windowSignals.get(id);
            for (let s of sigs) try { win.disconnect(s); } catch (_) {}
            this._windowSignals.delete(id);
        }
        this._spaceWindows.delete(win);
        this._syncPanelForCurrentWorkspace();
    }

    destroy() {
        if (this._hideTimeoutId) {
            GLib.source_remove(this._hideTimeoutId);
            this._hideTimeoutId = 0;
        }
        if (this._pointerWatch && this._pointerWatcher) {
            try { this._pointerWatcher._removeWatch?.(this._pointerWatch); } catch (_) {}
            this._pointerWatch = null;
        }
        if (this._topTrigger) {
            try { Main.layoutManager.removeChrome(this._topTrigger); this._topTrigger.destroy(); } catch (_) {}
            this._topTrigger = null;
        }
        if (this._windowCreatedId) {
            global.display.disconnect(this._windowCreatedId);
            this._windowCreatedId = 0;
        }
        if (this._wsChangedId) {
            global.workspace_manager.disconnect(this._wsChangedId);
            this._wsChangedId = 0;
        }
        if (this._stageResizeId) {
            global.stage.disconnect(this._stageResizeId);
            this._stageResizeId = 0;
        }
        for (let [, { win, sigs }] of this._windowSignals)
            for (let s of sigs) try { win.disconnect(s); } catch (_) {}
        this._windowSignals.clear();
        this._spaceWindows.clear();

        // Always restore struts and panel position on disable
        this._setPanelStruts(true);
        this._showPanel(false);
    }
}

