#!/usr/bin/env python3
"""NotchNux DnD helper for Wayland.

Mutter does not expose drops coming from other apps to shell extensions on
Wayland, so this small Gtk.Application provides a real window. Two modes
share that one window:

  * drop zone (default, --invisible): zero/near-zero opacity, sized and placed
    by the extension exactly over the expanded notch (Meta.Window.move_resize_frame).
    The notch subtree is made non-reactive while an external drag crosses it,
    so Mutter's REACTIVE pick skips the shell chrome and lands on this window
    -- an invisible drop rectangle at the notch. The Gtk.DropTarget payload
    is forwarded to the extension (StageFiles) and staged in the shelf.

  * drag-out card (org.freedesktop.Application.Open([notchnux://card, uris])):
    the window becomes a transparent drag surface covering the entire notch
    rectangle carrying a Gtk.DragSource with the shelf file(s). Pressing and
    dragging anywhere on the notch performs a real Wayland drag in 1 single gesture.
    Clicks and scrolls are forwarded back to GNOME Shell so the notch remains
    100% interactive.
"""

import sys

import gi
gi.require_version('Gtk', '4.0')
gi.require_version('Gio', '2.0')
gi.require_version('Gdk', '4.0')
gi.require_version('Pango', '1.0')
from gi.repository import Gtk, Gio, GLib, Gdk, GObject, Pango

EXT_BUS_NAME = 'org.gnome.Shell'
EXT_PATH = '/org/gnome/Shell/Extensions/NotchNux'
EXT_IFACE = 'org.gnome.Shell.Extensions.NotchNux'

CALL_TIMEOUT_MS = 5000
HIDE_IDLE_MS = 4000       # hide drop zone after this long without drag activity
HIDE_AFTER_DROP_MS = 1400  # give feedback briefly after a drop

DEFAULT_LABEL = 'Drop files here to add them to the Shelf'


def stage_uris(uris):
    """Forward file URIs to the extension. Returns True when it replied."""
    if not uris:
        return False
    try:
        conn = Gio.bus_get_sync(Gio.BusType.SESSION, None)
        conn.call_sync(
            EXT_BUS_NAME, EXT_PATH, EXT_IFACE, 'StageFiles',
            GLib.Variant('(as)', [list(uris)]),
            None, Gio.DBusCallFlags.NONE, CALL_TIMEOUT_MS, None)
        return True
    except Exception as exc:
        print(f'[NotchNux-Helper] StageFiles failed: {exc}', file=sys.stderr)
        return False


def files_from_value(value):
    """Normalise a Gtk.DropTarget drop value into a list of Gio.File."""
    if value is None:
        return []
    if isinstance(value, Gio.File):
        return [value]
    get_files = getattr(value, 'get_files', None)
    if callable(get_files):
        try:
            return list(get_files())
        except Exception:
            return []
    try:
        return [v for v in value if isinstance(v, Gio.File)]
    except TypeError:
        return []


