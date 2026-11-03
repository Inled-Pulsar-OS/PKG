"""BlueZ (org.bluez) backend: BLE scan for Apple manufacturer data + device tracking.

Runs unprivileged (verified on bluez 5.87 without --experimental): the LE discovery
filter with DuplicateData makes bluetoothd publish Device1.ManufacturerData / RSSI.

Self-healing: if bluetoothd restarts, the adapter is powered off, or discovery stops, the
backend re-attaches (retry every few seconds) instead of exiting.
"""

import logging
from typing import Callable, Dict, Optional

import dbus
from gi.repository import GLib

from .protocol import VENDOR_ID

log = logging.getLogger(__name__)

BLUEZ = "org.bluez"
ADAPTER = "org.bluez.Adapter1"
DEVICE = "org.bluez.Device1"
BATTERY = "org.bluez.Battery1"
PROPS = "org.freedesktop.DBus.Properties"
OBJMGR = "org.freedesktop.DBus.ObjectManager"
RETRY_SECONDS = 3
CALL_TIMEOUT_SECONDS = 30  # pairing-level connects can take a while

# on_adv(address, rssi, payload)
# on_device(path, props, battery): props None = device removed; battery None = unchanged
OnAdv = Callable[[str, int, bytes], None]
OnDevice = Callable[[str, Optional[dict], Optional[int]], None]


def plain(value):
    """python-dbus types -> plain Python types."""
    if isinstance(value, dbus.Boolean):
        return bool(value)
    if isinstance(value, (dbus.Int16, dbus.Int32, dbus.Int64, dbus.UInt16, dbus.UInt32, dbus.UInt64, dbus.Byte)):
        return int(value)
    if isinstance(value, dbus.String):
        return str(value)
    if isinstance(value, (dbus.Array, list)):
        return [plain(v) for v in value]
    if isinstance(value, (dbus.Dictionary, dict)):
        return {plain(k): plain(v) for k, v in value.items()}
    return value


def apple_payload(manufacturer_data) -> Optional[bytes]:
    """Return the Apple (company id 76) payload from a Device1.ManufacturerData dict."""
    if not manufacturer_data or VENDOR_ID not in manufacturer_data:
        return None
    return bytes(manufacturer_data[VENDOR_ID])


