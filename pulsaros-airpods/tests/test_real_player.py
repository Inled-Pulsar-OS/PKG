"""Seam 4 - MPRIS backend against a REAL player (VLC), not the fake. Skipped without vlc."""

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
import wave

sys.path.insert(0, os.path.dirname(__file__))
from harness import pump, requires_private_bus  # noqa: E402

VLC = shutil.which("vlc") or shutil.which("cvlc")
NAME = "org.mpris.MediaPlayer2.vlc"


def write_tone(path, seconds=60):
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(8000)
        w.writeframes(b"\x00\x00" * 8000 * seconds)


@requires_private_bus
@unittest.skipUnless(VLC, "vlc not installed")
class RealPlayerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import dbus.mainloop.glib

        dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)

    def setUp(self):
        import dbus

        self.tmp = tempfile.TemporaryDirectory()
        wav = os.path.join(self.tmp.name, "silence.wav")
        write_tone(wav)
        env = {k: v for k, v in os.environ.items() if k not in ("DISPLAY", "WAYLAND_DISPLAY")}
        self.vlc = subprocess.Popen(
            [VLC, "-I", "dummy", "--control", "dbus", "--no-video", "--aout", "dummy", wav],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
        self.bus = dbus.SessionBus()
        self.assertTrue(pump(lambda: self.bus.name_has_owner(NAME), 20), "vlc never appeared on the bus")
        self.assertTrue(pump(lambda: self.status() == "Playing", 20), f"vlc status {self.status()}")

    def tearDown(self):
        self.vlc.terminate()
        self.vlc.wait(10)
        self.tmp.cleanup()

    def status(self):
        import dbus

        try:
            obj = self.bus.get_object(NAME, "/org/mpris/MediaPlayer2")
            return str(dbus.Interface(obj, "org.freedesktop.DBus.Properties").Get(
                "org.mpris.MediaPlayer2.Player", "PlaybackStatus"))
        except dbus.DBusException:
            return None

    def ear(self):
        from pulsaros_airpods.mpris import DbusPlayers, EarDetection

        return EarDetection(DbusPlayers(self.bus), lambda: True)

    def test_backend_sees_the_playing_player(self):
        from pulsaros_airpods.mpris import DbusPlayers

        players = DbusPlayers(self.bus).playing()
        self.assertEqual(len(players), 1)  # VLC owns two names; it is one player
        self.assertTrue(players[0].startswith(NAME))

    def test_removing_the_buds_pauses_and_putting_them_back_resumes(self):
        ear = self.ear()
        ear.on_both_in_ear(False)
        self.assertTrue(pump(lambda: self.status() == "Paused"), self.status())
        ear.on_both_in_ear(True)
        self.assertTrue(pump(lambda: self.status() == "Playing"), self.status())

    def test_a_player_paused_by_the_user_is_not_resumed(self):
        import dbus

        obj = self.bus.get_object(NAME, "/org/mpris/MediaPlayer2")
        dbus.Interface(obj, "org.mpris.MediaPlayer2.Player").Pause()
        self.assertTrue(pump(lambda: self.status() == "Paused"))
        ear = self.ear()
        ear.on_both_in_ear(False)
        ear.on_both_in_ear(True)
        pump(lambda: False, 1.0)
        self.assertEqual(self.status(), "Paused")


if __name__ == "__main__":
    unittest.main()
