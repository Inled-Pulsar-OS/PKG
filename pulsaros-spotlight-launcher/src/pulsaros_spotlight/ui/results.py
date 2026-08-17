"""Result rendering widgets for PulsarOS Spotlight."""

from __future__ import annotations

import ast
import logging
import os
import subprocess
from pathlib import Path
from typing import Callable

import gi
gi.require_version("Gdk", "4.0")
gi.require_version("GdkPixbuf", "2.0")
gi.require_version("Gtk", "4.0")
from gi.repository import Gdk, GdkPixbuf, Gio, GLib, Gtk

from pulsaros_spotlight.search import SearchResult
from pulsaros_spotlight.utils import get_file_icon

logger = logging.getLogger(__name__)

_IMAGE_EXTS = frozenset({
    ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp",
    ".tiff", ".tif", ".ico", ".avif", ".heic",
})


def _app_icon_image(icon_name: str) -> Gtk.Image | None:
    """Build a Gtk.Image from a .desktop Icon value (name or absolute path)."""
    if not icon_name:
        return None
    icon_name = icon_name.strip()
    if icon_name.startswith("/") and os.path.isfile(icon_name):
        try:
            return Gtk.Image.new_from_file(icon_name)
        except Exception:
            pass
    try:
        return Gtk.Image.new_from_icon_name(icon_name)
    except Exception:
        return None


def _is_image_file(result: SearchResult) -> bool:
    """Check if a result is an image file that should show a thumbnail."""
    if not result.url.startswith("file://"):
        return False
    path = result.url.removeprefix("file://")
    if not os.path.isfile(path):
        return False
    mime = result.mime or ""
    if mime.startswith("image/"):
        return True
    return Path(path).suffix.lower() in _IMAGE_EXTS


def _build_icon_image(result: SearchResult, pixel_size: int, css_class: str) -> Gtk.Widget:
    """Return the best icon for a result, with thumbnail for images."""
    if _is_image_file(result):
        path = result.url.removeprefix("file://")
        try:
            # Pre-scale with GdkPixbuf so texture is already exact pixel_size
            pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(
                path, pixel_size, pixel_size, True  # preserve_aspect_ratio=True
            )
            texture = Gdk.Texture.new_for_pixbuf(pixbuf)
            image = Gtk.Image.new_from_paintable(texture)
            image.set_pixel_size(pixel_size)
            image.set_size_request(pixel_size, pixel_size)
            image.set_halign(Gtk.Align.CENTER)
            image.set_valign(Gtk.Align.CENTER)
            image.add_css_class(css_class)
            return image
        except Exception:
            pass

    icon = _app_icon_image(result.app.icon) if result.app else None
    if icon is None:
        icon = get_file_icon(result.url, result.mime)
    icon.set_pixel_size(pixel_size)
    icon.add_css_class(css_class)
    return icon


# -- dock helpers ---------------------------------------------------------------

def _get_favorites() -> list[str]:
    try:
        res = subprocess.run(
            ["gsettings", "get", "org.gnome.shell", "favorite-apps"],
            capture_output=True, text=True, check=True,
        )
        raw = res.stdout.strip()
        if raw.startswith("@as "):
            raw = raw[4:]
        return ast.literal_eval(raw)
    except Exception:
        return []


def _set_favorites(favs: list[str]) -> None:
    try:
        formatted = "[" + ", ".join(f"'{item}'" for item in favs) + "]"
        subprocess.run(
            ["gsettings", "set", "org.gnome.shell", "favorite-apps", formatted],
            check=True,
        )
    except Exception:
        pass


# -- result widgets -------------------------------------------------------------

