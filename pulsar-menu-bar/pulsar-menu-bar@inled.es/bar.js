/* Pulsar OS - menu bar status items.
 *
 * Three independent targets in the top bar, the way macOS lays them out:
 * Wi-Fi, Bluetooth and Control Center. Wi-Fi and Bluetooth each own a real
 * Tahoe-style status menu; Control Center opens the running macOS-style
 * Quick Settings popup, anchored on its own button.
 *
 * Everything lives in this file because extension.js is cached by the shell
 * for the whole session, while bar.js is imported again (with a cache-busting
 * query) on every enable.  GObject.registerClass is deliberately avoided: a
 * second copy of a registered class name crashes the shell on reload.
 */
import Cairo from 'cairo';
import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GioUnix from 'gi://GioUnix';
import GLib from 'gi://GLib';
import NM from 'gi://NM';
import Pango from 'gi://Pango';
import St from 'gi://St';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

const NM_PATH = '/org/freedesktop/NetworkManager';
const BLUEZ = 'org.bluez';
const ADAPTER_IFACE = 'org.bluez.Adapter1';
const DEVICE_IFACE = 'org.bluez.Device1';
const BATTERY_IFACE = 'org.bluez.Battery1';

const UPOWER = 'org.freedesktop.UPower';
const UPOWER_PATH = '/org/freedesktop/UPower';
const UPOWER_DISPLAY_DEVICE = '/org/freedesktop/UPower/devices/DisplayDevice';
const UPOWER_DEVICE_IFACE = 'org.freedesktop.UPower.Device';
const SYSTEMD = 'org.freedesktop.systemd1';
const OPTIMIZER_UNIT = '/org/freedesktop/systemd1/unit/pulsaros_2doptimizer_2eservice';
const UNIT_IFACE = 'org.freedesktop.systemd1.Unit';

function pget(dict, key, fallback = null) {
    if (!dict)
        return fallback;
    const v = dict[key];
    if (v === undefined || v === null)
        return fallback;
    try {
        if (typeof v.deepUnpack === 'function')
            return v.deepUnpack();
        if (typeof v.unpack === 'function')
            return v.unpack();
    } catch (e) {}
    return v;
}

const BLUE = '#0a84ff';
const LANG = (GLib.get_language_names()[0] ?? '').slice(0, 2);
const ITALIAN = LANG === 'it';
const SPANISH = LANG === 'es';

/* Built-in dictionary: PulsarOS supports Spanish, Italian and English.
 * Anything not listed falls back to English. */
function T(english, italian, spanish) {
    if (SPANISH && spanish)
        return spanish;
    if (ITALIAN && italian)
        return italian;
    return english;
}

function later(ms, fn) {
    return GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => {
        try {
            fn();
        } catch (e) {
            console.warn(`pulsar-bar: deferred callback failed: ${e}`);
        }
        return GLib.SOURCE_REMOVE;
    });
}

function openSettingsPanel(desktopId, fallbackCommand) {
    Main.overview.hide();
    try {
        const app = GioUnix.DesktopAppInfo.new(desktopId);
        if (app) {
            app.launch([], null);
            return;
        }
    } catch (e) {
        // Fall through to the command-line variant.
    }

    try {
        Gio.AppInfo.create_from_commandline(fallbackCommand, null,
            Gio.AppInfoCreateFlags.NONE).launch([], null);
    } catch (e) {
        console.warn(`pulsar-bar: could not open settings panel: ${e}`);
    }
}

function signalIconFor(strength) {
    let quality = 'excellent';
    if (strength < 20)
        quality = 'none';
    else if (strength < 40)
        quality = 'weak';
    else if (strength < 50)
        quality = 'ok';
    else if (strength < 80)
        quality = 'good';
    return `network-wireless-signal-${quality}-symbolic`;
}

function isSecureType(type) {
    return type !== NM.UtilsSecurityType.NONE &&
        type !== NM.UtilsSecurityType.OWE;
}

function isEnterpriseType(type) {
    return type === NM.UtilsSecurityType.WPA_ENTERPRISE ||
        type === NM.UtilsSecurityType.WPA2_ENTERPRISE ||
        type === NM.UtilsSecurityType.WPA3_SUITE_B_192;
}

/* ------------------------------------------------------------------ *
 * Wi-Fi model
 * ------------------------------------------------------------------ */

const SECURITY_TYPES = Object.values(NM.UtilsSecurityType)
    .filter(Number.isInteger)
    .sort((a, b) => b - a);

const WIFI_SCAN_SECONDS = 20;

class WifiService {
    constructor(onChanged) {
        this._onChanged = onChanged;
        this._client = null;
        this._clientSignals = [];
        this._deviceSignals = [];
        this._apSignals = [];
        this._devices = [];
        this._cancellable = new Gio.Cancellable();
        this._scanId = 0;
        this._scanning = false;
        this._destroyed = false;

        NM.Client.new_async(this._cancellable, (_source, result) => {
            let client = null;
            try {
                client = NM.Client.new_finish(result);
            } catch (e) {
                if (!this._destroyed)
                    console.warn(`pulsar-bar: NetworkManager client failed: ${e}`);
                return;
            }
            if (this._destroyed) {
                client.run_dispose();
                return;
            }
            this._setClient(client);
        });
    }

    get ready() {
        return Boolean(this._client);
    }

    get available() {
        return Boolean(this._client?.nm_running && this._devices.length);
    }

    get hardwareEnabled() {
        return this._client?.wireless_hardware_enabled ?? false;
    }

    get enabled() {
        return this._client?.wireless_enabled ?? false;
    }

    setEnabled(enabled) {
        if (!this._client || !this.hardwareEnabled)
            return;
        try {
            this._client.wireless_enabled = Boolean(enabled);
        } catch (e) {
            console.warn(`pulsar-bar: could not change Wi-Fi state: ${e}`);
        }
    }

    get activeNetwork() {
        return this.networks.find(network => network.active)?.name ?? '';
    }

    get iconName() {
        if (!this.enabled)
            return 'network-wireless-disabled-symbolic';

        const device = this._devices.find(d => d.active_connection) ??
            this._devices[0];
        if (!device)
            return 'network-wireless-signal-none-symbolic';

        if (device.state === NM.DeviceState.PREPARE ||
            device.state === NM.DeviceState.CONFIG ||
            device.state === NM.DeviceState.NEED_AUTH ||
            device.state === NM.DeviceState.IP_CONFIG ||
            device.state === NM.DeviceState.IP_CHECK ||
            device.state === NM.DeviceState.SECONDARIES)
            return 'network-wireless-acquiring-symbolic';

        const ap = device.active_access_point;
        if (!ap)
            return 'network-wireless-signal-none-symbolic';
        if (this._client.connectivity !== NM.ConnectivityState.FULL)
            return 'network-wireless-no-route-symbolic';
        return signalIconFor(ap.strength);
    }

    get networks() {
        const groups = new Map();

        for (const device of this._devices) {
            const connections = device.get_available_connections();
            const activePath = device.active_access_point?.get_path() ?? null;

            for (const ap of device.get_access_points()) {
                const ssid = ap.get_ssid();
                if (!ssid)
                    continue;

                let name = '';
                try {
                    name = NM.utils_ssid_to_utf8(ssid.get_data()) ?? '';
                } catch (e) {
                    name = '';
                }
                if (!name)
                    continue;

                const securityType = this._securityType(device, ap);
                if (securityType === NM.UtilsSecurityType.INVALID)
                    continue;

                const active = activePath !== null && ap.get_path() === activePath;
                const known = active || connections.some(connection => {
                    try {
                        return ap.connection_valid(connection);
                    } catch (e) {
                        return false;
                    }
                });

                const existing = groups.get(name);
                if (!existing) {
                    groups.set(name, {
                        name,
                        strength: ap.strength,
                        secure: isSecureType(securityType),
                        securityType,
                        active,
                        known,
                        iconName: signalIconFor(ap.strength),
                        device,
                        accessPoint: ap,
                    });
                    continue;
                }

                existing.active ||= active;
                existing.known ||= known;
                existing.secure ||= isSecureType(securityType);

                const better = active ||
                    (!existing.active && ap.strength > existing.strength);
                if (better) {
                    existing.strength = ap.strength;
                    existing.iconName = signalIconFor(ap.strength);
                    existing.securityType = securityType;
                    existing.device = device;
                    existing.accessPoint = ap;
                }
            }
        }

        return [...groups.values()].sort((a, b) =>
            Number(b.active) - Number(a.active) ||
            Number(b.known) - Number(a.known) ||
            b.strength - a.strength ||
            GLib.utf8_collate(a.name, b.name));
    }

