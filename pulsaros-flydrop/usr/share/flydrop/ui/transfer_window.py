#!/usr/bin/env python3
"""
FlyDrop Transfer Window (GTK4 / Libadwaita)
Frameless, floating modal with a fine circular progress ring and completion checkmark.
Features smooth animated interpolation (ensuring small / fast files animate gracefully)
and reliable checkmark + auto-dismiss triggers.
"""

import sys
import os
import math
import json
import argparse
import subprocess

# Add project root to sys.path
BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if BASE_DIR not in sys.path:
    sys.path.insert(0, BASE_DIR)

import dbus
import dbus.mainloop.glib
dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)

import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
gi.require_version("GLib", "2.0")
gi.require_version("Gio", "2.0")
from gi.repository import Gtk, Adw, GLib, Gio, Gdk

from ui.i18n import _


class CircularProgressWidget(Gtk.DrawingArea):
    def __init__(self):
        super().__init__()
        self.set_size_request(190, 190)
        self.set_draw_func(self.on_draw)

        self.display_progress = 0.0
        self.target_progress = 0.0
        self.is_completed = False
        self.is_failed = False
        self.trigger_check_on_end = False
        self.check_animation_step = 0.0
        self.speed_text = ""

        self._anim_timer_id = None
        self._check_timer_id = None
        self._on_finish_callback = None

    def set_progress(self, val, speed_text=""):
        self.target_progress = max(0.0, min(1.0, float(val)))
        if speed_text:
            self.speed_text = speed_text
        self._start_animation_loop()

    def set_completed(self, success=True, on_finish=None):
        if on_finish:
            self._on_finish_callback = on_finish
        if success:
            self.trigger_check_on_end = True
            self.target_progress = 1.0
            if self.display_progress >= 0.999:
                self._start_check_animation()
            else:
                self._start_animation_loop()
        else:
            self.is_failed = True
            self.queue_draw()
            if self._on_finish_callback:
                cb = self._on_finish_callback
                self._on_finish_callback = None
                cb()

    def _start_animation_loop(self):
        if self._anim_timer_id is not None:
            return

        def step():
            diff = self.target_progress - self.display_progress
            if abs(diff) < 0.008:
                self.display_progress = self.target_progress
                self.queue_draw()
                self._anim_timer_id = None
                if self.display_progress >= 0.999 and self.trigger_check_on_end:
                    self._start_check_animation()
                return GLib.SOURCE_REMOVE

            step_val = max(0.04, abs(diff) * 0.25)
            if diff > 0:
                self.display_progress = min(self.target_progress, self.display_progress + step_val)
            else:
                self.display_progress = max(self.target_progress, self.display_progress - step_val)

            self.queue_draw()
            return GLib.SOURCE_CONTINUE

        self._anim_timer_id = GLib.timeout_add(16, step)

    def _start_check_animation(self):
        if self.is_completed and self.check_animation_step >= 1.0:
            return

        self.is_completed = True
        self.check_animation_step = 0.0

        if self._check_timer_id is not None:
            GLib.source_remove(self._check_timer_id)
            self._check_timer_id = None

        def step():
            self.check_animation_step += 0.08
            self.queue_draw()
            if self.check_animation_step < 1.0:
                return GLib.SOURCE_CONTINUE
            self.check_animation_step = 1.0
            self._check_timer_id = None
            if self._on_finish_callback:
                cb = self._on_finish_callback
                self._on_finish_callback = None
                cb()
            return GLib.SOURCE_REMOVE

        self._check_timer_id = GLib.timeout_add(16, step)

    def on_draw(self, area, cr, width, height):
        cx = width / 2.0
        cy = height / 2.0
        radius = min(cx, cy) - 16.0
        line_width = 4.5

        cr.set_line_cap(1)
        cr.set_line_join(1)

        # 1. Background Track
        cr.set_line_width(line_width)
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.12)
        cr.arc(cx, cy, radius, 0, 2 * math.pi)
        cr.stroke()

        # 2. Progress Arc
        if self.display_progress > 0.001 or self.is_completed or self.is_failed:
            start_angle = -math.pi / 2.0
            end_angle = start_angle + (2 * math.pi * self.display_progress)

            if self.is_completed:
                cr.set_source_rgba(0.188, 0.820, 0.345, 1.0)
            elif self.is_failed:
                cr.set_source_rgba(0.95, 0.25, 0.25, 1.0)
            else:
                cr.set_source_rgba(0.0, 0.478, 1.0, 1.0)

            cr.set_line_width(line_width)
            cr.arc(cx, cy, radius, start_angle, end_angle)
            cr.stroke()

        # 3. Center Indicator
        if self.is_completed:
            cr.set_source_rgba(0.188, 0.820, 0.345, 1.0)
            cr.set_line_width(5.0)

            p1 = (cx - 22, cy + 2)
            p2 = (cx - 7, cy + 17)
            p3 = (cx + 25, cy - 15)

            anim = self.check_animation_step
            if anim > 0:
                cr.move_to(p1[0], p1[1])
                if anim <= 0.4:
                    t = anim / 0.4
                    cr.line_to(p1[0] + (p2[0] - p1[0]) * t, p1[1] + (p2[1] - p1[1]) * t)
                else:
                    cr.line_to(p2[0], p2[1])
                    t = (anim - 0.4) / 0.6
                    cr.line_to(p2[0] + (p3[0] - p2[0]) * t, p2[1] + (p3[1] - p2[1]) * t)
                cr.stroke()

        elif self.is_failed:
            cr.set_source_rgba(0.95, 0.25, 0.25, 1.0)
            cr.set_line_width(4.5)
            cr.move_to(cx - 16, cy - 16)
            cr.line_to(cx + 16, cy + 16)
            cr.move_to(cx + 16, cy - 16)
            cr.line_to(cx - 16, cy + 16)
            cr.stroke()
        else:
            pct_text = f"{int(self.display_progress * 100)}%"
            cr.select_font_face("Sans", 0, 1)
            cr.set_font_size(26)
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.95)

            extents = cr.text_extents(pct_text)
            cr.move_to(cx - extents.width / 2.0 - extents.x_bearing, cy + extents.height / 2.0 - 4)
            cr.show_text(pct_text)

            if self.speed_text:
                cr.select_font_face("Sans", 0, 0)
                cr.set_font_size(11)
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.55)
                s_ext = cr.text_extents(self.speed_text)
                cr.move_to(cx - s_ext.width / 2.0 - s_ext.x_bearing, cy + 20)
                cr.show_text(self.speed_text)


