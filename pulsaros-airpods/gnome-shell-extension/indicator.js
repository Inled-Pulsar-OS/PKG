import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import {Slider} from 'resource:///org/gnome/shell/ui/slider.js';

import {bannersEnabled, closeMenuAnimated, openMenuAnimated} from './compat.js';
import {loadTranslator} from './i18n.js';
import {BatteryAlerts, ConnectNotice} from './alerts.js';
import {runControl} from './ctl.js';
import {availableModes, parseStatus} from './model.js';
import {Pending} from './optimistic.js';
import {StatusSource, defaultStatusPath} from './status-source.js';

const TRANSIENT_SECONDS = 5;
const LOW_BATTERY = 20;
const LID_OPEN = 0;
const LID_CLOSED = 1;
const BRAND = 'AirPods'; // product name, never translated
const CTL = 'librepods-ctl'; // looked up on PATH; installed by pulsaros-librepods
const PENDING_REFRESH_MS = 4100; // just after the optimistic hold expires (optimistic.js)

// Set once per indicator, before any string is built (see _init).
let _ = text => text;

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

function modeLabel(id) {
    switch (id) {
    case 0: return _('Off');
    case 1: return _('Noise Cancellation');
    case 2: return _('Transparency');
    case 3: return _('Adaptive');
    default: return '';
    }
}

function updateRow(row, level, charging, stateText) {
    const available = level >= 0;
    row.box.visible = true;
    row.percent.text = available ? `${level}%` : '–';
    row.state.text = stateText ?? '';
    row.fill.set_width(available ? Math.round(56 * level / 100) : 0);
    row.fill.remove_style_class_name('airpods-low');
    row.fill.remove_style_class_name('airpods-charging');
    if (available && charging)
        row.fill.add_style_class_name('airpods-charging');
    else if (available && level <= LOW_BATTERY)
        row.fill.add_style_class_name('airpods-low');
}