    toggleNetwork(network) {
        if (!this._client || !network?.device || !network?.accessPoint)
            return false;

        if (network.active) {
            const active = network.device.active_connection;
            if (active)
                this._deactivate(active);
            return true;
        }

        const {device, accessPoint} = network;
        const saved = device.get_available_connections().find(connection => {
            try {
                return accessPoint.connection_valid(connection);
            } catch (e) {
                return false;
            }
        });

        if (saved) {
            this._activate(saved, device, accessPoint, network.name);
            return true;
        }

        /* NetworkManager cannot infer an unsaved 802.1x profile. */
        if (isEnterpriseType(network.securityType))
            return false;

        const connection = new NM.SimpleConnection();
        const setting = new NM.SettingConnection();
        setting.id = network.name;
        try {
            setting.add_permission('user', GLib.get_user_name(), null);
        } catch (e) {
            // Older libnm releases do not expose add_permission; the shell
            // agent still supplies the secret for the interactive connection.
        }
        connection.add_setting(setting);

        try {
            this._client.add_and_activate_connection_async(
                connection, device, accessPoint.get_path(), this._cancellable,
                (client, result) => {
                    if (this._destroyed)
                        return;
                    try {
                        client.add_and_activate_connection_finish(result);
                    } catch (e) {
                        console.warn(`pulsar-bar: could not join ${network.name}: ${e}`);
                    }
                });
        } catch (e) {
            console.warn(`pulsar-bar: could not join ${network.name}: ${e}`);
        }
        return true;
    }

    startScanning() {
        if (this._scanning)
            return;
        this._scanning = true;
        this.requestScan();
        this._scanId = GLib.timeout_add_seconds(
            GLib.PRIORITY_DEFAULT, WIFI_SCAN_SECONDS, () => {
                this.requestScan();
                return GLib.SOURCE_CONTINUE;
            });
    }

    stopScanning() {
        this._scanning = false;
        if (this._scanId) {
            GLib.Source.remove(this._scanId);
            this._scanId = 0;
        }
    }

    requestScan() {
        if (!this.enabled)
            return;
        for (const device of this._devices) {
            try {
                device.request_scan_async(this._cancellable, (source, result) => {
                    if (this._destroyed)
                        return;
                    try {
                        source.request_scan_finish(result);
                    } catch (e) {
                        // "Scanning not allowed immediately following a
                        // previous scan" is normal and not worth a log line.
                    }
                });
            } catch (e) {
                // Ignore transient scan errors: the AP list is still valid.
            }
        }
    }

    _setClient(client) {
        this._client = client;

        for (const signal of [
            'notify::nm-running',
            'notify::wireless-enabled',
            'notify::wireless-hardware-enabled',
            'notify::primary-connection',
            'notify::connectivity',
            'connection-added',
            'connection-removed',
        ]) {
            this._connect(client, signal, () => this._changed(),
                this._clientSignals);
        }

        this._connect(client, 'device-added', () => this._syncDevices(),
            this._clientSignals);
        this._connect(client, 'device-removed', () => this._syncDevices(),
            this._clientSignals);

        this._syncDevices();
        if (this._scanning)
            this.requestScan();
    }

    _syncDevices() {
        this._disconnectAll(this._deviceSignals);
        this._disconnectAll(this._apSignals);

        this._devices = this._client?.get_devices()
            .filter(device => device.device_type === NM.DeviceType.WIFI) ?? [];

        for (const device of this._devices) {
            for (const signal of [
                'state-changed',
                'notify::active-access-point',
                'notify::active-connection',
                'notify::available-connections',
            ]) {
                this._connect(device, signal, () => this._changed(),
                    this._deviceSignals);
            }
            this._connect(device, 'access-point-added', () => {
                this._syncAccessPoints();
                this._changed();
            }, this._deviceSignals);
            this._connect(device, 'access-point-removed', () => {
                this._syncAccessPoints();
                this._changed();
            }, this._deviceSignals);
        }

        this._syncAccessPoints();
        this._changed();
    }

    _syncAccessPoints() {
        this._disconnectAll(this._apSignals);
        for (const device of this._devices) {
            for (const ap of device.get_access_points()) {
                for (const signal of [
                    'notify::strength',
                    'notify::ssid',
                    'notify::flags',
                    'notify::wpa-flags',
                    'notify::rsn-flags',
                ]) {
                    this._connect(ap, signal, () => this._changed(),
                        this._apSignals);
                }
            }
        }
    }

    _securityType(device, ap) {
        const capabilities = device.wireless_capabilities;
        const adHoc = ap.mode === NM['80211Mode'].ADHOC;
        return SECURITY_TYPES.find(type => NM.utils_security_valid(
            type, capabilities, true, adHoc,
            ap.flags, ap.wpa_flags, ap.rsn_flags)) ??
            NM.UtilsSecurityType.INVALID;
    }

    _activate(connection, device, accessPoint, name) {
        try {
            this._client.activate_connection_async(
                connection, device, accessPoint.get_path(), this._cancellable,
                (client, result) => {
                    if (this._destroyed)
                        return;
                    try {
                        client.activate_connection_finish(result);
                    } catch (e) {
                        console.warn(`pulsar-bar: could not connect to ${name}: ${e}`);
                    }
                });
        } catch (e) {
            console.warn(`pulsar-bar: could not connect to ${name}: ${e}`);
        }
    }

    _deactivate(active) {
        try {
            this._client.deactivate_connection_async(
                active, this._cancellable, (client, result) => {
                    if (this._destroyed)
                        return;
                    try {
                        client.deactivate_connection_finish(result);
                    } catch (e) {
                        console.warn(`pulsar-bar: could not disconnect Wi-Fi: ${e}`);
                    }
                });
        } catch (e) {
            console.warn(`pulsar-bar: could not disconnect Wi-Fi: ${e}`);
        }
    }

    _connect(object, signal, callback, bucket) {
        try {
            bucket.push([object, object.connect(signal, callback)]);
        } catch (e) {
            console.warn(`pulsar-bar: could not watch Wi-Fi signal ${signal}: ${e}`);
        }
    }

    _disconnectAll(bucket) {
        for (const [object, id] of bucket) {
            try {
                object.disconnect(id);
            } catch (e) {
                // The GObject may already have been disposed by NetworkManager.
            }
        }
        bucket.length = 0;
    }

    _changed() {
        if (!this._destroyed)
            this._onChanged?.();
    }

    destroy() {
        if (this._destroyed)
            return;
        this._destroyed = true;
        this.stopScanning();
        this._cancellable?.cancel();

        const client = this._client;
        this._disconnectAll(this._apSignals);
        this._disconnectAll(this._deviceSignals);
        this._disconnectAll(this._clientSignals);
        this._devices = [];
        this._client = null;
        this._cancellable = null;
        this._onChanged = null;
        client?.run_dispose();
    }
}

/* ------------------------------------------------------------------ *
 * Bluetooth model (BlueZ over D-Bus: there is no BlueZ GIR on GNOME)
 * ------------------------------------------------------------------ */

class BluetoothService {
    constructor(onChanged) {
        this._onChanged = onChanged;
        this._bus = Gio.DBus.system;
        this._signals = [];
        this._adapterPath = null;
        this._adapter = null;
        this._devices = [];
        this._refreshId = 0;
        this._destroyed = false;
        this._ready = false;
        this._discovering = false;

        this._subscribe();
        this.refresh();
    }

    get ready() {
        return this._ready;
    }

    get available() {
        return Boolean(this._adapterPath);
    }

    get powered() {
        return Boolean(pget(this._adapter, 'Powered', false));
    }

    get discovering() {
        return Boolean(pget(this._adapter, 'Discovering', false)) || this._discovering;
    }

    get devices() {
        return this._devices;
    }

    setPowered(enabled) {
        if (!this._adapterPath)
            return;
        if (this._adapter)
            this._adapter.Powered = enabled;
        this._setProperty(this._adapterPath, ADAPTER_IFACE, 'Powered', enabled);
        this._scheduleRefresh(80);
    }

    startDiscovery() {
        if (!this._adapterPath || !this.powered || this._discovering)
            return;
        this._discovering = true;
        this._call(this._adapterPath, ADAPTER_IFACE, 'StartDiscovery');
    }

    stopDiscovery() {
        if (!this._adapterPath || !this._discovering)
            return;
        this._discovering = false;
        this._call(this._adapterPath, ADAPTER_IFACE, 'StopDiscovery');
    }

    toggleDevice(path, connect) {
        this._call(path, DEVICE_IFACE, connect ? 'Connect' : 'Disconnect');
    }