class BluezBackend:
    def __init__(self, on_adv: OnAdv, on_device: OnDevice, bus=None):
        self._on_adv = on_adv
        self._on_device = on_device
        self._bus = bus
        self._adapter: Optional[str] = None
        self._devices: Dict[str, dict] = {}
        self._retry_id: Optional[int] = None
        self._started = False
        self._matches = []  # signal receivers + name watch, removed again in stop()

    # -- lifecycle ---------------------------------------------------------------------

    def start(self) -> None:
        if self._started:
            return
        self._started = True
        if self._bus is None:
            self._bus = dbus.SystemBus()
        bus = self._bus
        self._matches = [
            bus.add_signal_receiver(self._interfaces_added, "InterfacesAdded", OBJMGR, BLUEZ),
            bus.add_signal_receiver(self._interfaces_removed, "InterfacesRemoved", OBJMGR, BLUEZ),
            bus.add_signal_receiver(
                self._properties_changed, "PropertiesChanged", PROPS, BLUEZ, path_keyword="path"
            ),
        ]
        self._name_watch = bus.watch_name_owner(BLUEZ, self._owner_changed)

    def stop(self) -> None:
        if not self._started:
            return
        self._started = False
        if self._retry_id is not None:
            GLib.source_remove(self._retry_id)
            self._retry_id = None
        if self._adapter is not None:
            try:
                self._call_adapter("StopDiscovery")
            except dbus.DBusException:
                pass
        for match in self._matches:
            match.remove()
        self._matches = []
        self._name_watch.cancel()
        self._detach()

    # -- commands ----------------------------------------------------------------------

    def connect_device(self, address: str, done: Callable[[Optional[str]], None]) -> None:
        self._device_call(address, "Connect", done)

    def disconnect_device(self, address: str, done: Callable[[Optional[str]], None]) -> None:
        self._device_call(address, "Disconnect", done)

    def _device_call(self, address: str, method: str, done: Callable[[Optional[str]], None]) -> None:
        """Call Device1.<method> without blocking. done(None) on success, done(reason) on failure."""
        path = next((p for p, d in self._devices.items() if d.get("Address") == address), None)
        if path is None or self._bus is None:
            done(f"unknown device {address}")
            return
        iface = dbus.Interface(self._bus.get_object(BLUEZ, path, introspect=False), DEVICE)
        getattr(iface, method)(
            reply_handler=lambda: done(None),
            error_handler=lambda e: done(e.get_dbus_message() or str(e)),
            timeout=CALL_TIMEOUT_SECONDS,
        )

    # -- attach / detach ---------------------------------------------------------------

    def _owner_changed(self, owner: str) -> None:
        if not self._started:
            return
        if owner:
            log.info("bluetoothd is available")
            self._attach_or_retry()
        else:
            log.warning("bluetoothd went away")
            self._detach()
            self._schedule_retry()

    def _attach_or_retry(self) -> None:
        if not self._attach():
            self._schedule_retry()

    def _schedule_retry(self) -> None:
        if self._retry_id is None and self._started:
            self._retry_id = GLib.timeout_add_seconds(RETRY_SECONDS, self._retry)

    def _retry(self) -> bool:
        if not self._started:
            self._retry_id = None
            return False
        if self._attach():
            self._retry_id = None
            return False
        return True

    def _detach(self) -> None:
        self._adapter = None
        for path in list(self._devices):
            self._on_device(path, None, None)
        self._devices.clear()

    def _attach(self) -> bool:
        try:
            om = dbus.Interface(self._bus.get_object(BLUEZ, "/"), OBJMGR)
            objects = om.GetManagedObjects()
        except dbus.DBusException as e:
            log.debug("bluez not ready: %s", e)
            return False

        adapter = next((p for p, i in objects.items() if ADAPTER in i), None)
        if adapter is None:
            log.warning("no Bluetooth adapter")
            return False

        self._adapter = str(adapter)
        for path, ifaces in objects.items():
            self._interfaces_added(path, ifaces, initial=True)

        if not plain(objects[adapter][ADAPTER].get("Powered", True)):
            log.info("adapter is powered off; waiting")
            return True  # attached; discovery starts when Powered becomes true
        self._start_discovery()
        return True

    def _call_adapter(self, method: str, *args):
        iface = dbus.Interface(self._bus.get_object(BLUEZ, self._adapter), ADAPTER)
        return getattr(iface, method)(*args)

    def _start_discovery(self) -> None:
        if self._adapter is None:
            return
        try:
            self._call_adapter(
                "SetDiscoveryFilter", {"Transport": "le", "DuplicateData": dbus.Boolean(True)}
            )
            self._call_adapter("StartDiscovery")
            log.info("BLE discovery started on %s", self._adapter)
        except dbus.DBusException as e:
            name = e.get_dbus_name()
            if name.endswith("InProgress"):
                return  # already discovering
            log.warning("cannot start discovery: %s", e)
            self._schedule_retry_discovery()

    def _schedule_retry_discovery(self) -> None:
        def retry() -> bool:
            self._start_discovery()
            return False

        GLib.timeout_add_seconds(RETRY_SECONDS, retry)

    # -- signals -----------------------------------------------------------------------

    def _interfaces_added(self, path, ifaces, initial: bool = False) -> None:
        path = str(path)
        if DEVICE in ifaces:
            props = plain(ifaces[DEVICE])
            self._devices[path] = props
            battery = ifaces.get(BATTERY, {}).get("Percentage")
            self._on_device(path, dict(props), None if battery is None else int(battery))
            self._report_adv(props)
        elif BATTERY in ifaces and path in self._devices:
            self._on_device(path, dict(self._devices[path]), int(ifaces[BATTERY].get("Percentage", 0)))

    def _interfaces_removed(self, path, ifaces) -> None:
        path = str(path)
        if DEVICE in ifaces and path in self._devices:
            del self._devices[path]
            self._on_device(path, None, None)

    def _properties_changed(self, iface, changed, invalidated, path=None) -> None:
        path = str(path)
        changed = plain(changed)
        if iface == DEVICE:
            props = self._devices.setdefault(path, {})
            props.update(changed)
            self._on_device(path, dict(props), None)
            if "ManufacturerData" in changed or "RSSI" in changed:
                self._report_adv(props)
        elif iface == BATTERY:
            if "Percentage" in changed:
                self._on_device(path, dict(self._devices.get(path, {})), int(changed["Percentage"]))
        elif iface == ADAPTER and path == self._adapter:
            if changed.get("Powered") is True or changed.get("Discovering") is False:
                self._start_discovery()

    def _report_adv(self, props: dict) -> None:
        payload = apple_payload(props.get("ManufacturerData"))
        if payload is None or "Address" not in props:
            return
        rssi = props.get("RSSI")
        if rssi is None:
            return
        self._on_adv(str(props["Address"]), int(rssi), payload)
