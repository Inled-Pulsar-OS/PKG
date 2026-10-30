/**
 * Pulsar Desktop Icons - Native wallpaper widget for Pulsar OS
 *
 * Lightweight, in-process desktop icon manager attached directly to
 * Main.layoutManager._backgroundGroup (above the wallpaper, below all windows).
 *
 * Features:
 * - Direct wallpaper layer integration (zero separate GTK windows / no app switch clutter)
 * - Move and drag icons freely on the desktop
 * - Persistent custom positions saved to ~/.config/pulsar/desktop-positions.json
 * - macOS-style right-to-left or top-to-bottom grid alignment fallback
 * - Single click to select, double click to launch / open
 * - Native right-click context menu (Open, Open with Nautilus, Trash)
 * - Live file monitoring of ~/Desktop
 */

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Clutter from 'gi://Clutter';
import St from 'gi://St';
import Shell from 'gi://Shell';
import Pango from 'gi://Pango';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const ITEM_WIDTH = 88;
const ITEM_HEIGHT = 96;
const GRID_SPACING = 16;
const DRAG_THRESHOLD = 6;
const DOUBLE_CLICK_TIME_MS = 400;

export default class PulsarDesktopIconsExtension extends Extension {
    enable() {
        this._items = new Map();
        this._selectedItem = null;
        this._signals = [];
        this._positions = this._loadPositions();
        this._desktopDir = GLib.get_user_special_dir(GLib.UserDirectory.DIRECTORY_DESKTOP) ||
                           GLib.build_filenamev([GLib.get_home_dir(), 'Desktop']);

        // Create container on wallpaper background group
        this._container = new St.Widget({
            name: 'pulsar-desktop-icons-container',
            style_class: 'pulsar-desktop-container',
            reactive: false,
            x: 0,
            y: 0,
        });

        if (Main.layoutManager?._backgroundGroup) {
            Main.layoutManager._backgroundGroup.add_child(this._container);
        }

        // Watch desktop folder changes
        this._monitorDesktop();

        // Listen for screen resolution changes to keep icons in bounds
        if (Main.layoutManager) {
            this._signals.push({
                obj: Main.layoutManager,
                id: Main.layoutManager.connect('monitors-changed', () => this._relayout()),
            });
        }
        if (global.display) {
            this._signals.push({
                obj: global.display,
                id: global.display.connect('workareas-changed', () => this._relayout()),
            });
        }

        // Background click to deselect
        if (global.stage) {
            this._stageClickId = global.stage.connect('button-press-event', (_stage, event) => {
                if (event.get_button() === Clutter.BUTTON_PRIMARY) {
                    const actor = global.stage.get_event_actor(event);
                    if (actor === global.stage || actor === Main.layoutManager?._backgroundGroup) {
                        this._deselectAll();
                    }
                }
                return Clutter.EVENT_PROPAGATE;
            });
        }

        // Initial load
        this._refreshIcons();
    }

    disable() {
        if (this._monitor) {
            this._monitor.cancel();
            this._monitor = null;
        }

        if (this._stageClickId && global.stage) {
            global.stage.disconnect(this._stageClickId);
            this._stageClickId = 0;
        }

        for (const sig of this._signals) {
            try { sig.obj.disconnect(sig.id); } catch (e) {}
        }
        this._signals = [];

        this._clearIcons();

        if (this._container) {
            try {
                this._container.destroy();
            } catch (e) {}
            this._container = null;
        }

        this._savePositions();
    }

    /* File Monitoring */
    _monitorDesktop() {
        try {
            const dir = Gio.File.new_for_path(this._desktopDir);
            if (!dir.query_exists(null)) {
                dir.make_directory_with_parents(null);
            }
            this._monitor = dir.monitor_directory(Gio.FileMonitorFlags.NONE, null);
            this._monitor.connect('changed', (_mon, _file, _otherFile, eventType) => {
                if (eventType === Gio.FileMonitorEvent.CREATED ||
                    eventType === Gio.FileMonitorEvent.DELETED ||
                    eventType === Gio.FileMonitorEvent.MOVED_IN ||
                    eventType === Gio.FileMonitorEvent.MOVED_OUT ||
                    eventType === Gio.FileMonitorEvent.CHANGES_DONE_HINT) {
                    GLib.timeout_add(GLib.PRIORITY_DEFAULT, 150, () => {
                        this._refreshIcons();
                        return GLib.SOURCE_REMOVE;
                    });
                }
            });
        } catch (e) {
            console.warn(`[pulsar-desktop-icons] Failed to monitor desktop directory: ${e}`);
        }
    }