    refresh() {
        if (this._destroyed)
            return;

        this._bus.call(
            BLUEZ, '/', 'org.freedesktop.DBus.ObjectManager',
            'GetManagedObjects', null,
            GLib.VariantType.new('(a{oa{sa{sv}}})'),
            Gio.DBusCallFlags.NONE, -1, null, (bus, result) => {
                if (this._destroyed)
                    return;

                let objects = {};
                try {
                    const res = bus.call_finish(result);
                    objects = typeof res.recursiveUnpack === 'function'
                        ? res.recursiveUnpack()[0]
                        : res.deepUnpack()[0];
                } catch (e) {
                    // BlueZ may be restarting; NameOwnerChanged will retry.
                    objects = {};
                }

                let adapterPath = null;
                let adapter = null;
                const devices = [];

                for (const path in objects) {
                    const interfaces = objects[path];
                    const adapterProps = interfaces[ADAPTER_IFACE];
                    if (adapterProps && !adapterPath) {
                        adapterPath = path;
                        adapter = {
                            Powered: Boolean(pget(adapterProps, 'Powered', false)),
                            Discovering: Boolean(pget(adapterProps, 'Discovering', false)),
                        };
                        continue;
                    }

                    const device = interfaces[DEVICE_IFACE];
                    if (!device)
                        continue;

                    const isPaired = Boolean(pget(device, 'Paired', false));
                    const isConnected = Boolean(pget(device, 'Connected', false));
                    const rawName = pget(device, 'Name', null);
                    const rawAlias = pget(device, 'Alias', null);
                    const rawAddr = pget(device, 'Address', null);

                    // Skip anonymous BLE MAC beacons without an advertised name unless paired/connected
                    if (!isPaired && !isConnected && !rawName)
                        continue;

                    const name = String(rawAlias || rawName || rawAddr || path.split('/').pop());
                    const icon = normalizeBluetoothIcon(pget(device, 'Icon'));
                    const battery = pget(interfaces[BATTERY_IFACE], 'Percentage', null);

                    devices.push({
                        path,
                        name,
                        icon,
                        connected: isConnected,
                        paired: isPaired,
                        battery: Number.isFinite(Number(battery))
                            ? Number(battery) : null,
                    });
                }

                devices.sort((a, b) =>
                    Number(b.connected) - Number(a.connected) ||
                    Number(b.paired) - Number(a.paired) ||
                    a.name.localeCompare(b.name));

                this._adapterPath = adapterPath;
                this._adapter = adapter;
                this._devices = devices;
                this._ready = true;
                this._changed();
            });
    }

    _subscribe() {
        const watch = (sender, iface, member, path, arg0) => {
            try {
                const id = this._bus.signal_subscribe(
                    sender, iface, member, path, arg0,
                    Gio.DBusSignalFlags.NONE, () => this._scheduleRefresh());
                this._signals.push(id);
            } catch (e) {
                console.warn(`pulsar-bar: could not watch ${sender} ${member}: ${e}`);
            }
        };

        watch(BLUEZ, 'org.freedesktop.DBus.ObjectManager',
            'InterfacesAdded', '/', null);
        watch(BLUEZ, 'org.freedesktop.DBus.ObjectManager',
            'InterfacesRemoved', '/', null);
        watch(BLUEZ, 'org.freedesktop.DBus.Properties',
            'PropertiesChanged', null, null);

        try {
            const id = this._bus.signal_subscribe(
                'org.freedesktop.DBus', 'org.freedesktop.DBus',
                'NameOwnerChanged', '/org/freedesktop/DBus', BLUEZ,
                Gio.DBusSignalFlags.NONE, () => this._scheduleRefresh(250));
            this._signals.push(id);
        } catch (e) {
            // Non-fatal: the other BlueZ signals still keep the menu fresh.
        }
    }

    _scheduleRefresh(delay = 120) {
        if (this._destroyed || this._refreshId)
            return;
        this._refreshId = GLib.timeout_add(
            GLib.PRIORITY_DEFAULT, delay, () => {
                this._refreshId = 0;
                this.refresh();
                return GLib.SOURCE_REMOVE;
            });
    }

    _setProperty(path, iface, property, value) {
        this._bus.call(
            BLUEZ, path, 'org.freedesktop.DBus.Properties', 'Set',
            new GLib.Variant('(ssv)',
                [iface, property, new GLib.Variant('b', value)]),
            null, Gio.DBusCallFlags.NONE, -1, null, (bus, result) => {
                try {
                    bus.call_finish(result);
                } catch (e) {
                    console.warn(`pulsar-bar: BlueZ ${property} change failed: ${e}`);
                }
                this._scheduleRefresh(250);
            });
    }

    _call(path, iface, method) {
        this._bus.call(
            BLUEZ, path, iface, method, null, null,
            Gio.DBusCallFlags.NONE, -1, null, (bus, result) => {
                try {
                    bus.call_finish(result);
                } catch (e) {
                    console.warn(`pulsar-bar: BlueZ ${method} failed: ${e}`);
                }
                this._scheduleRefresh(300);
            });
    }

    _changed() {
        if (!this._destroyed)
            this._onChanged?.();
    }

    destroy() {
        if (this._destroyed)
            return;
        this._destroyed = true;
        if (this._discovering) {
            this.stopDiscovery();
        }

        if (this._refreshId) {
            GLib.Source.remove(this._refreshId);
            this._refreshId = 0;
        }
        for (const id of this._signals) {
            try {
                this._bus.signal_unsubscribe(id);
            } catch (e) {
                // The bus connection is going away with the shell session.
            }
        }
        this._signals = [];
        this._devices = [];
        this._adapter = null;
        this._adapterPath = null;
        this._onChanged = null;
    }
}

/* ------------------------------------------------------------------ *
 * Battery model (UPower over D-Bus) + Optimizer service & stats
 * ------------------------------------------------------------------ */

class BatteryService {
    constructor(onChanged) {
        this._onChanged = onChanged;
        this._bus = Gio.DBus.system;
        this._signals = [];
        this._hasBattery = false;
        this._percentage = 100;
        this._state = 0;
        this._iconName = 'battery-full-symbolic';
        this._optimizerActive = false;
        this._batterySaverActive = false;
        this._optimizerStats = null;
        this._destroyed = false;

        this._subscribe();
        this.refresh();
        this.refreshOptimizer();
    }

    get hasBattery() {
        return this._hasBattery;
    }

    get percentage() {
        return this._percentage;
    }

    get state() {
        return this._state;
    }

    get isCharging() {
        return this._state === 1 || this._state === 5;
    }

    get iconName() {
        return this._iconName;
    }

    get optimizerActive() {
        return this._optimizerActive;
    }

    get batterySaverActive() {
        return this._batterySaverActive;
    }

    get optimizerStats() {
        return this._optimizerStats;
    }

    refresh() {
        if (this._destroyed)
            return;

        this._bus.call(
            UPOWER, UPOWER_DISPLAY_DEVICE, 'org.freedesktop.DBus.Properties', 'GetAll',
            new GLib.Variant('(s)', [UPOWER_DEVICE_IFACE]),
            null, Gio.DBusCallFlags.NONE, -1, null, (bus, result) => {
                if (this._destroyed)
                    return;
                try {
                    const res = bus.call_finish(result);
                    const props = typeof res.deepUnpack === 'function'
                        ? res.deepUnpack()[0]
                        : {};

                    const isPresent = Boolean(pget(props, 'IsPresent', false));
                    const type = Number(pget(props, 'Type', 0));
                    this._hasBattery = isPresent && (type === 2 || type === 3);
                    this._percentage = Math.round(Number(pget(props, 'Percentage', 100)));
                    this._state = Number(pget(props, 'State', 0));
                    this._iconName = String(pget(props, 'IconName', 'battery-full-symbolic'));
                } catch (e) {
                    this._hasBattery = false;
                }
                this._checkBatterySaver();
                this._changed();
            });
    }

    refreshOptimizer() {
        if (this._destroyed)
            return;

        this._bus.call(
            SYSTEMD, OPTIMIZER_UNIT, 'org.freedesktop.DBus.Properties', 'Get',
            new GLib.Variant('(ss)', [UNIT_IFACE, 'ActiveState']),
            null, Gio.DBusCallFlags.NONE, -1, null, (bus, result) => {
                if (this._destroyed)
                    return;
                try {
                    const res = bus.call_finish(result);
                    const val = res.deepUnpack()[0];
                    const stateStr = typeof val.unpack === 'function' ? val.unpack() : val;
                    this._optimizerActive = stateStr === 'active';
                } catch (e) {
                    this._optimizerActive = false;
                }
                this._readOptimizerStats();
                this._changed();
            });
    }

