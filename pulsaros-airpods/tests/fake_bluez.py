#!/usr/bin/env python3
"""Minimal fake org.bluez on the SESSION bus for integration tests and UI development.

Reads one JSON command per line from stdin:
  {"op": "device", "props": {...}}          update Device1 properties (emits PropertiesChanged)
  {"op": "battery", "percentage": 60}       update Battery1.Percentage
  {"op": "adapter", "props": {"Powered": false}}
"""

import json
import sys

import dbus
import dbus.mainloop.glib
import dbus.service
from gi.repository import GLib

ADAPTER_PATH = "/org/bluez/hci0"
DEVICE_PATH = "/org/bluez/hci0/dev_AA_BB_CC_DD_EE_01"

OBJMGR = "org.freedesktop.DBus.ObjectManager"
PROPS = "org.freedesktop.DBus.Properties"


class Props(dbus.service.Object):
    """An object with Properties support for its interfaces."""

    def __init__(self, bus, path, ifaces):
        super().__init__(bus, path)
        self.ifaces = ifaces  # iface -> dict

    @dbus.service.method(PROPS, in_signature="s", out_signature="a{sv}")
    def GetAll(self, iface):
        return self.ifaces[iface]

    @dbus.service.method(PROPS, in_signature="ss", out_signature="v")
    def Get(self, iface, name):
        return self.ifaces[iface][name]

    @dbus.service.signal(PROPS, signature="sa{sv}as")
    def PropertiesChanged(self, iface, changed, invalidated):
        pass

    def change(self, iface, changes):
        self.ifaces[iface].update(changes)
        self.PropertiesChanged(iface, changes, [])


class Adapter(Props):
    def __init__(self, bus):
        super().__init__(bus, ADAPTER_PATH, {"org.bluez.Adapter1": {
            "Powered": dbus.Boolean(True), "Discovering": dbus.Boolean(False)}})

    @dbus.service.method("org.bluez.Adapter1", in_signature="a{sv}")
    def SetDiscoveryFilter(self, f):
        plain_filter = {str(k): (bool(v) if isinstance(v, dbus.Boolean) else str(v)) for k, v in f.items()}
        print("filter " + json.dumps(plain_filter, sort_keys=True), flush=True)

    @dbus.service.method("org.bluez.Adapter1")
    def StartDiscovery(self):
        print("StartDiscovery", flush=True)
        self.change("org.bluez.Adapter1", {"Discovering": dbus.Boolean(True)})

    @dbus.service.method("org.bluez.Adapter1")
    def StopDiscovery(self):
        self.change("org.bluez.Adapter1", {"Discovering": dbus.Boolean(False)})


class Manager(dbus.service.Object):
    def __init__(self, bus, adapter, device):
        super().__init__(bus, "/")
        self.adapter, self.device = adapter, device

    @dbus.service.method(OBJMGR, out_signature="a{oa{sa{sv}}}")
    def GetManagedObjects(self):
        return {
            ADAPTER_PATH: self.adapter.ifaces,
            DEVICE_PATH: self.device.ifaces,
        }


def main():
    dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
    bus = dbus.SessionBus()
    name = dbus.service.BusName("org.bluez", bus)  # noqa: F841 (keep reference)
    adapter = Adapter(bus)
    device = Props(bus, DEVICE_PATH, {
        "org.bluez.Device1": {
            "Address": "AA:BB:CC:DD:EE:01",
            "Alias": "Test AirPods Pro",
            "Connected": dbus.Boolean(True),
            "Modalias": "bluetooth:v004Cp2014d0A00",
            "Icon": "audio-headphones",
        },
        "org.bluez.Battery1": {"Percentage": dbus.Byte(60)},
    })
    Manager(bus, adapter, device)
    print("ready", flush=True)

    def on_stdin(fd, cond):
        line = fd.readline()
        if not line:
            loop.quit()
            return False
        cmd = json.loads(line)
        if cmd["op"] == "device":
            changes = {}
            for k, v in cmd["props"].items():
                if k == "ManufacturerData":
                    v = dbus.Dictionary(
                        {dbus.UInt16(int(a)): dbus.Array([dbus.Byte(b) for b in bytes.fromhex(h)], signature="y")
                         for a, h in v.items()},
                        signature="qv",
                    )
                elif k == "RSSI":
                    v = dbus.Int16(v)
                elif isinstance(v, bool):
                    v = dbus.Boolean(v)
                changes[k] = v
            device.change("org.bluez.Device1", changes)
        elif cmd["op"] == "battery":
            device.change("org.bluez.Battery1", {"Percentage": dbus.Byte(cmd["percentage"])})
        elif cmd["op"] == "adapter":
            adapter.change("org.bluez.Adapter1", {k: dbus.Boolean(v) for k, v in cmd["props"].items()})
        return True

    GLib.io_add_watch(sys.stdin, GLib.PRIORITY_DEFAULT, GLib.IO_IN | GLib.IO_HUP, on_stdin)
    loop = GLib.MainLoop()
    loop.run()


if __name__ == "__main__":
    main()
