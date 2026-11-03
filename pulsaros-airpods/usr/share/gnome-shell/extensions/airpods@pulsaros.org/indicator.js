import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';

import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

import {bannersEnabled, closeMenuAnimated, openMenuAnimated} from './compat.js';
import {AirPodsProxy, BUS_NAME, OBJECT_PATH} from './interface.js';

const TRANSIENT_SECONDS = 5;
const LOW_BATTERY = 20;

// A row: icon, name, level bar, "80%" and a state line ("Charging", "In ear").
function makeRow(iconFile, title) {
    const box = new St.BoxLayout({style_class: 'airpods-row', x_expand: true});
    const icon = new St.Icon({
        gicon: Gio.FileIcon.new(iconFile),
        style_class: 'airpods-row-icon',
        icon_size: 20,
    });
    box.add_child(icon);

    const text = new St.BoxLayout({vertical: true, x_expand: true, y_align: Clutter.ActorAlign.CENTER});
    const name = new St.Label({text: title, style_class: 'airpods-row-title'});
    const state = new St.Label({text: '', style_class: 'airpods-row-state'});
    text.add_child(name);
    text.add_child(state);
    box.add_child(text);

    const track = new St.Widget({style_class: 'airpods-bar', y_align: Clutter.ActorAlign.CENTER});
    const fill = new St.Widget({style_class: 'airpods-bar-fill'});
    track.add_child(fill);
    box.add_child(track);

    const percent = new St.Label({text: '', style_class: 'airpods-row-percent', y_align: Clutter.ActorAlign.CENTER});
    box.add_child(percent);

    return {box, icon, name, state, fill, percent};
}

function updateRow(row, level, charging, stateText) {
    const available = level >= 0;
    row.box.visible = true;
    row.percent.text = available ? `${level}%` : '–';
    row.state.text = stateText ?? '';
    const width = available ? Math.round(56 * level / 100) : 0;
    row.fill.set_width(width);
    row.fill.remove_style_class_name('airpods-low');
    row.fill.remove_style_class_name('airpods-charging');
    if (available && charging)
        row.fill.add_style_class_name('airpods-charging');
    else if (available && level <= LOW_BATTERY)
        row.fill.add_style_class_name('airpods-low');
}

