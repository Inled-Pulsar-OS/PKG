"""Seam 4 - MPRIS D-Bus backend (vs fake players) and the extension <-> contract drift guard."""

import os
import re
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))
from harness import pump, requires_private_bus  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
SHARE = os.path.join(HERE, "..", "usr", "share")
XML = os.path.join(SHARE, "pulsaros-airpods", "dbus", "org.pulsaros.AirPods.xml")
EXT = os.path.join(SHARE, "gnome-shell", "extensions", "airpods@pulsaros.org")


@requires_private_bus
class MprisBackendTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import dbus.mainloop.glib

        dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)

    def setUp(self):
        self.procs = []

    def tearDown(self):
        for p in self.procs:
            p.terminate()
            p.wait(5)
            p.stdout.close()

    def player(self, suffix, *args):
        p = subprocess.Popen([sys.executable, os.path.join(HERE, "fake_player.py"), suffix, *args],
                             stdout=subprocess.PIPE, text=True)
        self.assertEqual(p.stdout.readline().strip(), "ready")
        self.procs.append(p)

    def status(self, suffix):
        import dbus

        bus = dbus.SessionBus()
        obj = bus.get_object(f"org.mpris.MediaPlayer2.{suffix}", "/org/mpris/MediaPlayer2")
        return str(dbus.Interface(obj, "org.freedesktop.DBus.Properties").Get(
            "org.mpris.MediaPlayer2.Player", "PlaybackStatus"))

    def backend(self):
        import dbus
        from pulsaros_airpods.mpris import DbusPlayers

        return DbusPlayers(dbus.SessionBus())

    def test_no_players(self):
        self.assertEqual(self.backend().playing(), [])

    def test_lists_only_playing_players(self):
        self.player("one", "Playing")
        self.player("two", "Paused")
        self.assertEqual(self.backend().playing(), ["org.mpris.MediaPlayer2.one"])

    def test_pause_then_play(self):
        self.player("one", "Playing")
        backend = self.backend()
        backend.pause("org.mpris.MediaPlayer2.one")
        self.assertTrue(pump(lambda: self.status("one") == "Paused"))
        backend.play("org.mpris.MediaPlayer2.one")
        self.assertTrue(pump(lambda: self.status("one") == "Playing"))

    def test_dead_player_name_is_skipped(self):
        self.player("dead", "--no-object")
        self.player("alive", "Playing")
        self.assertEqual(self.backend().playing(), ["org.mpris.MediaPlayer2.alive"])

    def test_calls_to_a_vanished_player_do_not_raise(self):
        backend = self.backend()
        backend.pause("org.mpris.MediaPlayer2.gone")
        backend.play("org.mpris.MediaPlayer2.gone")

    def test_ear_detection_end_to_end(self):
        from pulsaros_airpods.mpris import EarDetection

        self.player("one", "Playing")
        self.player("two", "Paused")  # not ours: must stay paused
        ear = EarDetection(self.backend(), lambda: True)
        ear.on_both_in_ear(False)
        self.assertTrue(pump(lambda: self.status("one") == "Paused"))
        ear.on_both_in_ear(True)
        self.assertTrue(pump(lambda: self.status("one") == "Playing"))
        self.assertEqual(self.status("two"), "Paused")


class ExtensionContractTests(unittest.TestCase):
    """The extension reads properties by name; a typo only shows up inside the Shell."""

    def setUp(self):
        with open(XML, encoding="utf-8") as f:
            self.xml = f.read()
        self.props = set(re.findall(r'<property name="(\w+)"', self.xml))
        self.signals = set(re.findall(r'<signal name="(\w+)"', self.xml))

    def js(self, name):
        with open(os.path.join(EXT, name), encoding="utf-8") as f:
            return f.read()

    def test_every_property_used_by_the_indicator_exists(self):
        used = set(re.findall(r"\b(?:p|proxy|_proxy)\.([A-Z]\w+)", self.js("indicator.js")))
        self.assertTrue(used, "regex found no property use")
        self.assertEqual(sorted(used - self.props), [])

    def test_every_signal_the_indicator_listens_to_exists(self):
        used = set(re.findall(r"connectSignal\('(\w+)'", self.js("indicator.js")))
        self.assertEqual(used, {"LidOpened"})
        self.assertTrue(used <= self.signals)

    def test_metadata_targets_48_and_51_and_matches_directory_name(self):
        import json

        with open(os.path.join(EXT, "metadata.json")) as f:
            meta = json.load(f)
        self.assertEqual(meta["uuid"], os.path.basename(EXT))
        self.assertEqual(meta["shell-version"], ["48", "51"])

    def test_every_icon_the_indicator_asks_for_ships(self):
        names = set(re.findall(r"'(airpods[\w-]*-symbolic)'", self.js("indicator.js")))
        self.assertTrue(names)
        for name in names:
            self.assertTrue(os.path.exists(os.path.join(EXT, "icons", name + ".svg")), name)


if __name__ == "__main__":
    unittest.main()
