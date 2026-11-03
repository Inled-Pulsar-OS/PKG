"""Seam 2 - BlueZ boundary: BluezBackend against the fake org.bluez on a private session bus."""

import json
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))
from harness import FakeBluez, pump, requires_private_bus, settle  # noqa: E402

import dbus  # noqa: E402
import dbus.mainloop.glib  # noqa: E402

from pulsaros_airpods.bluez import BluezBackend, apple_payload, plain  # noqa: E402
from pulsaros_airpods.protocol import Side  # noqa: E402
from test_protocol_state import make_packet  # noqa: E402

DEVICE_PATH = "/org/bluez/hci0/dev_AA_BB_CC_DD_EE_01"


class HelperTests(unittest.TestCase):
    """Pure helpers: no bus needed."""

    def test_apple_payload_picks_vendor_76_only(self):
        self.assertEqual(apple_payload({76: [7, 25]}), bytes([7, 25]))
        self.assertIsNone(apple_payload({6: [1, 2]}))
        self.assertIsNone(apple_payload({}))
        self.assertIsNone(apple_payload(None))

    def test_plain_converts_dbus_types(self):
        value = dbus.Dictionary(
            {dbus.UInt16(76): dbus.Array([dbus.Byte(7), dbus.Byte(25)], signature="y")},
            signature="qv",
        )
        converted = plain(value)
        self.assertEqual(converted, {76: [7, 25]})
        self.assertIs(type(converted[76][0]), int)
        self.assertIs(type(plain(dbus.Boolean(True))), bool)
        self.assertIs(type(plain(dbus.String("x"))), str)


@requires_private_bus
class BluezBoundaryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)

    def setUp(self):
        self.bluez = FakeBluez()
        self.advs, self.devices = [], []
        self.backend = BluezBackend(
            lambda a, r, p: self.advs.append((a, r, p)),
            lambda path, props, battery: self.devices.append((path, props, battery)),
            dbus.SessionBus(),
        )
        self.backend.start()

    def tearDown(self):
        self.backend.stop()
        self.bluez.stop()

    def last_props(self):
        for path, props, _ in reversed(self.devices):
            if path == DEVICE_PATH and props is not None:
                return props
        return None

    def test_initial_device_and_battery_are_reported(self):
        self.assertTrue(pump(lambda: self.devices))
        path, props, battery = self.devices[0]
        self.assertEqual(path, DEVICE_PATH)
        self.assertEqual(props["Address"], "AA:BB:CC:DD:EE:01")
        self.assertTrue(props["Connected"])
        self.assertEqual(battery, 60)

    def test_discovery_is_started_with_le_duplicate_filter(self):
        self.assertTrue(pump(lambda: self.bluez.count("StartDiscovery") == 1))
        flt = next(l for l in self.bluez.lines if l.startswith("filter "))
        self.assertEqual(json.loads(flt[len("filter "):]), {"DuplicateData": True, "Transport": "le"})

    def test_apple_advertisement_reaches_the_callback_as_bytes(self):
        pump(lambda: self.devices)
        packet = make_packet(0x2014, Side.LEFT)
        self.bluez.send(op="device", props={"RSSI": -55, "ManufacturerData": {"76": packet.hex()}})
        self.assertTrue(pump(lambda: self.advs))
        address, rssi, payload = self.advs[-1]
        self.assertEqual((address, rssi), ("AA:BB:CC:DD:EE:01", -55))
        self.assertIs(type(payload), bytes)
        self.assertEqual(payload, packet)

    def test_other_vendor_advertisement_is_ignored(self):
        pump(lambda: self.devices)
        self.bluez.send(op="device", props={"RSSI": -55, "ManufacturerData": {"6": "0109"}})
        settle(0.7)
        self.assertEqual(self.advs, [])

    def test_stopped_backend_stays_stopped_when_bluetoothd_returns(self):
        self.assertTrue(pump(lambda: self.bluez.count("StartDiscovery") == 1))
        self.backend.stop()
        self.backend.stop()  # idempotent
        self.bluez.stop()
        self.bluez.start()
        settle(1.5)
        self.assertEqual(self.bluez.count("StartDiscovery"), 1)  # still just the first one

    def test_battery_change_is_reported(self):
        pump(lambda: self.devices)
        self.bluez.send(op="battery", percentage=30)
        self.assertTrue(pump(lambda: any(b == 30 for _, _, b in self.devices)))

    def test_disconnect_property_change_is_reported(self):
        pump(lambda: self.devices)
        self.bluez.send(op="device", props={"Connected": False})
        self.assertTrue(pump(lambda: self.last_props() and self.last_props()["Connected"] is False))

    def test_bluetoothd_restart_removes_then_restores_devices(self):
        pump(lambda: self.devices)
        n = len(self.devices)
        self.bluez.stop()
        self.assertTrue(pump(lambda: any(p is None for _, p, _ in self.devices[n:])), "removal")
        self.bluez.start()
        self.assertTrue(pump(lambda: self.last_props() is not None
                             and len(self.devices) > n + 1 and self.devices[-1][1] is not None,
                             timeout=15), "restored")
        self.assertTrue(pump(lambda: self.bluez.count("StartDiscovery") >= 1))

    def test_power_cycle_restarts_discovery(self):
        self.assertTrue(pump(lambda: self.bluez.count("StartDiscovery") == 1))
        self.bluez.send(op="adapter", props={"Powered": False})
        settle(0.4)
        self.assertEqual(self.bluez.count("StartDiscovery"), 1)  # nothing to scan while off
        self.bluez.send(op="adapter", props={"Powered": True})
        self.assertTrue(pump(lambda: self.bluez.count("StartDiscovery") == 2))

    def test_discovery_stopped_by_someone_else_is_restarted(self):
        self.assertTrue(pump(lambda: self.bluez.count("StartDiscovery") == 1))
        self.bluez.send(op="adapter", props={"Discovering": False})
        self.assertTrue(pump(lambda: self.bluez.count("StartDiscovery") == 2))


if __name__ == "__main__":
    unittest.main()
