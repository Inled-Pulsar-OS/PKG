"""Seam 1 - domain: several AirPods paired - rebind to the active one, forget on request."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))

from pulsaros_airpods.tracker import Tracker  # noqa: E402

A = {"Address": "AA:BB:CC:DD:EE:01", "Alias": "Victor's AirPods Pro", "Connected": True,
     "Modalias": "bluetooth:v004Cp2014d0A00", "Icon": "audio-headphones"}
B = {"Address": "AA:BB:CC:DD:EE:02", "Alias": "Work AirPods", "Connected": True,
     "Modalias": "bluetooth:v004Cp2013d0A00", "Icon": "audio-headphones"}


class RebindTests(unittest.TestCase):
    def setUp(self):
        self.bound = []
        self.t = Tracker(None, self.bound.append)

    def test_most_recently_connected_pair_is_active(self):
        self.t.update("/a", A)
        self.t.update("/b", B)
        self.assertEqual(self.t.active.address, B["Address"])

    def test_active_falls_back_to_the_other_pair_when_it_leaves(self):
        self.t.update("/a", A)
        self.t.update("/b", B)
        self.t.update("/b", {"Connected": False})
        self.assertEqual(self.t.active.address, A["Address"])

    def test_active_does_not_flip_while_both_stay_connected(self):
        self.t.update("/a", A)
        self.t.update("/b", B)
        self.t.update("/a", {"RSSI": -40})  # unrelated property change on the older pair
        self.assertEqual(self.t.active.address, B["Address"])

    def test_every_change_of_active_pair_is_persisted(self):
        self.t.update("/a", A)
        self.t.active
        self.t.update("/b", B)
        self.t.active
        self.t.active  # reading again must not persist again
        self.assertEqual(self.bound, [A["Address"], B["Address"]])


class ForgetTests(unittest.TestCase):
    def setUp(self):
        self.bound = []
        self.t = Tracker(None, self.bound.append)
        self.t.update("/a", A)

    def test_forget_hides_the_connected_pair_and_clears_the_binding(self):
        self.assertIsNotNone(self.t.active)
        self.t.forget()
        self.assertIsNone(self.t.active)
        self.assertEqual(self.bound[-1], None)

    def test_forgotten_pair_binds_again_after_it_reconnects(self):
        self.t.active
        self.t.forget()
        self.t.update("/a", {"Connected": False})
        self.assertIsNone(self.t.active)
        self.t.update("/a", {"Connected": True})
        self.assertEqual(self.t.active.address, A["Address"])
        self.assertEqual(self.bound[-1], A["Address"])

    def test_forgetting_one_pair_leaves_the_other_usable(self):
        self.t.update("/b", B)
        self.t.forget()  # B is active
        self.assertEqual(self.t.active.address, A["Address"])

    def test_forget_with_nothing_connected_is_harmless(self):
        t = Tracker(None, self.bound.append)
        t.forget()
        self.assertIsNone(t.active)
        self.assertEqual(self.bound, [])


if __name__ == "__main__":
    unittest.main()
