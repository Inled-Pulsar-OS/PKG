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
XML_FILE = os.path.join(os.path.dirname(__file__), "..", "data", "org.pulsaros.AirPods.xml")


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

    def low_battery_signals(self):
        got = []
        self.bus.add_signal_receiver(lambda which, level: got.append((str(which), int(level))),
                                     "LowBattery", IFACE, NAME)
        return got

    def test_low_battery_signal_when_a_bud_crosses_20_percent(self):
        got = self.low_battery_signals()
        adv(self.bluez, 3)
        self.assertTrue(pump(lambda: self.props()["Left"] == 30))
        settle(0.5)
        self.assertEqual(got, [])
        adv(self.bluez, 2)
        self.assertTrue(pump(lambda: got == [("left", 20)]))
        adv(self.bluez, 2, 6)  # unrelated change, level still low: no repeat
        self.assertTrue(pump(lambda: self.props()["Right"] == 60))
        settle(0.5)
        self.assertEqual(got, [("left", 20)])

    def test_low_battery_alerts_setting_defaults_on_and_off_suppresses(self):
        self.assertTrue(self.props()["LowBatteryAlerts"])
        got = self.low_battery_signals()
        self.proxy().Set(IFACE, "LowBatteryAlerts", V(False))
        adv(self.bluez, 3)
        self.assertTrue(pump(lambda: self.props()["Left"] == 30))
        adv(self.bluez, 2)
        self.assertTrue(pump(lambda: self.props()["Left"] == 20))
        settle(0.7)
        self.assertEqual(got, [])

    def test_forget_hides_the_pair_until_it_reconnects(self):
        self.assertTrue(pump(lambda: os.path.exists(self.config)))
        dbus.Interface(self.bus.get_object(NAME, PATH, introspect=False), IFACE).Forget()
        self.assertTrue(pump(lambda: not self.props()["Connected"]))
        with open(self.config) as f:
            self.assertIsNone(json.load(f)["bound_address"])
        self.bluez.send(op="device", props={"Connected": False})
        settle(0.4)  # the fake reads one stdin line per wake-up: do not batch commands
        self.bluez.send(op="device", props={"Connected": True})
        self.assertTrue(pump(lambda: self.props()["Connected"]), self.daemon.log())
        with open(self.config) as f:
            self.assertEqual(json.load(f)["bound_address"], "AA:BB:CC:DD:EE:01")

    def iface(self):
        return dbus.Interface(self.bus.get_object(NAME, PATH, introspect=False), IFACE)

    def add_second_pair(self):
        self.bluez.send(op="add_device", props={
            "Address": "AA:BB:CC:DD:EE:02", "Alias": "Work AirPods", "Connected": False,
            "Paired": True, "Modalias": "bluetooth:v004Cp2013d0A00", "Icon": "audio-headphones"})
        self.assertTrue(pump(lambda: len(self.props()["PairedDevices"]) == 2), self.daemon.log())

    def test_paired_devices_lists_connected_and_idle_pairs(self):
        self.add_second_pair()
        listed = [(str(a), str(n), bool(c)) for a, n, c in self.props()["PairedDevices"]]
        self.assertEqual(listed, [("AA:BB:CC:DD:EE:01", "Test AirPods Pro", True),
                                  ("AA:BB:CC:DD:EE:02", "Work AirPods", False)])

    def test_connect_connects_an_idle_pair_and_it_becomes_active(self):
        self.add_second_pair()
        self.iface().Connect("AA:BB:CC:DD:EE:02", timeout=15)
        self.assertEqual(self.bluez.count("Connect AA:BB:CC:DD:EE:02"), 1)
        self.assertTrue(pump(lambda: self.props()["Address"] == "AA:BB:CC:DD:EE:02"), self.daemon.log())
        self.assertEqual(self.props()["Name"], "Work AirPods")

    def test_disconnect_disconnects_the_active_pair(self):
        self.iface().Disconnect("AA:BB:CC:DD:EE:01", timeout=15)
        self.assertTrue(pump(lambda: not self.props()["Connected"]))
        self.assertEqual(self.bluez.count("Disconnect AA:BB:CC:DD:EE:01"), 1)

    def test_connect_to_unknown_address_is_an_error_and_daemon_survives(self):
        with self.assertRaises(dbus.DBusException) as ctx:
            self.iface().Connect("11:22:33:44:55:66", timeout=15)
        self.assertEqual(ctx.exception.get_dbus_name(), "org.pulsaros.AirPods.Error.Failed")
        self.assertTrue(self.daemon.alive(), self.daemon.log())

    def test_failed_connect_reaches_the_caller_with_the_reason(self):
        self.add_second_pair()
        self.bluez.send(op="fail", connect=True)
        settle(0.3)
        with self.assertRaises(dbus.DBusException) as ctx:
            self.iface().Connect("AA:BB:CC:DD:EE:02", timeout=15)
        self.assertEqual(ctx.exception.get_dbus_name(), "org.pulsaros.AirPods.Error.Failed")
        self.assertIn("page timeout", ctx.exception.get_dbus_message())
        self.assertTrue(self.props()["Connected"])  # the earlier pair is untouched

    def test_show_when_disconnected_defaults_off_and_persists(self):
        self.assertFalse(self.props()["ShowWhenDisconnected"])
        self.proxy().Set(IFACE, "ShowWhenDisconnected", V(True))
        self.daemon.stop()
        self.assertTrue(pump(lambda: self.props() is None, 5))
        self.daemon.start()
        self.assertTrue(pump(lambda: self.props() is not None and self.props()["Connected"], 15),
                        self.daemon.log())
        self.assertTrue(self.props()["ShowWhenDisconnected"])

    def set_prop(self, name, value):
        self.proxy().Set(IFACE, name, value)

    def test_panel_battery_and_rssi_defaults(self):
        p = self.props()
        self.assertEqual((str(p["PanelBattery"]), int(p["RssiMin"])), ("off", -80))

    def test_panel_battery_accepts_known_values_only_and_persists(self):
        self.set_prop("PanelBattery", dbus.String("both", variant_level=1))
        with self.assertRaises(dbus.DBusException):
            self.set_prop("PanelBattery", dbus.String("loudest", variant_level=1))
        self.assertEqual(str(self.props()["PanelBattery"]), "both")
        self.daemon.stop()
        self.assertTrue(pump(lambda: self.props() is None, 5))
        self.daemon.start()
        self.assertTrue(pump(lambda: self.props() is not None and self.props()["Connected"], 15),
                        self.daemon.log())
        self.assertEqual(str(self.props()["PanelBattery"]), "both")

    def test_rssi_min_rejects_nonsense(self):
        for bad in (0, -20, -120):
            with self.subTest(bad=bad), self.assertRaises(dbus.DBusException):
                self.set_prop("RssiMin", dbus.Int32(bad, variant_level=1))
        self.assertEqual(int(self.props()["RssiMin"]), -80)

    def test_rssi_min_filters_weak_advertisements_immediately(self):
        self.set_prop("RssiMin", dbus.Int32(-60, variant_level=1))
        pkt = make_packet(0x2014, Side.LEFT)
        self.bluez.send(op="device", props={"RSSI": -70, "ManufacturerData": {"76": pkt.hex()}})
        settle(0.7)
        self.assertFalse(self.props()["HasAdvertisement"])
        self.bluez.send(op="device", props={"RSSI": -50, "ManufacturerData": {"76": pkt.hex()}})
        self.assertTrue(pump(lambda: self.props()["HasAdvertisement"]))

    def test_rssi_min_persists_across_restart(self):
        self.set_prop("RssiMin", dbus.Int32(-65, variant_level=1))
        self.daemon.stop()
        self.assertTrue(pump(lambda: self.props() is None, 5))
        self.daemon.start()
        self.assertTrue(pump(lambda: self.props() is not None and self.props()["Connected"], 15),
                        self.daemon.log())
        self.assertEqual(int(self.props()["RssiMin"]), -65)

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
