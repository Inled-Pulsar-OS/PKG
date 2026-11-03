"""Seam 1 - domain: protocol, state manager, tracker, controller, config.

Expected values are literals taken from the packet layout / protocol notes, never recomputed
the way the code does.
"""

import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))
sys.path.insert(0, os.path.dirname(__file__))

from pulsaros_airpods import protocol  # noqa: E402
from pulsaros_airpods.config import Config  # noqa: E402
from pulsaros_airpods.controller import Controller  # noqa: E402
from pulsaros_airpods.protocol import Model, Side  # noqa: E402
from pulsaros_airpods.state import StateManager  # noqa: E402
from pulsaros_airpods.tracker import DeviceInfo, Tracker  # noqa: E402
from test_protocol_state import Clock, adv, make_packet  # noqa: E402


class ProtocolEdges(unittest.TestCase):
    def test_every_known_model_id(self):
        # literal table: model id -> display name (from Apple model ids)
        expected = {
            0x2002: "AirPods 1", 0x200F: "AirPods 2", 0x2013: "AirPods 3",
            0x2019: "AirPods 4", 0x201B: "AirPods 4 (ANC)", 0x2030: "AirPods 5",
            0x2036: "AirPods 5", 0x200E: "AirPods Pro", 0x2014: "AirPods Pro 2",
            0x2024: "AirPods Pro 2 (USB-C)", 0x2027: "AirPods Pro 3", 0x200A: "AirPods Max",
            0x201F: "AirPods Max (USB-C)", 0x2012: "Beats Fit Pro",
        }
        for model_id, name in expected.items():
            with self.subTest(hex(model_id)):
                self.assertEqual(protocol.MODEL_NAMES[protocol.model_from_id(model_id)], name)

    def test_charging_bud_is_never_reported_in_ear(self):
        p = bytearray(make_packet(0x2014, Side.LEFT))  # both ear bits set
        p[7] |= 1 << 4  # current (left) charging
        s = protocol.parse(bytes(p))
        self.assertTrue(s.left.charging)
        self.assertFalse(s.left.in_ear)
        self.assertTrue(s.right.in_ear)

    def test_case_battery_unavailable_nibble(self):
        p = bytearray(make_packet(0x2014, Side.LEFT))
        p[7] = (p[7] & 0xF0) | 0x0F
        self.assertIsNone(protocol.parse(bytes(p)).case.battery)

    def test_lid_closed_bit(self):
        self.assertFalse(adv(Side.LEFT, lid_opened=False).case.lid_opened)
        self.assertTrue(adv(Side.LEFT, lid_opened=True).case.lid_opened)

    def test_wrong_length_rejected(self):
        good = make_packet(0x2014, Side.LEFT)
        self.assertIsNone(protocol.parse(good + b"\x00"))
        self.assertIsNone(protocol.parse(good[:-1]))

    def test_accepts_list_of_ints(self):
        # python-dbus hands arrays over; bluez.py converts to bytes, parse() must also cope
        self.assertIsNotNone(protocol.parse(list(make_packet(0x2014, Side.LEFT))))


class StateEdges(unittest.TestCase):
    def setUp(self):
        self.clock = Clock()
        self.mgr = StateManager(rssi_min=-200, clock=self.clock)

    def test_address_change_accepts_one_battery_step_rejects_two(self):
        self.mgr.on_adv("A", -50, adv(Side.LEFT, 8, 7, 5))
        ok = self.mgr.on_adv("B", -50, adv(Side.LEFT, 7, 7, 5))  # 80 -> 70: one step
        self.assertIsNotNone(ok)
        self.assertIsNone(self.mgr.on_adv("C", -50, adv(Side.LEFT, 5, 7, 5)))  # 70 -> 50

    def test_address_change_with_big_rssi_jump_rejected(self):
        self.mgr.on_adv("A", -40, adv(Side.LEFT))
        self.assertIsNone(self.mgr.on_adv("B", -95, adv(Side.LEFT, 8, 7, 5)))

    def test_stale_side_is_forgotten_after_ten_seconds(self):
        self.mgr.on_adv("A", -40, adv(Side.LEFT))
        self.mgr.on_adv("B", -85, adv(Side.RIGHT))
        # a different left address 51 dB away from A is rejected while A is remembered
        self.assertIsNone(self.mgr.on_adv("C", -91, adv(Side.LEFT, 8, 7, 5)))
        for _ in range(3):  # right keeps the device alive; left goes silent
            self.clock.now += 5
            self.mgr.on_adv("B", -85, adv(Side.RIGHT))
        self.mgr.expire()  # left entry is 15 s old: forgotten
        self.clock.now += 0.1
        update = self.mgr.on_adv("C", -91, adv(Side.LEFT, 9, 7, 5))
        self.assertEqual(update.new_state.left.battery, 90)

    def test_rssi_floor_can_change_at_runtime(self):
        self.mgr.set_rssi_min(-60)
        self.assertIsNone(self.mgr.on_adv("A", -70, adv(Side.LEFT)))
        self.mgr.set_rssi_min(-80)
        self.assertIsNotNone(self.mgr.on_adv("A", -70, adv(Side.LEFT)))

    def test_lost_timer_is_extended_by_each_advertisement(self):
        self.mgr.on_adv("A", -40, adv(Side.LEFT))
        for _ in range(4):
            self.clock.now += 8
            self.mgr.on_adv("A", -40, adv(Side.LEFT, 8, 7, 5))
            self.assertIsNone(self.mgr.expire())
        self.assertIsNotNone(self.mgr.state)

    def test_expire_without_state_is_noop(self):
        self.clock.now += 100
        self.assertIsNone(self.mgr.expire())


