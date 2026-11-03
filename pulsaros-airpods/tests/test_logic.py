"""Tracker, controller, ear detection and config tests (no D-Bus)."""

import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "usr", "share", "pulsaros-airpods"))
sys.path.insert(0, os.path.dirname(__file__))

from pulsaros_airpods.config import Config  # noqa: E402
from pulsaros_airpods.controller import Controller, Snapshot, build_snapshot  # noqa: E402
from pulsaros_airpods.mpris import EarDetection  # noqa: E402
from pulsaros_airpods.protocol import Model, Side  # noqa: E402
from pulsaros_airpods.state import StateManager  # noqa: E402
from pulsaros_airpods.tracker import DeviceInfo, Tracker, is_airpods, parse_modalias  # noqa: E402
from test_protocol_state import Clock, make_packet  # noqa: E402

PRO2 = {
    "Address": "AA:BB:CC:DD:EE:01",
    "Alias": "Victor's AirPods Pro",
    "Connected": True,
    "Modalias": "bluetooth:v004Cp2014d0A00",
    "Icon": "audio-headphones",
}
IPHONE = {
    "Address": "AA:BB:CC:DD:EE:02",
    "Alias": "iPhone",
    "Connected": True,
    "Modalias": "bluetooth:v004Cp1234d0A00",
    "Icon": "phone",
}


class TrackerTests(unittest.TestCase):
    def test_modalias(self):
        self.assertEqual(parse_modalias("bluetooth:v004Cp200Ed0A00"), (0x4C, 0x200E))
        self.assertIsNone(parse_modalias("garbage"))

    def test_detection(self):
        self.assertTrue(is_airpods(PRO2))
        self.assertFalse(is_airpods(IPHONE))
        self.assertTrue(is_airpods({"Alias": "My AirPods", "Icon": "audio-headset"}))
        self.assertFalse(is_airpods({"Alias": "Keychron K10", "Icon": "input-keyboard"}))

    def test_active_binds_first_connected(self):
        bound = []
        t = Tracker(None, bound.append)
        t.update("/p/1", PRO2)
        t.update("/p/2", IPHONE)
        dev = t.active
        self.assertEqual((dev.address, dev.model), (PRO2["Address"], Model.AIRPODS_PRO_2))
        self.assertEqual(bound, [PRO2["Address"]])

    def test_disconnect_and_battery(self):
        t = Tracker(PRO2["Address"])
        t.update("/p/1", PRO2)
        t.update_battery("/p/1", 70)
        self.assertEqual(t.active.battery, 70)
        t.update("/p/1", {"Connected": False})
        self.assertIsNone(t.active)

    def test_bound_device_wins_over_other_pair(self):
        other = dict(PRO2, Address="AA:BB:CC:DD:EE:99")
        t = Tracker(PRO2["Address"])
        t.update("/p/2", other)
        self.assertIsNone(t.active)  # bound pair not connected: ignore the other pair
        t.update("/p/1", PRO2)
        self.assertEqual(t.active.address, PRO2["Address"])


def device(model=Model.AIRPODS_PRO_2, battery=None):
    return DeviceInfo("AA:BB:CC:DD:EE:01", "AirPods", model, battery)


