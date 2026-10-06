#!/usr/bin/env python3
import gi
gi.require_version('Gtk','4.0'); gi.require_version('Gio','2.0'); gi.require_version('Gdk','4.0')
from gi.repository import Gtk,Gio,Gdk,GLib
import sys

class App(Gtk.Application):
    def __init__(self):
        super().__init__(application_id='es.pulsaros.NotchNuxHelper', flags=0)
        self._win = None
    def do_activate(self):
        if self._win is None:
            self._win = Gtk.ApplicationWindow(application=self)
            self._win.set_decorated(False)
            self._win.set_default_size(360,160)
            box = Gtk.Box()
            self._win.set_child(box)
            tgt = Gtk.DropTarget.new(Gio.File, Gdk.DragAction.COPY)
            box.add_controller(tgt)
            self._win.set_visible(False)
        self._win.present()

if __name__=='__main__':
    sys.exit(App().run(sys.argv))
