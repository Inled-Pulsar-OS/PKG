"""Apple-Intelligence style Cajita widget with animated Chroma-Ring border (GTK4).

Provides a frosted floating card with animated chromatic border (chroma-ring),
chat transcript, text input, microphone toggle, and settings button.
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

.sayri-line-user {
    color: #60a5fa;
    font-size: 13.5px;
    font-weight: 600;
    margin-bottom: 2px;
}

.sayri-line-assistant {
    color: #f1f5f9;
    font-size: 13.5px;
    line-height: 1.35;
    margin-bottom: 2px;
}

.sayri-line-partial {
    color: #94a3b8;
    font-size: 13px;
    font-style: italic;
    margin-bottom: 2px;
}

.sayri-line-hint {
    color: #94a3b8;
    font-size: 12.5px;
    margin-bottom: 2px;
}

.sayri-line-error {
    color: #f87171;
    font-size: 13px;
    font-weight: bold;
    margin-bottom: 2px;
}

.sayri-placeholder {
    color: #64748b;
    font-size: 13px;
}

entry.sayri-input,
entry.sayri-input:focus,
entry.sayri-input:backdrop {
    background: rgba(255, 255, 255, 0.08);
    background-color: rgba(255, 255, 255, 0.08);
    background-image: none;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 16px;
    color: #f8fafc;
    font-size: 13px;
    padding: 4px 12px;
    min-height: 32px;
    box-shadow: none;
    outline: none;
}

entry.sayri-input:focus {
    border-color: rgba(110, 168, 254, 0.85);
    background-color: rgba(255, 255, 255, 0.12);
}

button.sayri-btn,
button.sayri-btn:active,
button.sayri-btn:focus,
button.sayri-btn:checked,
button.sayri-btn:disabled,
button.sayri-btn:backdrop {
    background: rgba(255, 255, 255, 0.08);
    background-color: rgba(255, 255, 255, 0.08);
    background-image: none;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 16px;
    color: #cbd5e1;
    min-width: 32px;
    min-height: 32px;
    padding: 0;
    box-shadow: none;
    outline: none;
}

button.sayri-btn:hover {
    background: rgba(255, 255, 255, 0.18);
    background-color: rgba(255, 255, 255, 0.18);
    border-color: rgba(255, 255, 255, 0.25);
    color: #ffffff;
}

button.sayri-btn-mic-active,
button.sayri-btn-mic-active:hover,
button.sayri-btn-mic-active:focus {
    background: rgba(239, 68, 68, 0.85);
    background-color: rgba(239, 68, 68, 0.85);
    border-color: rgba(239, 68, 68, 0.95);
    color: #ffffff;
}
"""


class ChromaBackground(Gtk.DrawingArea):
    """Animated Apple-Intelligence chroma-ring border and dark acrylic card."""

    def __init__(self, width: int = 420, height: int = 140) -> None:
        super().__init__()
        self.set_content_width(width)
        self.set_content_height(height)
        self.set_hexpand(True)
        self.set_vexpand(True)
        self.phase = 0.0
        self.last_tick = time.monotonic()
        self.speed = 1.5

        self.set_draw_func(self._draw)
        self.add_tick_callback(self._on_tick)

    def set_speed(self, speed: float) -> None:
        self.speed = speed

    def _on_tick(self, _widget, _frame_clock) -> bool:
        now = time.monotonic()
        dt = now - self.last_tick
        self.last_tick = now
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

        pad = 6.0
        r = 22.0
        bx = pad
        by = pad
        bw = w - 2 * pad
        bh = h - 2 * pad

        cx = w / 2.0
        cy = h / 2.0
        t = self.phase

        # 1. Dark frosted card interior
        self._draw_rounded_rect(cr, bx, by, bw, bh, r)
        cr.set_source_rgba(0.06, 0.07, 0.12, 0.92)
        cr.fill()

        # 2. Outer chromatic glow halo (3 Apple Intelligence colors)
        cr.set_operator(cairo.OPERATOR_ADD)
        glow_dist = math.hypot(bw, bh) * 0.5
        x0 = cx + glow_dist * math.cos(t)
        y0 = cy + glow_dist * math.sin(t)
        x1 = cx - glow_dist * math.cos(t)
        y1 = cy - glow_dist * math.sin(t)

        grad = cairo.LinearGradient(x0, y0, x1, y1)
        grad.add_color_stop_rgba(0.00, 0.22, 0.74, 0.98, 0.40)  # Apple Cyan
        grad.add_color_stop_rgba(0.50, 0.66, 0.33, 0.97, 0.45)  # Apple Purple / Violet
        grad.add_color_stop_rgba(1.00, 0.93, 0.28, 0.60, 0.40)  # Apple Neon Magenta

        halo_width = 5.5 + 2.8 * math.sin(t * 2.2)
        self._draw_rounded_rect(cr, bx, by, bw, bh, r)
        cr.set_source(grad)
        cr.set_line_width(halo_width)
        cr.stroke()

        # 3. Dynamic pulsing Chroma-Ring border (3 Apple Intelligence colors)
        cr.set_operator(cairo.OPERATOR_OVER)
        ring_grad = cairo.LinearGradient(x0, y0, x1, y1)
        ring_grad.add_color_stop_rgba(0.00, 0.22, 0.74, 0.98, 0.95)  # Apple Cyan
        ring_grad.add_color_stop_rgba(0.50, 0.66, 0.33, 0.97, 0.95)  # Apple Purple / Violet
        ring_grad.add_color_stop_rgba(1.00, 0.93, 0.28, 0.60, 0.95)  # Apple Neon Magenta

        dyn_border_width = 1.6 + 1.2 * math.sin(t * 2.2)
        self._draw_rounded_rect(cr, bx, by, bw, bh, r)
        cr.set_source(ring_grad)
        cr.set_line_width(dyn_border_width)
        cr.stroke()