class ControllerTests(unittest.TestCase):
    def setUp(self):
        self.clock = Clock()
        self.snaps, self.lids, self.ears = [], [], []
        self.c = Controller(
            StateManager(clock=self.clock),
            self.snaps.append,
            lambda: self.lids.append(1),
            self.ears.append,
        )

    def adv(self, side=Side.LEFT, addr="L", rssi=-50, **kw):
        self.c.on_adv(addr, rssi, make_packet(kw.pop("model_id", 0x2014), side, **kw))

    def test_ignores_adv_when_not_connected(self):
        self.adv()
        self.assertEqual(self.snaps, [])
        self.assertFalse(self.c.snapshot.connected)

    def test_connected_without_adv_uses_fallback_battery(self):
        self.c.set_device(device(battery=60))
        s = self.c.snapshot
        self.assertTrue(s.connected)
        self.assertFalse(s.has_advertisement)
        self.assertEqual((s.left, s.right, s.case, s.fallback_battery), (-1, -1, -1, 60))
        self.assertEqual(s.model, "AirPods Pro 2")

    def test_adv_fills_snapshot(self):
        self.c.set_device(device())
        self.adv(left=8, right=7, case=5)
        s = self.c.snapshot
        self.assertTrue(s.has_advertisement)
        self.assertEqual((s.left, s.right, s.case), (80, 70, 50))
        self.assertTrue(s.left_in_ear and s.right_in_ear)

    def test_other_model_ignored(self):
        self.c.set_device(device(Model.AIRPODS_PRO_2))
        self.adv(model_id=0x2013)
        self.assertFalse(self.c.snapshot.has_advertisement)

    def test_disconnect_clears(self):
        self.c.set_device(device())
        self.adv()
        self.c.set_device(None)
        self.assertEqual(self.c.snapshot, Snapshot())

    def test_lid_opened_signal_only_on_transition(self):
        self.c.set_device(device())
        self.adv(lid_opened=False)
        self.assertEqual(self.lids, [])
        self.adv(lid_opened=True)
        self.assertEqual(self.lids, [1])
        self.adv(lid_opened=True, left=9)
        self.assertEqual(self.lids, [1])

    def test_both_in_ear_transitions(self):
        self.c.set_device(device())
        self.adv()  # both in ear (make_packet sets both ear bits)
        self.assertEqual(self.ears, [True])
        p = bytearray(make_packet(0x2014, Side.LEFT))
        p[5] &= ~(1 << 1)  # left bud out
        self.c.on_adv("L", -50, bytes(p))
        self.assertEqual(self.ears, [True, False])

    def test_lost_after_timeout_pauses_and_clears_state(self):
        self.c.set_device(device(battery=40))
        self.adv()
        self.clock.now += 11
        self.c.tick()
        s = self.c.snapshot
        self.assertTrue(s.connected and not s.has_advertisement)
        self.assertEqual(s.fallback_battery, 40)
        self.assertEqual(self.ears[-1], False)

    def test_headphones_flag(self):
        s = build_snapshot(device(Model.AIRPODS_MAX), None)
        self.assertTrue(s.is_headphones)


class FakePlayers:
    def __init__(self, playing):
        self._playing = list(playing)
        self.calls = []

    def playing(self):
        return list(self._playing)

    def pause(self, n):
        self.calls.append(("pause", n))
        self._playing.remove(n)

    def play(self, n):
        self.calls.append(("play", n))
        self._playing.append(n)


class EarDetectionTests(unittest.TestCase):
    def test_pause_then_resume_only_ours(self):
        players = FakePlayers(["spotify"])
        e = EarDetection(players, lambda: True)
        e.on_both_in_ear(False)
        e.on_both_in_ear(True)
        self.assertEqual(players.calls, [("pause", "spotify"), ("play", "spotify")])

    def test_does_not_resume_what_it_did_not_pause(self):
        players = FakePlayers([])
        e = EarDetection(players, lambda: True)
        e.on_both_in_ear(False)
        e.on_both_in_ear(True)
        self.assertEqual(players.calls, [])

    def test_second_removal_keeps_paused_set(self):
        players = FakePlayers(["a"])
        e = EarDetection(players, lambda: True)
        e.on_both_in_ear(False)
        e.on_both_in_ear(False)
        e.on_both_in_ear(True)
        self.assertEqual(players.calls, [("pause", "a"), ("play", "a")])

    def test_disabled_does_nothing(self):
        players = FakePlayers(["a"])
        e = EarDetection(players, lambda: False)
        e.on_both_in_ear(False)
        self.assertEqual(players.calls, [])


class ConfigTests(unittest.TestCase):
    def test_defaults_roundtrip_and_bad_file(self):
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, "sub", "config.json")
            c = Config(path)
            self.assertTrue(c.get("ear_detection"))
            c.set("ear_detection", False)
            c.set("bound_address", "AA:BB")
            c2 = Config(path)
            self.assertFalse(c2.get("ear_detection"))
            self.assertEqual(c2.get("bound_address"), "AA:BB")
            with open(path, "w") as f:
                json.dump({"rssi_min": "nope", "ear_detection": 1}, f)
            c3 = Config(path)
            self.assertEqual(c3.get("rssi_min"), -80)  # wrong type: default
            with open(path, "w") as f:
                f.write("{not json")
            self.assertEqual(Config(path).get("rssi_min"), -80)
            with self.assertRaises(KeyError):
                c.set("nope", 1)


if __name__ == "__main__":
    unittest.main()
