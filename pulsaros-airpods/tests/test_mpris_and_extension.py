"""Seam 4 - MPRIS D-Bus backend (vs fake players) and the extension <-> contract drift guard."""

import os
import re
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))
from harness import pump, requires_private_bus  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
XML = os.path.join(HERE, "..", "data", "org.pulsaros.AirPods.xml")
EXT = os.path.join(HERE, "..", "gnome-shell-extension")


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
    """Guards that only show up inside the Shell: a missing import or a stale metadata file."""

    def js(self, name):
        with open(os.path.join(EXT, name), encoding="utf-8") as f:
            return f.read()

    def test_every_local_import_resolves_to_a_file(self):
        for name in sorted(n for n in os.listdir(EXT) if n.endswith(".js")):
            for target in re.findall(r"from '\./([\w.-]+)'", self.js(name)):
                with self.subTest(importer=name, target=target):
                    self.assertTrue(os.path.exists(os.path.join(EXT, target)), target)

    def test_indicator_no_longer_uses_the_v1_dbus_service(self):
        source = self.js("indicator.js")
        self.assertNotIn("interface.js", source)
        self.assertNotIn("org.pulsaros", source)
        self.assertIn("StatusSource", source)

    def test_metadata_targets_48_and_51_and_matches_directory_name(self):
        import json

        with open(os.path.join(EXT, "metadata.json")) as f:
            meta = json.load(f)
        self.assertEqual(meta["uuid"], "airpods@pulsaros.org")  # the installed directory name
        self.assertEqual(meta["shell-version"], ["48", "51"])

    def test_every_icon_the_indicator_asks_for_ships(self):
        names = set(re.findall(r"'(airpods[\w-]*-symbolic)'", self.js("indicator.js")))
        self.assertTrue(names)
        for name in names:
            self.assertTrue(os.path.exists(os.path.join(EXT, "icons", name + ".svg")), name)


if __name__ == "__main__":
    unittest.main()