    _readOptimizerStats() {
        let zramSavedStr = null;
        try {
            const [ok, bytes] = GLib.file_get_contents('/sys/block/zram0/mm_stat');
            if (ok) {
                const parts = new TextDecoder().decode(bytes).trim().split(/\s+/);
                const orig = parseInt(parts[0], 10) || 0;
                const total = parseInt(parts[2], 10) || 0;
                if (orig > 0 && total > 0) {
                    const savedMb = Math.round((orig - total) / (1024 * 1024));
                    const ratio = (orig / (parseInt(parts[1], 10) || total)).toFixed(1);
                    zramSavedStr = `${savedMb} MB (${ratio}:1)`;
                }
            }
        } catch (e) {}

        let focusedApp = null;
        try {
            const paths = [`/run/user/${GLib.get_uid()}/pulsaros-focused.json`];
            for (const p of paths) {
                if (GLib.file_test(p, GLib.FileTest.EXISTS)) {
                    const [ok, content] = GLib.file_get_contents(p);
                    if (ok) {
                        const data = JSON.parse(new TextDecoder().decode(content));
                        focusedApp = data.wm_class || data.app_id || (data.pid ? `PID ${data.pid}` : null);
                        break;
                    }
                }
            }
        } catch (e) {}

        let loadStr = null;
        try {
            const [ok, content] = GLib.file_get_contents('/proc/loadavg');
            if (ok) {
                const parts = new TextDecoder().decode(content).trim().split(' ');
                loadStr = `${parts[0]}`;
            }
        } catch (e) {}

        this._optimizerStats = {
            zramSaved: zramSavedStr || T('Active', 'Attivo', 'Activo'),
            focusedApp: focusedApp || T('Normal', 'Normale', 'Normal'),
            load: loadStr || '0.0',
        };
    }

    _checkBatterySaver() {
        try {
            const [ok, content] = GLib.file_get_contents('/sys/devices/system/cpu/cpu0/cpufreq/energy_performance_preference');
            if (ok) {
                const epp = new TextDecoder().decode(content).trim();
                this._batterySaverActive = epp === 'power' || epp === 'balance_power';
            }
        } catch (e) {}
    }

    setBatterySaver(enabled) {
        this._batterySaverActive = enabled;
        const epp = enabled ? 'power' : 'balance_performance';
        const cmd = `sudo -n bash -c 'for f in /sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference; do echo ${epp} > "$f" 2>/dev/null; done'`;
        try {
            GLib.spawn_command_line_async(cmd);
        } catch (e) {
            console.warn(`pulsar-bar: setBatterySaver failed: ${e}`);
        }
        later(350, () => {
            this._checkBatterySaver();
            this._changed();
        });
    }

    setOptimizer(enabled) {
        this._optimizerActive = enabled;
        const action = enabled ? 'start' : 'stop';
        const cmd = `sudo -n systemctl ${action} pulsaros-optimizer.service`;
        try {
            GLib.spawn_command_line_async(cmd);
        } catch (e) {
            console.warn(`pulsar-bar: setOptimizer failed: ${e}`);
        }
        later(600, () => {
            this.refreshOptimizer();
        });
    }

    _subscribe() {
        try {
            const id = this._bus.signal_subscribe(
                UPOWER, 'org.freedesktop.DBus.Properties', 'PropertiesChanged',
                null, null, Gio.DBusSignalFlags.NONE, () => this.refresh());
            this._signals.push(id);
        } catch (e) {}

        try {
            const id = this._bus.signal_subscribe(
                SYSTEMD, 'org.freedesktop.DBus.Properties', 'PropertiesChanged',
                OPTIMIZER_UNIT, null, Gio.DBusSignalFlags.NONE, () => this.refreshOptimizer());
            this._signals.push(id);
        } catch (e) {}
    }

    _changed() {
        if (!this._destroyed)
            this._onChanged?.();
    }

    destroy() {
        if (this._destroyed)
            return;
        this._destroyed = true;
        for (const id of this._signals) {
            try { this._bus.signal_unsubscribe(id); } catch (e) {}
        }
        this._signals = [];
        this._onChanged = null;
    }
}

function getDirectChild(actor, targetBox) {
    if (!actor || !targetBox) return null;
    let cur = actor;
    while (cur) {
        if (cur.get_parent() === targetBox)
            return cur;
        cur = cur.get_parent();
    }
    return null;
}

function normalizeBluetoothIcon(icon) {
    if (!icon || icon === 'bluetooth')
        return 'bluetooth-active-symbolic';
    const name = String(icon);
    return name.endsWith('-symbolic') ? name : `${name}-symbolic`;
}

/* ------------------------------------------------------------------ *
 * Panel widgets
 * ------------------------------------------------------------------ */

function createBarButton(accessibleName, dontCreateMenu, menuAlignment = 0.5) {
    const button = new PanelMenu.Button(menuAlignment, accessibleName,
        Boolean(dontCreateMenu));
    button.add_style_class_name('pulsar-bar-button');

    try {
        button.set_pivot_point(0.5, 0.5);
    } catch (e) {
        // Older shell versions still work without an explicit pivot.
    }

    const animateScale = (scale, duration, mode) => {
        try {
            button.ease({
                scale_x: scale,
                scale_y: scale,
                duration,
                mode,
            });
        } catch (e) {
            // The button may already be destroyed during a fast reload.
        }
    };

    button.connect('button-press-event', () => {
        animateScale(0.94, 90, Clutter.AnimationMode.EASE_OUT_QUAD);
        return Clutter.EVENT_PROPAGATE;
    });
    button.connect('button-release-event', () => {
        animateScale(1, 180, Clutter.AnimationMode.EASE_OUT_CUBIC);
        return Clutter.EVENT_PROPAGATE;
    });

    return button;
}

function createStatusIcon(iconName) {
    return new St.Icon({
        icon_name: iconName,
        fallback_icon_name: iconName,
        style_class: 'system-status-icon pulsar-bar-icon',
    });
}

/* Two stacked switches, measured on the Tahoe Control Center glyph.  Drawn by
 * hand so it inherits the panel foreground colour and stays crisp at any DPI. */
const CC_TRACK_WIDTH = 12.4;
const CC_TRACK_HEIGHT = 5.6;
const CC_TRACK_GAP = 1.2;
const CC_STROKE = 1.1;
const CC_KNOB_WIDTH = 4.6;
const CC_KNOB_HEIGHT = 2.8;

function createControlCenterIcon() {
    const icon = new St.DrawingArea({
        style_class: 'system-status-icon pulsar-bar-icon pulsar-cc-icon',
        width: 16,
        height: 16,
        reactive: false,
    });

    icon.connect('repaint', area => {
        const cr = area.get_context();
        const [width, height] = area.get_surface_size();
        const color = area.get_theme_node().get_foreground_color();
        const scale = Math.min(width, height) / 16;
        const offsetX = (width - 16 * scale) / 2;
        const offsetY = (height - 16 * scale) / 2;

        const px = value => offsetX + value * scale;
        const py = value => offsetY + value * scale;

        const capsule = (x, y, w, h) => {
            const radius = h / 2;
            cr.newSubPath();
            cr.arc(px(x + radius), py(y + radius), radius * scale,
                Math.PI / 2, Math.PI * 1.5);
            cr.arc(px(x + w - radius), py(y + radius), radius * scale,
                Math.PI * 1.5, Math.PI / 2);
            cr.closePath();
        };

        const trackX = (16 - CC_TRACK_WIDTH) / 2;
        const upperY = (16 - (2 * CC_TRACK_HEIGHT + CC_TRACK_GAP)) / 2;
        const lowerY = upperY + CC_TRACK_HEIGHT + CC_TRACK_GAP;
        const knobInset = (CC_TRACK_HEIGHT - CC_KNOB_HEIGHT) / 2;
        const knobY = trackY => trackY + knobInset;

        cr.setSourceColor(color);
        cr.setLineWidth(CC_STROKE * scale);

        capsule(trackX + CC_STROKE / 2, upperY + CC_STROKE / 2,
            CC_TRACK_WIDTH - CC_STROKE, CC_TRACK_HEIGHT - CC_STROKE);
        cr.stroke();
        capsule(trackX + knobInset, knobY(upperY),
            CC_KNOB_WIDTH, CC_KNOB_HEIGHT);
        cr.fill();

        cr.setFillRule(Cairo.FillRule.EVEN_ODD);
        capsule(trackX, lowerY, CC_TRACK_WIDTH, CC_TRACK_HEIGHT);
        capsule(trackX + CC_TRACK_WIDTH - knobInset - CC_KNOB_WIDTH,
            knobY(lowerY), CC_KNOB_WIDTH, CC_KNOB_HEIGHT);
        cr.fill();

        cr.$dispose();
    });

    return icon;
}

function styleStatusMenu(menu, contentClass) {
    menu.actor.add_style_class_name('pulsar-status-menu');
    menu.box.add_style_class_name('pulsar-status-menu-content');
    menu.box.add_style_class_name(contentClass);
}