class ResultListRow(Gtk.ListBoxRow):
    """A single result row in list view: icon + title + path + snippet."""

    def __init__(
        self,
        result: SearchResult,
        on_activate: Callable[[SearchResult], None],
        on_context_menu: Callable[[SearchResult, Gtk.Widget, float, float], None] | None = None,
    ) -> None:
        super().__init__()
        self._result = result
        self._on_activate = on_activate
        self._on_context_menu = on_context_menu

        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        box.add_css_class("result-item-list")

        icon = _build_icon_image(result, 32, "result-icon")

        text_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)

        title_label = Gtk.Label(label=result.title, xalign=0)
        title_label.add_css_class("result-title")
        title_label.set_ellipsize(True)
        title_label.set_max_width_chars(60)
        text_box.append(title_label)

        if result.snippet:
            snippet_label = Gtk.Label(label=result.snippet, xalign=0)
            snippet_label.add_css_class("result-snippet")
            snippet_label.set_ellipsize(True)
            snippet_label.set_max_width_chars(60)
            text_box.append(snippet_label)

        box.append(icon)
        box.append(text_box)
        self.set_child(box)

        if self._on_context_menu:
            click = Gtk.GestureClick()
            click.set_button(3)
            click.set_propagation_phase(Gtk.PropagationPhase.CAPTURE)
            click.connect("pressed", self._on_right_click)
            self.add_controller(click)

    def _on_right_click(self, _gesture, _n_press, x, y) -> None:
        parent = self.get_parent()
        if isinstance(parent, Gtk.ListBox):
            parent.select_row(self)
        if self._on_context_menu:
            self._on_context_menu(self._result, self, x, y)

    @property
    def result(self) -> SearchResult:
        return self._result


class ResultGridChild(Gtk.FlowBoxChild):
    """A single result tile in grid view: icon + title."""

    def __init__(
        self,
        result: SearchResult,
        on_activate: Callable[[SearchResult], None],
        on_context_menu: Callable[[SearchResult, Gtk.Widget, float, float], None] | None = None,
    ) -> None:
        super().__init__()
        self._result = result
        self._on_activate = on_activate
        self._on_context_menu = on_context_menu

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        box.add_css_class("result-item-grid")
        box.set_size_request(90, -1)

        icon = _build_icon_image(result, 48, "result-icon-grid")

        title_label = Gtk.Label(label=result.title)
        title_label.add_css_class("result-title-grid")
        title_label.set_wrap(True)
        title_label.set_justify(Gtk.Justification.CENTER)
        title_label.set_max_width_chars(12)
        title_label.set_halign(Gtk.Align.CENTER)

        box.append(icon)
        box.append(title_label)
        self.set_child(box)

        if self._on_context_menu:
            click = Gtk.GestureClick()
            click.set_button(3)
            click.set_propagation_phase(Gtk.PropagationPhase.CAPTURE)
            click.connect("pressed", self._on_right_click)
            self.add_controller(click)

    def _on_right_click(self, _gesture, _n_press, x, y) -> None:
        parent = self.get_parent()
        if isinstance(parent, Gtk.FlowBox):
            parent.select_child(self)
        if self._on_context_menu:
            self._on_context_menu(self._result, self, x, y)

    @property
    def result(self) -> SearchResult:
        return self._result


# -- main container -------------------------------------------------------------