class FlyDropTransferWindow(Gtk.Window):
    def __init__(self, app, session_id, peer_name, summary, mode="incoming", saved_file=""):
        super().__init__(application=app)
        self.session_id = session_id
        self.peer_name = peer_name
        self.summary = summary
        self.mode = mode
        self.saved_file = saved_file
        self.finished = False
        self._status_poll_id = None

        self.add_css_class("flydrop-window")
        self.set_decorated(False)
        self.set_resizable(False)
        self.set_default_size(320, 360)

        self._load_css()

        card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        card.add_css_class("flydrop-card")

        title_label = Gtk.Label()
        prefix = _("receiving") if mode == "incoming" else _("sending")
        title_label.set_markup(f"<span size='medium' weight='bold'>{prefix} {peer_name}</span>")
        title_label.set_ellipsize(3)
        card.append(title_label)

        self.subtitle_label = Gtk.Label(label=summary)
        self.subtitle_label.add_css_class("dim-label")
        self.subtitle_label.set_ellipsize(3)
        card.append(self.subtitle_label)

        self.progress_widget = CircularProgressWidget()
        card.append(self.progress_widget)

        self.status_label = Gtk.Label(label=summary)
        self.status_label.add_css_class("status-detail")
        self.status_label.set_ellipsize(3)
        card.append(self.status_label)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        btn_box.set_halign(Gtk.Align.CENTER)

        self.cancel_btn = Gtk.Button(label=_("cancel"))
        self.cancel_btn.add_css_class("cancel-button")
        self.cancel_btn.connect("clicked", lambda b: self.close())
        btn_box.append(self.cancel_btn)
        card.append(btn_box)

        self.set_child(card)

        self._setup_dbus_listener()
        GLib.idle_add(self._check_status)
        self._status_poll_id = GLib.timeout_add(400, self._check_status_timer)

    def _load_css(self):
        css_provider = Gtk.CssProvider()
        css = """
        window.flydrop-window, window.flydrop-window.background {
            background-color: transparent;
            background: none;
            box-shadow: none;
        }
        window.flydrop-window .flydrop-card {
            background: rgba(22, 22, 28, 0.96);
            border: 1px solid rgba(255, 255, 255, 0.16);
            border-radius: 28px;
            box-shadow: 0 20px 50px rgba(0, 0, 0, 0.65);
            padding: 24px;
            margin: 12px;
        }
        window.flydrop-window .dim-label {
            color: rgba(255, 255, 255, 0.65);
            font-size: 12px;
        }
        window.flydrop-window .status-detail {
            color: rgba(255, 255, 255, 0.85);
            font-size: 12px;
            font-weight: 500;
        }
        window.flydrop-window .cancel-button {
            background: rgba(255, 255, 255, 0.12);
            color: #ffffff;
            border-radius: 20px;
            padding: 6px 24px;
            font-size: 13px;
            font-weight: 600;
            border: none;
        }
        window.flydrop-window .cancel-button:hover {
            background: rgba(255, 255, 255, 0.22);
        }
        """
        css_provider.load_from_data(css.encode("utf-8"))
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(),
            css_provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

    def _check_status(self):
        if not self.session_id or self.finished:
            return
        try:
            bus = dbus.SessionBus()
            proxy = bus.get_object("es.pulsaros.FlyDrop", "/es/pulsaros/FlyDrop")
            iface = dbus.Interface(proxy, "es.pulsaros.FlyDrop")
            raw = iface.GetTransferStatus(self.session_id)
            status_data = json.loads(raw)
            if status_data.get("status") == "completed":
                self._finish_ui(True, status_data.get("message", ""))
            elif status_data.get("status") == "failed":
                self._finish_ui(False, status_data.get("message", "Error"))
            elif status_data.get("status") == "in_progress":
                prog = float(status_data.get("progress", 0.0))
                self.progress_widget.set_progress(prog, status_data.get("speed_str", ""))
                cur_file = status_data.get("current_file")
                if cur_file:
                    self.status_label.set_text(cur_file)
        except Exception:
            pass

    def _check_status_timer(self):
        if self.finished:
            return GLib.SOURCE_REMOVE
        self._check_status()
        return GLib.SOURCE_CONTINUE

    def _setup_dbus_listener(self):
        try:
            bus = dbus.SessionBus()
            bus.add_signal_receiver(
                self._on_transfer_progress,
                signal_name="TransferProgress",
                dbus_interface="es.pulsaros.FlyDrop"
            )
            bus.add_signal_receiver(
                self._on_transfer_completed,
                signal_name="TransferCompleted",
                dbus_interface="es.pulsaros.FlyDrop"
            )
        except Exception as e:
            print(f"Error setting up D-Bus listener: {e}", file=sys.stderr)

    def _on_transfer_progress(self, session_id, progress, speed_str, current_file, total_bytes_str):
        if session_id == self.session_id or not self.session_id:
            GLib.idle_add(self._update_progress_ui, progress, speed_str, current_file)

    def _update_progress_ui(self, progress, speed_str, current_file):
        if self.finished:
            return
        self.progress_widget.set_progress(progress, speed_str)
        if current_file:
            self.status_label.set_text(current_file)

    def _on_transfer_completed(self, session_id, success, message):
        if session_id == self.session_id or not self.session_id:
            GLib.idle_add(self._finish_ui, success, message)

    def _finish_ui(self, success, message):
        if self.finished:
            return
        self.finished = True
        if self._status_poll_id is not None:
            GLib.source_remove(self._status_poll_id)
            self._status_poll_id = None

        def on_anim_done():
            if success:
                self.status_label.set_text(_("completed"))
                self.cancel_btn.set_visible(False)
                GLib.timeout_add(1500, self.close)
            else:
                self.status_label.set_text(f"{_('failed')}: {message}")
                self.cancel_btn.set_label(_("cancel"))

        self.progress_widget.set_completed(success, on_finish=on_anim_done)


class FlyDropTransferApp(Adw.Application):
    def __init__(self, session_id, peer_name, summary, mode, saved_file=""):
        app_id_suffix = f"t{abs(hash(session_id)) % 100000}"
        super().__init__(application_id=f"es.pulsaros.FlyDrop.Transfer.{app_id_suffix}")
        self.session_id = session_id
        self.peer_name = peer_name
        self.summary = summary
        self.mode = mode
        self.saved_file = saved_file

    def do_activate(self):
        win = FlyDropTransferWindow(self, self.session_id, self.peer_name, self.summary, self.mode, self.saved_file)
        win.present()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--session", default="")
    parser.add_argument("--peer", default="Dispositivo")
    parser.add_argument("--summary", default="Transfiriendo...")
    parser.add_argument("--mode", default="incoming")
    parser.add_argument("--saved-file", default="")
    args, unknown = parser.parse_known_args()

    app = FlyDropTransferApp(args.session, args.peer, args.summary, args.mode, args.saved_file)
    app.run(sys.argv[:1])


if __name__ == "__main__":
    main()
