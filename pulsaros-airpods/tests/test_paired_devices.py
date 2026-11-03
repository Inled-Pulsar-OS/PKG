"""Seam 1 - domain: the list of paired AirPods (connected or not) shown for quick connect."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))

from pulsaros_airpods.controller import Controller, Snapshot, build_snapshot  # noqa: E402
from pulsaros_airpods.state import StateManager  # noqa: E402
from pulsaros_airpods.tracker import Tracker  # noqa: E402

A = {"Address": "AA:BB:CC:DD:EE:01", "Alias": "Victor's AirPods Pro", "Connected": True, "Paired": True,
     "Modalias": "bluetooth:v004Cp2014d0A00", "Icon": "audio-headphones"}
B = {"Address": "AA:BB:CC:DD:EE:02", "Alias": "Work AirPods", "Connected": False, "Paired": True,
     "Modalias": "bluetooth:v004Cp2013d0A00", "Icon": "audio-headphones"}
PHONE = {"Address": "AA:BB:CC:DD:EE:03", "Alias": "iPhone", "Connected": False, "Paired": True,
         "Modalias": "bluetooth:v004Cp1234d0A00", "Icon": "phone"}
SCANNED = dict(B, Address="AA:BB:CC:DD:EE:04", Alias="Stranger AirPods", Paired=False)


class PairedListTests(unittest.TestCase):
    def test_lists_paired_airpods_connected_or_not_sorted_by_name(self):
        t = Tracker()
        for path, props in (("/a", A), ("/b", B), ("/p", PHONE)):
            t.update(path, props)
        self.assertEqual(t.paired(), [
            ("AA:BB:CC:DD:EE:01", "Victor's AirPods Pro", True),
            ("AA:BB:CC:DD:EE:02", "Work AirPods", False),
        ])

    def test_unpaired_airpods_seen_by_a_scan_are_not_listed(self):
        t = Tracker()
        t.update("/s", SCANNED)
        self.assertEqual(t.paired(), [])

    def test_removed_device_leaves_the_list(self):
        t = Tracker()
        t.update("/b", B)
        t.remove("/b")
        self.assertEqual(t.paired(), [])

    def test_listing_does_not_bind_a_pair(self):
        bound = []
        t = Tracker(None, bound.append)
        t.update("/a", A)
        t.paired()
        self.assertEqual(bound, [])


class SnapshotPairedTests(unittest.TestCase):
    PAIRED = (("AA:BB:CC:DD:EE:02", "Work AirPods", False),)

    def test_paired_devices_are_published_even_when_nothing_is_connected(self):
        self.assertEqual(build_snapshot(None, None, self.PAIRED).paired_devices, self.PAIRED)
        self.assertFalse(build_snapshot(None, None, self.PAIRED).connected)

    def test_default_is_empty(self):
        self.assertEqual(Snapshot().paired_devices, ())

    def test_controller_publishes_when_the_list_changes(self):
        snaps = []
        c = Controller(StateManager(), snaps.append)
        c.set_paired(list(self.PAIRED))
        c.set_paired(list(self.PAIRED))  # same list: no second publish
        self.assertEqual([s.paired_devices for s in snaps], [self.PAIRED])


if __name__ == "__main__":
    unittest.main()