/* ------------------------------------------------------------------ *
 * Extension controller
 * ------------------------------------------------------------------ */

export function crea(extension) {
    return new MenuBarIcons(extension);
}

class MenuBarIcons {
    constructor(extension) {
        this._ext = extension;
        this._alive = false;
        this._buttons = [];
        this._ids = [];

        this._qs = null;
        this._qsSourceActor = null;
        this._qsWasVisible = true;
        this._qsHidden = false;

        this._wifiService = null;
        this._wifiButton = null;
        this._wifiIcon = null;
        this._wifiToggle = null;
        this._wifiSection = null;
        this._wifiRows = new Map();
        this._wifiSignature = null;
        this._wifiSyncing = false;

        this._btService = null;
        this._btButton = null;
        this._btIcon = null;
        this._btToggle = null;
        this._btSection = null;
        this._btRows = new Map();
        this._btSignature = null;
        this._btSyncing = false;

        this._batteryService = null;
        this._batteryButton = null;
        this._batteryIcon = null;
        this._batteryLabel = null;
        this._batteryHeaderLabel = null;
        this._batteryStatusLabel = null;
        this._batteryProgressFill = null;
        this._batteryProgressBox = null;
        this._batterySaverToggle = null;
        this._optimizerToggle = null;
        this._optimizerStatsSection = null;
        this._batterySyncing = false;

        this._ccButton = null;
        this._island = null;
    }

    enable() {
        this._alive = true;

        this._qs = Main.panel.statusArea.quickSettings ?? null;
        this._qsSourceActor = this._qs?.container ?? null;

        try {
            if (this._qs?.menu && this._qsSourceActor)
                this._qs.menu.sourceActor = this._qsSourceActor;
        } catch (e) {
            // A disabled extension may have left a dead anchor behind.
        }

        this._wifiService = new WifiService(() => this._syncWifi());
        this._btService = new BluetoothService(() => this._syncBluetooth());
        this._batteryService = new BatteryService(() => this._syncBattery());

        this._wifiButton = this._buildWifi();
        this._btButton = this._buildBluetooth();
        this._batteryButton = this._buildBattery();
        this._ccButton = this._buildControlCenter();

        this._add(this._wifiButton, 'pulsar-wifi', 0);
        this._add(this._btButton, 'pulsar-bluetooth', 1);
        this._add(this._batteryButton, 'pulsar-battery', 2);
        this._add(this._ccButton, 'pulsar-control-center', 3);

        this._repositionButtons();
        later(50, () => this._repositionButtons());
        later(150, () => this._repositionButtons());
        later(400, () => this._repositionButtons());
        later(1000, () => this._repositionButtons());

        try {
            const box = Main.panel._rightBox;
            if (box) {
                const onLayoutChange = () => {
                    if (this._repositionTimer) return;
                    this._repositionTimer = later(40, () => {
                        this._repositionTimer = 0;
                        this._repositionButtons();
                    });
                };
                for (const sig of ['child-added', 'child-removed', 'actor-added', 'actor-removed']) {
                    try {
                        const id = box.connect(sig, onLayoutChange);
                        this._ids.push([box, id]);
                    } catch (e) {}
                }
            }
        } catch (e) {}

        this._watchQuickSettings();
        this._hideStockQuickSettings();

        this._syncWifi(true);
        this._syncBluetooth(true);
        this._syncBattery();
    }

    disable() {
        this._alive = false;

        for (const [object, id] of this._ids) {
            try {
                object.disconnect(id);
            } catch (e) {
                // Object already gone.
            }
        }
        this._ids = [];

        this._wifiService?.destroy();
        this._btService?.destroy();
        this._batteryService?.destroy();
        this._wifiService = null;
        this._btService = null;
        this._batteryService = null;

        /* If the Quick Settings popup is open it is anchored to the CC button
         * we are about to remove: close it and put the original anchor back. */
        try {
            if (this._qs?.menu?.isOpen)
                this._qs.menu.close();
            if (this._qs?.menu && this._qsSourceActor)
                this._qs.menu.sourceActor = this._qsSourceActor;
        } catch (e) {
            // Shell already tearing the popup down.
        }

        for (const button of this._buttons) {
            try {
                button.destroy();
            } catch (e) {
                // Already destroyed by a session-mode change.
            }
        }
        this._buttons = [];

        if (this._qsVisibilityId && this._qs?.container) {
            try { this._qs.container.disconnect(this._qsVisibilityId); } catch (e) {}
            this._qsVisibilityId = 0;
        }

        if (this._qsHidden && this._qs?.container) {
            try {
                this._qs.container.visible = this._qsWasVisible;
            } catch (e) {
                // Panel is gone (lock screen / logout).
            }
        }
        this._qsHidden = false;

        this._wifiButton = null;
        this._wifiIcon = null;
        this._wifiToggle = null;
        this._wifiSection = null;
        this._wifiRows.clear();
        this._wifiSignature = null;

        this._btButton = null;
        this._btIcon = null;
        this._btToggle = null;
        this._btSection = null;
        this._btRows.clear();
        this._btSignature = null;

        this._batteryButton = null;
        this._batteryIcon = null;
        this._batteryLabel = null;
        this._batteryHeaderLabel = null;
        this._batteryStatusLabel = null;
        this._batteryProgressFill = null;
        this._batteryProgressBox = null;
        this._batterySaverToggle = null;
        this._optimizerToggle = null;
        this._optimizerStatsSection = null;

        this._ccButton = null;
        this._qs = null;
        this._qsSourceActor = null;
    }

    _add(button, role, position) {
        try {
            // Clean up any existing role in case of reload
            try {
                if (Main.panel.statusArea && Main.panel.statusArea[role]) {
                    try { Main.panel.statusArea[role].destroy(); } catch (e) {}
                    try { Main.panel._rightBox.remove_child(Main.panel.statusArea[role]); } catch (e) {}
                    try { delete Main.panel.statusArea[role]; } catch (e) {}
                }
            } catch (e) {}
            // Also search by name
            try {
                const kids = Main.panel._rightBox.get_children();
                for (let c of kids) {
                    if (c && c.get_name && c.get_name() === role) {
                        try { c.destroy(); Main.panel._rightBox.remove_child(c); } catch (e) {}
                    }
                }
            } catch (e) {}
            Main.panel.addToStatusArea(role, button, position, 'right');
            this._buttons.push(button);
        } catch (e) {
            console.warn(`pulsar-bar: could not add ${role}: ${e}`);
            // Try direct insert as fallback
            try {
                Main.panel._rightBox.insert_child_at_index(button, position || 0);
                this._buttons.push(button);
            } catch (e2) {}
        }
    }

