"""Ports of the protocol/state cases in AirPodsDesktop Tests/AirPodsDomainTests.cpp."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "usr", "share", "pulsaros-airpods"))
sys.path.insert(0, os.path.dirname(__file__))

from pulsaros_airpods import protocol  # noqa: E402
from pulsaros_airpods.protocol import Model, Side  # noqa: E402
from pulsaros_airpods.state import StateManager  # noqa: E402


def make_packet(model_id, side, left=8, right=7, case=5, both_in_case=True, lid_opened=True):
    p = bytearray(27)
    p[0] = 0x07
    p[1] = 25
    p[3] = model_id & 0xFF
    p[4] = model_id >> 8
    p[5] = (1 << 1) | (1 << 3)  # curr and another in ear
    if both_in_case:
        p[5] |= 1 << 2
    if side is Side.LEFT:
        p[5] |= 1 << 5
        p[6] = (right << 4) | left
    else:
        p[6] = (left << 4) | right
    p[7] = case
    p[8] = 0 if lid_opened else 1 << 3
    return bytes(p)


def adv(side, *args, **kw):
    state = protocol.parse(make_packet(kw.pop("model_id", 0x2014), side, *args, **kw))
    assert state is not None
    return state


class Clock:
    def __init__(self):
        self.now = 1000.0

    def __call__(self):
        return self.now


class ProtocolTests(unittest.TestCase):
    def test_rejects_malformed_packets(self):
        self.assertIsNone(protocol.parse(b""))
        packet = bytearray(make_packet(0x2014, Side.LEFT))
        self.assertIsNotNone(protocol.parse(bytes(packet)))
        packet[0] = 0x05  # AirDrop
        self.assertIsNone(protocol.parse(bytes(packet)))
        packet = bytearray(make_packet(0x2014, Side.LEFT))
        packet[1] = 24
        self.assertIsNone(protocol.parse(bytes(packet)))

    def test_parses_advertisement_state(self):
        s = adv(Side.LEFT)
        self.assertEqual(s.model, Model.AIRPODS_PRO_2)
        self.assertEqual(s.side, Side.LEFT)
        self.assertEqual((s.left.battery, s.right.battery, s.case.battery), (80, 70, 50))
        self.assertTrue(s.left.in_ear and s.right.in_ear)
        self.assertTrue(s.case.both_pods_in_case)
        self.assertTrue(s.case.lid_opened)

    def test_right_broadcast_is_flipped(self):
        s = adv(Side.RIGHT)
        self.assertEqual(s.side, Side.RIGHT)
        self.assertEqual((s.left.battery, s.right.battery), (80, 70))

    def test_out_of_range_battery_is_unavailable(self):
        s = adv(Side.LEFT, 15, 7, 5)
        self.assertIsNone(s.left.battery)
        self.assertEqual(s.right.battery, 70)

    def test_recognizes_airpods_5(self):
        self.assertEqual(protocol.model_from_id(0x2030), Model.AIRPODS_5)
        self.assertEqual(protocol.model_from_id(0x2036), Model.AIRPODS_5)
        self.assertEqual(protocol.MODEL_NAMES[Model.AIRPODS_5], "AirPods 5")
        self.assertEqual(protocol.model_from_id(0x201F), Model.AIRPODS_MAX_USB_C)
        self.assertEqual(protocol.model_from_id(0xFFFF), Model.UNKNOWN)

    def test_sanitized_airpods_5_captures(self):
        left = bytes.fromhex("0719013020" "35aaba3200" "05" + "00" * 16)
        right = bytes.fromhex("0719013020" "55aaba3200" "05" + "00" * 16)
        for packet, side in ((left, Side.LEFT), (right, Side.RIGHT)):
            s = protocol.parse(packet)
            self.assertEqual(s.model, Model.AIRPODS_5)
            self.assertEqual(s.side, side)
            self.assertEqual((s.left.battery, s.right.battery, s.case.battery), (100, 100, 100))
            self.assertTrue(s.left.charging and s.right.charging)
            self.assertFalse(s.case.charging)
            self.assertFalse(s.left.in_ear or s.right.in_ear)
            self.assertTrue(s.case.both_pods_in_case and s.case.lid_opened)


class StateManagerTests(unittest.TestCase):
    def setUp(self):
        self.clock = Clock()
        self.mgr = StateManager(rssi_min=-80, clock=self.clock)

    def test_filters_duplicate_and_weak(self):
        first = self.mgr.on_adv("A", -45, adv(Side.LEFT))
        self.assertIsNotNone(first)
        self.assertIsNone(first.old_state)

        self.assertIsNone(self.mgr.on_adv("A", -45, adv(Side.LEFT)))
        self.assertIsNone(self.mgr.on_adv("A", -90, adv(Side.LEFT, 7, 7, 5)))
        self.assertEqual(self.mgr.state.left.battery, 80)

    def test_merges_both_sides(self):
        self.assertIsNotNone(self.mgr.on_adv("A", -45, adv(Side.LEFT, 8, 6, 4)))
        self.clock.now += 0.1
        update = self.mgr.on_adv("B", -46, adv(Side.RIGHT, 9, 7, 5))
        self.assertIsNotNone(update)
        self.assertIsNotNone(update.old_state)
        self.assertEqual(update.new_state.left.battery, 90)
        self.assertEqual(update.new_state.right.battery, 70)
        self.assertEqual(update.new_state.case.battery, 50)

    def test_rejects_different_models(self):
        first = self.mgr.on_adv("A", -45, adv(Side.LEFT))
        self.assertEqual(first.new_state.model, Model.AIRPODS_PRO_2)
        other = self.mgr.on_adv("B", -46, adv(Side.RIGHT, 8, 7, 5, model_id=0x2013))
        self.assertIsNone(other)
        self.assertEqual(self.mgr.state.model, Model.AIRPODS_PRO_2)

    def test_accepts_known_model_after_unknown(self):
        unknown = self.mgr.on_adv("A", -45, adv(Side.LEFT, 8, 7, 5, model_id=0xFFFF))
        self.assertEqual(unknown.new_state.model, Model.UNKNOWN)
        known = self.mgr.on_adv("B", -46, adv(Side.RIGHT))
        self.assertEqual(known.new_state.model, Model.AIRPODS_PRO_2)

    def test_rejects_large_rssi_jump_from_another_side(self):
        self.mgr.on_adv("A", -45, adv(Side.LEFT))
        self.mgr.set_rssi_min(-200)
        self.assertIsNone(self.mgr.on_adv("B", -120, adv(Side.RIGHT)))

    def test_lost_after_timeout(self):
        self.mgr.on_adv("A", -45, adv(Side.LEFT))
        self.clock.now += 9
        self.assertIsNone(self.mgr.expire())
        self.assertIsNotNone(self.mgr.state)
        self.clock.now += 2
        lost = self.mgr.expire()
        self.assertIsNotNone(lost)
        self.assertIsNone(self.mgr.state)

    def test_disconnect_resets(self):
        self.mgr.on_adv("A", -45, adv(Side.LEFT))
        self.assertIsNotNone(self.mgr.disconnect())
        self.assertIsNone(self.mgr.state)
        self.assertIsNone(self.mgr.disconnect())


if __name__ == "__main__":
    unittest.main()
