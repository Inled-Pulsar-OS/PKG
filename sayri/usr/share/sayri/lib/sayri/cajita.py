"""Apple-Intelligence style Cajita widget matching macOS / iPadOS Siri UI (GTK4).

Features:
- Top Input Pill with Siri/Settings button, live transcription text entry, and Mic toggle.
  - Border glow activates when speech is detected or typing.
  - Border glow rotates around the pill while processing (thinking).
- Bottom Response Card with frosted acrylic card.
  - Border glow rotates around the response card during TTS dictation playback (speaking).
"""

from __future__ import annotations

import math
import time

import cairo
import gi

gi.require_version("Gtk", "4.0")
from gi.repository import Gdk, GLib, Gtk, Pango  # noqa: E402

CAJITA_CSS = b"""
.sayri-cajita-container {
    background: transparent;
    background-color: transparent;
}

.sayri-pill-container {
    background: transparent;
    background-color: transparent;
}

entry,
entry.sayri-pill-entry,
entry.sayri-pill-entry:focus,
entry.sayri-pill-entry:backdrop,
entry.sayri-pill-entry text,
entry.sayri-pill-entry text:focus,
entry.sayri-pill-entry text:backdrop,
entry.sayri-pill-entry > text,
entry.sayri-pill-entry > text:focus,
entry.sayri-pill-entry > text:backdrop,
entry.sayri-pill-entry > text > placeholder {
    background: transparent;
    background-color: transparent;
    background-image: none;
    border: none;
    box-shadow: none;
    outline: none;
    color: #f8fafc;
    font-size: 14.5px;
    font-weight: 500;
    padding: 0 6px;
    min-height: 36px;
}

button,
button.flat,
button.sayri-icon-btn,
button.sayri-icon-btn:hover,
button.sayri-icon-btn:active,
button.sayri-icon-btn:focus,
button.sayri-icon-btn:checked,
button.sayri-icon-btn:disabled,
button.sayri-icon-btn:backdrop,
button.sayri-icon-btn * {
    background: transparent;
    background-color: transparent;
    background-image: none;
    border: none;
    border-radius: 0;
    box-shadow: none;
    outline: none;
    color: #cbd5e1;
    min-width: 28px;
    min-height: 28px;
    padding: 0 4px;
}

.sayri-response-card {
    background: rgba(18, 21, 30, 0.92);
    background-color: rgba(18, 21, 30, 0.92);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 20px;
    padding: 16px 20px;
    min-height: 48px;
}

.sayri-response-label {
    color: #f8fafc;
    font-size: 14.5px;
    line-height: 1.45;
    background: transparent;
}
"""

SVG_SIRI_ICON = b"""<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="10" stroke="#a855f7" stroke-width="2.2"/><path d="M6 12c3-4 9-4 12 0" stroke="#38bdf8" stroke-width="2.2" stroke-linecap="round"/><path d="M6 12c3 4 9 4 12 0" stroke="#ec4899" stroke-width="2.2" stroke-linecap="round"/></svg>"""

SVG_MIC = b"""<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#f1f5f9" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" x2="12" y1="19" y2="22"/></svg>"""

SVG_MIC_ACTIVE = b"""<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="#ffffff" stroke="none"><circle cx="12" cy="12" r="6"/></svg>"""


def _svg_icon(svg_bytes: bytes) -> Gtk.Widget:
    try:
        tex = Gdk.Texture.new_from_bytes(GLib.Bytes.new(svg_bytes))
        pic = Gtk.Picture.new_for_paintable(tex)
        pic.set_content_fit(Gtk.ContentFit.CONTAIN)
        pic.set_size_request(16, 16)
        pic.set_valign(Gtk.Align.CENTER)
        pic.set_halign(Gtk.Align.CENTER)
        return pic
    except Exception:
        return Gtk.Box()