export const AirPodsIndicator = GObject.registerClass(
class AirPodsIndicator extends PanelMenu.Button {
    // statusPath is a parameter so a test or a nested shell can point at a hand-written file.
    _init(extensionPath, statusPath = defaultStatusPath()) {
        super._init(0.0, BRAND, false);
        _ = loadTranslator(extensionPath);
        this._extensionPath = extensionPath;
        this._wasConnected = false;
        this._lastLid = -1;
        this._transientId = 0;
        this._raw = parseStatus(null);            // what the daemon last said
        this._status = this._raw;                  // what we show: clicks held on top (optimistic.js)
        this._pending = new Pending(() => GLib.get_monotonic_time() / 1000);
        this._alerts = new BatteryAlerts();
        this._connectNotice = new ConnectNotice();
        this._refreshId = 0;
        this._modeKey = '';
        this._syncing = false;

        this._icon = new St.Icon({style_class: 'system-status-icon', icon_size: 16});
        this.add_child(this._icon);
        this._setIcon('airpods-symbolic');

        this._buildMenu();
        this.visible = false;

        this._source = new StatusSource(statusPath, text => this._onStatus(parseStatus(text)));
        this._source.start();
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
            left: makeRow(this._iconFile('airpods-symbolic'), _('Left')),
            right: makeRow(this._iconFile('airpods-symbolic'), _('Right')),
            case: makeRow(this._iconFile('airpods-case-symbolic'), _('Case')),
            headphones: makeRow(this._iconFile('airpods-max-symbolic'), _('Battery')),
        };
        for (const row of Object.values(this._rows))
            rows.add_child(row.box);
        rowsItem.add_child(rows);
        this.menu.addMenuItem(rowsItem);

        this._note = new PopupMenu.PopupMenuItem('', {reactive: false, can_focus: false});
        this._note.label.add_style_class_name('airpods-note');
        this.menu.addMenuItem(this._note);

        this._buildControls();
    }

    // Controls appear only where the model has them (capability keys from the daemon).
    _buildControls() {
        this._separator = new PopupMenu.PopupSeparatorMenuItem();
        this.menu.addMenuItem(this._separator);

        this._modeItem = new PopupMenu.PopupBaseMenuItem({reactive: false, can_focus: false});
        this._modeBox = new St.BoxLayout({style_class: 'airpods-segmented', x_expand: true});
        this._modeItem.add_child(this._modeBox);
        this._modeButtons = new Map();
        this.menu.addMenuItem(this._modeItem);

        this._levelItem = new PopupMenu.PopupBaseMenuItem({reactive: false, can_focus: false});
        this._levelItem.add_child(new St.Label({
            text: _('Noise level'), style_class: 'airpods-row-state', y_align: Clutter.ActorAlign.CENTER,
        }));
        this._levelSlider = new Slider(0.5);
        this._levelSlider.x_expand = true;
        this._levelSlider.connect('drag-end', () =>
            this._change('adaptive', 'adaptiveNoiseLevel', Math.round(this._levelSlider.value * 100)));
        this._levelItem.add_child(this._levelSlider);
        this.menu.addMenuItem(this._levelItem);

        this._caSwitch = new PopupMenu.PopupSwitchMenuItem(_('Conversation Awareness'), false);
        this._caSwitch.connect('toggled', (_item, state) => {
            if (!this._syncing)
                this._change('ca', 'conversationalAwareness', state);
        });
        this.menu.addMenuItem(this._caSwitch);

        this._oneBudSwitch = new PopupMenu.PopupSwitchMenuItem(_('One-Bud ANC'), false);
        this._oneBudSwitch.connect('toggled', (_item, state) => {
            if (!this._syncing)
                this._change('onebud', 'oneBudAnc', state);
        });
        this.menu.addMenuItem(this._oneBudSwitch);

        this._earMenu = new PopupMenu.PopupSubMenuMenuItem(_('Ear detection'));
        this._earItems = [
            _('Pause when one earbud is removed'),
            _('Pause when both earbuds are removed'),
            _('Never pause'),
        ].map((label, value) => {
            const item = new PopupMenu.PopupMenuItem(label);
            item.connect('activate', () => this._change('ear', 'earDetection', value));
            this._earMenu.menu.addMenuItem(item);
            return item;
        });
        this.menu.addMenuItem(this._earMenu);
    }

    // A click: show it now, hold it against stale reads, send it, and undo if the daemon cannot be reached.
    _change(kind, field, value) {
        this._pending.set(field, value);
        this._refresh();
        runControl(CTL, kind, value, ok => {
            if (!this._source)
                return; // destroyed while the command ran
            if (!ok) {
                this._pending.clear(field);
                Main.notifyError(BRAND, _('Could not reach the AirPods daemon.'));
            }
            this._refresh();
        });
        if (this._refreshId)
            GLib.source_remove(this._refreshId);
        this._refreshId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, PENDING_REFRESH_MS, () => {
            this._refreshId = 0;
            this._refresh();
            return GLib.SOURCE_REMOVE;
        });
    }

    _refresh() {
        this._status = this._pending.overlay(this._raw);
        this._sync();
    }

    _syncControls(s) {
        this._syncing = true;
        const modes = availableModes(s);
        const key = modes.map(m => m.id).join(',');
        if (key !== this._modeKey) {
            this._modeKey = key;
            this._modeBox.destroy_all_children();
            this._modeButtons.clear();
            for (const mode of modes) {
                const button = new St.Button({
                    label: modeLabel(mode.id), style_class: 'airpods-segment', can_focus: true, x_expand: true,
                });
                button.connect('clicked', () => this._change('noise', 'noiseMode', mode.id));
                this._modeBox.add_child(button);
                this._modeButtons.set(mode.id, button);
            }
        }
        this._modeItem.visible = modes.length > 0;
        for (const [id, button] of this._modeButtons) {
            if (id === s.noiseMode)
                button.add_style_pseudo_class('active');
            else
                button.remove_style_pseudo_class('active');
        }

        this._levelItem.visible = s.supportsAdaptive && s.noiseMode === 3;
        if (!this._levelSlider._dragging)
            this._levelSlider.value = s.adaptiveNoiseLevel / 100;

        this._caSwitch.visible = s.supportsConversationalAwareness;
        this._caSwitch.setToggleState(s.conversationalAwareness);
        this._oneBudSwitch.visible = s.supportsOneBudAnc;
        this._oneBudSwitch.setToggleState(s.oneBudAnc);

        this._earMenu.visible = !s.isHeadset;
        this._earItems.forEach((item, value) =>
            item.setOrnament(s.earDetection === value ? PopupMenu.Ornament.DOT : PopupMenu.Ornament.NONE));
        this._separator.visible = modes.length > 0 || s.supportsConversationalAwareness || !s.isHeadset;
        this._syncing = false;
    }

    // The daemon is not running or its file is unreadable: nothing to show, like macOS without AirPods.
    _onStatus(status) {
        if (status.state === 'newer-schema' && this._status.state !== 'newer-schema')
            console.warn('[AirPods] status.json has a newer schema than this extension reads; update the extension');
        this._raw = status;
        this._status = this._pending.overlay(status);
        this._sync();
        this._announceConnected(status);
        this._announceLowBattery(status);
    }

    _announceConnected(status) {
        const notice = this._connectNotice.update(status);
        if (!notice)
            return;
        const label = {left: _('Left'), right: _('Right'), case: _('Case'), headphones: _('Battery')};
        const body = notice.levels.map(([which, level]) => `${label[which]} ${level}%`).join(' · ');
        Main.notify(_('{name} connected').replace('{name}', notice.name || BRAND), body || _('Battery level not available yet.'));
    }

    // Shell notifications already honour Do Not Disturb.
    _announceLowBattery(status) {
        for (const {which, level} of this._alerts.update(status)) {
            const name = {
                left: _('Left AirPod'),
                right: _('Right AirPod'),
                case: _('AirPods case'),
                headphones: status.modelName || BRAND,
            }[which];
            Main.notify(
                _('{name} battery low').replace('{name}', name),
                _('{level}% remaining. Charge soon.').replace('{level}', level));
        }
    }

    _sync() {
        const s = this._status;
        const connected = s.state === 'ok' && s.connected;
        this.visible = connected;
        if (!connected) {
            this._cancelTransient();
            if (this.menu.isOpen)
                this.menu.close();
            this._wasConnected = false;
            this._lastLid = -1;
            return;
        }

        this._title.text = s.modelName || s.deviceName || BRAND;
        this._subtitle.text = s.modelName && s.deviceName ? s.deviceName : '';
        this._setIcon(s.isHeadset ? 'airpods-max-symbolic' : 'airpods-symbolic');

        const rows = this._rows;
        for (const row of Object.values(rows))
            row.box.visible = false;

        if (s.isHeadset) {
            updateRow(rows.headphones, s.headsetLevel, s.left.charging, '');
            this._note.visible = s.headsetLevel < 0;
        } else {
            updateRow(rows.left, s.left.level, s.left.charging, this._earText(s.left));
            updateRow(rows.right, s.right.level, s.right.charging, this._earText(s.right));
            if (s.case.available)
                updateRow(rows.case, s.case.level, s.case.charging, this._lidText(s.lidState));
            this._note.visible = !s.left.available && !s.right.available && !s.case.available;
        }
        if (this._note.visible)
            this._note.label.text = _('Battery level not available yet.');

        this._syncControls(s);

        if (!this._wasConnected)
            this._onConnected();
        else if (this._lastLid === LID_CLOSED && s.lidState === LID_OPEN)
            this._showTransient();
        this._wasConnected = true;
        this._lastLid = s.lidState;
    }

    _earText(pod) {
        if (pod.charging)
            return _('Charging');
        return pod.inEar ? _('In ear') : '';
    }

    _lidText(lidState) {
        if (lidState === LID_OPEN)
            return _('Open');
        return lidState === LID_CLOSED ? _('Closed') : '';
    }

    _onConnected() {
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
        if (this._refreshId) {
            GLib.source_remove(this._refreshId);
            this._refreshId = 0;
        }
        this._source?.stop();
        this._source = null;
        super.destroy();
    }
});
