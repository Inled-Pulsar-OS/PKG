"""PulsarOS Spotlight application entry point."""

from __future__ import annotations

import logging
import sys

from gi.repository import Gio, GLib, Gtk

from pulsaros_spotlight import __app_id__, __version__
from pulsaros_spotlight.clipboard import ClipboardManager
from pulsaros_spotlight.config import SpotlightConfig
from pulsaros_spotlight.search import SearchBackend
from pulsaros_spotlight.ui.window import SpotlightWindow

logger = logging.getLogger(__name__)


class SpotlightApp(Gtk.Application):
    """Main Adwaita application."""

    def __init__(self) -> None:
        super().__init__(
            application_id=__app_id__,
            flags=Gio.ApplicationFlags.FLAGS_NONE,
        )
        self._config = SpotlightConfig()
        self._clipboard_mgr = ClipboardManager(self._config)
        self._backend = SearchBackend(clipboard_mgr=self._clipboard_mgr)
        self._window: SpotlightWindow | None = None

    def do_activate(self) -> None:
        self.hold()

        if not self._backend.is_ready:
            self._backend.connect()

        if not self._window:
            self._window = SpotlightWindow(
                application=self,
                config=self._config,
                backend=self._backend,
                clipboard_mgr=self._clipboard_mgr,
            )

        is_hidden = "--hidden" in sys.argv
        if is_hidden:
            return

        self._window.present_with_focus()


def main() -> None:
    logging.basicConfig(level=logging.INFO, format="%(name)s: %(message)s")
    app = SpotlightApp()
    clean_argv = [arg for arg in sys.argv if arg != "--hidden"]
    app.run(clean_argv)