export const AirPodsIndicator = GObject.registerClass(
class AirPodsIndicator extends PanelMenu.Button {
    _init(extensionPath) {
        super._init(0.0, 'AirPods', false);
        this._extensionPath = extensionPath;
        this._wasConnected = false;
        this._transientId = 0;
        this._proxy = null;

        this._icon = new St.Icon({style_class: 'system-status-icon', icon_size: 16});
        this.add_child(this._icon);
        this._setIcon('airpods-symbolic');

        this._buildMenu();
        this.visible = false;

        this._proxy = new AirPodsProxy(Gio.DBus.session, BUS_NAME, OBJECT_PATH, (proxy, error) => {
            if (error) {
                console.error(`[AirPods] cannot create proxy: ${error.message}`);
                return;
            }
            this._propsId = proxy.connect('g-properties-changed', () => this._sync());
            this._ownerId = proxy.connect('notify::g-name-owner', () => this._sync());
            this._lidId = proxy.connectSignal('LidOpened', () => this._onLidOpened());
            this._sync();
        });
    }

    _iconFile(name) {
        return Gio.File.new_for_path(`${this._extensionPath}/icons/${name}.svg`);
    }

    _setIcon(name) {
        this._icon.gicon = Gio.FileIcon.new(this._iconFile(name));
    }

    _buildMenu() {
        const header = new PopupMenu.PopupBaseMenuItem({reactive: false, can_focus: false});
        const headerBox = new St.BoxLayout({vertical: true, x_expand: true});
        this._title = new St.Label({style_class: 'airpods-title'});
        this._subtitle = new St.Label({style_class: 'airpods-subtitle'});
        headerBox.add_child(this._title);
        headerBox.add_child(this._subtitle);
        header.add_child(headerBox);
        this.menu.addMenuItem(header);

        const rowsItem = new PopupMenu.PopupBaseMenuItem({reactive: false, can_focus: false});
        const rows = new St.BoxLayout({vertical: true, x_expand: true, style_class: 'airpods-rows'});
        this._rows = {
            left: makeRow(this._iconFile('airpods-symbolic'), 'Left'),
            right: makeRow(this._iconFile('airpods-symbolic'), 'Right'),
            case: makeRow(this._iconFile('airpods-case-symbolic'), 'Case'),
            headphones: makeRow(this._iconFile('airpods-max-symbolic'), 'Battery'),
        };
        for (const row of Object.values(this._rows))
            rows.add_child(row.box);
        rowsItem.add_child(rows);
        this.menu.addMenuItem(rowsItem);

        this._note = new PopupMenu.PopupMenuItem('', {reactive: false, can_focus: false});
        this._note.label.add_style_class_name('airpods-note');
        this.menu.addMenuItem(this._note);

        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        this._earSwitch = new PopupMenu.PopupSwitchMenuItem('Pause when removed from ears', true);
        this._earSwitch.connect('toggled', (_item, state) => {
            if (this._proxy && !this._syncing)
                this._proxy.EarDetection = state;
        });
        this.menu.addMenuItem(this._earSwitch);

        this._popupSwitch = new PopupMenu.PopupSwitchMenuItem('Show when case opens', true);
        this._popupSwitch.connect('toggled', (_item, state) => {
            if (this._proxy && !this._syncing)
                this._proxy.LidPopup = state;
        });
        this.menu.addMenuItem(this._popupSwitch);
    }

    _sync() {
        const p = this._proxy;
        if (!p)
            return;

        const connected = p.g_name_owner !== null && !!p.Connected;
        this.visible = connected;
        if (!connected) {
            this._cancelTransient();
            if (this.menu.isOpen)
                this.menu.close();
            this._wasConnected = false;
            return;
        }

        this._syncing = true;
        const headphones = p.IsHeadphones;
        const hasAdv = p.HasAdvertisement;
        const fallback = p.FallbackBattery;

        this._title.text = p.Model || p.Name || 'AirPods';
        this._subtitle.text = p.Model && p.Name ? p.Name : '';
        this._setIcon(this._iconNameFor(p.ModelId));

        const rows = this._rows;
        rows.left.box.visible = rows.right.box.visible = rows.case.box.visible = false;
        rows.headphones.box.visible = false;

        if (hasAdv && !headphones) {
            updateRow(rows.left, p.Left, p.LeftCharging, this._earText(p.LeftInEar, p.LeftCharging));
            updateRow(rows.right, p.Right, p.RightCharging, this._earText(p.RightInEar, p.RightCharging));
            // The case battery is unknown while both buds are out of it.
            if (p.Case >= 0 || p.BothPodsInCase)
                updateRow(rows.case, p.Case, p.CaseCharging, p.CaseLidOpened ? 'Open' : 'Closed');
            this._note.visible = false;
        } else if (hasAdv && headphones) {
            updateRow(rows.headphones, p.Left >= 0 ? p.Left : p.Right, p.LeftCharging, '');
            this._note.visible = false;
        } else if (fallback >= 0) {
            updateRow(rows.headphones, fallback, false, 'Combined level');
            this._note.label.text = 'Open the case near this computer for per-earbud levels.';
            this._note.visible = true;
        } else {
            this._note.label.text = 'Battery level not available yet.';
            this._note.visible = true;
        }

        this._earSwitch.setToggleState(p.EarDetection);
        this._popupSwitch.setToggleState(p.LidPopup);
        this._syncing = false;

        if (!this._wasConnected)
            this._onConnected();
        this._wasConnected = true;
    }

    _earText(inEar, charging) {
        if (charging)
            return 'Charging';
        return inEar ? 'In ear' : '';
    }

    _iconNameFor(modelId) {
        if (modelId.startsWith('airpods_max'))
            return 'airpods-max-symbolic';
        return 'airpods-symbolic';
    }

    _onConnected() {
        this._showTransient();
    }

    _onLidOpened() {
        if (this._proxy?.LidPopup)
            this._showTransient();
    }

    // macOS-style: pop the card open for a few seconds, unless banners are off (Do Not Disturb).
    _showTransient() {
        if (!this.visible || this.menu.isOpen || !bannersEnabled())
            return;
        openMenuAnimated(this.menu);
        this._cancelTransient();
        this._transientId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, TRANSIENT_SECONDS, () => {
            this._transientId = 0;
            if (this.menu.isOpen)
                closeMenuAnimated(this.menu);
            return GLib.SOURCE_REMOVE;
        });
    }

    _cancelTransient() {
        if (this._transientId) {
            GLib.source_remove(this._transientId);
            this._transientId = 0;
        }
    }

    destroy() {
        this._cancelTransient();
        if (this._proxy) {
            if (this._propsId)
                this._proxy.disconnect(this._propsId);
            if (this._ownerId)
                this._proxy.disconnect(this._ownerId);
            if (this._lidId)
                this._proxy.disconnectSignal(this._lidId);
            this._proxy = null;
        }
        super.destroy();
    }
});