    _refreshIcons() {
        if (!this._container) return;

        const dir = Gio.File.new_for_path(this._desktopDir);
        let enumerator;
        try {
            enumerator = dir.enumerate_children(
                'standard::name,standard::display-name,standard::icon,standard::type,standard::content-type',
                Gio.FileQueryInfoFlags.NONE,
                null
            );
        } catch (e) {
            return;
        }

        const currentFiles = new Set();
        let fileInfo;
        while ((fileInfo = enumerator.next_file(null)) !== null) {
            const name = fileInfo.get_name();
            if (name.startsWith('.')) continue; // ignore hidden files
            currentFiles.add(name);

            if (!this._items.has(name)) {
                const item = this._createItem(name, fileInfo);
                this._items.set(name, item);
                this._container.add_child(item.actor);
            } else {
                // Update label/icon in case of rename
                const item = this._items.get(name);
                this._updateItem(item, fileInfo);
            }
        }

        // Remove deleted files
        for (const [name, item] of this._items.entries()) {
            if (!currentFiles.has(name)) {
                try { item.actor.destroy(); } catch (e) {}
                this._items.delete(name);
            }
        }

        this._relayout();
    }

    _clearIcons() {
        for (const item of this._items.values()) {
            try { item.actor.destroy(); } catch (e) {}
        }
        this._items.clear();
    }

