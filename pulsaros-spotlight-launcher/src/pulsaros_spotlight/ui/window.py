"""Main Spotlight window for PulsarOS."""

from __future__ import annotations

import time

from gi.repository import Gdk, GLib, Gtk

from pulsaros_spotlight.config import Config
from pulsaros_spotlight.search import SearchBackend
from pulsaros_spotlight.ui.results import ResultView
from pulsaros_spotlight.utils import open_file

_DEBOUNCE_MS = 150
_FOCUS_LOSE_DELAY = 0.5

_CATEGORIES = [
    ("all", "All"),
    ("apps", "Apps"),
    ("documents", "Docs"),
    ("images", "Images"),
    ("music", "Music"),
    ("video", "Video"),
]


class SpotlightWindow(Gtk.ApplicationWindow):
    """The main Spotlight search window."""

    def __init__(
        self,
        app: Gtk.Application,
        backend: SearchBackend,
        config: Config,
    ) -> None:
        super().__init__(application=app)
        self._backend = backend
        self._config = config
        self._debounce_id: int | None = None
        self._focus_since: float | None = None
        self._category = "all"

        self.set_default_size(680, 500)
        self.set_decorated(False)
        self.set_resizable(False)
        self.set_title("Spotlight")

        self._build_ui()
        self._connect_events()

    # -- UI construction ------------------------------------------------------

    def _build_ui(self) -> None:
        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        main_box.add_css_class("spotlight-main")
        self.set_child(main_box)

        # -- search header --
        search_container = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        search_container.add_css_class("search-header")

        search_icon = Gtk.Image.new_from_icon_name("system-search-symbolic")
        search_icon.add_css_class("search-icon")

        self._search_entry = Gtk.Entry()
        self._search_entry.set_placeholder_text("Search applications and files...")
        self._search_entry.set_hexpand(True)
        self._search_entry.add_css_class("search-input")

        self._view_toggle = Gtk.Button()
        icon_name = "view-list-symbolic" if self._config.is_grid_view else "view-grid-symbolic"
        self._view_toggle.set_icon_name(icon_name)
        self._view_toggle.add_css_class("view-toggle")

        search_container.append(search_icon)
        search_container.append(self._search_entry)
        search_container.append(self._view_toggle)
        main_box.append(search_container)

        # -- category bar --
        self._category_bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        self._category_bar.add_css_class("category-bar")
        self._category_buttons: dict[str, Gtk.ToggleButton] = {}
        for cat_id, cat_label in _CATEGORIES:
            btn = Gtk.ToggleButton(label=cat_label)
            btn.add_css_class("category-btn")
            btn.set_active(cat_id == self._category)
            btn.connect("toggled", self._on_category_toggled, cat_id)
            self._category_bar.append(btn)
            self._category_buttons[cat_id] = btn
        main_box.append(self._category_bar)

        # -- results area --
        self._result_view = ResultView(on_activate=self._on_result_activated)

        scroll = Gtk.ScrolledWindow()
        scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scroll.set_vexpand(True)
        scroll.add_css_class("results-area")
        scroll.set_child(self._result_view)
        main_box.append(scroll)

    def _connect_events(self) -> None:
        self._search_entry.connect("changed", self._on_search_changed)
        self._search_entry.connect("activate", self._on_activate_first)
        self._view_toggle.connect("clicked", self._on_toggle_view)

        focus_ctrl = Gtk.EventControllerFocus()
        focus_ctrl.connect("enter", self._on_focus_enter)
        focus_ctrl.connect("leave", self._on_focus_leave)
        self.add_controller(focus_ctrl)

        key_ctrl = Gtk.EventControllerKey()
        key_ctrl.connect("key-pressed", self._on_key_pressed)
        self.add_controller(key_ctrl)

        self.connect("close-request", self._on_close_request)

    # -- event handlers -------------------------------------------------------

    def _on_search_changed(self, entry: Gtk.Entry) -> None:
        if self._debounce_id is not None:
            GLib.source_remove(self._debounce_id)
        self._debounce_id = GLib.timeout_add(_DEBOUNCE_MS, self._do_search)

    def _do_search(self) -> bool:
        self._debounce_id = None
        query = self._search_entry.get_text().strip()
        if not query:
            self._result_view.set_results([], self._config.is_grid_view)
            return False
        results = self._backend.search(query, category=self._category)
        self._result_view.set_results(results, self._config.is_grid_view)
        return False

    def _on_activate_first(self, _entry: Gtk.Entry) -> None:
        self._result_view.activate_selected()

    def _on_result_activated(self, result) -> None:
        open_file(result.url)
        self.set_visible(False)

    def _on_toggle_view(self, _btn: Gtk.Button) -> None:
        self._config.is_grid_view = not self._config.is_grid_view
        self._config.save()
        icon_name = "view-list-symbolic" if self._config.is_grid_view else "view-grid-symbolic"
        self._view_toggle.set_icon_name(icon_name)
        # re-render current results in new view
        query = self._search_entry.get_text().strip()
        if query:
            results = self._backend.search(query, category=self._category)
            self._result_view.set_results(results, self._config.is_grid_view)

    def _on_category_toggled(self, btn: Gtk.ToggleButton, cat_id: str) -> None:
        if not btn.get_active():
            return
        self._category = cat_id
        # deactivate other buttons
        for cid, b in self._category_buttons.items():
            if cid != cat_id:
                b.set_active(False)
        # re-search with new category
        query = self._search_entry.get_text().strip()
        if query:
            results = self._backend.search(query, category=self._category)
            self._result_view.set_results(results, self._config.is_grid_view)

    def _on_focus_enter(self, _ctrl: Gtk.EventControllerFocus) -> None:
        self._focus_since = time.time()

    def _on_focus_leave(self, _ctrl: Gtk.EventControllerFocus) -> None:
        if self._focus_since is not None and (time.time() - self._focus_since) > _FOCUS_LOSE_DELAY:
            self.set_visible(False)
        self._focus_since = None

    def _on_key_pressed(
        self, _ctrl: Gtk.EventControllerKey, keyval: int, _keycode: int, state: int
    ) -> bool:
        if keyval == Gdk.KEY_Escape:
            self.set_visible(False)
            return True
        if (state & Gdk.ModifierType.CONTROL_MASK) and keyval == Gdk.KEY_q:
            self.get_application().quit()
            return True
        if keyval == Gdk.KEY_Up:
            self._result_view.move_selection_up()
            return True
        if keyval == Gdk.KEY_Down:
            self._result_view.move_selection_down()
            return True
        return False

    def _on_close_request(self, _win: Gtk.ApplicationWindow) -> bool:
        self.set_visible(False)
        return True

    # -- public API -----------------------------------------------------------

    def present_with_focus(self) -> None:
        """Show the window and focus the search entry."""
        self.present()
        self._search_entry.set_text("")
        self._search_entry.grab_focus()
