#!/usr/bin/env python3
"""Pulsar OS - Clipboard & Drop Island daemon.

Renders the island as a GTK4 layer-shell surface so it can:
  - receive native Wayland drops (files, text, images) from any app
  - start native drags of every held item toward any app

The GNOME Shell extension (pulsar-island@inled.es) monitors the clipboard
and the Screenshots folder and forwards items over D-Bus:
  bus:     org.pulsaros.Island
  path:    /org/pulsaros/Island
  iface:   org.pulsaros.Island
  methods: ShowItem(s kind, s payload), Ping() -> s
"""

import os
import sys
import tempfile
import time

import gi

gi.require_version('Gtk', '4.0')
gi.require_version('Gdk', '4.0')
from gi.repository import Gtk, Gdk, Gio, GLib

HAVE_LAYER = False
try:
    gi.require_version('Gtk4LayerShell', '1.0')
    from gi.repository import Gtk4LayerShell
    HAVE_LAYER = True
except (ValueError, ImportError):
    pass

BUS_NAME = 'org.pulsaros.Island'
OBJ_PATH = '/org/pulsaros/Island'
IFACE = 'org.pulsaros.Island'

ISLAND_IFACE_XML = f"""
<node>
  <interface name='{IFACE}'>
    <method name='ShowItem'>
      <arg type='s' name='kind' direction='in'/>
      <arg type='s' name='payload' direction='in'/>
    </method>
    <method name='Ping'>
      <arg type='s' name='pong' direction='out'/>
    </method>
  </interface>
</node>
"""

CSS = b"""
.island {
    background: rgba(20, 22, 28, 0.86);
    border-radius: 24px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    padding: 10px;
}
.island-drag {
    background: rgba(255, 255, 255, 0.05);
    border-radius: 18px;
    padding: 10px 12px;
    min-width: 150px;
    min-height: 48px;
}
.island-drag:hover {
    background: rgba(255, 255, 255, 0.10);
}
.island-label {
    color: rgba(235, 238, 244, 0.94);
    font-size: 12px;
}
.island-badge {
    color: #ffffff;
    background: rgba(90, 140, 255, 0.9);
    border-radius: 10px;
    padding: 2px 8px;
    font-size: 11px;
    font-weight: bold;
}
.island-close {
    color: rgba(235, 238, 244, 0.75);
    background: transparent;
    border: none;
    padding: 2px 6px;
    min-width: 20px;
}
.island-close:hover {
    background: rgba(255, 255, 255, 0.12);
    border-radius: 10px;
}
"""


