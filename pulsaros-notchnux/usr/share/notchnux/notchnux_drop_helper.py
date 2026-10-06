#!/usr/bin/env python3
"""
NotchNux DnD helper for Wayland

Provides a Gtk.Application with a Gtk.DropTarget to receive files from Nautilus
(and other GTK apps) and a Gtk.DragSource to drag files out to other apps.
Receives URIs via D-Bus from gnome-shell extension when requested, or can
be controlled via a simple D-Bus service.

This is a workaround: Mutter's xdndHandler on Wayland does not expose drop
events to shell extensions, so we need a real GTK window/overlay to accept
drops.
"""
import os
import sys
import json
import logging
import threading

import gi
gi.require_version('Gtk', '4.0')
gi.require_version('Gio', '2.0')
gi.require_version('GLib', '2.0')
gi.require_version('Adw', '1')

from gi.repository import Gtk, Gio, GLib, Adw, Gdk

logging.basicConfig(level=logging.INFO, format='[NotchNux-Helper] %(levelname)s: %(message)s')
logger = logging.getLogger('notchnux-helper')

BUS_NAME = 'es.pulsaros.NotchNuxHelper'
BUS_PATH = '/es/pulsaros/NotchNuxHelper'
IFACE_NAME = 'es.pulsaros.NotchNuxHelper'

# Extension bus (to tell extension to stage files)
EXT_BUS_NAME = 'org.gnome.Shell'
EXT_PATH = '/org/gnome/Shell/Extensions/NotchNux'
EXT_IFACE = 'org.gnome.Shell.Extensions.NotchNux'