class ChromaBackground(Gtk.DrawingArea):
    """Draws frosted acrylic surface with dynamic Apple Intelligence Chroma-Ring glow."""

    def __init__(self, is_pill: bool = True) -> None:
        super().__init__()
        self.is_pill = is_pill
        self.mode = "idle"  # "idle", "active", "rotating"
        self.phase = 0.0
        self.last_tick = time.monotonic()
        self.speed = 2.0
        self.audio_level = 0.0

        self.set_hexpand(True)
        self.set_vexpand(True)
        self.set_draw_func(self._draw)
        self.add_tick_callback(self._on_tick)

    def set_mode(self, mode: str) -> None:
        if self.mode != mode:
            self.mode = mode
            if mode == "rotating":
                self.speed = 3.5
            else:
                self.speed = 1.8
            self.queue_draw()

    def set_audio_level(self, lvl: float) -> None:
        self.audio_level = lvl
        self.queue_draw()

    def _on_tick(self, _widget, _frame_clock) -> bool:
        now = time.monotonic()
        dt = now - self.last_tick
        self.last_tick = now
        if self.mode in ("active", "rotating"):
            self.phase = (self.phase + dt * self.speed) % (2.0 * math.pi)
            self.queue_draw()
        return GLib.SOURCE_CONTINUE

    def _draw_rounded_rect(self, cr: cairo.Context, x: float, y: float, w: float, h: float, r: float) -> None:
        cr.new_sub_path()
        cr.arc(x + w - r, y + r, r, -math.pi / 2.0, 0)
        cr.arc(x + w - r, y + h - r, r, 0, math.pi / 2.0)
        cr.arc(x + r, y + h - r, r, math.pi / 2.0, math.pi)
        cr.arc(x + r, y + r, r, math.pi, 3.0 * math.pi / 2.0)
        cr.close_path()

    def _draw(self, _area, cr: cairo.Context, w: int, h: int) -> None:
        if w <= 0 or h <= 0:
            return

        pad = 4.0
        r = (h - 2 * pad) / 2.0 if self.is_pill else 18.0
        bx = pad
        by = pad
        bw = w - 2 * pad
        bh = h - 2 * pad

        cx = w / 2.0
        cy = h / 2.0
        t = self.phase

        # 1. Frosted Dark Glass Interior
        self._draw_rounded_rect(cr, bx, by, bw, bh, r)
        cr.set_source_rgba(0.08, 0.09, 0.14, 0.90)
        cr.fill()

        # 2. Border & Glow rendering based on state
        if self.mode == "idle":
            # Subtle neutral acrylic glass border
            self._draw_rounded_rect(cr, bx, by, bw, bh, r)
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.14)
            cr.set_line_width(1.0)
            cr.stroke()
        else:
            # Chromatic glow (Active or Rotating)
            glow_dist = math.hypot(bw, bh) * 0.5
            angle = t if self.mode == "rotating" else 0.5
            x0 = cx + glow_dist * math.cos(angle)
            y0 = cy + glow_dist * math.sin(angle)
            x1 = cx - glow_dist * math.cos(angle)
            y1 = cy - glow_dist * math.sin(angle)

            # Outer glow halo
            cr.set_operator(cairo.OPERATOR_ADD)
            grad = cairo.LinearGradient(x0, y0, x1, y1)
            grad.add_color_stop_rgba(0.00, 0.22, 0.74, 0.98, 0.35)  # Cyan
            grad.add_color_stop_rgba(0.50, 0.66, 0.33, 0.97, 0.40)  # Violet
            grad.add_color_stop_rgba(1.00, 0.93, 0.28, 0.60, 0.35)  # Magenta

            halo_w = 4.5 + 2.0 * math.sin(t * 2.0)
            self._draw_rounded_rect(cr, bx, by, bw, bh, r)
            cr.set_source(grad)
            cr.set_line_width(halo_w)
            cr.stroke()

            # Dynamic Chroma-Ring border
            cr.set_operator(cairo.OPERATOR_OVER)
            ring_grad = cairo.LinearGradient(x0, y0, x1, y1)
            ring_grad.add_color_stop_rgba(0.00, 0.22, 0.74, 0.98, 0.95)
            ring_grad.add_color_stop_rgba(0.50, 0.66, 0.33, 0.97, 0.95)
            ring_grad.add_color_stop_rgba(1.00, 0.93, 0.28, 0.60, 0.95)

            border_w = 1.6 + 0.8 * math.sin(t * 2.0)
            self._draw_rounded_rect(cr, bx, by, bw, bh, r)
            cr.set_source(ring_grad)
            cr.set_line_width(border_w)
            cr.stroke()