class IslandWindow(Gtk.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app, default_width=210, default_height=76)
        self.set_decorated(False)
        self.set_resizable(False)
        self._items = []  # {kind: 'file'|'text', path/text}
        self._hide_id = 0
        self._auto_hide = int(os.environ.get('ISLAND_AUTO_HIDE', '12'))

        # Layer-shell init must happen before the window is mapped.
        if HAVE_LAYER:
            Gtk4LayerShell.init_for_window(self)
            Gtk4LayerShell.set_layer(self, Gtk4LayerShell.Layer.OVERLAY)
            Gtk4LayerShell.set_anchor(self, Gtk4LayerShell.Edge.TOP, True)
            Gtk4LayerShell.set_anchor(self, Gtk4LayerShell.Edge.RIGHT, True)
            Gtk4LayerShell.set_margin(self, Gtk4LayerShell.Edge.TOP, 40)
            Gtk4LayerShell.set_margin(self, Gtk4LayerShell.Edge.RIGHT, 12)
            Gtk4LayerShell.set_exclusive_zone(self, -1)

        self._build_ui()

        # Accept external drops: files (via FileList and single File), text
        # and images. One DropTarget per GType (PyGObject takes a single type).
        for gtype in (Gdk.FileList, Gio.File, str, Gdk.Texture):
            target = Gtk.DropTarget()
            target.set_gtypes([gtype])
            target.set_actions(Gdk.DragAction.COPY | Gdk.DragAction.MOVE)
            target.connect('drop', self._on_drop)
            self.add_controller(target)

    # UI -----------------------------------------------------------------

    def _build_ui(self):
        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        box.add_css_class('island')

        self._drag_area = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        self._drag_area.add_css_class('island-drag')
        self._drag_area.set_hexpand(True)

        self._icon = Gtk.Label(label='📋')
        self._icon.add_css_class('island-label')
        self._label = Gtk.Label(label='Suéltalo aquí')
        self._label.add_css_class('island-label')
        self._label.set_ellipsize(3)  # PANGO_ELLIPSIZE_END
        self._label.set_max_width_chars(24)
        self._label.set_hexpand(True)
        self._label.set_xalign(0.0)

        self._drag_area.append(self._icon)
        self._drag_area.append(self._label)

        self._badge = Gtk.Label(label='0')
        self._badge.add_css_class('island-badge')
        self._badge.set_visible(False)

        close = Gtk.Button(label='✕')
        close.add_css_class('island-close')
        close.connect('clicked', self._on_clear)

        box.append(self._drag_area)
        box.append(self._badge)
        box.append(close)
        self.set_child(box)

        # Drag out: takes every held item at once.
        source = Gtk.DragSource()
        source.set_actions(Gdk.DragAction.COPY | Gdk.DragAction.MOVE)
        source.connect('prepare', self._on_drag_prepare)
        source.connect('drag-begin', self._on_drag_begin)
        self._drag_area.add_controller(source)

        # Hover keeps the island alive; idle hides it.
        motion = Gtk.EventControllerMotion()
        motion.connect('enter', self._on_enter)
        motion.connect('leave', self._on_leave)
        self.add_controller(motion)

    # Items ----------------------------------------------------------------

    def add_item(self, kind, payload):
        self._items.append({'kind': kind, 'payload': payload})
        self._refresh()
        self.show_island()

    def _refresh(self):
        count = len(self._items)
        self._badge.set_text(str(count))
        self._badge.set_visible(count > 1)

        if count == 0:
            self._icon.set_text('📋')
            self._label.set_text('Suéltalo aquí')
            return

        last = self._items[-1]
        if last['kind'] == 'file':
            self._icon.set_text('📄')
            self._label.set_text(os.path.basename(last['payload']))
        elif last['kind'] == 'image':
            self._icon.set_text('🖼️')
            self._label.set_text(os.path.basename(last['payload']))
        else:
            self._icon.set_text('📝')
            text = (last['payload'] or '').strip().replace('\n', ' ')
            self._label.set_text(text[:48] + ('…' if len(text) > 48 else ''))

    # Visibility -------------------------------------------------------------

    def show_island(self):
        if self._hide_id:
            GLib.source_remove(self._hide_id)
            self._hide_id = 0
        self.present()
        self._schedule_hide()

    def _on_enter(self, *args):
        if self._hide_id:
            GLib.source_remove(self._hide_id)
            self._hide_id = 0

    def _on_leave(self, *args):
        self._schedule_hide()

    def _schedule_hide(self):
        if self._auto_hide <= 0:
            return
        if self._hide_id:
            GLib.source_remove(self._hide_id)
        self._hide_id = GLib.timeout_add_seconds(
            self._auto_hide, self._auto_hide_tick)

    def _auto_hide_tick(self):
        self._hide_id = 0
        self.hide()
        return GLib.SOURCE_REMOVE

    def _on_clear(self, button):
        self._items = []
        self._refresh()
        if self._hide_id:
            GLib.source_remove(self._hide_id)
            self._hide_id = 0
        self.hide()

    # Drops -------------------------------------------------------------------

    def _on_drop(self, target, value, x, y, data):
        added = 0
        if isinstance(value, Gdk.FileList):
            for f in value.get_files():
                self.add_item('file', f.get_path())
                added += 1
        elif isinstance(value, Gio.File):
            self.add_item('file', value.get_path())
            added += 1
        elif isinstance(value, Gdk.Texture):
            path = os.path.join(
                tempfile.gettempdir(), f'pulsar-island-drop-{int(time.time())}.png')
            value.save_to_png(path)
            self.add_item('image', path)
            added += 1
        elif isinstance(value, str):
            # May be a uri-list or plain text.
            stripped = value.strip()
            if '://' in stripped.split('\n')[0]:
                for uri in [u for u in stripped.split('\n') if u.strip()]:
                    f = Gio.File.new_for_uri(uri.strip())
                    self.add_item('file', f.get_path() or uri.strip())
                    added += 1
            else:
                self.add_item('text', value)
                added += 1
        return added > 0

    # Drag out -------------------------------------------------------------

    def _held_files(self):
        files = []
        for item in self._items:
            if item['kind'] in ('file', 'image') and os.path.exists(item['payload']):
                files.append(Gio.File.new_for_path(item['payload']))
        return files

    def _on_drag_prepare(self, source, x, y):
        files = self._held_files()
        texts = [i['payload'] for i in self._items if i['kind'] == 'text']

        providers = []
        if files:
            try:
                file_list = Gdk.FileList.new_from_files(files)
                providers.append(Gdk.ContentProvider.new_for_value(file_list))
            except Exception:
                providers.extend(
                    Gdk.ContentProvider.new_for_value(f) for f in files)
        if texts:
            providers.append(Gdk.ContentProvider.new_for_value(texts[-1]))

        if not providers:
            return None
        if len(providers) == 1:
            return providers[0]
        return Gdk.ContentProvider.new_union(providers)

    def _on_drag_begin(self, source, drag, *args):
        # Trash icon-ish feedback: use the last held file's icon if possible.
        files = self._held_files()
        paintable = None
        if files:
            icon = Gio.content_type_get_icon(
                Gio.File.query_info(
                    files[0], Gio.FILE_ATTRIBUTE_STANDARD_CONTENT_TYPE,
                    Gio.FileQueryInfoFlags.NONE).lookup_value(
                    Gio.FILE_ATTRIBUTE_STANDARD_CONTENT_TYPE, None)
                and Gio.File.new_for_path(files[0].get_path()).query_info(
                    Gio.FILE_ATTRIBUTE_STANDARD_ICON,
                    Gio.FileQueryInfoFlags.NONE, None).get_icon())
            try:
                themed = Gtk.IconTheme.get_for_display(Gdk.Display.get_default())
                paintable = themed.lookup_by_gicon(
                    icon, 48, 1, Gtk.TextDirection.NONE, 0).paintable
            except Exception:
                paintable = None
        if paintable:
            source.set_icon(paintable, 24, 24)


