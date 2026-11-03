#!/usr/bin/env python3
"""Spike: can an unprivileged process read Apple manufacturer data (company id 76)
from BLE advertisements through BlueZ, without root and without --experimental?

Usage: scan_spike.py [seconds]   (default 20)
Prints every Apple (id 76) payload seen; marks ProximityPairing (type 0x07, len 0x19).
"""
import sys
import dbus
import dbus.mainloop.glib
from gi.repository import GLib

APPLE = 76
BLUEZ = "org.bluez"
seen = {}


def report(path, props):
    md = props.get("ManufacturerData")
    if not md or APPLE not in md:
        return
    data = bytes(md[APPLE])
    key = (str(path), data)
    if key in seen:
        return
    seen[key] = True
    kind = "ProximityPairing" if len(data) >= 2 and data[0] == 0x07 and data[1] == 0x19 else "other-apple"
    print(f"{kind} dev={path} rssi={props.get('RSSI')} len={len(data)} data={data.hex()}", flush=True)


def main():
    secs = int(sys.argv[1]) if len(sys.argv) > 1 else 20
    dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
    bus = dbus.SystemBus()
    om = dbus.Interface(bus.get_object(BLUEZ, "/"), "org.freedesktop.DBus.ObjectManager")
    adapter_path = next(p for p, i in om.GetManagedObjects().items() if "org.bluez.Adapter1" in i)
    adapter = dbus.Interface(bus.get_object(BLUEZ, adapter_path), "org.bluez.Adapter1")

    def added(path, ifaces):
        if "org.bluez.Device1" in ifaces:
            report(path, ifaces["org.bluez.Device1"])

    def changed(iface, props, inval, path=None):
        if iface == "org.bluez.Device1":
            report(path, props)

    bus.add_signal_receiver(added, "InterfacesAdded", "org.freedesktop.DBus.ObjectManager", BLUEZ)
    bus.add_signal_receiver(changed, "PropertiesChanged", "org.freedesktop.DBus.Properties", BLUEZ, path_keyword="path")

    # Cached devices
    for path, ifaces in om.GetManagedObjects().items():
        if "org.bluez.Device1" in ifaces:
            report(path, ifaces["org.bluez.Device1"])

    adapter.SetDiscoveryFilter({"Transport": "le", "DuplicateData": True})
    adapter.StartDiscovery()
    print(f"scanning {secs}s on {adapter_path} (no root, no --experimental)", flush=True)
    loop = GLib.MainLoop()
    GLib.timeout_add_seconds(secs, loop.quit)
    try:
        loop.run()
    finally:
        try:
            adapter.StopDiscovery()
        except dbus.DBusException:
            pass
    pp = sum(1 for _, d in seen if len(d) >= 2 and d[0] == 0x07 and d[1] == 0x19)
    print(f"done: {len(seen)} distinct Apple payloads, {pp} ProximityPairing")


if __name__ == "__main__":
    main()
