"""Main Spotlight search window widget with GTK4 and Libadwaita."""

from __future__ import annotations

import os
from pathlib import Path
from typing import TYPE_CHECKING

from gi.repository import Adw, Gdk, GLib, Gtk

from pulsaros_spotlight.search import SearchBackend, SearchResult
from pulsaros_spotlight.ui.results import ResultView
from pulsaros_spotlight.utils import open_file

if TYPE_CHECKING:
    from pulsaros_spotlight.config import SpotlightConfig
    from pulsaros_spotlight.clipboard import ClipboardManager

_ICON_DIR = Path("/usr/share/pulsaros-spotlight/icons")
_LOCAL_ICON_DIR = Path(__file__).resolve().parent.parent.parent / "data" / "icons"

_CATEGORIES = [
    ("all", "All"),
    ("applications", "Apps"),
    ("documents", "Docs"),
    ("images", "Images"),
    ("audio", "Music"),
    ("video", "Video"),
    ("clipboard", "Clip"),
    ("web", "Web"),
]

_DEBOUNCE_MS = 150


class SpotlightWindow(Gtk.ApplicationWindow):
    """The floating macOS-style Spotlight search bar window."""

    def __init__(
        self,
        application: Gtk.Application,
        config: SpotlightConfig,
        backend: SearchBackend,
        clipboard_mgr: ClipboardManager | None = None,
    ) -> None:
        super().__init__(application=application)
        self._config = config
        self._backend = backend
        self._clipboard_mgr = clipboard_mgr
        self._category: str = "all"
        self._debounce_id: int | None = None
        self._current_dir: str | None = None
        self._has_been_active: bool = False

        self.set_title("Spotlight")
        self.set_default_size(680, 520)
        self.set_resizable(False)
        self.set_decorated(False)
        self.add_css_class("spotlight-window")

        self._build_ui()
        self._setup_events()

    # -- UI construction ------------------------------------------------------

    def _build_ui(self) -> None:
        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        main_box.add_css_class("spotlight-main")
        self.set_child(main_box)

        # -- search header --
        search_container = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        search_container.add_css_class("search-header")

        # Authentic macOS Spotlight search icon
        search_icon = None
        for base in (_LOCAL_ICON_DIR, _ICON_DIR):
            f = base / "spotlight-symbolic.svg"
            if f.exists():
                search_icon = Gtk.Image.new_from_file(str(f))
                break
        if search_icon is None:
            search_icon = Gtk.Image.new_from_icon_name("system-search-symbolic")
        search_icon.add_css_class("search-icon")

        self._search_entry = Gtk.Entry()
        self._search_entry.set_placeholder_text("Search applications, files, or clipboard...")
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
        scroll.set_min_content_height(300)
        scroll.set_max_content_height(480)
        scroll.add_css_class("results-area")
        scroll.set_child(self._result_view)
        main_box.append(scroll)

    # -- event setup ----------------------------------------------------------

    def _setup_events(self) -> None:
        self._search_entry.connect("changed", self._on_search_changed)
        self._search_entry.connect("activate", self._on_activate_first)
        self._view_toggle.connect("clicked", self._on_toggle_view)

        focus_ctrl = Gtk.EventControllerFocus()
        focus_ctrl.connect("leave", self._on_focus_leave)
        self.add_controller(focus_ctrl)

        self.connect("notify::is-active", self._on_active_changed)

        key_ctrl = Gtk.EventControllerKey()
        key_ctrl.connect("key-pressed", self._on_key_pressed)
        self.add_controller(key_ctrl)

        self.connect("close-request", self._on_close_request)

    # -- focus & dismiss handlers ---------------------------------------------

    def _on_active_changed(self, window: Gtk.Window, _pspec) -> None:
        if window.is_active():
            self._has_been_active = True
        elif self._has_been_active:
            GLib.timeout_add(100, self._check_focus_and_hide)

    def _on_focus_leave(self, _ctrl: Gtk.EventControllerFocus) -> None:
        if self._has_been_active:
            GLib.timeout_add(100, self._check_focus_and_hide)

    def _check_focus_and_hide(self) -> bool:
        if self._has_been_active and self.is_visible() and not self.is_active():
            self._has_been_active = False
            self.set_visible(False)
        return False

    # -- event handlers -------------------------------------------------------

    def _on_search_changed(self, _entry: Gtk.Entry) -> None:
        if self._debounce_id is not None:
            GLib.source_remove(self._debounce_id)
        self._debounce_id = GLib.timeout_add(_DEBOUNCE_MS, self._do_search)

    def _do_search(self) -> bool:
        self._debounce_id = None
        query = self._search_entry.get_text()

        # If currently browsing a directory
        if self._current_dir:
            results = self._browse_directory(self._current_dir, query.strip())
            self._result_view.set_results(results, self._config.is_grid_view)
            return False

        if not query.strip():
            if self._category == "clipboard" and self._clipboard_mgr:
                results = self._clipboard_mgr.search_history("")
                self._result_view.set_results(results, self._config.is_grid_view)
            else:
                self._result_view.set_results([], self._config.is_grid_view)
            return False

        results = self._backend.search(query.strip(), category=self._category)
        self._result_view.set_results(results, self._config.is_grid_view)
        return False

    def _browse_directory(self, path_str: str, filter_q: str = "") -> list[SearchResult]:
        """Browse filesystem directory entries with navigation support."""
        results: list[SearchResult] = []
        try:
            p = Path(path_str).expanduser().resolve()
            if not p.is_dir():
                return results

            # Parent directory navigation
            if p != p.parent:
                results.append(
                    SearchResult(
                        url=f"file://{p.parent}",
                        title=".. (Subir al directorio superior)",
                        mime="inode/directory",
                        snippet=str(p.parent),
                        app=None,
                    )
                )

            for entry in sorted(p.iterdir(), key=lambda x: (not x.is_dir(), x.name.lower())):
                if entry.name.startswith("."):
                    continue
                if filter_q and filter_q.lower() not in entry.name.lower():
                    continue

                is_d = entry.is_dir()
                mime = "inode/directory" if is_d else "application/octet-stream"
                results.append(
                    SearchResult(
                        url=f"file://{entry}",
                        title=entry.name,
                        mime=mime,
                        snippet=str(entry),
                        app=None,
                    )
                )
        except Exception:
            pass
        return results

    def _on_activate_first(self, _entry: Gtk.Entry) -> None:
        if not self._result_view.activate_selected():
            # If nothing selected, activate first result
            row = self._result_view._list_box.get_row_at_index(0)
            if row and hasattr(row, 'result'):
                self._on_result_activated(row.result)

    def _on_result_activated(self, result: SearchResult) -> None:
        # 1. Clipboard Item Activation (Paste & close)
        if result.url.startswith("clipboard://"):
            try:
                idx = int(result.url.split("://")[1])
                if self._clipboard_mgr:
                    text = self._clipboard_mgr.get_clip_by_index(idx)
                    if text:
                        self.set_visible(False)
                        self._clipboard_mgr.paste_clip(text)
                        return
            except Exception:
                pass
            self.set_visible(False)
            return

        # 2. Directory Navigation
        clean_path = result.url.removeprefix("file://")
        if os.path.isdir(clean_path) and (result.mime in ("inode/directory", "folder") or not result.app):
            self._current_dir = clean_path
            self._search_entry.set_text("")
            self._search_entry.set_placeholder_text(f"Navegando: {clean_path}")
            results = self._browse_directory(clean_path)
            self._result_view.set_results(results, self._config.is_grid_view)
            return

        # 3. Regular file or application launch
        open_file(result.url)
        self.set_visible(False)

    def _on_toggle_view(self, _btn: Gtk.Button) -> None:
        self._config.is_grid_view = not self._config.is_grid_view
        self._config.save()
        icon_name = "view-list-symbolic" if self._config.is_grid_view else "view-grid-symbolic"
        self._view_toggle.set_icon_name(icon_name)

        query = self._search_entry.get_text().strip()
        if self._current_dir:
            results = self._browse_directory(self._current_dir, query)
            self._result_view.set_results(results, self._config.is_grid_view)
        elif query or self._category == "clipboard":
            self._do_search()

    def _on_category_toggled(self, btn: Gtk.ToggleButton, cat_id: str) -> None:
        if not btn.get_active():
            return
        self._category = cat_id
        self._current_dir = None
        self._search_entry.set_placeholder_text("Search applications, files, or clipboard...")

        for cid, b in self._category_buttons.items():
            if cid != cat_id:
                b.set_active(False)

        self._do_search()

    def _cycle_category(self, step: int) -> None:
        cats = [cid for cid, _ in _CATEGORIES]
        try:
            curr_idx = cats.index(self._category)
            next_idx = (curr_idx + step) % len(cats)
            next_cat = cats[next_idx]
            btn = self._category_buttons.get(next_cat)
            if btn:
                btn.set_active(True)
        except ValueError:
            pass

    def _on_key_pressed(
        self, _ctrl: Gtk.EventControllerKey, keyval: int, _keycode: int, state: int
    ) -> bool:
        # Escape: exit directory or hide
        if keyval == Gdk.KEY_Escape:
            if self._current_dir:
                self._current_dir = None
                self._search_entry.set_text("")
                self._search_entry.set_placeholder_text("Search applications, files, or clipboard...")
                self._do_search()
                return True
            self.set_visible(False)
            return True

        if (state & Gdk.ModifierType.CONTROL_MASK) and keyval == Gdk.KEY_q:
            self.get_application().quit()
            return True

        # Tab / Shift+Tab: cycle categories
        if keyval == Gdk.KEY_Tab:
            if state & Gdk.ModifierType.SHIFT_MASK:
                self._cycle_category(-1)
            else:
                self._cycle_category(1)
            return True
        if keyval == Gdk.KEY_ISO_Left_Tab:
            self._cycle_category(-1)
            return True

        # Alt+Left/Right or Ctrl+Left/Right: cycle categories
        if (state & (Gdk.ModifierType.ALT_MASK | Gdk.ModifierType.CONTROL_MASK)):
            if keyval == Gdk.KEY_Left:
                self._cycle_category(-1)
                return True
            if keyval == Gdk.KEY_Right:
                self._cycle_category(1)
                return True

        entry_has_focus = self._search_entry.has_focus()

        # Arrow key navigation in results
        if keyval == Gdk.KEY_Up:
            self._result_view.move_selection_up()
            return True
        if keyval == Gdk.KEY_Down:
            self._result_view.move_selection_down()
            return True
        if keyval == Gdk.KEY_Left and not entry_has_focus:
            self._result_view.move_selection_left()
            return True
        if keyval == Gdk.KEY_Right and not entry_has_focus:
            self._result_view.move_selection_right()
            return True

        # Enter / Return on result
        if keyval in (Gdk.KEY_Return, Gdk.KEY_KP_Enter):
            if self._result_view.activate_selected():
                return True

        # Backspace on empty entry: ascend directory
        if keyval == Gdk.KEY_BackSpace and not self._search_entry.get_text() and self._current_dir:
            parent = str(Path(self._current_dir).parent)
            if parent != self._current_dir:
                self._current_dir = parent
                self._search_entry.set_placeholder_text(f"Navegando: {parent}")
                results = self._browse_directory(parent)
                self._result_view.set_results(results, self._config.is_grid_view)
                return True

        # Automatic typing activation: if user is typing any character while focus is elsewhere
        if not entry_has_focus and not (state & (Gdk.ModifierType.CONTROL_MASK | Gdk.ModifierType.ALT_MASK | Gdk.ModifierType.SUPER_MASK)):
            unicode_char = Gdk.keyval_to_unicode(keyval)
            if unicode_char != 0:
                char_str = chr(unicode_char)
                if char_str.isprintable():
                    self._search_entry.grab_focus()
                    curr = self._search_entry.get_text()
                    self._search_entry.set_text(curr + char_str)
                    self._search_entry.set_position(-1)
                    return True
            elif keyval == Gdk.KEY_BackSpace:
                self._search_entry.grab_focus()
                curr = self._search_entry.get_text()
                if curr:
                    self._search_entry.set_text(curr[:-1])
                    self._search_entry.set_position(-1)
                return True

        return False

    def _on_close_request(self, _win: Gtk.ApplicationWindow) -> bool:
        self.set_visible(False)
        return True

    # -- public API -----------------------------------------------------------

    def present_with_focus(self) -> None:
        """Show the window and focus the search entry with always-on-top positioning."""
        self._current_dir = None
        self._search_entry.set_placeholder_text("Search applications, files, or clipboard...")
        self._search_entry.set_text("")
        self._do_search()

        self.present()
        self._search_entry.grab_focus()