ISLAND_DBUS_VTABLE = (
    {
        'method': 'ShowItem',
        'args': None,
    },
)


class IslandService:
    def __init__(self, window):
        self._window = window
        self._bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
        node = Gio.DBusNodeInfo.new_for_xml(ISLAND_IFACE_XML)
        self._reg_id = self._bus.register_object(
            OBJ_PATH, node.interfaces[0], self._handle_call, None, None)
        self._own_id = Gio.bus_own_name(
            Gio.BusType.SESSION, BUS_NAME, Gio.BusNameOwnerFlags.NONE,
            None, self._on_name_acquired, self._on_name_lost)

    def _on_name_acquired(self, connection, name):
        print(f'{name} ready', flush=True)

    def _on_name_lost(self, connection, name):
        print(f'{name} lost, exiting', flush=True)
        sys.exit(1)

    def _handle_call(self, connection, sender, path, iface, method, params, invocation):
        if method == 'Ping':
            invocation.return_value(GLib.Variant('(s)', ['pong']))
            return
        if method == 'ShowItem':
            kind, payload = params.unpack()
            GLib.idle_add(self._window.add_item, kind, payload)
            invocation.return_value(None)
            return
        invocation.return_error_literal(
            Gio.DBusError, Gio.DBusError.UNKNOWN_METHOD, 'Unknown method')


class IslandApp(Gtk.Application):
    def __init__(self):
        super().__init__(application_id='es.inled.pulsaros.Island')

    def do_activate(self):
        win = IslandWindow(self)
        self._win = win
        IslandService(win)
        self.hold()
        # Unless launched with --show, keep the island hidden until content.
        if '--show' in sys.argv:
            win.show_island()


def load_css():
    provider = Gtk.CssProvider()
    provider.load_from_data(CSS)
    Gtk.StyleContext.add_provider_for_display(
        Gdk.Display.get_default(), provider,
        Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)


def main():
    app = IslandApp()
    GLib.set_application_name('Pulsar Island')
    app.connect('activate', lambda a: load_css())
    try:
        sys.exit(app.run(sys.argv))
    except KeyboardInterrupt:
        sys.exit(0)


if __name__ == '__main__':
    main()
