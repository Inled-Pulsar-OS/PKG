#!/usr/bin/env python3
"""Fake MPRIS player on the session bus.

  fake_player.py <suffix> [Playing|Paused] [--no-object]

Owns org.mpris.MediaPlayer2.<suffix>. With --no-object it only owns the name (a dead player).
Prints "ready" when up. Pause()/Play() change PlaybackStatus.
"""

import sys

import dbus
import dbus.mainloop.glib
import dbus.service
from gi.repository import GLib

PLAYER = "org.mpris.MediaPlayer2.Player"


class Player(dbus.service.Object):
    def __init__(self, bus, status):
        super().__init__(bus, "/org/mpris/MediaPlayer2")
        self.status = status

    @dbus.service.method("org.freedesktop.DBus.Properties", in_signature="ss", out_signature="v")
    def Get(self, iface, name):
        if iface == PLAYER and name == "PlaybackStatus":
            return self.status
        raise dbus.exceptions.DBusException("unknown property")

    @dbus.service.method(PLAYER)
    def Pause(self):
        self.status = "Paused"

    @dbus.service.method(PLAYER)
    def Play(self):
        self.status = "Playing"


def main():
    suffix = sys.argv[1]
    status = sys.argv[2] if len(sys.argv) > 2 and not sys.argv[2].startswith("--") else "Playing"
    dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
    bus = dbus.SessionBus()
    name = dbus.service.BusName(f"org.mpris.MediaPlayer2.{suffix}", bus)  # noqa: F841
    if "--no-object" not in sys.argv:
        Player(bus, status)
    print("ready", flush=True)
    GLib.MainLoop().run()


if __name__ == "__main__":
    main()
