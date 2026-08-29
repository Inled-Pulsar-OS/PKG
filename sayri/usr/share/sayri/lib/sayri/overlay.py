"""Sayri overlay: single transparent window containing the native Siri Orb
and animated Chroma-Ring Cajita, pinned to the TOP-RIGHT corner of the monitor.
"""

import time

import gi

gi.require_version("Gtk", "4.0")
from gi.repository import Gdk, GLib, Gtk  # noqa: E402

from .cajita import SayriCajita
from .orb import SiriOrb
from .webkit import pin_window

BUBBLE_WIDTH = 420
BUBBLE_HEIGHT = 380
GAP = 14
MARGIN = 16
TOP_MARGIN = 44


class SayriOverlay:
    """Single transparent window with both the native orb and the animated chroma cajita."""

    def __init__(self, app) -> None:
        self.app = app
        self.cfg = app.cfg

        orb_size = self.cfg.get_int("ui", "orb_size")
        if orb_size < 100 or orb_size > 300:
            orb_size = 140

        width = BUBBLE_WIDTH + GAP + orb_size
        height = BUBBLE_HEIGHT

        # ── window ──────────────────────────────────────────────────
        self.win = Gtk.ApplicationWindow(application=app)
        self.win.set_default_size(width, height)
        self.win.set_resizable(False)
        self.win.set_decorated(False)
        self.win.add_css_class("sayri-overlay")

        try:
            css = Gtk.CssProvider()
            css.load_from_data(
                b".sayri-overlay { background: transparent; background-color: transparent; } "
                b"window.sayri-overlay { background: transparent; border: none; box-shadow: none; }"
            )
            Gtk.StyleContext.add_provider_for_display(
                Gdk.Display.get_default(), css,
                Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)
        except Exception:  # noqa: BLE001
            pass

        self.win.connect("close-request", lambda *_: (self.win.set_visible(False), True)[-1])

        # ── pin to top-right (always on top, fixed) ───────────────────
        pin_window(self.win, top_margin=TOP_MARGIN, right_margin=MARGIN,
                   width=width, height=height)

        # ── layout: horizontal box [ cajita · orb ] ─────────────────
        hbox = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=GAP)
        hbox.set_halign(Gtk.Align.END)
        hbox.set_valign(Gtk.Align.START)

        # 1. Cajita with Chroma-Ring animated border
        self.cajita = SayriCajita(app)
        self.cajita.set_valign(Gtk.Align.START)
        hbox.append(self.cajita)

        # 2. Native animated Cairo Siri Orb
        self.orb = SiriOrb(size=orb_size, on_click=app.on_orb_click)
        self.orb.set_valign(Gtk.Align.START)
        hbox.append(self.orb)

        self.win.set_child(hbox)

    def set_state_sync(self, state: str, _opts: dict | None = None) -> None:
        self.orb.set_state(state)
        if state == "speaking":
            self.cajita.set_speaking(True)
        else:
            self.cajita.set_speaking(False)
        if state in ("listening", "activated"):
            self.cajita.pill_bg.set_mode("active")
        elif state == "thinking":
            self.cajita.pill_bg.set_mode("rotating")
        elif state == "idle":
            if not self.cajita.entry.get_text().strip():
                self.cajita.pill_bg.set_mode("idle")

    def set_audio_level(self, level: float) -> None:
        self.orb.set_audio_level(level)

    def set_content(self, kind: str, text: str) -> None:
        self.cajita.set_content(kind, text)

    def set_mic(self, active: bool) -> None:
        self.cajita.set_mic(active)

    def set_busy(self, busy: bool) -> None:
        self.cajita.set_busy(busy)

    def clear(self) -> None:
        self.cajita.clear()

    @property
    def is_visible(self) -> bool:
        return getattr(self, "_is_visible", False) or bool(self.win.get_visible())

    def show(self) -> None:
        self._is_visible = True
        self._just_shown = time.monotonic()
        self._was_active = False
        self.win.set_visible(True)
        try:
            self.win.present_with_time(0)
        except Exception:
            self.win.present()
        def _focus():
            self.cajita.entry.grab_focus()
            self.cajita.entry.set_position(-1)
        GLib.idle_add(_focus)

    def hide(self) -> None:
        self._is_visible = False
        self._was_active = False
        self.win.set_visible(False)
        if hasattr(self, "app") and self.app:
            self.app.on_hidden()

    def toggle(self) -> None:
        if self.is_visible:
            self.hide()
        else:
            self.show()

    def apply_config(self) -> None:
        orb_size = self.cfg.get_int("ui", "orb_size")
        if orb_size < 100 or orb_size > 300:
            orb_size = 140
        width = BUBBLE_WIDTH + GAP + orb_size
        height = max(BUBBLE_HEIGHT, orb_size)
        self.orb.set_content_width(orb_size)
        self.orb.set_content_height(orb_size)
        self.win.set_default_size(width, height)
        pin_window(self.win, top_margin=TOP_MARGIN, right_margin=MARGIN,
                   width=width, height=height)