    _repositionButtons() {
        if (!this._alive) return;
        try {
            const box = Main.panel._rightBox;
            if (!box) return;

            // Ensure dateMenu is located in rightBox at the right end (macOS layout)
            const dateMenu = Main.panel.statusArea.dateMenu;
            if (dateMenu) {
                const centerBox = Main.panel._centerBox;
                if (centerBox) {
                    const directCenterChild = getDirectChild(dateMenu, centerBox);
                    if (directCenterChild) {
                        centerBox.remove_child(directCenterChild);
                        box.add_child(directCenterChild);
                    }
                }
            }

            // Align notification banners to the top-right corner
            try {
                if (Main.messageTray) {
                    Main.messageTray.bannerAlignment = Clutter.ActorAlign.END;
                    const bannerBin = Main.messageTray._bannerBin;
                    if (bannerBin) {
                        bannerBin.margin_left = 20;
                        bannerBin.margin_right = 20;
                        bannerBin.margin_top = 10;
                    }
                }
            } catch (ne) {}

            const dateDirectChild = getDirectChild(dateMenu, box);
            const ccDirectChild = getDirectChild(this._ccButton, box);

            if (!dateDirectChild || !ccDirectChild) return;

            // 0. Ensure Date is at the very end of rightBox
            let children = box.get_children();
            let dateIdx = children.indexOf(dateDirectChild);
            if (dateIdx >= 0 && dateIdx !== children.length - 1) {
                box.set_child_at_index(dateDirectChild, children.length - 1);
            }

            // 1. Move Control Center immediately before Date
            children = box.get_children();
            dateIdx = children.indexOf(dateDirectChild);
            let ccIdx = children.indexOf(ccDirectChild);

            if (dateIdx >= 0 && ccIdx >= 0) {
                const targetCcIdx = ccIdx < dateIdx ? dateIdx - 1 : dateIdx;
                if (ccIdx !== targetCcIdx && targetCcIdx >= 0) {
                    box.set_child_at_index(ccDirectChild, targetCcIdx);
                }
            }

            // 2. Move Battery immediately before Control Center
            const battDirectChild = getDirectChild(this._batteryButton, box);
            if (battDirectChild) {
                children = box.get_children();
                const currentCcIdx = children.indexOf(ccDirectChild);
                const battIdx = children.indexOf(battDirectChild);
                if (battIdx >= 0 && currentCcIdx > 0) {
                    const targetBattIdx = battIdx < currentCcIdx ? currentCcIdx - 1 : currentCcIdx;
                    if (battIdx !== targetBattIdx && targetBattIdx >= 0) {
                        box.set_child_at_index(battDirectChild, targetBattIdx);
                    }
                }
            }

            // 3. Move Bluetooth immediately before Battery (or Control Center if no Battery)
            const btDirectChild = getDirectChild(this._btButton, box);
            if (btDirectChild) {
                children = box.get_children();
                const anchorChild = battDirectChild || ccDirectChild;
                const anchorIdx = children.indexOf(anchorChild);
                const btIdx = children.indexOf(btDirectChild);
                if (btIdx >= 0 && anchorIdx > 0) {
                    const targetBtIdx = btIdx < anchorIdx ? anchorIdx - 1 : anchorIdx;
                    if (btIdx !== targetBtIdx && targetBtIdx >= 0) {
                        box.set_child_at_index(btDirectChild, targetBtIdx);
                    }
                }
            }

            // 4. Move Wi-Fi immediately before Bluetooth
            const wifiDirectChild = getDirectChild(this._wifiButton, box);
            if (wifiDirectChild) {
                children = box.get_children();
                const anchorChild = btDirectChild || battDirectChild || ccDirectChild;
                const anchorIdx = children.indexOf(anchorChild);
                const wifiIdx = children.indexOf(wifiDirectChild);
                if (wifiIdx >= 0 && anchorIdx > 0) {
                    const targetWifiIdx = wifiIdx < anchorIdx ? anchorIdx - 1 : anchorIdx;
                    if (wifiIdx !== targetWifiIdx && targetWifiIdx >= 0) {
                        box.set_child_at_index(wifiDirectChild, targetWifiIdx);
                    }
                }
            }
        } catch (e) {
            console.warn(`pulsar-bar: reposition error: ${e}`);
        }
    }

    _watchQuickSettings() {
        if (!this._qs?.menu)
            return;

        this._ids.push([this._qs.menu,
            this._qs.menu.connect('open-state-changed', (_menu, open) => {
                if (!this._ccButton)
                    return;
                if (open)
                    this._ccButton.add_style_class_name('pulsar-cc-open');
                else
                    this._ccButton.remove_style_class_name('pulsar-cc-open');
            })]);
    }

    _hideStockQuickSettings() {
        try {
            if (!this._qs?.container)
                return;
            this._qsWasVisible = this._qs.container.visible;
            this._qs.container.visible = false;
            this._qsHidden = true;

            if (!this._qsVisibilityId) {
                this._qsVisibilityId = this._qs.container.connect('notify::visible', () => {
                    if (this._alive && this._qsHidden && this._qs?.container?.visible) {
                        this._qs.container.visible = false;
                    }
                });
            }
        } catch (e) {
            this._qsHidden = false;
        }
    }

    /* ---------- Control Center ---------- */

    _buildControlCenter() {
        const button = createBarButton(T('Control Center', 'Centro di Controllo', 'Centro de control'), true);
        /* Il tema non contiene 'control-center-symbolic': il nome veniva cercato
         * nei temi icone e, non trovato, la Shell disegnava image-missing (il
         * riquadro rosso). Carico l'SVG della nostra cartella icons/ come file
         * concreto, senza dipendere dal search path dei temi. */
        try {
            const dir = GLib.path_get_dirname(
                GLib.filename_from_uri(import.meta.url)[0]);
            const file = Gio.File.new_for_path(
                GLib.build_filenamev([dir, 'icons', 'control-center-symbolic.svg']));
            if (file.query_exists(null)) {
                button.add_child(new St.Icon({
                    gicon: new Gio.FileIcon({ file }),
                    style_class: 'system-status-icon pulsar-bar-icon pulsar-cc-icon',
                }));
            } else {
                button.add_child(createStatusIcon('control-center-symbolic'));
            }
        } catch (e) {
            logError(e, 'pulsar-bar: control center icon');
            button.add_child(createControlCenterIcon());
        }
        button.connect('button-release-event', () => {
            later(10, () => this._toggleControlCenter());
            return Clutter.EVENT_STOP;
        });
        return button;
    }

    _toggleControlCenter() {
        if (!this._alive)
            return false;

        /* macOS 27: l'icona CC apre il Control Centre Pulsar, ancorato al
         * bottone stesso. Il CC non e' un pannello separato: sostituisce il
         * contenuto di Quick Settings, che il bottone stock ora e' nascosto. */
        const menu = Main.panel.statusArea.quickSettings?.menu;
        if (!menu)
            return false;

        try {
            if (menu.isOpen) {
                menu.close();
                return true;
            }
            if (this._ccButton)
                menu.sourceActor = this._ccButton;
            menu.open();
            return true;
        } catch (e) {
            console.warn(`pulsar-bar: could not toggle Control Center: ${e}`);
            return false;
        }
    }

    /* ---------- Wi-Fi ---------- */

    _buildWifi() {
        const button = createBarButton(T('Wi-Fi', 'Wi-Fi'), false, 1.0);
        this._wifiIcon = createStatusIcon('network-wireless-signal-none-symbolic');
        button.add_child(this._wifiIcon);

        styleStatusMenu(button.menu, 'pulsar-wifi-menu-content');

        this._wifiToggle = new PopupMenu.PopupSwitchMenuItem(T('Wi-Fi', 'Wi-Fi', 'Wi-Fi'), false);
        this._wifiToggle.add_style_class_name('pulsar-status-toggle');
        this._wifiToggle.connect('toggled', (_item, state) => {
            if (this._wifiSyncing)
                return;
            this._wifiService?.setEnabled(state);
        });
        button.menu.addMenuItem(this._wifiToggle);

        this._wifiSection = new PopupMenu.PopupMenuSection();
        button.menu.addMenuItem(this._wifiSection);
        button.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        const settings = new PopupMenu.PopupImageMenuItem(
            T('Wi-Fi Settings…', 'Impostazioni Wi-Fi…', 'Ajustes de Wi-Fi…'),
            'preferences-system-symbolic');
        settings.connect('activate', () => {
            openSettingsPanel('gnome-wifi-panel.desktop',
                'gnome-control-center wifi');
        });
        button.menu.addMenuItem(settings);

        this._ids.push([button.menu,
            button.menu.connect('open-state-changed', (_menu, open) => {
                if (open) {
                    this._wifiService?.startScanning();
                    this._syncWifi(true);
                } else {
                    this._wifiService?.stopScanning();
                }
            })]);

        return button;
    }

    _syncWifi(forceRows = false) {
        if (!this._alive || !this._wifiService?.ready ||
            !this._wifiButton || !this._wifiToggle)
            return;

        const service = this._wifiService;
        const active = service.enabled ? service.activeNetwork : '';

        /* macOS: voce presente solo mentre il Wi-Fi e' attivo. */
        this._wifiButton.visible = Boolean(service.enabled && service.hardwareEnabled);

        this._wifiIcon.icon_name = service.iconName;
        this._wifiButton.accessible_name = active
            ? T(`Wi-Fi, connected to ${active}`, `Wi-Fi, connesso a ${active}`, `Wi-Fi, conectado a ${active}`)
            : T(`Wi-Fi, ${service.enabled ? 'not connected' : 'off'}`,
                `Wi-Fi, ${service.enabled ? 'non connesso' : 'disattivato'}`,
                `Wi-Fi, ${service.enabled ? 'no conectado' : 'desactivado'}`);

        this._wifiSyncing = true;
        try {
            this._wifiToggle.setToggleState(service.enabled);
            this._wifiToggle.sensitive = service.hardwareEnabled;
            if (this._wifiToggle.setStatus)
                this._wifiToggle.setStatus(service.hardwareEnabled
                    ? null
                    : T('Unavailable', 'Non disponibile', 'No disponible'));
        } finally {
            this._wifiSyncing = false;
        }

        this._updateWifiRows(forceRows);
    }

