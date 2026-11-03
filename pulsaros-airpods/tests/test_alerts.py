"""Seam 1 - domain: low-battery alerts (macOS thresholds 20 / 10 / 5 percent)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))

from pulsaros_airpods.alerts import BatteryAlerts  # noqa: E402
from pulsaros_airpods.controller import Snapshot  # noqa: E402


def snap(left=-1, right=-1, case=-1, **kw):
    return Snapshot(connected=True, has_advertisement=True, left=left, right=right, case=case, **kw)


class LowBatteryAlertTests(unittest.TestCase):
    def setUp(self):
        self.alerts = BatteryAlerts()

    def test_no_alert_above_20(self):
        self.assertEqual(self.alerts.update(snap(left=30, right=90, case=100)), [])

    def test_crossing_20_alerts_once_for_that_bud(self):
        self.alerts.update(snap(left=30, right=70))
        self.assertEqual(self.alerts.update(snap(left=20, right=70)), [("left", 20)])
        self.assertEqual(self.alerts.update(snap(left=20, right=70)), [])
        self.assertEqual(self.alerts.update(snap(left=20, right=70)), [])

    def test_each_lower_threshold_alerts_once(self):
        self.alerts.update(snap(right=30))
        self.assertEqual(self.alerts.update(snap(right=20)), [("right", 20)])
        self.assertEqual(self.alerts.update(snap(right=10)), [("right", 10)])
        self.assertEqual(self.alerts.update(snap(right=10)), [])
        self.assertEqual(self.alerts.update(snap(right=0)), [("right", 0)])

    def test_jump_across_thresholds_alerts_only_the_lowest(self):
        self.alerts.update(snap(left=50))
        self.assertEqual(self.alerts.update(snap(left=10)), [("left", 10)])

    def test_case_alerts_independently(self):
        self.alerts.update(snap(left=60, right=60, case=30))
        self.assertEqual(self.alerts.update(snap(left=60, right=60, case=20)), [("case", 20)])

    def test_charging_pod_never_alerts(self):
        self.assertEqual(self.alerts.update(snap(left=10, left_charging=True)), [])
        self.assertEqual(self.alerts.update(snap(case=10, case_charging=True)), [])

    def test_charging_then_draining_again_alerts_again(self):
        self.alerts.update(snap(left=10))
        self.alerts.update(snap(left=60, left_charging=True))
        self.assertEqual(self.alerts.update(snap(left=20)), [("left", 20)])

    def test_recharge_without_charging_flag_rearms(self):
        self.assertEqual(self.alerts.update(snap(left=20)), [("left", 20)])
        self.alerts.update(snap(left=60))  # level rose above the threshold
        self.assertEqual(self.alerts.update(snap(left=20)), [("left", 20)])

    def test_unavailable_level_is_ignored_and_does_not_rearm(self):
        self.alerts.update(snap(left=20))
        self.assertEqual(self.alerts.update(snap(left=-1)), [])
        self.assertEqual(self.alerts.update(snap(left=20)), [])

    def test_reconnect_rearms(self):
        self.alerts.update(snap(left=20))
        self.alerts.update(Snapshot())  # disconnected
        self.assertEqual(self.alerts.update(snap(left=20)), [("left", 20)])

    def test_no_advertisement_means_no_alert(self):
        self.assertEqual(
            self.alerts.update(Snapshot(connected=True, fallback_battery=10)), [])

    def test_headphones_alert_as_headphones(self):
        self.assertEqual(
            self.alerts.update(snap(left=20, right=-1, is_headphones=True)), [("headphones", 20)])
        self.assertEqual(
            self.alerts.update(snap(left=20, right=-1, is_headphones=True)), [])


if __name__ == "__main__":
    unittest.main()
