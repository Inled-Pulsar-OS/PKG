import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

import {bannersEnabled, closeMenuAnimated, openMenuAnimated} from './compat.js';
import {loadTranslator} from './i18n.js';
import {AirPodsProxy, BUS_NAME, OBJECT_PATH} from './interface.js';

const TRANSIENT_SECONDS = 5;
const LOW_BATTERY = 20;
// Detection range -> Device1 RSSI floor in dBm (the daemon accepts -100..-40).
const RANGE_DBM = [-60, -80, -90]; // Near, Normal, Far (labels are translated in _buildMenu)
const BRAND = 'AirPods'; // product name, never translated

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
        _ = loadTranslator(extensionPath);
        this._extensionPath = extensionPath;
        this._wasConnected = false;
        this._transientId = 0;
        this._proxy = null;
        this._pending = new Map(); // address -> _('Connecting…') | _('Disconnecting…')

        this._icon = new St.Icon({style_class: 'system-status-icon', icon_size: 16});
        // Icon plus an optional battery label, like the macOS menu bar.
        const bar = new St.BoxLayout({style_class: 'panel-status-menu-box'});
        this._panelLabel = new St.Label({
            style_class: 'airpods-panel-label',
            y_align: Clutter.ActorAlign.CENTER,
            visible: false,
        });
        bar.add_child(this._icon);
        bar.add_child(this._panelLabel);
        this.add_child(bar);
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
            this._lowId = proxy.connectSignal('LowBattery', (_p, _s, [which, level]) =>
                this._onLowBattery(which, level));
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

        // Paired AirPods with Connect / Disconnect, like the macOS Bluetooth menu.
        this._devices = new PopupMenu.PopupMenuSection();
        this.menu.addMenuItem(this._devices);

        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        this._earSwitch = new PopupMenu.PopupSwitchMenuItem(_('Pause when removed from ears'), true);
        this._earSwitch.connect('toggled', (_item, state) => {
            if (this._proxy && !this._syncing)
                this._proxy.EarDetection = state;
        });
        this.menu.addMenuItem(this._earSwitch);

        this._lowSwitch = new PopupMenu.PopupSwitchMenuItem(_('Low battery alerts'), true);
        this._lowSwitch.connect('toggled', (_item, state) => {
            if (this._proxy && !this._syncing)
                this._proxy.LowBatteryAlerts = state;
        });
        this.menu.addMenuItem(this._lowSwitch);

        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        const forget = new PopupMenu.PopupMenuItem(_('Forget these AirPods'));
        forget.connect('activate', () => this._proxy?.ForgetRemote(() => {}));
        this.menu.addMenuItem(forget);

        this._range = new PopupMenu.PopupSubMenuMenuItem(_('Detection range'));
        this._rangeItems = [_('Near'), _('Normal'), _('Far')].map((label, i) => {
            const dbm = RANGE_DBM[i];
            const item = new PopupMenu.PopupMenuItem(label);
            item.connect('activate', () => {
                if (this._proxy)
                    this._proxy.RssiMin = dbm;
            });
            this._range.menu.addMenuItem(item);
            return item;
        });
        this.menu.addMenuItem(this._range);

        this._batterySetting = new PopupMenu.PopupSubMenuMenuItem(_('Battery in the top bar'));
        this._batteryItems = [['off', _('Hidden')], ['lowest', _('Lowest earbud')], ['both', _('Both earbuds')]].map(
            ([value, label]) => {
                const item = new PopupMenu.PopupMenuItem(label);
                item.connect('activate', () => {
                    if (this._proxy)
                        this._proxy.PanelBattery = value;
                });
                this._batterySetting.menu.addMenuItem(item);
                return [value, item];
            });
        this.menu.addMenuItem(this._batterySetting);

        this._idleSwitch = new PopupMenu.PopupSwitchMenuItem(_('Show when not connected'), false);
        this._idleSwitch.connect('toggled', (_item, state) => {
            if (this._proxy && !this._syncing)
                this._proxy.ShowWhenDisconnected = state;
        });
        this.menu.addMenuItem(this._idleSwitch);

        this._popupSwitch = new PopupMenu.PopupSwitchMenuItem(_('Show when case opens'), true);
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

        const owner = p.g_name_owner !== null;
        const connected = owner && !!p.Connected;
        const paired = owner ? p.PairedDevices : [];
        const idle = owner && !connected && p.ShowWhenDisconnected && paired.length > 0;
        this.visible = connected || idle;
        this._icon.opacity = connected ? 255 : 128;
        this._syncDevices(paired);
        const panelText = connected ? this._panelText(p) : '';
        this._panelLabel.text = panelText;
        this._panelLabel.visible = panelText !== '';

        if (!connected) {
            this._cancelTransient();
            if (!idle && this.menu.isOpen)
                this.menu.close();
            this._wasConnected = false;
            if (idle)
                this._showIdle(p);
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
                updateRow(rows.case, p.Case, p.CaseCharging, p.CaseLidOpened ? _('Open') : _('Closed'));
            this._note.visible = false;
        } else if (hasAdv && headphones) {
            updateRow(rows.headphones, p.Left >= 0 ? p.Left : p.Right, p.LeftCharging, '');
            this._note.visible = false;
        } else if (fallback >= 0) {
            updateRow(rows.headphones, fallback, false, _('Combined level'));
            this._note.label.text = _('Open the case near this computer for per-earbud levels.');
            this._note.visible = true;
        } else {
            this._note.label.text = _('Battery level not available yet.');
            this._note.visible = true;
        }

        this._syncSwitches(p);
        this._syncing = false;

        if (!this._wasConnected)
            this._onConnected();
        this._wasConnected = true;
    }

    _showIdle(p) {
        this._syncing = true;
        this._title.text = BRAND;
        this._subtitle.text = _('Not connected');
        for (const row of Object.values(this._rows))
            row.box.visible = false;
        this._note.visible = false;
        this._syncSwitches(p);
        this._syncing = false;
    }

    _syncSwitches(p) {
        this._earSwitch.setToggleState(p.EarDetection);
        this._lowSwitch.setToggleState(p.LowBatteryAlerts);
        this._popupSwitch.setToggleState(p.LidPopup);
        this._idleSwitch.setToggleState(p.ShowWhenDisconnected);
        // Radio-style check marks on the current choices.
        const ornament = PopupMenu.Ornament;
        RANGE_DBM.forEach((dbm, i) =>
            this._rangeItems[i].setOrnament(p.RssiMin === dbm ? ornament.DOT : ornament.NONE));
        for (const [value, item] of this._batteryItems)
            item.setOrnament(p.PanelBattery === value ? ornament.DOT : ornament.NONE);
    }

    // "70%" or "80% 70%", only while per-earbud levels are known.
    _panelText(p) {
        if (p.PanelBattery === 'off' || !p.HasAdvertisement)
            return '';
        const levels = p.IsHeadphones ? [p.Left] : [p.Left, p.Right];
        const known = levels.filter(l => l >= 0);
        if (known.length === 0)
            return '';
        if (p.PanelBattery === 'lowest' || known.length === 1)
            return `${Math.min(...known)}%`;
        return levels.map(l => (l >= 0 ? `${l}%` : '–')).join(' ');
    }

    _syncDevices(paired) {
        this._devices.removeAll();
        for (const [address, name, isConnected] of paired) {
            const busy = this._pending.get(address);
            const item = new PopupMenu.PopupBaseMenuItem();
            item.add_child(new St.Label({text: name, x_expand: true, y_align: Clutter.ActorAlign.CENTER}));
            item.add_child(new St.Label({
                text: busy ?? (isConnected ? _('Disconnect') : _('Connect')),
                style_class: 'airpods-row-state',
                y_align: Clutter.ActorAlign.CENTER,
            }));
            // Keep the menu open so the "Connecting…" state stays visible: the default activate
            // closes the menu. Both GNOME 48 and 51 call item.activate() on click and Enter.
            item.activate = () => {
                if (!busy)
                    this._toggleConnection(address, isConnected);
            };
            this._devices.addMenuItem(item);
        }
    }

    _toggleConnection(address, isConnected) {
        this._pending.set(address, isConnected ? _('Disconnecting…') : _('Connecting…'));
        this._sync();
        const done = (_result, error) => {
            this._pending.delete(address);
            if (error)
                Main.notifyError('AirPods', error.message.replace(/^.*Error\.Failed:\s*/, ''));
            if (this._proxy)
                this._sync();
        };
        if (isConnected)
            this._proxy?.DisconnectRemote(address, done);
        else
            this._proxy?.ConnectRemote(address, done);
    }

    _earText(inEar, charging) {
        if (charging)
            return _('Charging');
        return inEar ? _('In ear') : '';
    }

    _iconNameFor(modelId) {
        if (modelId.startsWith('airpods_max'))
            return 'airpods-max-symbolic';
        return 'airpods-symbolic';
    }

    _onConnected() {
        this._showTransient();
    }

    // Shell notifications already honour Do Not Disturb.
    _onLowBattery(which, level) {
        if (!this._proxy?.LowBatteryAlerts)
            return;
        const what = {
            left: _('Left AirPod'),
            right: _('Right AirPod'),
            case: _('AirPods case'),
            headphones: this._proxy.Model || 'AirPods',
        }[which] ?? 'AirPods';
        Main.notify(
            _('{name} battery low').replace('{name}', what),
            _('{level}% remaining. Charge soon.').replace('{level}', level));
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
            if (this._lowId)
                this._proxy.disconnectSignal(this._lowId);
            this._proxy = null;
        }
        super.destroy();
    }
});
