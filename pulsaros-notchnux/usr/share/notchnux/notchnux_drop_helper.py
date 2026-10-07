#!/usr/bin/env python3
"""NotchNux DnD helper for Wayland.

Mutter does not expose drops coming from other apps to shell extensions on
Wayland, so this small Gtk.Application provides a real window. Two modes
share that one window:

  * drop zone (default, --invisible): zero opacity, sized and placed by the
    extension exactly over the expanded notch (Meta.Window.move_resize_frame).
    The notch subtree is made non-reactive while an external drag crosses it,
    so Mutter's REACTIVE pick skips the shell chrome and lands on this window
    -- an *invisible* drop rectangle at the notch, which is what the notch UI
    promises. The Gtk.DropTarget payload is forwarded to the extension
    (StageFiles) and staged in the shelf.

  * drag-out card (org.freedesktop.Application.Open(files, hints)): the window
    becomes the drag source of a real Wayland drag: the extension's shelf row
    gesture sends Open with the 'start-drag' hint and the helper begins the
    drag with Gdk.Drag.begin (content = the staged files). A shell actor can
    never be a Wayland drag source, so this companion window is the only way
    to move staged files out with the pointer -- and because the drag is
    started programmatically, the shelf itself stays fully interactive.

Protocol with the extension:
  * started with --hidden (window built but never mapped, so the first drop
    pays no Python/GTK startup cost) and --invisible at enable();
  * an external drag crossing the notch calls org.freedesktop.Application.
    Activate: maps the zone and the extension moves/resizes it to the notch;
  * dragging a shelf row out calls org.freedesktop.Application.Open with the
    'start-drag' hint: the helper loads the content and starts the drag
    itself; it hides when the drag finishes;
  * the zone hides itself a few seconds after the last drag activity.

Deliberately minimal: no custom D-Bus registration, no proxies, no Adw -- those
paths segfaulted in this PyGObject/Gtk4 environment (see git history). The only
D-Bus usage is the outgoing StageFiles call_sync().
"""

import sys

import gi
gi.require_version('Gtk', '4.0')
gi.require_version('Gio', '2.0')
gi.require_version('Gdk', '4.0')
gi.require_version('Pango', '1.0')
from gi.repository import Gtk, Gio, GLib, Gdk, Pango

EXT_BUS_NAME = 'org.gnome.Shell'
EXT_PATH = '/org/gnome/Shell/Extensions/NotchNux'
EXT_IFACE = 'org.gnome.Shell.Extensions.NotchNux'

