"""Ear detection: pause the playing MPRIS players when the buds leave the ears, and resume
only the players we paused when both buds are back."""

import logging
from typing import Callable, List, Protocol, Set

log = logging.getLogger(__name__)

MPRIS_PREFIX = "org.mpris.MediaPlayer2."
MPRIS_PATH = "/org/mpris/MediaPlayer2"
PLAYER_IFACE = "org.mpris.MediaPlayer2.Player"


class Players(Protocol):
    def playing(self) -> List[str]: ...
    def pause(self, name: str) -> None: ...
    def play(self, name: str) -> None: ...


class EarDetection:
    def __init__(self, players: Players, enabled: Callable[[], bool]):
        self._players = players
        self._enabled = enabled
        self._paused: Set[str] = set()

    def on_both_in_ear(self, both_in_ear: bool) -> None:
        if not self._enabled():
            self._paused.clear()
            return

        if both_in_ear:
            for name in sorted(self._paused):
                log.info("ear detection: resume %s", name)
                self._players.play(name)
            self._paused.clear()
            return

        for name in self._players.playing():
            log.info("ear detection: pause %s", name)
            self._players.pause(name)
            self._paused.add(name)


class DbusPlayers:
    """MPRIS over the session bus (python-dbus)."""

    def __init__(self, bus):
        self._bus = bus

    def _names(self) -> List[str]:
        return [n for n in self._bus.list_names() if n.startswith(MPRIS_PREFIX)]

    def _iface(self, name: str, iface: str):
        import dbus

        return dbus.Interface(self._bus.get_object(name, MPRIS_PATH), iface)

    def _pid(self, name: str):
        import dbus

        daemon = dbus.Interface(
            self._bus.get_object("org.freedesktop.DBus", "/org/freedesktop/DBus"), "org.freedesktop.DBus"
        )
        try:
            return int(daemon.GetConnectionUnixProcessID(name))
        except dbus.DBusException:
            return name  # cannot tell: treat the name as its own player

    def playing(self) -> List[str]:
        import dbus

        result, seen_owners = [], set()
        for name in self._names():
            try:
                # One process can own several MPRIS names on separate connections
                # (VLC: ...vlc and ...vlc.instance<pid>): count it once, by process id.
                owner = self._pid(name)
                if owner in seen_owners:
                    continue
                props = self._iface(name, "org.freedesktop.DBus.Properties")
                if str(props.Get(PLAYER_IFACE, "PlaybackStatus", timeout=1)) == "Playing":
                    result.append(name)
                    seen_owners.add(owner)
            except dbus.DBusException as e:
                log.debug("mpris %s: %s", name, e)
        return result

    def _call(self, name: str, method: str) -> None:
        import dbus

        try:
            getattr(self._iface(name, PLAYER_IFACE), method)(
                reply_handler=lambda: None,
                error_handler=lambda e: log.debug("mpris %s %s: %s", name, method, e),
            )
        except dbus.DBusException as e:
            log.debug("mpris %s %s: %s", name, method, e)

    def pause(self, name: str) -> None:
        self._call(name, "Pause")

    def play(self, name: str) -> None:
        self._call(name, "Play")