    _createItem(filename, fileInfo) {
        const actor = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'pulsar-desktop-item',
            reactive: true,
            can_focus: true,
            track_hover: true,
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.START,
        });

        // 1. Icon
        let gicon = fileInfo.get_icon();
        const iconActor = new St.Icon({
            gicon: gicon,
            style_class: 'pulsar-desktop-icon',
            icon_size: 48,
            x_align: Clutter.ActorAlign.CENTER,
        });
        actor.add_child(iconActor);

        // 2. Label Box
        const labelBox = new St.Bin({
            style_class: 'pulsar-desktop-label-box',
            x_align: Clutter.ActorAlign.CENTER,
        });
        const label = new St.Label({
            text: fileInfo.get_display_name() || filename,
            style_class: 'pulsar-desktop-label',
        });
        label.clutter_text.line_wrap = true;
        label.clutter_text.line_wrap_mode = Pango.WrapMode.WORD_CHAR;
        label.clutter_text.ellipsize = Pango.EllipsizeMode.END;
        labelBox.set_child(label);
        actor.add_child(labelBox);

        const item = {
            id: filename,
            actor,
            iconActor,
            label,
            filePath: GLib.build_filenamev([this._desktopDir, filename]),
            lastClickTime: 0,
        };

        this._wireInteraction(item);
        return item;
    }

    _updateItem(item, fileInfo) {
        if (!item || !item.actor) return;
        item.label.text = fileInfo.get_display_name() || item.id;
        item.iconActor.gicon = fileInfo.get_icon();
    }

    _wireInteraction(item) {
        let isDragging = false;
        let dragStartX = 0;
        let dragStartY = 0;
        let origX = 0;
        let origY = 0;
        let stageCaptureId = 0;

        item.actor.connect('button-press-event', (_actor, event) => {
            const button = event.get_button();

            if (button === Clutter.BUTTON_SECONDARY) {
                this._selectItem(item);
                this._showContextMenu(item, event);
                return Clutter.EVENT_STOP;
            }

            if (button === Clutter.BUTTON_PRIMARY) {
                const [cx, cy] = event.get_coords();
                const now = GLib.get_monotonic_time() / 1000;

                // Double click check
                if (now - item.lastClickTime < DOUBLE_CLICK_TIME_MS) {
                    item.lastClickTime = 0;
                    this._launch(item);
                    return Clutter.EVENT_STOP;
                }
                item.lastClickTime = now;

                this._selectItem(item);

                dragStartX = cx;
                dragStartY = cy;
                [origX, origY] = item.actor.get_position();
                isDragging = false;

                // Capture pointer motion on stage
                stageCaptureId = global.stage.connect('captured-event', (_stage, stageEvent) => {
                    const type = stageEvent.type();

                    if (type === Clutter.EventType.MOTION) {
                        const [mx, my] = stageEvent.get_coords();
                        const dx = mx - dragStartX;
                        const dy = my - dragStartY;

                        if (!isDragging && Math.hypot(dx, dy) >= DRAG_THRESHOLD) {
                            isDragging = true;
                        }

                        if (isDragging) {
                            item.actor.set_position(origX + dx, origY + dy);
                        }
                        return Clutter.EVENT_PROPAGATE;
                    }

                    if (type === Clutter.EventType.BUTTON_RELEASE) {
                        if (stageCaptureId) {
                            global.stage.disconnect(stageCaptureId);
                            stageCaptureId = 0;
                        }

                        if (isDragging) {
                            isDragging = false;
                            // Clamp to screen bounds and save position
                            this._clampAndSave(item);
                        }
                        return Clutter.EVENT_PROPAGATE;
                    }

                    return Clutter.EVENT_PROPAGATE;
                });

                return Clutter.EVENT_STOP;
            }

            return Clutter.EVENT_PROPAGATE;
        });
    }

    _selectItem(item) {
        this._deselectAll();
        this._selectedItem = item;
        item.actor.add_style_class_name('selected');
    }

    _deselectAll() {
        if (this._selectedItem) {
            this._selectedItem.actor.remove_style_class_name('selected');
            this._selectedItem = null;
        }
    }

    _showContextMenu(item, event) {
        if (this._menu) {
            this._menu.close();
            this._menu.destroy();
            this._menu = null;
        }

        const menu = new PopupMenu.PopupMenu(item.actor, 0.5, St.Side.BOTTOM);
        Main.uiGroup.add_child(menu.actor);
        this._menu = menu;

        menu.addAction('Abrir', () => this._launch(item));
        menu.addAction('Mostrar en Archivos', () => {
            try {
                Gio.AppInfo.launch_default_for_uri(Gio.File.new_for_path(this._desktopDir).get_uri(), null);
            } catch (e) {}
        });

        menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        menu.addAction('Mover a la papelera', () => this._trash(item));

        menu.connect('open-state-changed', (_m, open) => {
            if (!open) {
                GLib.timeout_add(GLib.PRIORITY_DEFAULT, 100, () => {
                    if (this._menu === menu) {
                        menu.destroy();
                        this._menu = null;
                    }
                    return GLib.SOURCE_REMOVE;
                });
            }
        });

        menu.open(true);
    }

    _launch(item) {
        const file = Gio.File.new_for_path(item.filePath);
        try {
            if (item.id.endsWith('.desktop')) {
                const appInfo = Gio.DesktopAppInfo.new_from_filename(item.filePath);
                if (appInfo) {
                    appInfo.launch([], null);
                    return;
                }
            }
            Gio.AppInfo.launch_default_for_uri(file.get_uri(), null);
        } catch (e) {
            console.warn(`[pulsar-desktop-icons] Launch failed for ${item.id}: ${e}`);
        }
    }

    _trash(item) {
        const file = Gio.File.new_for_path(item.filePath);
        file.trash_async(GLib.PRIORITY_DEFAULT, null, (_f, res) => {
            try {
                file.trash_finish(res);
                this._refreshIcons();
            } catch (e) {
                console.warn(`[pulsar-desktop-icons] Failed to trash ${item.id}: ${e}`);
            }
        });
    }

    /* Layout & Positioning */
    _clampAndSave(item) {
        const monitor = Main.layoutManager.primaryMonitor || { x: 0, y: 0, width: 1920, height: 1080 };
        const minX = monitor.x + 20;
        const maxX = monitor.x + monitor.width - ITEM_WIDTH - 20;
        const minY = monitor.y + 36;
        const maxY = monitor.y + monitor.height - ITEM_HEIGHT - 70;

        const posX = Math.max(minX, Math.min(maxX, item.actor.x));
        const posY = Math.max(minY, Math.min(maxY, item.actor.y));

        item.actor.set_position(posX, posY);
        this._positions[item.id] = [posX, posY];
        this._savePositions();
    }

    _relayout() {
        const monitor = Main.layoutManager.primaryMonitor || { x: 0, y: 0, width: 1920, height: 1080 };
        const startX = monitor.x + 24;
        const startY = monitor.y + 40;
        const maxY = monitor.y + monitor.height - 80;

        let curX = startX;
        let curY = startY;

        for (const [name, item] of this._items.entries()) {
            if (this._positions[name]) {
                const [px, py] = this._positions[name];
                item.actor.set_position(px, py);
            } else {
                // Auto-place on grid
                item.actor.set_position(curX, curY);
                this._positions[name] = [curX, curY];

                curY += ITEM_HEIGHT + GRID_SPACING;
                if (curY + ITEM_HEIGHT > maxY) {
                    curY = startY;
                    curX += ITEM_WIDTH + GRID_SPACING;
                }
            }
        }
        this._savePositions();
    }

    _positionsPath() {
        const dir = GLib.build_filenamev([GLib.get_user_config_dir(), 'pulsar']);
        GLib.mkdir_with_parents(dir, 0o755);
        return GLib.build_filenamev([dir, 'desktop-positions.json']);
    }

    _loadPositions() {
        try {
            const path = this._positionsPath();
            if (GLib.file_test(path, GLib.FileTest.EXISTS)) {
                const [ok, content] = GLib.file_get_contents(path);
                if (ok) {
                    return JSON.parse(new TextDecoder().decode(content));
                }
            }
        } catch (e) {}
        return {};
    }

    _savePositions() {
        try {
            const path = this._positionsPath();
            const data = JSON.stringify(this._positions, null, 2);
            GLib.file_set_contents(path, new TextEncoder().encode(data));
        } catch (e) {}
    }
}