class DropApp(Gtk.Application):
    def __init__(self, warm_start=False, invisible=False):
        super().__init__(application_id='es.pulsaros.NotchNuxHelper',
                         flags=Gio.ApplicationFlags.HANDLES_OPEN)
        self._cold_hidden = warm_start
        self._invisible = invisible
        self._win = None
        self._zone_box = None
        self._label = None
        self._drag_source = None
        self._drag_files = []
        self._drag_active = False
        self._hideId = 0
        self._bus = None

    def do_activate(self):
        if self._win is None:
            self._build_window()
        if self._cold_hidden:
            self._cold_hidden = False
            return
        self._show_zone(present=True)

    def do_open(self, files, *rest):
        if self._win is None:
            self._build_window()
        if self._cold_hidden:
            self._cold_hidden = False
        cmd = None
        w = 0
        h = 0
        real = []
        for f in files:
            if not isinstance(f, Gio.File):
                continue
            uri = f.get_uri() or ''
            if uri.startswith('notchnux:'):
                cmd = uri
                if '?w=' in uri or '&w=' in uri:
                    try:
                        query = uri.split('?')[1] if '?' in uri else ''
                        params = dict(p.split('=') for p in query.split('&') if '=' in p)
                        w = int(params.get('w', 0))
                        h = int(params.get('h', 0))
                    except Exception:
                        pass
                continue
            real.append(f)
        if cmd and 'hide' in cmd:
            self._cancel_hide()
            self._drag_files = []
            if self._win.get_visible():
                self._win.set_visible(False)
                print('[NotchNux-Helper] drag card hidden', file=sys.stderr)
            return
        if cmd and 'zone' in cmd:
            self._show_zone(present=True, w=w, h=h)
            return
        self._show_card(real, w=w, h=h)

    def _build_window(self):
        css_provider = Gtk.CssProvider()
        css_provider.load_from_data(b"""
        .notch-drag-window {
            background-color: transparent;
        }
        .notch-drag-icon-box {
            background-color: rgba(28, 28, 32, 0.96);
            border: 1px solid rgba(255, 255, 255, 0.25);
            border-radius: 14px;
            padding: 8px 16px;
            box-shadow: 0 6px 20px rgba(0, 0, 0, 0.6);
        }
        .notch-drag-icon-box label {
            font-size: 13px;
            font-weight: 600;
            color: #ffffff;
        }
        """)
        disp = Gdk.Display.get_default()
        if disp:
            Gtk.StyleContext.add_provider_for_display(
                disp, css_provider, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)

        self._win = Gtk.ApplicationWindow(application=self, title='NotchNux')
        self._win.set_default_size(600, 300)
        self._win.set_size_request(600, 300)
        self._win.set_decorated(False)
        self._win.set_resizable(True)
        self._win.add_css_class('notch-drag-window')
        self._win.set_opacity(0.01)

        # -- zone child: fills the entire window --
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        box.set_hexpand(True)
        box.set_vexpand(True)
        box.set_halign(Gtk.Align.FILL)
        box.set_valign(Gtk.Align.FILL)
        box.set_size_request(600, 300)

        self._label = Gtk.Label(label=DEFAULT_LABEL)
        self._label.set_wrap(True)
        self._label.set_valign(Gtk.Align.CENTER)
        self._label.set_hexpand(True)
        self._label.set_vexpand(True)
        box.append(self._label)

        self._zone_box = box

        # Drop target for incoming external drops (ALWAYS active!)
        target = Gtk.DropTarget.new(Gio.File, Gdk.DragAction.COPY)
        gtypes = [Gio.File]
        if hasattr(Gdk, 'FileList'):
            gtypes.insert(0, Gdk.FileList)
        try:
            target.set_gtypes(gtypes)
        except Exception:
            pass
        target.connect('drop', self._on_drop)
        target.connect('enter', self._on_enter)
        target.connect('motion', self._on_motion)
        target.connect('leave', self._on_leave)
        self._win.add_controller(target)

        # Drag source for drag out - attached to the WINDOW directly
        self._drag_source = Gtk.DragSource.new()
        self._drag_source.set_actions(Gdk.DragAction.COPY)
        self._drag_source.connect('drag-begin', self._on_card_drag_begin)
        self._drag_source.connect('drag-end', self._on_card_drag_end)
        self._drag_source.connect('drag-cancel', self._on_card_drag_end)
        self._win.add_controller(self._drag_source)

        # Click gesture: when the user clicks without dragging, forward the click to GNOME Shell
        click = Gtk.GestureClick.new()
        click.set_button(0)
        click.connect('released', self._on_click_released)
        self._win.add_controller(click)

        # Scroll controller: forward wheel scrolling to GNOME Shell shelf list
        scroll = Gtk.EventControllerScroll.new(
            Gtk.EventControllerScrollFlags.VERTICAL | Gtk.EventControllerScrollFlags.SMOOTH)
        scroll.connect('scroll', self._on_scroll)
        self._win.add_controller(scroll)

        self._win.set_child(self._zone_box)

    # -- mode switching ----------------------------------------------------

    def _show_zone(self, present=True, w=0, h=0):
        self._win.set_opacity(0.01)
        if w > 50 and h > 50:
            self._win.set_size_request(w, h)
            self._win.set_default_size(w, h)
            if self._zone_box:
                self._zone_box.set_size_request(w, h)
        if not present:
            return
        was_visible = self._win.get_visible()
        self._label.set_text(DEFAULT_LABEL)
        self._win.present()
        self._arm_hide(HIDE_IDLE_MS)
        if not was_visible:
            print(f'[NotchNux-Helper] drop zone shown ({w}x{h})', file=sys.stderr)

    def _show_card(self, files, w=0, h=0):
        self._drag_files = files
        self._win.set_opacity(0.01)
        if w > 50 and h > 50:
            self._win.set_size_request(w, h)
            self._win.set_default_size(w, h)
            if self._zone_box:
                self._zone_box.set_size_request(w, h)
        provider = self._make_file_provider(files)
        self._drag_source.set_content(provider)
        self._cancel_hide()
        was_visible = self._win.get_visible()
        self._win.present()
        print(f'[NotchNux-Helper] drag-out active for {len(files)} file(s), size={w}x{h}', file=sys.stderr)

    def _make_drag_icon(self, files):
        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        box.add_css_class('notch-drag-icon-box')
        box.append(Gtk.Image.new_from_icon_name('document-send-symbolic'))
        count = len(files)
        if count == 1:
            name = files[0].get_basename()
            lbl = Gtk.Label(label=name or '1 file')
            lbl.set_max_width_chars(28)
            lbl.set_ellipsize(Pango.EllipsizeMode.MIDDLE)
            box.append(lbl)
        else:
            box.append(Gtk.Label(label=f'{count} files'))
        return box

    def _make_file_provider(self, files):
        providers = []
        # Gdk.FileList for GTK4/GNOME apps (Nautilus, etc.)
        if hasattr(Gdk, 'FileList'):
            try:
                val = GObject.Value()
                val.init(Gdk.FileList)
                fl = Gdk.FileList.new_from_list(files)
                val.set_boxed(fl)
                providers.append(Gdk.ContentProvider.new_for_value(val))
            except Exception as e:
                print(f'[NotchNux-Helper] Gdk.FileList error: {e}', file=sys.stderr)
        # text/uri-list for universal file drop (browsers, editors, terminals)
        try:
            lines = ''.join((f.get_uri() or '') + '\r\n' for f in files)
            b = GLib.Bytes.new(lines.encode('utf-8'))
            providers.append(Gdk.ContentProvider.new_for_bytes('text/uri-list', b))
        except Exception as e:
            print(f'[NotchNux-Helper] uri-list error: {e}', file=sys.stderr)

        if len(providers) > 1:
            return Gdk.ContentProvider.new_union(providers)
        elif len(providers) == 1:
            return providers[0]
        return None

    def _on_click_released(self, gesture, n_press, x, y):
        if self._drag_active:
            return
        btn = gesture.get_current_button()
        self._notify_click(x, y, btn)

    def _on_scroll(self, controller, dx, dy):
        self._notify_scroll(dx, dy)

    def _on_card_drag_begin(self, drag_source, drag):
        if not self._drag_files:
            return
        self._drag_active = True
        try:
            icon = Gtk.DragIcon.get_for_drag(drag)
            icon.set_child(self._make_drag_icon(self._drag_files))
        except Exception as exc:
            print(f'[NotchNux-Helper] drag icon error: {exc}', file=sys.stderr)
        self._notify_companion_drag(True)
        print('[NotchNux-Helper] companion drag begin', file=sys.stderr)

    def _on_card_drag_end(self, *_args):
        GLib.timeout_add(100, self._reset_drag_active)
        self._notify_companion_drag(False)
        print('[NotchNux-Helper] companion drag end', file=sys.stderr)

    def _reset_drag_active(self):
        self._drag_active = False
        return GLib.SOURCE_REMOVE

    def _notify_click(self, x, y, button):
        try:
            if self._bus is None:
                self._bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
            self._bus.call(
                'org.gnome.Shell',
                '/org/gnome/Shell/Extensions/NotchNux',
                'org.gnome.Shell.Extensions.NotchNux',
                'ForwardClick',
                GLib.Variant('(iid)', (int(x), int(y), float(button))), None,
                Gio.DBusCallFlags.NONE, 1000, None, None)
        except Exception as e:
            print(f'[NotchNux-Helper] click notify failed: {e}', file=sys.stderr)

    def _notify_scroll(self, dx, dy):
        try:
            if self._bus is None:
                self._bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
            self._bus.call(
                'org.gnome.Shell',
                '/org/gnome/Shell/Extensions/NotchNux',
                'org.gnome.Shell.Extensions.NotchNux',
                'ForwardScroll',
                GLib.Variant('(dd)', (float(dx), float(dy))), None,
                Gio.DBusCallFlags.NONE, 1000, None, None)
        except Exception as e:
            pass

    def _notify_companion_drag(self, active):
        try:
            if self._bus is None:
                self._bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
            self._bus.call_sync(
                'org.gnome.Shell',
                '/org/gnome/Shell/Extensions/NotchNux',
                'org.gnome.Shell.Extensions.NotchNux',
                'CompanionDragState',
                GLib.Variant('(b)', (bool(active),)), None,
                Gio.DBusCallFlags.NONE, 2000, None)
        except Exception as e:
            print(f'[NotchNux-Helper] drag state notify failed: {e}',
                  file=sys.stderr)

    # -- visibility --------------------------------------------------------

    def _arm_hide(self, ms):
        self._cancel_hide()
        self._hideId = GLib.timeout_add(ms, self._on_hide_timeout)

    def _cancel_hide(self):
        if self._hideId:
            GLib.source_remove(self._hideId)
            self._hideId = 0

    def _on_hide_timeout(self):
        self._hideId = 0
        if self._win.get_visible() and not self._drag_files:
            self._win.set_visible(False)
            print('[NotchNux-Helper] drop zone hidden by timeout', file=sys.stderr)
        return GLib.SOURCE_REMOVE

    # -- drag callbacks (drop-in) ------------------------------------------

    def _on_enter(self, target, x, y):
        print(f'[NotchNux-Helper] ZONE-ENTER x={x:.0f} y={y:.0f}', file=sys.stderr)
        self._label.set_text('Release to stage the files')
        self._arm_hide(HIDE_IDLE_MS)
        return Gdk.DragAction.COPY

    def _on_motion(self, target, x, y):
        self._arm_hide(HIDE_IDLE_MS)
        return Gdk.DragAction.COPY

    def _on_leave(self, target):
        self._label.set_text(DEFAULT_LABEL)
        self._arm_hide(HIDE_IDLE_MS)

    def _on_drop(self, target, value, x, y):
        uris = [f.get_uri() for f in files_from_value(value)]
        print(f'[NotchNux-Helper] drop: {len(uris)} file(s), '
              f'value={type(value).__name__}', file=sys.stderr)
        if not uris:
            self._label.set_text('Nothing to stage')
            self._arm_hide(HIDE_AFTER_DROP_MS)
            return True
        ok = stage_uris(uris)
        if ok:
            print(f'[NotchNux-Helper] staged {len(uris)} file(s)', file=sys.stderr)
            self._label.set_text(f'Staged {len(uris)} file(s) in the Shelf')
        else:
            self._label.set_text('Could not reach NotchNux')
        self._arm_hide(HIDE_AFTER_DROP_MS)
        return True


def main():
    args = sys.argv[1:]
    warm = '--hidden' in args
    invisible = '--invisible' in args
    app = DropApp(warm_start=warm, invisible=invisible)
    sys.exit(app.run([sys.argv[0]]))


if __name__ == '__main__':
    main()