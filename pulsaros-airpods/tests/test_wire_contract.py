"""Seam 3 - wire contract: the real daemon process, observed only through D-Bus."""

import json
import os
import sys
import unittest
import xml.etree.ElementTree as ET

sys.path.insert(0, os.path.dirname(__file__))
from harness import Daemon, FakeBluez, pump, requires_private_bus, settle, tempdir  # noqa: E402

import dbus  # noqa: E402
import dbus.mainloop.glib  # noqa: E402

from pulsaros_airpods.protocol import Side  # noqa: E402
from test_protocol_state import make_packet  # noqa: E402

NAME, PATH, IFACE = "org.pulsaros.AirPods", "/org/pulsaros/AirPods", "org.pulsaros.AirPods"
PROPS = "org.freedesktop.DBus.Properties"
XML_FILE = os.path.join(os.path.dirname(__file__), "..", "usr", "share", "pulsaros-airpods",
                        "dbus", "org.pulsaros.AirPods.xml")


def V(value):
    return dbus.Boolean(value, variant_level=1)


def adv(bluez, *args, **kw):
    pkt = make_packet(kw.pop("model_id", 0x2014), kw.pop("side", Side.LEFT), *args, **kw)
    bluez.send(op="device", props={"RSSI": -50, "ManufacturerData": {"76": pkt.hex()}})


@requires_private_bus
class WireContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)

    def setUp(self):
        self.tmp = tempdir()
        self.config = os.path.join(self.tmp.name, "config.json")
        self.bluez = FakeBluez()
        self.daemon = Daemon(self.config)
        self.bus = dbus.SessionBus()
        self.signals, self.changes = [], []
        self.bus.add_signal_receiver(lambda: self.signals.append("LidOpened"), "LidOpened", IFACE, NAME)
        self.bus.add_signal_receiver(
            lambda iface, changed, inval: self.changes.append(dict(changed)) if iface == IFACE else None,
            "PropertiesChanged", PROPS, NAME)
        self.assertTrue(pump(lambda: self.props() is not None and self.props()["Connected"]),
                        self.daemon.log())

    def tearDown(self):
        self.daemon.stop()
        self.bluez.stop()
        self.tmp.cleanup()

    def proxy(self):
        return dbus.Interface(self.bus.get_object(NAME, PATH, introspect=False), PROPS)

    def props(self):
        try:
            return dict(self.proxy().GetAll(IFACE))
        except dbus.DBusException:
            return None

    def test_introspection_matches_the_contract_file(self):
        xml = dbus.Interface(self.bus.get_object(NAME, PATH, introspect=False),
                             "org.freedesktop.DBus.Introspectable").Introspect()

        def members(text):
            root = ET.fromstring(text)
            iface = next(i for i in root.iter("interface") if i.get("name") == IFACE)
            return (sorted((p.get("name"), p.get("type"), p.get("access")) for p in iface.iter("property")),
                    sorted(s.get("name") for s in iface.iter("signal")))

        with open(XML_FILE, encoding="utf-8") as f:
            self.assertEqual(members(xml), members(f.read()))

    def test_unavailable_values_are_minus_one_never_zero(self):
        p = self.props()
        self.assertEqual((p["Left"], p["Right"], p["Case"]), (-1, -1, -1))
        self.assertEqual(p["FallbackBattery"], 60)

    def test_value_types_on_the_wire(self):
        adv(self.bluez)
        self.assertTrue(pump(lambda: self.props()["HasAdvertisement"]))
        p = self.props()
        self.assertIsInstance(p["Left"], dbus.Int32)
        self.assertIsInstance(p["Model"], dbus.String)
        self.assertIsInstance(p["LeftInEar"], dbus.Boolean)

    def test_properties_changed_carries_only_what_changed(self):
        adv(self.bluez, 8, 7, 5)
        self.assertTrue(pump(lambda: any("Left" in c for c in self.changes)))
        change = next(c for c in self.changes if "Left" in c)
        self.assertEqual((change["Left"], change["Right"], change["Case"]), (80, 70, 50))
        self.assertNotIn("Name", change)

    def test_lid_opened_signal_on_closed_to_open_only(self):
        adv(self.bluez, lid_opened=False)
        self.assertTrue(pump(lambda: self.props()["HasAdvertisement"]))
        settle(0.5)
        self.assertEqual(self.signals, [])
        adv(self.bluez, lid_opened=True)
        self.assertTrue(pump(lambda: self.signals == ["LidOpened"]))
        adv(self.bluez, 9, lid_opened=True)  # still open: no second signal
        settle(0.7)
        self.assertEqual(self.signals, ["LidOpened"])

    def test_lid_popup_setting_off_suppresses_the_signal(self):
        self.proxy().Set(IFACE, "LidPopup", V(False))
        adv(self.bluez, lid_opened=False)
        self.assertTrue(pump(lambda: self.props()["HasAdvertisement"]))
        adv(self.bluez, lid_opened=True)
        self.assertTrue(pump(lambda: self.props()["CaseLidOpened"]))
        settle(0.7)
        self.assertEqual(self.signals, [])

    def test_readonly_property_cannot_be_set(self):
        with self.assertRaises(dbus.DBusException):
            self.proxy().Set(IFACE, "Left", dbus.Int32(5, variant_level=1))

    def test_settings_survive_a_daemon_restart(self):
        self.proxy().Set(IFACE, "EarDetection", V(False))
        self.proxy().Set(IFACE, "LidPopup", V(False))
        self.daemon.stop()
        self.assertTrue(pump(lambda: self.props() is None, 5))
        self.daemon.start()
        self.assertTrue(pump(lambda: self.props() is not None and self.props()["Connected"], 15),
                        self.daemon.log())
        p = self.props()
        self.assertFalse(p["EarDetection"])
        self.assertFalse(p["LidPopup"])

    def test_first_connected_airpods_get_bound_and_persisted(self):
        self.assertTrue(pump(lambda: os.path.exists(self.config)))
        with open(self.config) as f:
            self.assertEqual(json.load(f)["bound_address"], "AA:BB:CC:DD:EE:01")

    def test_removing_buds_with_no_media_player_does_not_crash(self):
        adv(self.bluez)
        self.assertTrue(pump(lambda: self.props()["LeftInEar"]))
        pkt = bytearray(make_packet(0x2014, Side.LEFT))
        pkt[5] &= ~(1 << 1)  # left bud out of the ear
        self.bluez.send(op="device", props={"RSSI": -50, "ManufacturerData": {"76": bytes(pkt).hex()}})
        self.assertTrue(pump(lambda: not self.props()["LeftInEar"]))
        self.assertTrue(self.daemon.alive(), self.daemon.log())

    def test_disconnect_clears_everything(self):
        adv(self.bluez)
        self.assertTrue(pump(lambda: self.props()["HasAdvertisement"]))
        self.bluez.send(op="device", props={"Connected": False})
        self.assertTrue(pump(lambda: not self.props()["Connected"]))
        p = self.props()
        self.assertEqual((p["Left"], p["Name"], p["ModelId"], p["FallbackBattery"]), (-1, "", "unknown", -1))


if __name__ == "__main__":
    unittest.main()