class HelperDBus(Gio.Application):
    def __init__(self):
        super().__init__(application_id='es.pulsaros.NotchNuxHelper',
                         flags=Gio.ApplicationFlags.FLAGS_NONE)
        self._bus_id = 0
        self._window = None
        self._drop_area = None
        self._target = None
        self._staged_uris = []
        self._ext_proxy = None

    def do_startup(self):
        Adw.Application.do_startup(self)
        # Try to connect to extension DBus if it exposes staging
        self._connect_ext_proxy()

    def do_activate(self):
        if self._window is None:
            self._create_window()
        self._window.present()
        self._window.set_visible(False)  # keep hidden by default, show on drop hover

    def _create_window(self):
        self._window = Gtk.ApplicationWindow(application=self, title='NotchNux Drop Helper')
        self._window.set_default_size(400, 200)
        self._window.set_decorated(False)
        self._window.set_resizable(False)
        self._window.set_modal(False)
        self._window.set_transient_for(None)
        self._window.set_keep_above(True)
        self._window.set_can_focus(False)
        self._window.set_opacity(0.85)

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        box.set_margin_top(20)
        box.set_margin_bottom(20)
        box.set_margin_start(20)
        box.set_margin_end(20)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)

        label = Gtk.Label(label='Drop files to add to NotchNux shelf')
        box.append(label)

        self._drop_area = Gtk.Box()
        self._drop_area.set_size_request(360, 160)
        self._drop_area.set_hexpand(True)
        self._drop_area.set_vexpand(True)
        box.append(self._drop_area)

        self._window.set_child(box)

        # Setup drop target
        self._target = Gtk.DropTarget.new(Gio.File, Gdk.DragAction.COPY)
        self._target.connect('drop', self.on_drop)
        self._target.connect('enter', self.on_enter)
        self._target.connect('leave', self.on_leave)
        self._drop_area.add_controller(self._target)

        # Initially hide
        self._window.set_visible(False)

    def _connect_ext_proxy(self):
        try:
            self._ext_proxy = Gio.DBusProxy.new_for_bus_sync(
                Gio.BusType.SESSION,
                Gio.DBusProxyFlags.DO_NOT_CONNECT_SIGNALS,
                None,
                EXT_BUS_NAME,
                EXT_PATH,
                EXT_IFACE,
                None)
        except Exception as e:
            logger.debug(f'Could not connect to extension proxy: {e}')

    def stage_uris_via_proxy(self, uris):
        if not uris:
            return False
        if self._ext_proxy:
            try:
                self._ext_proxy.call_sync('StageFiles',
                                         GLib.Variant('(ass)', (uris,)),
                                         Gio.DBusCallFlags.NONE,
                                         -1,
                                         None)
                return True
            except Exception as e:
                logger.warning(f'Failed to call StageFiles: {e}')
        # Fallback: write to temp file and notify
        try:
            path = os.path.expanduser('~/.local/share/notchnux/helper-staged.json')
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, 'w') as f:
                json.dump({'uris': uris}, f)
            return True
        except Exception as e:
            logger.error(f'Fallback staging failed: {e}')
            return False

    def on_enter(self, target, x, y):
        self._window.set_visible(True)
        self._window.present()
        return Gdk.DragAction.COPY

    def on_leave(self, target):
        # Small delay to allow moving between drop_area and window
        GLib.timeout_add(200, self._maybe_hide)
        return True

    def _maybe_hide(self):
        # Keep visible if we are dragging over
        if self._window.get_visible():
            pass  # don't hide immediately
        return GLib.SOURCE_REMOVE

    def on_drop(self, target, value, x, y):
        uris = []
        if isinstance(value, Gio.File):
            uris.append(value.get_uri())
        elif hasattr(value, '__iter__'):
            for v in value:
                if isinstance(v, Gio.File):
                    uris.append(v.get_uri())
        if uris:
            self.stage_uris_via_proxy(uris)
        self._window.set_visible(False)
        return True

    def do_dbus_register(self, connection, object_path):
        info = Gio.DBusNodeInfo.new_for_xml("""
<node>
  <interface name="es.pulsaros.NotchNuxHelper">
    <method name="ShowDropZone">
      <arg type="b" direction="in" name="show"/>
    </method>
    <method name="StageFiles">
      <arg type="as" direction="in" name="uris"/>
      <arg type="b" direction="out" name="result"/>
    </method>
    <method name="Exit"/>
  </interface>
</node>
""")
        self._bus_id = connection.register_object(object_path, info.interfaces[0],
                                                  self.handle_method_call,
                                                  self.handle_get_property,
                                                  self.handle_set_property)
        return Gio.Application.do_dbus_register(self, connection, object_path)

    def do_dbus_unregister(self, connection, object_path):
        if self._bus_id > 0:
            connection.unregister_object(self._bus_id)
            self._bus_id = 0
        Gio.Application.do_dbus_unregister(self, connection, object_path)

    def handle_method_call(self, connection, sender, object_path, interface_name,
                           method_name, parameters, invocation):
        if method_name == 'ShowDropZone':
            show = parameters.unpack()[0]
            if show:
                if self._window is None:
                    self._create_window()
                self._window.present()
                self._window.set_visible(True)
            else:
                if self._window:
                    self._window.set_visible(False)
            invocation.return_value(GLib.Variant('()'))
        elif method_name == 'StageFiles':
            uris = parameters.unpack()[0]
            res = self.stage_uris_via_proxy(uris)
            invocation.return_value(GLib.Variant('(b)', (res,)))
        elif method_name == 'Exit':
            self.quit()
            invocation.return_value(GLib.Variant('()'))
        else:
            invocation.return_error_literal(Gio.dbus_error_quark(),
                                            Gio.DBUS_ERROR_UNKNOWN_METHOD,
                                            'Unknown method')

    def handle_get_property(self, connection, sender, object_path, interface_name,
                            property_name):
        return None

    def handle_set_property(self, connection, sender, object_path, interface_name,
                            property_name, value):
        return False


def main():
    app = HelperDBus()
    exit_status = app.run(sys.argv)
    sys.exit(exit_status)


if __name__ == '__main__':
    main()
