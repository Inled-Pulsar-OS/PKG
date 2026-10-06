import GObject from 'gi://GObject';
import St from 'gi://St';
import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as DND from 'resource:///org/gnome/shell/ui/dnd.js';

const FLYDROP_BUS = 'es.pulsaros.FlyDrop';
const FLYDROP_PATH = '/es/pulsaros/FlyDrop';
const FLYDROP_IFACE = 'es.pulsaros.FlyDrop';

export const DynamicDropIsland = GObject.registerClass(
class DynamicDropIsland extends St.Widget {
    _init() {
        super._init({
            name: 'flydrop-island-container',
            reactive: true,
            visible: false,
            opacity: 0,
            track_hover: true,
            can_focus: true,
        });

        // Set delegate directly on this top-level widget
        this._delegate = this;

        this._stagedItems = []; // list of file paths or text
        this._stagedType = null; // 'files' or 'text'
        this._hideTimeoutId = null;
        this._isHovered = false;

        // Inner Pill Container
        this._pill = new St.BoxLayout({
            style_class: 'flydrop-island',
            reactive: true,
            track_hover: true,
            can_focus: true,
        });
        this._pill._delegate = this;
        this.add_child(this._pill);

        // Build Initial Drop Zone UI
        this._showDropZoneUI();

        // Connect hover events on container and pill
        this.connect('enter-event', () => this._onEnter());
        this.connect('leave-event', () => this._onLeave());
        this._pill.connect('enter-event', () => this._onEnter());
        this._pill.connect('leave-event', () => this._onLeave());

        // Register Global Drag Monitor
        this._dragMonitor = {
            dragMotion: (dropEvent) => this._onGlobalDragMotion(dropEvent),
        };
        DND.addDragMonitor(this._dragMonitor);

        // Position on Top Center of Screen
        this._reposition();
        this._monitorsChangedId = Main.layoutManager.connect('monitors-changed', () => this._reposition());
    }

    _onEnter() {
        this._isHovered = true;
        if (this._hideTimeoutId) {
            GLib.source_remove(this._hideTimeoutId);
            this._hideTimeoutId = null;
        }
    }

    _onLeave() {
        this._isHovered = false;
        if (this._stagedItems.length === 0) {
            this._scheduleHide(800);
        }
    }

    _reposition() {
        const monitor = Main.layoutManager.primaryMonitor;
        if (!monitor) return;

        const panelHeight = Main.panel ? Main.panel.height : 32;
        const width = this._stagedItems.length > 0 ? 360 : 280;
        this.set_size(width, 48);
        this.set_position(
            monitor.x + Math.floor((monitor.width - width) / 2),
            monitor.y + panelHeight + 6
        );
    }

    _showDropZoneUI() {
        this._pill.destroy_all_children();

        const icon = new St.Icon({
            icon_name: 'folder-download-symbolic',
            icon_size: 18,
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._pill.add_child(icon);

        const hint = new St.Label({
            text: 'Soltar aquí para retener',
            style_class: 'flydrop-island-title',
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._pill.add_child(hint);
    }

    _showShelfUI() {
        this._pill.destroy_all_children();

        // Icon
        const icon = new St.Icon({
            icon_name: this._stagedType === 'text' ? 'text-x-generic-symbolic' : 'edit-copy-symbolic',
            icon_size: 18,
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._pill.add_child(icon);

        // File/Text Chip
        let summary = '';
        if (this._stagedType === 'files') {
            const count = this._stagedItems.length;
            const first = this._stagedItems[0].split('/').pop();
            summary = count === 1 ? first : `${first} (+${count - 1})`;
        } else {
            summary = `Texto (${this._stagedItems[0].length} chars)`;
        }

        const chip = new St.Label({
            text: summary,
            style_class: 'flydrop-shelf-chip',
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._pill.add_child(chip);

        // Dedicated FlyDrop Button (AirDrop Icon + Label)
        const sendBtn = new St.Button({
            label: 'FlyDrop',
            style_class: 'flydrop-action-btn',
            y_align: Clutter.ActorAlign.CENTER,
            can_focus: true,
            reactive: true,
        });
        sendBtn.connect('clicked', () => this._sendStagedItems());
        this._pill.add_child(sendBtn);

        // Close / Clear Button
        const closeBtn = new St.Button({
            label: '✕',
            style_class: 'flydrop-close-btn',
            y_align: Clutter.ActorAlign.CENTER,
            can_focus: true,
            reactive: true,
        });
        closeBtn.connect('clicked', () => this.clearAndHide());
        this._pill.add_child(closeBtn);

        this._reposition();
        this._setupDraggableSource();
    }

    // DND Protocol Implementation for Drop Target
    handleDragOver(source, actor, x, y, time) {
        this._pill.add_style_pseudo_class('hover');
        return DND.DragMotionResult.COPY_DROP;
    }

    acceptDrop(source, actor, x, y, time) {
        this._pill.remove_style_pseudo_class('hover');
        this._extractAndStageData(source);
        return true;
    }

    _extractAndStageData(source) {
        let files = [];
        let text = '';

        if (source) {
            // Check URI list in source
            if (source.dragData && source.dragData.data) {
                const raw = String(source.dragData.data);
                if (raw.includes('file://')) {
                    const lines = raw.split(/[\r\n]+/);
                    for (let l of lines) {
                        l = l.trim();
                        if (l.startsWith('file://')) {
                            try {
                                const path = decodeURIComponent(l.substring(7));
                                if (path) files.push(path);
                            } catch (e) {}
                        }
                    }
                } else if (raw.trim()) {
                    text = raw.trim();
                }
            }

            // Check GNOME Shell / Nautilus file objects
            if (files.length === 0 && source._file) {
                try {
                    const path = source._file.get_path();
                    if (path) files.push(path);
                } catch (e) {}
            }

            if (files.length === 0 && source.file) {
                try {
                    const path = source.file.get_path();
                    if (path) files.push(path);
                } catch (e) {}
            }

            if (files.length === 0 && source.getFiles) {
                try {
                    const fl = source.getFiles();
                    if (Array.isArray(fl)) {
                        for (let f of fl) {
                            const p = f.get_path ? f.get_path() : String(f);
                            if (p) files.push(p);
                        }
                    }
                } catch (e) {}
            }
        }

        if (files.length > 0) {
            this._stagedItems = files;
            this._stagedType = 'files';
            this._showShelfUI();
            this.reveal();
        } else if (text) {
            this._stagedItems = [text];
            this._stagedType = 'text';
            this._showShelfUI();
            this.reveal();
        } else {
            // Staged fallback
            this._stagedItems = ['Elemento guardado'];
            this._stagedType = 'text';
            this._showShelfUI();
            this.reveal();
        }
    }

    _setupDraggableSource() {
        if (!this._stagedItems || this._stagedItems.length === 0) return;

        // Makes the staged item draggable to other apps/folders
        this._draggable = new DND._Draggable(this._pill, {
            manualMode: false,
            restoreOnSuccess: false,
        });

        this.dragData = {
            type: this._stagedType === 'files' ? 'text/uri-list' : 'text/plain',
            data: this._stagedType === 'files'
                ? this._stagedItems.map(p => `file://${p}`).join('\r\n')
                : this._stagedItems[0],
        };
    }

    _onGlobalDragMotion(dropEvent) {
        // As long as drag is happening, reveal island
        if (this._stagedItems.length === 0) {
            this.reveal();
            this._scheduleHide(2000);
        }
        return DND.DragMotionResult.CONTINUE;
    }

    _scheduleHide(delayMs) {
        if (this._hideTimeoutId) {
            GLib.source_remove(this._hideTimeoutId);
            this._hideTimeoutId = null;
        }

        if (this._stagedItems.length === 0 && !this._isHovered) {
            this._hideTimeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, delayMs, () => {
                this.hideIsland();
                this._hideTimeoutId = null;
                return GLib.SOURCE_REMOVE;
            });
        }
    }

    reveal() {
        this.visible = true;
        this.ease({
            opacity: 255,
            duration: 220,
            mode: Clutter.AnimationMode.EASE_OUT_QUAD,
        });
    }

    hideIsland() {
        if (this._stagedItems.length > 0 || this._isHovered) {
            return;
        }
        this.ease({
            opacity: 0,
            duration: 200,
            mode: Clutter.AnimationMode.EASE_IN_QUAD,
            onComplete: () => {
                this.visible = false;
                this._showDropZoneUI();
            },
        });
    }

    clearAndHide() {
        this._stagedItems = [];
        this._stagedType = null;
        this._isHovered = false;
        this.hideIsland();
    }

    _sendStagedItems() {
        if (!this._stagedItems || this._stagedItems.length === 0) return;

        try {
            const bus = Gio.bus_get_sync(Gio.BusType.SESSION, null);
            if (this._stagedType === 'files') {
                const filesJson = JSON.stringify(this._stagedItems);
                bus.call(
                    FLYDROP_BUS,
                    FLYDROP_PATH,
                    FLYDROP_IFACE,
                    'OpenSendDialog',
                    new GLib.Variant('(s)', [filesJson]),
                    null,
                    Gio.DBusCallFlags.NONE,
                    -1,
                    null,
                    null
                );
            } else {
                bus.call(
                    FLYDROP_BUS,
                    FLYDROP_PATH,
                    FLYDROP_IFACE,
                    'SendText',
                    new GLib.Variant('(ss)', ['', this._stagedItems[0]]),
                    null,
                    Gio.DBusCallFlags.NONE,
                    -1,
                    null,
                    null
                );
            }
        } catch (e) {
            console.log(`[FlyDrop] Error sending staged items: ${e.message}`);
        }

        this.clearAndHide();
    }

    destroy() {
        if (this._dragMonitor) {
            DND.removeDragMonitor(this._dragMonitor);
            this._dragMonitor = null;
        }
        if (this._hideTimeoutId) {
            GLib.source_remove(this._hideTimeoutId);
            this._hideTimeoutId = null;
        }
        if (this._monitorsChangedId) {
            Main.layoutManager.disconnect(this._monitorsChangedId);
            this._monitorsChangedId = null;
        }
        super.destroy();
    }
});
