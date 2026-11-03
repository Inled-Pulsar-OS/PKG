"""End to end: daemon <-> fake org.bluez <-> D-Bus client, on a private session bus.

Run with:  dbus-run-session -- python3 -m unittest tests/test_integration.py
Skipped when no session bus is available or when it is the user's real session
(the test needs to own org.bluez and org.pulsaros.AirPods).
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import unittest

import dbus

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "src")
sys.path.insert(0, HERE)
from test_protocol_state import make_packet  # noqa: E402
from pulsaros_airpods.protocol import Side  # noqa: E402

IFACE = "org.pulsaros.AirPods"


def private_bus() -> bool:
    return bool(os.environ.get("DBUS_SESSION_BUS_ADDRESS")) and os.environ.get("PULSAROS_TEST_BUS") == "1"


@unittest.skipUnless(private_bus(), "run under: PULSAROS_TEST_BUS=1 dbus-run-session -- ...")
class IntegrationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.bluez = None
        self.daemon = None
        self.start_bluez()
        env = dict(os.environ, PULSAROS_AIRPODS_ROOT=ROOT, PYTHONPATH=ROOT)
        self.daemon = subprocess.Popen(
            [sys.executable, "-m", "pulsaros_airpods.daemon", "--bluez-bus", "session",
             "--config", os.path.join(self.tmp, "c.json"), "--debug"],
            env=env, stderr=subprocess.PIPE, text=True,
        )
        self.bus = dbus.SessionBus()
        self.wait(lambda: self.props() is not None, "service up")

    def tearDown(self):
        for p in (self.daemon, self.bluez):
            if p:
                p.terminate()
                try:
                    p.wait(5)
                except subprocess.TimeoutExpired:
                    p.kill()
        shutil.rmtree(self.tmp, ignore_errors=True)

    def start_bluez(self):
        self.bluez = subprocess.Popen(
            [sys.executable, os.path.join(HERE, "fake_bluez.py")],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
        )
        self.assertEqual(self.bluez.stdout.readline().strip(), "ready")

    def send(self, **cmd):
        self.bluez.stdin.write(json.dumps(cmd) + "\n")
        self.bluez.stdin.flush()

    def props(self):
        try:
            obj = self.bus.get_object("org.pulsaros.AirPods", "/org/pulsaros/AirPods")
            return dict(dbus.Interface(obj, "org.freedesktop.DBus.Properties").GetAll(IFACE))
        except dbus.DBusException:
            return None

    def wait(self, cond, what, timeout=10.0):
        end = time.time() + timeout
        while time.time() < end:
            if cond():
                return
            time.sleep(0.1)
        self.fail(f"timeout waiting for: {what}; props={self.props()}")

    def test_full_flow(self):
        # connected, no advertisement yet: BlueZ Battery1 fallback
        self.wait(lambda: self.props()["Connected"], "connected")
        p = self.props()
        self.assertEqual(p["FallbackBattery"], 60)
        self.assertFalse(p["HasAdvertisement"])
        self.assertEqual(p["Model"], "AirPods Pro 2")
        self.assertEqual(p["Left"], -1)

        # advertisement fills L/R/case
        pkt = make_packet(0x2014, Side.LEFT, 8, 7, 5).hex()
        self.send(op="device", props={"RSSI": -50, "ManufacturerData": {"76": pkt}})
        self.wait(lambda: self.props()["HasAdvertisement"], "advertisement")
        p = self.props()
        self.assertEqual((p["Left"], p["Right"], p["Case"]), (80, 70, 50))
        self.assertTrue(p["LeftInEar"] and p["RightInEar"])

        # settings are writable and persisted
        obj = self.bus.get_object("org.pulsaros.AirPods", "/org/pulsaros/AirPods")
        dbus.Interface(obj, "org.freedesktop.DBus.Properties").Set(IFACE, "EarDetection", False)
        self.assertFalse(self.props()["EarDetection"])

        # bluetoothd restarts: the daemon recovers
        self.bluez.terminate()
        self.bluez.wait(5)
        self.wait(lambda: not self.props()["Connected"], "disconnected after bluez exit")
        self.start_bluez()
        self.wait(lambda: self.props()["Connected"], "re-attached to bluez", timeout=15)

        # disconnect
        self.send(op="device", props={"Connected": False})
        self.wait(lambda: not self.props()["Connected"], "disconnect")
        p = self.props()
        self.assertEqual((p["Left"], p["FallbackBattery"], p["Name"]), (-1, -1, ""))


if __name__ == "__main__":
    unittest.main()