PRO2 = {
    "Address": "AA:BB:CC:DD:EE:01", "Alias": "Pods", "Connected": True,
    "Modalias": "bluetooth:v004Cp2014d0A00", "Icon": "audio-headphones",
}


class TrackerEdges(unittest.TestCase):
    def test_bind_callback_fires_once(self):
        bound = []
        t = Tracker(None, bound.append)
        t.update("/p/1", PRO2)
        t.active
        t.active
        t.update("/p/1", {"Connected": False})
        t.update("/p/1", {"Connected": True})
        t.active
        self.assertEqual(bound, [PRO2["Address"]])

    def test_removed_device_is_forgotten_and_can_return(self):
        t = Tracker(PRO2["Address"])
        t.update("/p/1", PRO2)
        t.remove("/p/1")
        self.assertIsNone(t.active)
        t.update("/p/1", PRO2)
        self.assertEqual(t.active.address, PRO2["Address"])

    def test_usb_modalias_apple_vendor(self):
        usb = dict(PRO2, Modalias="usb:v05ACp200Ed0A00")
        t = Tracker(None)
        t.update("/p/1", usb)
        self.assertEqual(t.active.model, Model.AIRPODS_PRO)

    def test_non_apple_vendor_with_audio_icon_is_not_airpods(self):
        sony = dict(PRO2, Alias="WH-1000XM5", Modalias="bluetooth:v054Cp0DF0d0A00")
        t = Tracker(None)
        t.update("/p/1", sony)
        self.assertIsNone(t.active)

    def test_battery_cleared(self):
        t = Tracker(PRO2["Address"])
        t.update("/p/1", PRO2)
        t.update_battery("/p/1", 50)
        t.update_battery("/p/1", None)
        self.assertIsNone(t.active.battery)

    def test_device_without_address_ignored(self):
        t = Tracker(None)
        t.update("/p/1", {k: v for k, v in PRO2.items() if k != "Address"})
        self.assertIsNone(t.active)


class ControllerEdges(unittest.TestCase):
    def setUp(self):
        self.clock = Clock()
        self.snaps = []
        self.c = Controller(StateManager(clock=self.clock), self.snaps.append)

    def dev(self, address="AA:BB:CC:DD:EE:01", battery=None, model=Model.AIRPODS_PRO_2):
        return DeviceInfo(address, "Pods", model, battery)

    def test_same_device_new_battery_keeps_advertisement_state(self):
        self.c.set_device(self.dev(battery=50))
        self.c.on_adv("L", -50, make_packet(0x2014, Side.LEFT))
        self.c.set_device(self.dev(battery=40))
        self.assertTrue(self.c.snapshot.has_advertisement)
        self.assertEqual(self.c.snapshot.fallback_battery, 40)

    def test_different_device_resets_state(self):
        self.c.set_device(self.dev())
        self.c.on_adv("L", -50, make_packet(0x2014, Side.LEFT))
        self.c.set_device(self.dev(address="AA:BB:CC:DD:EE:99"))
        self.assertFalse(self.c.snapshot.has_advertisement)

    def test_unknown_expected_model_accepts_any_model(self):
        self.c.set_device(self.dev(model=Model.UNKNOWN))
        self.c.on_adv("L", -50, make_packet(0x2013, Side.LEFT))
        self.assertEqual(self.c.snapshot.model_id, "airpods_3")

    def test_garbage_payload_is_ignored(self):
        self.c.set_device(self.dev())
        before = self.c.snapshot
        self.c.on_adv("L", -50, b"\x01\x02\x03")
        self.assertEqual(self.c.snapshot, before)

    def test_headphones_never_show_case(self):
        self.c.set_device(self.dev(model=Model.AIRPODS_MAX))
        self.c.on_adv("L", -50, make_packet(0x200A, Side.LEFT))
        self.assertTrue(self.c.snapshot.is_headphones)


class ConfigEdges(unittest.TestCase):
    def write(self, data):
        d = tempfile.mkdtemp()
        path = os.path.join(d, "config.json")
        with open(path, "w") as f:
            json.dump(data, f)
        return path

    def test_bool_is_not_accepted_for_an_int_setting(self):
        self.assertEqual(Config(self.write({"rssi_min": True})).get("rssi_min"), -80)

    def test_int_is_not_accepted_for_a_bool_setting(self):
        self.assertTrue(Config(self.write({"ear_detection": 0})).get("ear_detection"))

    def test_unknown_keys_are_dropped_on_save(self):
        path = self.write({"future": 1})
        c = Config(path)
        c.set("lid_popup", False)
        with open(path) as f:
            self.assertNotIn("future", json.load(f))

    def test_bound_address_must_be_a_string_or_null(self):
        self.assertIsNone(Config(self.write({"bound_address": 5})).get("bound_address"))
        self.assertEqual(Config(self.write({"bound_address": "AA:BB"})).get("bound_address"), "AA:BB")

    def test_non_dict_json_ignored(self):
        self.assertEqual(Config(self.write([1, 2])).get("rssi_min"), -80)


if __name__ == "__main__":
    unittest.main()