class ResultView(Gtk.Stack):
    """Container that switches between list and grid result views."""

    def __init__(
        self,
        on_activate: Callable[[SearchResult], None],
        on_uninstall: Callable[[str, str], None] | None = None,
    ) -> None:
        super().__init__()
        self._on_activate = on_activate
        self._on_uninstall = on_uninstall
        self._results: list[SearchResult] = []
        self._popover_parent: Gtk.Widget | None = None

        self.set_transition_type(Gtk.StackTransitionType.CROSSFADE)

        self._list_box = Gtk.ListBox()
        self._list_box.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self._list_box.connect("row-activated", self._on_list_row_activated)

        list_click = Gtk.GestureClick()
        list_click.set_button(3)
        list_click.set_propagation_phase(Gtk.PropagationPhase.CAPTURE)
        list_click.connect("pressed", self._on_list_box_right_click)
        self._list_box.add_controller(list_click)

        self._grid = Gtk.FlowBox()
        self._grid.set_valign(Gtk.Align.START)
        self._grid.set_max_children_per_line(6)
        self._grid.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self._grid.connect("child-activated", self._on_grid_child_activated)

        grid_click = Gtk.GestureClick()
        grid_click.set_button(3)
        grid_click.set_propagation_phase(Gtk.PropagationPhase.CAPTURE)
        grid_click.connect("pressed", self._on_grid_right_click)
        self._grid.add_controller(grid_click)

        self.add_named(self._list_box, "list")
        self.add_named(self._grid, "grid")

        self._context_menu_result: SearchResult | None = None
        self._popover = self._build_context_menu()

    def set_popover_parent(self, widget: Gtk.Widget) -> None:
        self._popover_parent = widget
        self._popover.unparent()
        self._popover.set_parent(widget)

    def set_results(self, results: list[SearchResult], as_grid: bool = False) -> None:
        self._results = results
        self._clear(self._list_box)
        self._clear(self._grid)

        for result in results[:200]:
            self._list_box.append(ResultListRow(result, self._on_activate, self.show_context_menu_for))
            self._grid.append(ResultGridChild(result, self._on_activate, self.show_context_menu_for))

        self.set_visible_child_name("grid" if as_grid else "list")
        if not as_grid:
            first = self._list_box.get_row_at_index(0)
            if first:
                self._list_box.select_row(first)

    def select_first(self) -> None:
        row = self._list_box.get_row_at_index(0)
        if row:
            self._list_box.select_row(row)
            row.grab_focus()

    def activate_selected(self) -> SearchResult | None:
        if self.get_visible_child_name() == "grid":
            selected = self._grid.get_selected_children()
            if selected:
                child = selected[0]
                if isinstance(child, ResultGridChild):
                    self._on_activate(child.result)
                    return child.result
        else:
            row = self._list_box.get_selected_row()
            if row and isinstance(row, ResultListRow):
                self._on_activate(row.result)
                return row.result
        return None

    def move_selection_up(self) -> None:
        if self.get_visible_child_name() == "list":
            row = self._list_box.get_selected_row()
            if row:
                idx = row.get_index()
                if idx > 0:
                    prev_row = self._list_box.get_row_at_index(idx - 1)
                    if prev_row:
                        self._list_box.select_row(prev_row)
                        prev_row.grab_focus()
        else:
            selected = self._grid.get_selected_children()
            if selected:
                idx = selected[0].get_index()
                if idx >= 6:
                    prev_child = self._grid.get_child_at_index(idx - 6)
                    if prev_child:
                        self._grid.select_child(prev_child)
                        prev_child.grab_focus()

    def move_selection_down(self) -> None:
        if self.get_visible_child_name() == "list":
            row = self._list_box.get_selected_row()
            if row:
                idx = row.get_index()
                next_row = self._list_box.get_row_at_index(idx + 1)
                if next_row:
                    self._list_box.select_row(next_row)
                    next_row.grab_focus()
        else:
            selected = self._grid.get_selected_children()
            if selected:
                idx = selected[0].get_index()
                next_child = self._grid.get_child_at_index(idx + 6)
                if next_child:
                    self._grid.select_child(next_child)
                    next_child.grab_focus()

    def move_selection_left(self) -> None:
        if self.get_visible_child_name() == "grid":
            selected = self._grid.get_selected_children()
            if selected:
                idx = selected[0].get_index()
                if idx > 0:
                    prev_child = self._grid.get_child_at_index(idx - 1)
                    if prev_child:
                        self._grid.select_child(prev_child)
                        prev_child.grab_focus()

    def move_selection_right(self) -> None:
        if self.get_visible_child_name() == "grid":
            selected = self._grid.get_selected_children()
            if selected:
                idx = selected[0].get_index()
                next_child = self._grid.get_child_at_index(idx + 1)
                if next_child:
                    self._grid.select_child(next_child)
                    next_child.grab_focus()

    # -- internal -------------------------------------------------------------

    def _on_list_row_activated(self, _listbox: Gtk.ListBox, row: Gtk.ListBoxRow) -> None:
        if isinstance(row, ResultListRow):
            self._on_activate(row.result)

    def _on_grid_child_activated(self, _flowbox: Gtk.FlowBox, child: Gtk.FlowBoxChild) -> None:
        if isinstance(child, ResultGridChild):
            self._on_activate(child.result)

    @staticmethod
    def _clear(container: Gtk.ListBox | Gtk.FlowBox) -> None:
        while child := container.get_first_child():
            container.remove(child)

    # -- context menu ---------------------------------------------------------

    def _build_context_menu(self) -> Gtk.Popover:
        popover = Gtk.Popover()
        popover.set_has_arrow(False)
        popover.set_position(Gtk.PositionType.BOTTOM)
        popover.set_autohide(True)
        popover.add_css_class("ctx-menu")

        outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        outer.add_css_class("ctx-menu-box")

        self._btn_open = Gtk.Button(label="Open")
        self._btn_open.add_css_class("ctx-menu-btn")
        self._btn_open.set_halign(Gtk.Align.FILL)
        self._btn_open.connect("clicked", lambda *_: self._ctx_open())
        outer.append(self._btn_open)

        self._btn_open_dir = Gtk.Button(label="Open containing folder")
        self._btn_open_dir.add_css_class("ctx-menu-btn")
        self._btn_open_dir.set_halign(Gtk.Align.FILL)
        self._btn_open_dir.connect("clicked", lambda *_: self._ctx_open_dir())
        outer.append(self._btn_open_dir)

        sep1 = Gtk.Separator(orientation=Gtk.Orientation.HORIZONTAL)
        outer.append(sep1)

        self._btn_pin = Gtk.Button(label="Pin to dock")
        self._btn_pin.add_css_class("ctx-menu-btn")
        self._btn_pin.set_halign(Gtk.Align.FILL)
        self._btn_pin.connect("clicked", lambda *_: self._ctx_toggle_pin())
        outer.append(self._btn_pin)

        self._btn_uninstall = Gtk.Button(label="Uninstall")
        self._btn_uninstall.add_css_class("ctx-menu-btn")
        self._btn_uninstall.add_css_class("ctx-menu-btn-danger")
        self._btn_uninstall.set_halign(Gtk.Align.FILL)
        self._btn_uninstall.connect("clicked", lambda *_: self._ctx_uninstall())
        outer.append(self._btn_uninstall)

        sep2 = Gtk.Separator(orientation=Gtk.Orientation.HORIZONTAL)
        outer.append(sep2)

        self._btn_copy_name = Gtk.Button(label="Copy name")
        self._btn_copy_name.add_css_class("ctx-menu-btn")
        self._btn_copy_name.set_halign(Gtk.Align.FILL)
        self._btn_copy_name.connect("clicked", lambda *_: self._ctx_copy_name())
        outer.append(self._btn_copy_name)

        self._btn_copy_path = Gtk.Button(label="Copy path")
        self._btn_copy_path.add_css_class("ctx-menu-btn")
        self._btn_copy_path.set_halign(Gtk.Align.FILL)
        self._btn_copy_path.connect("clicked", lambda *_: self._ctx_copy_path())
        outer.append(self._btn_copy_path)

        popover.set_child(outer)
        if self._popover_parent:
            popover.set_parent(self._popover_parent)
        else:
            popover.set_parent(self)
        return popover

    def _on_list_box_right_click(self, _gesture, _n_press, x, y) -> None:
        row = self._list_box.get_row_at_y(int(y))
        if isinstance(row, ResultListRow):
            self._list_box.select_row(row)
            row_y = y - row.get_allocation().y
            self.show_context_menu_for(row.result, row, x, row_y)

    def _on_grid_right_click(self, _gesture, _n_press, x, y) -> None:
        child = self._grid.get_child_at_pos(int(x), int(y))
        if isinstance(child, ResultGridChild):
            self._grid.select_child(child)
            child_alloc = child.get_allocation()
            child_x = x - child_alloc.x
            child_y = y - child_alloc.y
            self.show_context_menu_for(child.result, child, child_x, child_y)

    def show_context_menu_for(
        self,
        result: SearchResult,
        source_widget: Gtk.Widget,
        x: float,
        y: float,
    ) -> None:
        """Show context menu pointing directly to the clicked spot on source_widget."""
        self._context_menu_result = result
        is_app = result.app is not None

        self._btn_open_dir.set_visible(not is_app)
        self._btn_copy_path.set_visible(not is_app)

        if is_app:
            desktop_id = result.app.filename
            favs = _get_favorites()
            is_pinned = desktop_id in favs
            self._btn_pin.set_label("Unpin from dock" if is_pinned else "Pin to dock")
            self._btn_pin.set_visible(True)
            self._btn_uninstall.set_visible(True)
        else:
            self._btn_pin.set_visible(False)
            self._btn_uninstall.set_visible(False)

        # Parent the popover directly to the clicked row/item
        if self._popover.get_parent() != source_widget:
            if self._popover.get_parent():
                self._popover.unparent()
            self._popover.set_parent(source_widget)

        # In PyGObject, Gdk.Rectangle kwargs in __init__ are ignored; set attributes directly!
        rect = Gdk.Rectangle()
        rect.x = int(x)
        rect.y = int(y)
        rect.width = 1
        rect.height = 1

        self._popover.set_pointing_to(rect)
        self._popover.popup()

    def _get_result_at(self, x: float, y: float) -> SearchResult | None:
        if self.get_visible_child_name() == "list":
            widget = self._list_box.get_row_at_y(int(y))
            if isinstance(widget, ResultListRow):
                return widget.result
        else:
            child = self._grid.get_child_at_pos(int(x), int(y))
            if isinstance(child, ResultGridChild):
                return child.result
        return None

    def show_context_menu(
        self,
        hit_x: float, hit_y: float,
        popup_x: int, popup_y: int,
    ) -> None:
        result = self._get_result_at(hit_x, hit_y)
        if result is None:
            return

        self._context_menu_result = result
        is_app = result.app is not None

        self._btn_open_dir.set_visible(not is_app)
        self._btn_copy_path.set_visible(not is_app)

        if is_app:
            desktop_id = result.app.filename
            favs = _get_favorites()
            is_pinned = desktop_id in favs
            self._btn_pin.set_label("Unpin from dock" if is_pinned else "Pin to dock")
            self._btn_pin.set_visible(True)
            self._btn_uninstall.set_visible(True)
        else:
            self._btn_pin.set_visible(False)
            self._btn_uninstall.set_visible(False)

        rect = Gdk.Rectangle(x=popup_x, y=popup_y, width=1, height=1)
        self._popover.set_pointing_to(rect)
        self._popover.popup()

    def _ctx_open(self) -> None:
        if self._context_menu_result:
            self._on_activate(self._context_menu_result)
            self._popover.popdown()

    def _ctx_open_dir(self) -> None:
        if self._context_menu_result:
            url = self._context_menu_result.url
            path = url.removeprefix("file://")
            parent = os.path.dirname(path)
            if parent:
                from pulsaros_spotlight.utils import open_file
                open_file(f"file://{parent}")
            self._popover.popdown()

    def _ctx_toggle_pin(self) -> None:
        if self._context_menu_result and self._context_menu_result.app:
            desktop_id = self._context_menu_result.app.filename
            favs = _get_favorites()
            if desktop_id in favs:
                favs = [f for f in favs if f != desktop_id]
            else:
                favs.append(desktop_id)
            _set_favorites(favs)
            is_pinned = desktop_id in favs
            self._btn_pin.set_label("Unpin from dock" if is_pinned else "Pin to dock")
        self._popover.popdown()

    def _ctx_uninstall(self) -> None:
        if self._context_menu_result and self._context_menu_result.app:
            desktop_id = self._context_menu_result.app.filename
            app_name = self._context_menu_result.app.name
            self._popover.popdown()
            if self._on_uninstall:
                self._on_uninstall(desktop_id, app_name)
            else:
                logger.warning("No uninstall handler registered for %s", desktop_id)
        else:
            self._popover.popdown()

    def _ctx_copy_name(self) -> None:
        if self._context_menu_result:
            clipboard = Gdk.Display.get_default().get_clipboard()
            clipboard.set(self._context_menu_result.title)
            self._popover.popdown()

    def _ctx_copy_path(self) -> None:
        if self._context_menu_result:
            clipboard = Gdk.Display.get_default().get_clipboard()
            clipboard.set(self._context_menu_result.url)
            self._popover.popdown()
