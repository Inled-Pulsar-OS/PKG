"""Result rendering widgets for PulsarOS Spotlight."""

from __future__ import annotations

import os
from typing import Callable

from gi.repository import Gdk, Gio, GLib, Gtk

from pulsaros_spotlight.search import SearchResult
from pulsaros_spotlight.utils import get_file_icon


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


def _build_icon_image(result: SearchResult, pixel_size: int, css_class: str) -> Gtk.Image:
    """Return the best icon for a result, falling back to a generic file icon."""
    icon = _app_icon_image(result.app.icon) if result.app else None
    if icon is None:
        icon = get_file_icon(result.url, result.mime)
    icon.set_pixel_size(pixel_size)
    icon.add_css_class(css_class)
    return icon


class ResultListRow(Gtk.ListBoxRow):
    """A single result row in list view: icon + title + path + snippet."""

    def __init__(self, result: SearchResult, on_activate: Callable[[SearchResult], None]) -> None:
        super().__init__()
        self._result = result
        self._on_activate = on_activate

        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        box.add_css_class("result-item-list")

        icon = _build_icon_image(result, 32, "result-icon")
        icon.add_css_class("result-icon")

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

    @property
    def result(self) -> SearchResult:
        return self._result


class ResultGridChild(Gtk.FlowBoxChild):
    """A single result tile in grid view: icon + title."""

    def __init__(self, result: SearchResult, on_activate: Callable[[SearchResult], None]) -> None:
        super().__init__()
        self._result = result
        self._on_activate = on_activate

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

    @property
    def result(self) -> SearchResult:
        return self._result


class ResultView(Gtk.Stack):
    """Container that switches between list and grid result views."""

    def __init__(self, on_activate: Callable[[SearchResult], None]) -> None:
        super().__init__()
        self._on_activate = on_activate
        self._results: list[SearchResult] = []

        self.set_transition_type(Gtk.StackTransitionType.CROSSFADE)

        self._list_box = Gtk.ListBox()
        self._list_box.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self._list_box.connect("row-activated", self._on_list_row_activated)

        self._grid = Gtk.FlowBox()
        self._grid.set_valign(Gtk.Align.START)
        self._grid.set_max_children_per_line(6)
        self._grid.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self._grid.connect("child-activated", self._on_grid_child_activated)

        self.add_named(self._list_box, "list")
        self.add_named(self._grid, "grid")

        self._context_menu_result: SearchResult | None = None
        self._popover = self._build_context_menu()

    def set_results(self, results: list[SearchResult], as_grid: bool = False) -> None:
        """Replace the current results and render."""
        self._results = results
        self._clear(self._list_box)
        self._clear(self._grid)

        for result in results[:200]:
            self._list_box.append(ResultListRow(result, self._on_activate))
            self._grid.append(ResultGridChild(result, self._on_activate))

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
        """Activate the currently selected result. Returns it if found."""
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

    def _build_context_menu(self) -> Gtk.PopoverMenu:
        menu = Gio.Menu()
        menu.append("Abrir", "result.open")
        menu.append("Copiar nombre", "result.copy-name")
        menu.append("Copiar ruta", "result.copy-path")

        popover = Gtk.PopoverMenu(menu_model=menu)
        popover.set_parent(self)

        actions = Gio.SimpleActionGroup()
        for name, handler in [
            ("open", self._ctx_open),
            ("copy-name", self._ctx_copy_name),
            ("copy-path", self._ctx_copy_path),
        ]:
            action = Gio.SimpleAction(name=name)
            action.connect("activate", handler)
            actions.add_action(action)

        self.insert_action_group("result", actions)
        return popover

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

    def show_context_menu(self, x: float, y: float) -> None:
        result = self._get_result_at(x, y)
        if result is None:
            return
        self._context_menu_result = result
        self._popover.set_pointing_to(Gdk.Rectangle(x=int(x), y=int(y), width=1, height=1))
        self._popover.popup()

    def _ctx_open(self, _action, _param) -> None:
        if self._context_menu_result:
            self._on_activate(self._context_menu_result)
            self._popover.popdown()

    def _ctx_copy_name(self, _action, _param) -> None:
        if self._context_menu_result:
            clipboard = Gdk.Display.get_default().get_clipboard()
            clipboard.set(self._context_menu_result.title)
            self._popover.popdown()

    def _ctx_copy_path(self, _action, _param) -> None:
        if self._context_menu_result:
            clipboard = Gdk.Display.get_default().get_clipboard()
            clipboard.set(self._context_menu_result.url)
            self._popover.popdown()