    _updateWifiRows(force) {
        if (!this._alive || !this._wifiSection || !this._wifiButton)
            return;
        if (!force && !this._wifiButton.menu.isOpen)
            return;

        const service = this._wifiService;
        const networks = service.enabled ? service.networks : [];
        const known = networks.filter(network => network.known || network.active)
            .slice(0, 6);
        const other = networks.filter(network => !(network.known || network.active))
            .slice(0, 6);

        const signature = [
            service.enabled,
            known.map(network => `${network.name}\u0001${network.active ? 1 : 0}\u0001${network.secure ? 1 : 0}`).join('\u0002'),
            other.map(network => `${network.name}\u0001${network.secure ? 1 : 0}`).join('\u0002'),
        ].join('\u0003');

        if (!force && signature === this._wifiSignature) {
            /* Signal strength changes must not rebuild (and reset the hover of)
             * the whole list: refresh just the glyphs in place. */
            for (const network of known.concat(other)) {
                const row = this._wifiRows.get(network.name);
                if (row)
                    row.item.setIcon(network.iconName);
            }
            return;
        }

        this._wifiSignature = signature;
        this._wifiSection.removeAll();
        this._wifiRows.clear();

        if (!service.enabled)
            return;

        if (!networks.length) {
            this._wifiSection.addMenuItem(this._emptyRow(
                T('Searching for networks…', 'Ricerca reti…', 'Buscando redes…')));
            return;
        }

        if (known.length) {
            this._wifiSection.addMenuItem(this._heading(
                T('Known Networks', 'Reti conosciute', 'Redes conocidas')));
            for (const network of known)
                this._wifiSection.addMenuItem(this._wifiRow(network));
        }

        if (other.length) {
            this._wifiSection.addMenuItem(this._heading(
                known.length
                    ? T('Other Networks', 'Altre reti', 'Otras redes')
                    : T('Networks', 'Reti', 'Redes')));
            for (const network of other)
                this._wifiSection.addMenuItem(this._wifiRow(network));
        }
    }

    _wifiRow(network) {
        const item = new PopupMenu.PopupImageMenuItem(
            network.name, network.iconName);
        item.label.x_expand = true;
        item.label.clutter_text.ellipsize = Pango.EllipsizeMode.END;

        if (network.active) {
            item.add_style_class_name('pulsar-wifi-active');
            item.setOrnament(PopupMenu.Ornament.CHECK);
        }

        if (network.secure) {
            const lock = new St.Icon({
                icon_name: 'network-wireless-encrypted-symbolic',
                style_class: 'pulsar-lock-icon',
                y_align: Clutter.ActorAlign.CENTER,
            });
            item.insert_child_at_index(lock, 2);
        }

        item.accessible_name = network.active
            ? T(`Disconnect from ${network.name}`, `Disconnetti da ${network.name}`, `Desconectar de ${network.name}`)
            : T(`Connect to ${network.name}`, `Connetti a ${network.name}`, `Conectar a ${network.name}`);

        item.connect('activate', () => {
            const handled = this._wifiService?.toggleNetwork(network);
            if (!handled) {
                openSettingsPanel('gnome-wifi-panel.desktop',
                    'gnome-control-center wifi');
            }
        });

        this._wifiRows.set(network.name, {item});
        return item;
    }

    /* ---------- Bluetooth ---------- */

    _buildBluetooth() {
        const button = createBarButton(T('Bluetooth', 'Bluetooth', 'Bluetooth'), false, 1.0);
        this._btIcon = createStatusIcon('bluetooth-disabled-symbolic');
        button.add_child(this._btIcon);

        styleStatusMenu(button.menu, 'pulsar-bluetooth-menu-content');

        this._btToggle = new PopupMenu.PopupSwitchMenuItem(
            T('Bluetooth', 'Bluetooth', 'Bluetooth'), false);
        this._btToggle.add_style_class_name('pulsar-status-toggle');
        this._btToggle.connect('toggled', (_item, state) => {
            if (this._btSyncing)
                return;
            this._btService?.setPowered(state);
        });
        button.menu.addMenuItem(this._btToggle);

        this._btSection = new PopupMenu.PopupMenuSection();
        button.menu.addMenuItem(this._btSection);
        button.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        const settings = new PopupMenu.PopupImageMenuItem(
            T('Bluetooth Settings…', 'Impostazioni Bluetooth…', 'Ajustes de Bluetooth…'),
            'preferences-system-symbolic');
        settings.connect('activate', () => {
            openSettingsPanel('gnome-bluetooth-panel.desktop',
                'gnome-control-center bluetooth');
        });
        button.menu.addMenuItem(settings);

        this._ids.push([button.menu,
            button.menu.connect('open-state-changed', (_menu, open) => {
                if (open) {
                    this._btService?.startDiscovery();
                    this._btService?.refresh();
                    this._syncBluetooth(true);
                } else {
                    this._btService?.stopDiscovery();
                }
            })]);

        return button;
    }

    _syncBluetooth(forceRows = false) {
        if (!this._alive || !this._btService?.ready ||
            !this._btButton || !this._btToggle)
            return;

        const service = this._btService;
        const powered = service.powered;
        const connected = service.devices.find(device => device.connected);

        // Keep visible in the menu bar as long as an adapter is available
        this._btButton.visible = Boolean(service.available);

        this._btIcon.icon_name = powered
            ? 'bluetooth-active-symbolic'
            : 'bluetooth-disabled-symbolic';

        if (!service.available) {
            this._btButton.accessible_name =
                T('Bluetooth, unavailable', 'Bluetooth, non disponibile', 'Bluetooth, no disponible');
        } else if (!powered) {
            this._btButton.accessible_name =
                T('Bluetooth, off', 'Bluetooth, disattivato', 'Bluetooth, desactivado');
        } else if (connected) {
            this._btButton.accessible_name = T(
                `Bluetooth, connected to ${connected.name}`,
                `Bluetooth, connesso a ${connected.name}`,
                `Bluetooth, conectado a ${connected.name}`);
        } else {
            this._btButton.accessible_name =
                T('Bluetooth, on', 'Bluetooth, attivo', 'Bluetooth, activado');
        }

        this._btSyncing = true;
        try {
            this._btToggle.setToggleState(powered);
            this._btToggle.sensitive = service.available;
            if (this._btToggle.setStatus)
                this._btToggle.setStatus(service.available
                    ? null
                    : T('Unavailable', 'Non disponibile', 'No disponible'));
        } finally {
            this._btSyncing = false;
        }

        this._updateBluetoothRows(forceRows);
    }

    _updateBluetoothRows(force) {
        if (!this._alive || !this._btSection || !this._btButton)
            return;
        if (!force && !this._btButton.menu.isOpen)
            return;

        const service = this._btService;
        const devices = service.powered ? service.devices : [];
        const signature = [
            service.available,
            service.powered,
            service.discovering,
            devices.map(device => [
                device.path, device.name, device.icon,
                device.connected ? 1 : 0, device.paired ? 1 : 0, device.battery ?? '',
            ].join('\u0001')).join('\u0002'),
        ].join('\u0003');

        if (!force && signature === this._btSignature)
            return;

        this._btSignature = signature;
        this._btSection.removeAll();
        this._btRows.clear();

        if (!service.powered)
            return;

        const saved = devices.filter(d => d.paired || d.connected);
        const discovered = devices.filter(d => !d.paired && !d.connected);

        if (saved.length > 0) {
            this._btSection.addMenuItem(this._heading(
                T('Saved Devices', 'Dispositivi salvati', 'Dispositivos guardados')));
            for (const device of saved)
                this._btSection.addMenuItem(this._bluetoothRow(device));
        }

        if (discovered.length > 0) {
            this._btSection.addMenuItem(this._heading(
                T('Nearby Devices', 'Dispositivi vicini', 'Dispositivos cercanos')));
            for (const device of discovered)
                this._btSection.addMenuItem(this._bluetoothRow(device));
        } else if (saved.length === 0) {
            this._btSection.addMenuItem(this._emptyRow(
                service.discovering
                    ? T('Searching for devices…', 'Ricerca dispositivi…', 'Buscando dispositivos…')
                    : T('No devices found', 'Nessun dispositivo', 'No se encontraron dispositivos')));
        }
    }

    _bluetoothRow(device) {
        const item = new PopupMenu.PopupImageMenuItem(
            device.name, device.icon);
        item.label.x_expand = true;
        item.label.clutter_text.ellipsize = Pango.EllipsizeMode.END;

        if (device.connected) {
            item.add_style_class_name('pulsar-bt-connected');
            item.setOrnament(PopupMenu.Ornament.CHECK);
        }

        if (device.battery !== null) {
            const battery = new St.Label({
                text: `${device.battery}%`,
                style_class: 'pulsar-status-note',
                y_align: Clutter.ActorAlign.CENTER,
            });
            item.insert_child_at_index(battery, 2);
        }

        item.accessible_name = device.connected
            ? T(`Disconnect ${device.name}`, `Disconnetti ${device.name}`, `Desconectar ${device.name}`)
            : T(`Connect ${device.name}`, `Connetti ${device.name}`, `Conectar ${device.name}`);

        item.connect('activate', () => {
            this._btService?.toggleDevice(device.path, !device.connected);
        });

        this._btRows.set(device.path, {item});
        return item;
    }