CALL_TIMEOUT_MS = 5000
HIDE_IDLE_MS = 4000       # hide after this long without drag activity
HIDE_AFTER_DROP_MS = 1400  # give the 'staged' feedback briefly after a drop
CARD_HIDE_MS = 8000       # a drag-out card lingers a bit longer under the pointer

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
    get_files = getattr(value, 'get_files', None)  # Gdk.FileList (GTK >= 4.14)
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
        # HANDLES_OPEN is required: without it GApplication rejects the
        # org.freedesktop.Application.Open used for the drag-out card
        # ("Application does not open files").
        super().__init__(application_id='es.pulsaros.NotchNuxHelper',
                         flags=Gio.ApplicationFlags.HANDLES_OPEN)
        # A warm (--hidden) cold start builds the window but leaves it unmapped;
        # the first Activate() from the extension then shows it.
        self._cold_hidden = warm_start
        self._invisible = invisible
        self._win = None
        self._zone_box = None
        self._card_box = None
        self._card_label = None
        self._label = None
        self._area = None
        self._drag_source = None
        self._drag_files = []
        self._hideId = 0

    def do_activate(self):
        if self._win is None:
            self._build_window()
        if self._cold_hidden:
            self._cold_hidden = False
            return
        self._show_zone(present=True)

    def do_open(self, files, *rest):
        # GApplication::open vfunc; rest carries n_files and the hint string
        # (GIO extracts only the a{sv} 'hint' key over D-Bus, not a dict).
        # The shelf stays fully interactive; drag-out begins here: the
        # extension sends 'start-drag' in the hint and the helper starts the
        # Gdk drag itself (see _begin_gdk_drag).
        if self._win is None:
            self._build_window()
        if self._cold_hidden:
            self._cold_hidden = False
        files = [f for f in files if isinstance(f, Gio.File)]
        hint = rest[1] if len(rest) > 1 else ''
        self._show_card(files)
        if hint and hint.startswith('start-drag'):
            self._begin_gdk_drag()

    def _build_window(self):
        self._win = Gtk.ApplicationWindow(application=self, title='NotchNux')
        self._win.set_default_size(360, 150)
        self._win.set_decorated(False)
        # Resizable on purpose: the extension sizes this window through
        # Meta.Window.move_resize_frame() to match the notch (zone) or the
        # pointer (drag-out card).
        self._win.set_resizable(True)
        if self._invisible:
            # 0.0 exacto = sin buffer visible => Mutter no lo trata como
            # target de drag (nunca llega enter). 0.01 es imperceptible pero
            # mantiene la ventana como destino DnD real.
            self._win.set_opacity(0.01)

        # -- zone child ----------------------------------------------------
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        box.set_margin_top(16)
        box.set_margin_bottom(16)
        box.set_margin_start(16)
        box.set_margin_end(16)

        self._label = Gtk.Label(label=DEFAULT_LABEL)
        self._label.set_wrap(True)
        box.append(self._label)

        self._zone_box = box

        target = Gtk.DropTarget.new(Gio.File, Gdk.DragAction.COPY)
        gtypes = [Gio.File]
        if hasattr(Gdk, 'FileList'):
            gtypes.insert(0, Gdk.FileList)  # multi-file drops
        try:
            target.set_gtypes(gtypes)
        except Exception:
            pass
        target.connect('drop', self._on_drop)
        target.connect('enter', self._on_enter)
        target.connect('motion', self._on_motion)
        target.connect('leave', self._on_leave)
        # The whole zone window is the drop surface: dragging to any point of
        # the rectangle stages the files.
        self._win.add_controller(target)

        # -- drag-out card child -------------------------------------------
        card = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        card.set_margin_top(12)
        card.set_margin_bottom(12)
        card.set_margin_start(16)
        card.set_margin_end(16)

        icon = Gtk.Image.new_from_icon_name('text-x-generic-symbolic')
        icon.set_pixel_size(28)
        icon.set_valign(Gtk.Align.CENTER)
        card.append(icon)

        self._card_label = Gtk.Label(label='')
        self._card_label.set_xalign(0.0)
        self._card_label.set_ellipsize(Pango.EllipsizeMode.MIDDLE)
        self._card_label.set_max_width_chars(28)
        self._card_label.set_hexpand(True)
        self._card_label.set_valign(Gtk.Align.CENTER)
        card.append(self._card_label)

        self._card_box = card

        self._drag_source = Gtk.DragSource.new()
        self._drag_source.set_actions(Gdk.DragAction.COPY)
        self._drag_source.connect('drag-begin', self._on_card_drag_begin)
        self._drag_source.connect('drag-end', self._on_card_drag_end)
        self._drag_source.connect('drag-cancel', self._on_card_drag_end)
        self._card_box.add_controller(self._drag_source)

        self._win.set_child(self._zone_box)

    # -- mode switching ----------------------------------------------------

    def _show_zone(self, present=True):
        if self._win.get_child() is not self._zone_box:
            self._win.set_child(self._zone_box)
        if self._invisible:
            # 0.0 exacto = sin buffer visible => Mutter no lo trata como
            # target de drag (nunca llega enter). 0.01 es imperceptible pero
            # mantiene la ventana como destino DnD real.
            self._win.set_opacity(0.01)
        if not present:
            return
        was_visible = self._win.get_visible()
        self._label.set_text(DEFAULT_LABEL)
        self._win.present()
        self._arm_hide(HIDE_IDLE_MS)
        if not was_visible:
            print('[NotchNux-Helper] drop zone shown', file=sys.stderr)

    def _show_card(self, files):
        self._drag_files = files
        if files:
            name = files[0].get_basename() if len(files) == 1 else None
            if name:
                self._card_label.set_text(f'Drag “{name}” into an app')
            else:
                self._card_label.set_text(f'Drag {len(files)} files into an app')
        else:
            self._card_label.set_text('Drag into an app')
        if self._win.get_child() is not self._card_box:
            self._win.set_child(self._card_box)
        # Every drag-out card is a transparent drag surface (imperceptible but
        # a real surface, like the drop zone): the visible drag-out card was a
        # gray rectangle behind the notch. Only the opaque drag icon below
        # tells the user what they are carrying.
        self._win.set_opacity(0.01 if self._invisible else 1.0)
        print(f'[NotchNux-Helper] drag-out card opacity {self._win.get_opacity():.2f}',
              file=sys.stderr)
        self._drag_source.set_content(self._make_file_provider(files))
        try:
            self._drag_source.set_icon(self._make_drag_icon(files))
        except Exception:
            pass
        was_visible = self._win.get_visible()
        self._win.present()
        self._arm_hide(CARD_HIDE_MS)
        if not was_visible:
            print(f'[NotchNux-Helper] drag-out card: {len(files)} file(s)',
                  file=sys.stderr)

    def _make_drag_icon(self, files):
        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        box.set_margin_top(6)
        box.set_margin_bottom(6)
        box.set_margin_start(10)
        box.set_margin_end(10)
        box.append(Gtk.Image.new_from_icon_name('document-send-symbolic'))
        box.append(Gtk.Label(label=f'{len(files)} file(s)'))
        return box

    def _make_file_provider(self, files):
        """text/uri-list is the universal file-drag mime (Nautilus, browsers,
        GTK apps all speak it); GDK deserializes it to a Gdk.FileList on the
        receiving side."""
        lines = ''.join((f.get_uri() or '') + '\r\n' for f in files)
        return Gdk.ContentProvider.new_for_bytes(
            'text/uri-list', GLib.Bytes.new(lines.encode('utf-8')))

    def _on_card_drag_begin(self, *_args):
        # Tell the shell this drag is OURS: the drag monitor must NOT treat it
        # as an external drop-in and swap this window back to the zone mode
        # mid-drag (that unmaps the drag source and cancels the drag).
        self._notify_companion_drag(True)
        print('[NotchNux-Helper] companion drag begin', file=sys.stderr)

    def _on_card_drag_end(self, *_args):
        # The drag finished (dropped elsewhere or cancelled): tell the shell
        # the card drag is over, revert to the (invisible) zone mode and hide
        # shortly.
        self._notify_companion_drag(False)
        self._show_zone(present=False)
        if self._win.get_visible():
            self._arm_hide(300)

    # -- programmatic drag-out --------------------------------------------

    def _begin_gdk_drag(self):
        # The shelf never stops being interactive: instead of grabbing a card
        # under the cursor, the extension's row gesture calls Open with the
        # 'start-drag' hint and the drag is begun *here*, from the helper
        # window, with Gdk.Drag.begin. That creates a real Wayland drag the
        # user can drop into any app, continuing their motion -- no re-grab,
        # no non-reactive chrome.
        try:
            display = self._win.get_display()
            seat = display.get_default_seat()
            device = seat.get_pointer()
            content = self._drag_source.get_content()
            if content is None:
                content = self._make_file_provider(self._drag_files)
            drag = Gdk.Drag.begin(self._win, device, content,
                                  Gdk.DragAction.COPY, 0, 0)
            if drag is None:
                raise RuntimeError('gdk_drag_begin returned None')
            drag.connect('dnd-finished', self._on_gdk_drag_finished)
            drag.connect('cancel', self._on_gdk_drag_finished)
            self._attach_drag_icon(drag)
            self._notify_companion_drag(True)
            print('[NotchNux-Helper] gdk drag begin', file=sys.stderr)
        except Exception as exc:
            print(f'[NotchNux-Helper] gdk drag begin failed: {exc}',
                  file=sys.stderr)
            self._notify_companion_drag(False)

    def _attach_drag_icon(self, drag):
        # Gtk.DragIcon is a separate surface that follows the pointer during
        # the drag and is destroyed when it ends; the window's own 0.01
        # opacity does not affect it, so the "N file(s)" hint stays visible.
        try:
            icon = Gtk.DragIcon.get_for_drag(drag)
            icon.set_child(self._make_drag_icon(self._drag_files))
        except Exception as exc:
            print(f'[NotchNux-Helper] drag icon failed: {exc}', file=sys.stderr)

    def _on_gdk_drag_finished(self, _drag):
        # Dropped into an app or cancelled: tell the shell the companion drag
        # is over, revert to the (invisible) zone mode and hide shortly.
        self._notify_companion_drag(False)
        self._show_zone(present=False)
        if self._win.get_visible():
            self._arm_hide(300)
        print('[NotchNux-Helper] gdk drag finished', file=sys.stderr)

    def _notify_companion_drag(self, active):
        try:
            if getattr(self, '_bus', None) is None:
                self._bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
            self._bus.call_sync(
                'org.gnome.Shell.Extensions.NotchNux',
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
        if self._win.get_visible():
            self._win.set_visible(False)
            print('[NotchNux-Helper] drop zone hidden', file=sys.stderr)
        return GLib.SOURCE_REMOVE

    # -- drag callbacks ----------------------------------------------------

    def _on_enter(self, target, x, y):
        # Only fires while a drag is in progress.
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
            # Distinguish "GTK handed us nothing" from "NotchNux unreachable".
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
    # argv is ours; do not hand the flags over to GApplication's parser.
    sys.exit(app.run([sys.argv[0]]))


if __name__ == '__main__':
    main()