LUCIDE_MIC = b"""<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#e2e8f0" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" x2="12" y1="19" y2="22"/></svg>"""

LUCIDE_MIC_ACTIVE = b"""<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="#ffffff" stroke="none"><circle cx="12" cy="12" r="6"/></svg>"""

LUCIDE_SETTINGS = b"""<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#e2e8f0" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/></svg>"""

LUCIDE_X = b"""<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#e2e8f0" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>"""


def _lucide_icon(svg_bytes: bytes) -> Gtk.Widget:
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


class SayriCajita(Gtk.Overlay):
    """Apple-Intelligence style Cajita widget wrapped with animated Chroma-Ring."""

    def __init__(self, app) -> None:
        super().__init__()
        self.app = app
        self.add_css_class("sayri-cajita-container")
        self.set_size_request(420, 140)
        self.set_hexpand(False)
        self.set_vexpand(False)
        self.set_valign(Gtk.Align.CENTER)

        self._load_css()

        # Background: Animated Chroma-Ring
        self.chroma_bg = ChromaBackground(420, 140)
        self.set_child(self.chroma_bg)

        # Foreground Content Box
        content_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        content_box.set_margin_top(12)
        content_box.set_margin_bottom(12)
        content_box.set_margin_start(16)
        content_box.set_margin_end(16)
        content_box.set_hexpand(True)
        content_box.set_vexpand(True)

        # Transcript area (scrolled window)
        self.scroll = Gtk.ScrolledWindow()
        self.scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        self.scroll.set_vexpand(True)
        self.scroll.set_hexpand(True)

        self.messages_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        self.placeholder_label = Gtk.Label(label="Ask Sayri or click the orb to speak…")
        self.placeholder_label.add_css_class("sayri-placeholder")
        self.placeholder_label.set_halign(Gtk.Align.START)
        self.placeholder_label.set_valign(Gtk.Align.CENTER)
        self.messages_box.append(self.placeholder_label)

        self.scroll.set_child(self.messages_box)
        content_box.append(self.scroll)

        # Input row
        row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        row.set_hexpand(True)
        row.set_valign(Gtk.Align.END)

        # Mic button with Lucide Mic
        self.mic_btn = Gtk.Button()
        self.mic_btn.set_child(_lucide_icon(LUCIDE_MIC))
        self.mic_btn.add_css_class("sayri-btn")
        self.mic_btn.set_tooltip_text("Toggle Microphone")
        self.mic_btn.connect("clicked", lambda _b: self.app.toggle_listening())
        row.append(self.mic_btn)

        # Text input entry
        self.entry = Gtk.Entry()
        self.entry.add_css_class("sayri-input")
        self.entry.set_placeholder_text("Ask anything or speak…")
        self.entry.set_hexpand(True)
        self.entry.connect("activate", self._on_entry_activate)
        row.append(self.entry)

        # Settings button with Lucide Settings
        self.settings_btn = Gtk.Button()
        self.settings_btn.set_child(_lucide_icon(LUCIDE_SETTINGS))
        self.settings_btn.add_css_class("sayri-btn")
        self.settings_btn.set_tooltip_text("Sayri Settings")
        self.settings_btn.connect("clicked", lambda _b: self.app.open_settings())
        row.append(self.settings_btn)

        # Close button with Lucide X
        self.close_btn = Gtk.Button()
        self.close_btn.set_child(_lucide_icon(LUCIDE_X))
        self.close_btn.add_css_class("sayri-btn")
        self.close_btn.set_tooltip_text("Close")
        self.close_btn.connect("clicked", lambda _b: self.app.quit_app())
        row.append(self.close_btn)

        content_box.append(row)
        self.add_overlay(content_box)

        # State tracking
        self._partial_label: Gtk.Label | None = None
        self._assistant_label: Gtk.Label | None = None

    def _load_css(self) -> None:
        try:
            provider = Gtk.CssProvider()
            provider.load_from_data(CAJITA_CSS)
            Gtk.StyleContext.add_provider_for_display(
                Gdk.Display.get_default(), provider,
                Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
            )
        except Exception:  # noqa: BLE001
            pass

    def _on_entry_activate(self, entry: Gtk.Entry) -> None:
        text = entry.get_text().strip()
        if text:
            entry.set_text("")
            self.app.send_text(text)

    def set_content(self, kind: str, text: str) -> None:
        if not text:
            return

        if self.placeholder_label.get_parent() is not None:
            self.messages_box.remove(self.placeholder_label)

        if kind == "user":
            self._partial_label = None
            self._assistant_label = None
            lbl = Gtk.Label(label=text)
            lbl.add_css_class("sayri-line-user")
            lbl.set_halign(Gtk.Align.START)
            lbl.set_wrap(True)
            lbl.set_wrap_mode(Pango.WrapMode.WORD_CHAR)
            self.messages_box.append(lbl)
        elif kind == "assistant":
            if self._assistant_label is not None and self._assistant_label.get_parent() is not None:
                self._assistant_label.set_label(text)
            else:
                if self._partial_label is not None and self._partial_label.get_parent() is not None:
                    self.messages_box.remove(self._partial_label)
                    self._partial_label = None
                lbl = Gtk.Label(label=text)
                lbl.add_css_class("sayri-line-assistant")
                lbl.set_halign(Gtk.Align.START)
                lbl.set_wrap(True)
                lbl.set_wrap_mode(Pango.WrapMode.WORD_CHAR)
                lbl.set_selectable(True)
                self.messages_box.append(lbl)
                self._assistant_label = lbl
        elif kind == "partial":
            if self._partial_label is not None and self._partial_label.get_parent() is not None:
                self._partial_label.set_label(text)
            else:
                lbl = Gtk.Label(label=text)
                lbl.add_css_class("sayri-line-partial")
                lbl.set_halign(Gtk.Align.START)
                lbl.set_wrap(True)
                lbl.set_wrap_mode(Pango.WrapMode.WORD_CHAR)
                self.messages_box.append(lbl)
                self._partial_label = lbl
        elif kind in ("hint", "error"):
            lbl = Gtk.Label(label=text)
            lbl.add_css_class(f"sayri-line-{kind}")
            lbl.set_halign(Gtk.Align.START)
            lbl.set_wrap(True)
            lbl.set_wrap_mode(Pango.WrapMode.WORD_CHAR)
            self.messages_box.append(lbl)

        GLib.idle_add(self._scroll_to_bottom)

    def _scroll_to_bottom(self) -> None:
        adj = self.scroll.get_vadjustment()
        if adj:
            adj.set_value(adj.get_upper() - adj.get_page_size())

    def set_mic(self, active: bool) -> None:
        if active:
            self.mic_btn.set_child(_lucide_icon(LUCIDE_MIC_ACTIVE))
            self.mic_btn.add_css_class("sayri-btn-mic-active")
        else:
            self.mic_btn.set_child(_lucide_icon(LUCIDE_MIC))
            self.mic_btn.remove_css_class("sayri-btn-mic-active")

    def set_busy(self, busy: bool) -> None:
        self.entry.set_sensitive(not busy)
        if busy:
            self.chroma_bg.set_speed(3.5)
        else:
            self.chroma_bg.set_speed(1.5)

    def clear(self) -> None:
        self._partial_label = None
        self._assistant_label = None
        while child := self.messages_box.get_first_child():
            self.messages_box.remove(child)
        self.messages_box.append(self.placeholder_label)