    /* ---------- Battery & Optimizer ---------- */

    _buildBattery() {
        const button = createBarButton(T('Battery', 'Batteria', 'Batería'), false, 1.0);

        const box = new St.BoxLayout({
            style_class: 'pulsar-bar-button-box',
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._batteryIcon = createStatusIcon('battery-full-symbolic');
        this._batteryLabel = new St.Label({
            text: '100%',
            style_class: 'pulsar-bar-label',
            y_align: Clutter.ActorAlign.CENTER,
        });
        box.add_child(this._batteryIcon);
        box.add_child(this._batteryLabel);
        button.add_child(box);

        styleStatusMenu(button.menu, 'pulsar-battery-menu-content');

        // Header with percentage & status
        this._batteryHeaderLabel = new St.Label({
            text: '100%',
            style_class: 'pulsar-battery-title',
        });
        this._batteryStatusLabel = new St.Label({
            text: T('Fully Charged', 'Carica completa', 'Carga completa'),
            style_class: 'pulsar-status-note',
            y_align: Clutter.ActorAlign.CENTER,
        });

        this._batteryProgressBox = new St.BoxLayout({
            style_class: 'pulsar-battery-bar-trough',
            x_expand: true,
        });
        this._batteryProgressFill = new St.Widget({
            style_class: 'pulsar-battery-bar-fill',
            width: 100,
        });
        this._batteryProgressBox.add_child(this._batteryProgressFill);

        const headerBox = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            style_class: 'pulsar-battery-header-box',
            x_expand: true,
        });
        const titleRow = new St.BoxLayout({
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        });
        titleRow.add_child(this._batteryHeaderLabel);
        const spacer = new St.Widget({x_expand: true});
        titleRow.add_child(spacer);
        titleRow.add_child(this._batteryStatusLabel);

        headerBox.add_child(titleRow);
        headerBox.add_child(this._batteryProgressBox);

        const headerItem = new PopupMenu.PopupBaseMenuItem({reactive: false, can_focus: false});
        headerItem.add_child(headerBox);
        button.menu.addMenuItem(headerItem);

        // Power Saver switch
        this._batterySaverToggle = new PopupMenu.PopupSwitchMenuItem(
            T('Battery Saver', 'Risparmio energetico', 'Ahorro de batería'), false);
        this._batterySaverToggle.add_style_class_name('pulsar-status-toggle');
        this._batterySaverToggle.connect('toggled', (_item, state) => {
            if (this._batterySyncing) return;
            this._batteryService?.setBatterySaver(state);
        });
        button.menu.addMenuItem(this._batterySaverToggle);

        button.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        // Optimizer Section
        this._optimizerToggle = new PopupMenu.PopupSwitchMenuItem(
            T('Pulsar OS Optimizer', 'Ottimizzatore Pulsar OS', 'Optimizador de Pulsar OS'), false);
        this._optimizerToggle.add_style_class_name('pulsar-status-toggle');
        this._optimizerToggle.connect('toggled', (_item, state) => {
            if (this._batterySyncing) return;
            this._batteryService?.setOptimizer(state);
        });
        button.menu.addMenuItem(this._optimizerToggle);

        this._optimizerStatsSection = new PopupMenu.PopupMenuSection();
        button.menu.addMenuItem(this._optimizerStatsSection);

        button.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        const optSettings = new PopupMenu.PopupImageMenuItem(
            T('Open Optimizer Panel…', 'Apri pannello ottimizzatore…', 'Abrir panel de optimizador…'),
            'emblem-system-symbolic');
        optSettings.connect('activate', () => {
            try {
                GLib.spawn_command_line_async('pulsaros-optimizer-gui');
            } catch (e) {
                console.warn(`pulsar-bar: launch optimizer gui failed: ${e}`);
            }
        });
        button.menu.addMenuItem(optSettings);

        const powerSettings = new PopupMenu.PopupImageMenuItem(
            T('Power Settings…', 'Impostazioni batteria…', 'Ajustes de energía…'),
            'preferences-system-symbolic');
        powerSettings.connect('activate', () => {
            openSettingsPanel('gnome-power-panel.desktop', 'gnome-control-center power');
        });
        button.menu.addMenuItem(powerSettings);

        this._ids.push([button.menu,
            button.menu.connect('open-state-changed', (_menu, open) => {
                if (open) {
                    this._batteryService?.refresh();
                    this._batteryService?.refreshOptimizer();
                    this._syncBattery();
                }
            })]);

        return button;
    }

    _syncBattery() {
        if (!this._alive || !this._batteryService || !this._batteryButton)
            return;

        const s = this._batteryService;
        this._batteryButton.visible = Boolean(s.hasBattery);
        if (!s.hasBattery)
            return;

        const pct = Math.max(0, Math.min(100, s.percentage));
        if (this._batteryLabel)
            this._batteryLabel.text = `${pct}%`;

        let iconName = s.iconName;
        if (!iconName || iconName === 'battery-missing-symbolic') {
            const level = pct >= 88 ? 100 : pct >= 63 ? 75 : pct >= 38 ? 50 : pct >= 13 ? 25 : 0;
            iconName = `battery-level-${level}-symbolic`;
        }
        if (this._batteryIcon)
            this._batteryIcon.icon_name = iconName;

        if (this._batteryHeaderLabel)
            this._batteryHeaderLabel.text = `${pct}%`;

        const stateText = s.isCharging
            ? T('Charging', 'In carica', 'Cargando')
            : (pct === 100 ? T('Fully Charged', 'Carica completa', 'Carga completa')
                           : T('On Battery', 'A batteria', 'En uso'));
        if (this._batteryStatusLabel)
            this._batteryStatusLabel.text = stateText;

        if (this._batteryProgressFill) {
            const fillWidth = Math.max(4, Math.round(260 * (pct / 100)));
            this._batteryProgressFill.width = fillWidth;
            if (s.isCharging) {
                this._batteryProgressFill.style = 'background-color: #0a84ff;';
            } else if (pct <= 20) {
                this._batteryProgressFill.style = 'background-color: #ff9f0a;';
            } else {
                this._batteryProgressFill.style = 'background-color: #30d158;';
            }
        }

        this._batterySyncing = true;
        try {
            this._batterySaverToggle?.setToggleState(s.batterySaverActive);
            this._optimizerToggle?.setToggleState(s.optimizerActive);
        } finally {
            this._batterySyncing = false;
        }

        this._updateOptimizerStats();
    }

    _updateOptimizerStats() {
        if (!this._optimizerStatsSection)
            return;
        this._optimizerStatsSection.removeAll();

        const s = this._batteryService;
        const stats = s?.optimizerStats;
        if (!stats) return;

        this._optimizerStatsSection.addMenuItem(this._heading(
            T('Optimizer Status & Performance', 'Stato e prestazioni ottimizzatore', 'Estado y rendimiento del optimizador')));

        const mkStatRow = (label, val) => {
            const item = new PopupMenu.PopupMenuItem(label, {reactive: false, can_focus: false});
            item.add_style_class_name('pulsar-stat-row');
            const vLabel = new St.Label({
                text: String(val),
                style_class: 'pulsar-status-note',
                y_align: Clutter.ActorAlign.CENTER,
            });
            item.insert_child_at_index(vLabel, 2);
            return item;
        };

        const stateLabel = s.optimizerActive
            ? T('Active (zero-lag governor)', 'Attivo (governor zero-lag)', 'Activo (protección fluidez)')
            : T('Stopped', 'Fermo', 'Detenido');
        this._optimizerStatsSection.addMenuItem(mkStatRow(T('Governor', 'Governor', 'Gobernador'), stateLabel));
        this._optimizerStatsSection.addMenuItem(mkStatRow(T('zRAM Memory Saved', 'RAM risparmiata zRAM', 'RAM ahorrada con zRAM'), stats.zramSaved));
        this._optimizerStatsSection.addMenuItem(mkStatRow(T('Focused App', 'App in primo piano', 'App en primer plano'), stats.focusedApp));
        this._optimizerStatsSection.addMenuItem(mkStatRow(T('CPU Load Avg', 'Carico CPU medio', 'Carga CPU media'), stats.load));
    }

    /* ---------- shared menu pieces ---------- */

    _heading(text) {
        const item = new PopupMenu.PopupMenuItem(text, {
            reactive: false,
            can_focus: false,
        });
        item.add_style_class_name('pulsar-status-heading');
        return item;
    }

    _emptyRow(text) {
        const item = new PopupMenu.PopupMenuItem(text, {
            reactive: false,
            can_focus: false,
        });
        item.add_style_class_name('pulsar-status-empty');
        return item;
    }
}