class SayriCajita(Gtk.Box):
    """Apple-Intelligence style Dual-Card UI (Top Input Pill + Bottom Response Card)."""

    def __init__(self, app) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self.app = app
        self.add_css_class("sayri-cajita-container")
        self.set_size_request(400, -1)
        self.set_hexpand(False)
        self.set_vexpand(False)
        self.set_valign(Gtk.Align.CENTER)

        self._load_css()
        self._build_ui()

    def _load_css(self) -> None:
        try:
            provider = Gtk.CssProvider()
            provider.load_from_data(CAJITA_CSS)
            Gtk.StyleContext.add_provider_for_display(
                Gdk.Display.get_default(), provider,
                Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION,
            )
        except Exception:
            pass

    def _build_ui(self) -> None:
        # ── 1. Top Input Pill ─────────────────────────────────────────
        self.pill_overlay = Gtk.Overlay()
        self.pill_overlay.add_css_class("sayri-pill-container")
        self.pill_overlay.set_size_request(400, 48)

        # Background drawing area for Pill
        self.pill_bg = ChromaBackground(is_pill=True)
        self.pill_overlay.set_child(self.pill_bg)

        # Foreground content of Pill
        pill_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        pill_row.set_margin_start(8)
        pill_row.set_margin_end(8)
        pill_row.set_margin_top(6)
        pill_row.set_margin_bottom(6)
        pill_row.set_valign(Gtk.Align.CENTER)

        # Left: Siri / Settings icon button
        self.siri_btn = Gtk.Button()
        self.siri_btn.set_child(_svg_icon(SVG_SIRI_ICON))
        self.siri_btn.set_has_frame(False)
        self.siri_btn.add_css_class("sayri-icon-btn")
        self.siri_btn.set_tooltip_text("Sayri Settings")
        self.siri_btn.connect("clicked", lambda _b: self.app.open_settings())
        pill_row.append(self.siri_btn)

        # Center: Text input entry
        self.entry = Gtk.Entry()
        self.entry.set_has_frame(False)
        self.entry.add_css_class("sayri-pill-entry")
        self.entry.set_placeholder_text("Talk to Siri…")
        self.entry.set_hexpand(True)
        self.entry.connect("activate", self._on_entry_activate)
        self.entry.connect("changed", self._on_entry_changed)
        self.entry.connect("notify::is-focus", self._on_entry_focus)
        pill_row.append(self.entry)

        # Right: Mic toggle button
        self.mic_btn = Gtk.Button()
        self.mic_btn.set_child(_svg_icon(SVG_MIC))
        self.mic_btn.set_has_frame(False)
        self.mic_btn.add_css_class("sayri-icon-btn")
        self.mic_btn.set_tooltip_text("Toggle Microphone")
        self.mic_btn.connect("clicked", lambda _b: self.app.toggle_listening())
        pill_row.append(self.mic_btn)

        self.pill_overlay.add_overlay(pill_row)
        self.append(self.pill_overlay)

        # ── 2. Bottom Response Card ───────────────────────────────────
        self.card_overlay = Gtk.Overlay()
        self.card_overlay.set_size_request(400, -1)
        self.card_overlay.set_hexpand(True)

        card_content = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        card_content.set_margin_start(16)
        card_content.set_margin_end(16)
        card_content.set_margin_top(14)
        card_content.set_margin_bottom(14)

        self.scroll = Gtk.ScrolledWindow()
        self.scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        self.scroll.set_propagate_natural_height(True)
        self.scroll.set_max_content_height(240)

        self.response_label = Gtk.Label(label="")
        self.response_label.add_css_class("sayri-response-label")
        self.response_label.set_halign(Gtk.Align.START)
        self.response_label.set_valign(Gtk.Align.START)
        self.response_label.set_wrap(True)
        self.response_label.set_wrap_mode(Pango.WrapMode.WORD_CHAR)
        self.response_label.set_selectable(True)
        self.scroll.set_child(self.response_label)
        card_content.append(self.scroll)

        self.card_overlay.set_child(card_content)

        # card_bg drawn behind/around the card
        self.card_bg = ChromaBackground(is_pill=False)
        self.card_bg.set_can_target(False)
        self.card_overlay.add_overlay(self.card_bg)

        self.append(self.card_overlay)

        # Hide bottom card initially until response exists
        self.card_overlay.set_visible(False)

    def _on_entry_activate(self, entry: Gtk.Entry) -> None:
        text = entry.get_text().strip()
        if text:
            entry.set_text("")
            self.app.send_text(text)

    def _on_entry_changed(self, entry: Gtk.Entry) -> None:
        text = entry.get_text().strip()
        if text and self.pill_bg.mode == "idle":
            self.pill_bg.set_mode("active")
        elif not text and not self.app.listening_now() and self.pill_bg.mode == "active":
            self.pill_bg.set_mode("idle")

    def _on_entry_focus(self, entry: Gtk.Entry, _pspec) -> None:
        if entry.has_focus():
            self.pill_bg.set_mode("active")
        elif not entry.get_text().strip() and not self.app.listening_now():
            self.pill_bg.set_mode("idle")

    def set_content(self, kind: str, text: str) -> None:
        if not text:
            return

        if kind == "user":
            self.entry.set_text(text)
            self.pill_bg.set_mode("active")
        elif kind == "partial":
            if not self.entry.has_focus():
                self.entry.set_text(text)
                self.pill_bg.set_mode("active")
        elif kind == "assistant":
            self.response_label.set_label(text)
            self.card_overlay.set_visible(True)
            self.card_bg.queue_draw()
        elif kind == "hint":
            pass
        elif kind == "error":
            self.response_label.set_label(f"⚠️ {text}")
            self.card_overlay.set_visible(True)

    def set_mic(self, active: bool) -> None:
        if active:
            self.mic_btn.set_child(_svg_icon(SVG_MIC_ACTIVE))
            self.pill_bg.set_mode("active")
        else:
            self.mic_btn.set_child(_svg_icon(SVG_MIC))
            if not self.entry.get_text().strip():
                self.pill_bg.set_mode("idle")

    def set_busy(self, busy: bool) -> None:
        if busy:
            self.pill_bg.set_mode("rotating")
        else:
            self.entry.set_sensitive(True)
            if not self.app.listening_now() and not self.entry.get_text().strip():
                self.pill_bg.set_mode("idle")

    def set_speaking(self, speaking: bool) -> None:
        if speaking:
            self.card_bg.set_mode("rotating")
        else:
            self.card_bg.set_mode("idle")

    def clear(self) -> None:
        self.response_label.set_label("")
        self.card_overlay.set_visible(False)
        self.card_bg.set_mode("idle")
        self.entry.set_sensitive(True)
        if not self.app.listening_now() and not self.entry.get_text().strip():
            self.pill_bg.set_mode("idle")
