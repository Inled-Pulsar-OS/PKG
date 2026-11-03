"""pulsaros-airpods-daemon entry point."""

import argparse
import logging
import signal
import sys

import dbus
import dbus.mainloop.glib
from gi.repository import GLib

from .bluez import BluezBackend
from .config import Config
from .controller import Controller
from .mpris import DbusPlayers, EarDetection
from .service import Service
from .state import StateManager
from .tracker import Tracker

log = logging.getLogger("pulsaros-airpods")
TICK_SECONDS = 1


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="AirPods battery and state for Pulsar OS")
    parser.add_argument("--debug", action="store_true", help="verbose logging")
    parser.add_argument("--config", help="path of the config file")
    parser.add_argument(
        "--bluez-bus", choices=("system", "session"), default="system", help=argparse.SUPPRESS
    )  # session: talk to a fake org.bluez in tests
    args = parser.parse_args(argv)

    logging.basicConfig(
        level=logging.DEBUG if args.debug else logging.INFO,
        format="%(levelname)s %(name)s: %(message)s",  # journald adds the timestamp
        stream=sys.stderr,
    )

    dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
    loop = GLib.MainLoop()

    config = Config(args.config)
    tracker = Tracker(config.get("bound_address"), lambda addr: config.set("bound_address", addr))
    state_mgr = StateManager(rssi_min=config.get("rssi_min"))

    service = Service(config.get, config.set)
    ear = EarDetection(DbusPlayers(dbus.SessionBus()), lambda: config.get("ear_detection"))

    def on_lid_opened() -> None:
        if config.get("lid_popup"):
            service.emit_lid_opened()

    controller = Controller(
        state_mgr,
        on_snapshot=service.publish,
        on_lid_opened=on_lid_opened,
        on_both_in_ear=ear.on_both_in_ear,
    )

    def on_device(path, props, battery) -> None:
        if props is None:
            tracker.remove(path)
        else:
            tracker.update(path, props)
            if battery is not None:
                tracker.update_battery(path, battery)
        controller.set_device(tracker.active)
        controller.refresh()

    bluez_bus = dbus.SessionBus() if args.bluez_bus == "session" else dbus.SystemBus()
    backend = BluezBackend(controller.on_adv, on_device, bluez_bus)

    def tick() -> bool:
        controller.tick()
        return True

    def quit_(*_) -> bool:
        log.info("shutting down")
        loop.quit()
        return False

    GLib.unix_signal_add(GLib.PRIORITY_DEFAULT, signal.SIGTERM, quit_)
    GLib.unix_signal_add(GLib.PRIORITY_DEFAULT, signal.SIGINT, quit_)

    service.start()
    backend.start()
    GLib.timeout_add_seconds(TICK_SECONDS, tick)
    log.info("started")
    try:
        loop.run()
    finally:
        backend.stop()
        service.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
