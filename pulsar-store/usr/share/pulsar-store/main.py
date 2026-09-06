#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar Store - Modern App & Ecosystem Center for Pulsar OS.
Native Libadwaita / GTK4 Interface.
"""

import os
import sys
import threading
import urllib.request
from typing import Dict, Any, List, Optional

import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
gi.require_version("GdkPixbuf", "2.0")
from gi.repository import Gtk, Adw, GLib, Gio, Gdk, GdkPixbuf

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if SCRIPT_DIR not in sys.path:
    sys.path.insert(0, SCRIPT_DIR)

from store_core import StoreCore


class PulsarStoreWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app, title="Store")
        self.set_default_size(1040, 700)
        self.set_size_request(800, 520)

        self.core = StoreCore(
            log_fn=self.on_log_message,
            icon_loaded_cb=lambda: GLib.idle_add(self.render_current_view)
        )
        self.current_category = "discover"
        self.current_item = None
        self.active_operations: Dict[str, str] = {}  # item_id -> action

        self.load_css()
        self.build_ui()

        threading.Thread(target=self._initial_load, daemon=True).start()

    def load_css(self):
        css_file = os.path.join(SCRIPT_DIR, "styles.css")
        if os.path.exists(css_file):
            provider = Gtk.CssProvider()
            provider.load_from_path(css_file)
            Gtk.StyleContext.add_provider_for_display(
                Gdk.Display.get_default(),
                provider,
                Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
            )

    def on_log_message(self, msg: str):
        print(f"[PulsarStore] {msg}")

    def build_ui(self):
        self.toast_overlay = Adw.ToastOverlay()
        self.set_content(self.toast_overlay)

        self.toolbar_view = Adw.ToolbarView()
        self.toast_overlay.set_child(self.toolbar_view)

        # HeaderBar
        self.header_bar = Adw.HeaderBar()
        self.toolbar_view.add_top_bar(self.header_bar)

        # Back Button
        self.btn_back = Gtk.Button(icon_name="go-previous-symbolic")
        self.btn_back.set_tooltip_text("Back")
        self.btn_back.set_visible(False)
        self.btn_back.connect("clicked", self.on_back_clicked)
        self.header_bar.pack_start(self.btn_back)

        # Search Entry in Header
        self.search_entry = Gtk.SearchEntry(placeholder_text="Search apps, extensions, Sayri AI...")
        self.search_entry.set_hexpand(True)
        self.search_entry.set_max_width_chars(32)
        self.search_entry.connect("search-changed", self.on_search_changed)
        self.header_bar.set_title_widget(self.search_entry)

        # Refresh Button
        self.btn_refresh = Gtk.Button(icon_name="view-refresh-symbolic")
        self.btn_refresh.set_tooltip_text("Refresh Store")
        self.btn_refresh.connect("clicked", self.on_refresh_clicked)
        self.header_bar.pack_end(self.btn_refresh)

        # Navigation Split View
        self.split_view = Adw.NavigationSplitView()
        self.toolbar_view.set_content(self.split_view)

        # Sidebar Navigation (Compact)
        sidebar_page = Adw.NavigationPage(title="Categories")
        sidebar_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        sidebar_box.set_margin_top(8)
        sidebar_box.set_margin_bottom(8)
        sidebar_box.set_margin_start(8)
        sidebar_box.set_margin_end(8)
        sidebar_page.set_child(sidebar_box)

        self.sidebar_list = Gtk.ListBox()
        self.sidebar_list.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self.sidebar_list.add_css_class("navigation-sidebar")
        self.sidebar_list.connect("row-selected", self.on_category_selected)
        sidebar_box.append(self.sidebar_list)

        self.categories_data = [
            ("discover", "Discover", "starred-symbolic"),
            ("apps", "Applications", "application-x-executable-symbolic"),
            ("extensions", "Extensions", "application-x-addon-symbolic"),
            ("sayri", "Sayri AI", "emblem-system-symbolic"),
            ("installed", "Installed", "emblem-default-symbolic"),
            ("updates", "Updates", "software-update-available-symbolic"),
        ]

        for cat_id, title, icon_name in self.categories_data:
            row = Gtk.ListBoxRow()
            row.add_css_class("sidebar-compact-row")
            row.cat_id = cat_id

            hbox = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
            hbox.set_margin_top(4)
            hbox.set_margin_bottom(4)
            hbox.set_margin_start(6)
            hbox.set_margin_end(6)

            icon = Gtk.Image.new_from_icon_name(icon_name)
            icon.set_pixel_size(16)
            icon.add_css_class("sidebar-icon")
            hbox.append(icon)

            lbl = Gtk.Label(label=title)
            lbl.add_css_class("sidebar-label")
            lbl.set_halign(Gtk.Align.START)
            hbox.append(lbl)

            row.set_child(hbox)
            self.sidebar_list.append(row)

        self.split_view.set_sidebar(sidebar_page)

        # Content View
        content_page = Adw.NavigationPage(title="Store Content")
        self.content_stack = Gtk.Stack()
        self.content_stack.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
        content_page.set_child(self.content_stack)
        self.split_view.set_content(content_page)

        # Page 1: Browser Scroll View
        self.browser_scroll = Gtk.ScrolledWindow()
        self.browser_scroll.set_vexpand(True)
        self.browser_clamp = Adw.Clamp()
        self.browser_clamp.set_maximum_size(960)
        self.browser_clamp.set_tightening_threshold(700)
        self.browser_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        self.browser_box.set_valign(Gtk.Align.START)
        self.browser_box.set_vexpand(False)
        self.browser_box.set_margin_top(18)
        self.browser_box.set_margin_bottom(24)
        self.browser_box.set_margin_start(18)
        self.browser_box.set_margin_end(18)
        self.browser_clamp.set_child(self.browser_box)
        self.browser_scroll.set_child(self.browser_clamp)
        self.content_stack.add_named(self.browser_scroll, "browser")

        # Page 2: Detail Scroll View
        self.detail_scroll = Gtk.ScrolledWindow()
        self.detail_scroll.set_vexpand(True)
        self.detail_clamp = Adw.Clamp()
        self.detail_clamp.set_maximum_size(860)
        self.detail_clamp.set_tightening_threshold(620)
        self.detail_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=18)
        self.detail_box.set_valign(Gtk.Align.START)
        self.detail_box.set_vexpand(False)
        self.detail_box.set_margin_top(18)
        self.detail_box.set_margin_bottom(28)
        self.detail_box.set_margin_start(18)
        self.detail_box.set_margin_end(18)
        self.detail_clamp.set_child(self.detail_box)
        self.detail_scroll.set_child(self.detail_clamp)
        self.content_stack.add_named(self.detail_scroll, "details")

        first_row = self.sidebar_list.get_row_at_index(0)
        if first_row:
            self.sidebar_list.select_row(first_row)

    def _initial_load(self):
        self.core.refresh_catalog()
        GLib.idle_add(self.render_current_view)

    def show_toast(self, message: str):
        toast = Adw.Toast.new(message)
        toast.set_timeout(3)
        self.toast_overlay.add_toast(toast)

    def show_success_dialog(self, item: Dict[str, Any], action: str):
        """Displays an overlay popup with green checkmark when action completes."""
        item_name = item.get("name", item.get("id", "Package"))
        action_map = {
            "install": ("Installed", "has been installed successfully"),
            "uninstall": ("Removed", "has been removed from your system"),
            "update": ("Updated", "has been updated to the latest version")
        }
        act_title, act_desc = action_map.get(action, ("Completed", f"{action} completed"))

        dialog = Adw.MessageDialog(
            transient_for=self,
            heading=f"{act_title}: {item_name}",
            body=f"{item_name} {act_desc}."
        )
        dialog.add_response("ok", "Done")
        dialog.set_default_response("ok")

        # Green checkmark badge
        check_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        check_box.set_halign(Gtk.Align.CENTER)
        check_box.set_valign(Gtk.Align.CENTER)

        badge = Gtk.Box()
        badge.add_css_class("success-check-badge")
        badge.set_halign(Gtk.Align.CENTER)

        check_icon = Gtk.Image.new_from_icon_name("emblem-ok-symbolic")
        check_icon.set_pixel_size(48)
        badge.append(check_icon)
        check_box.append(badge)

        dialog.set_extra_child(check_box)
        dialog.present()

        # Auto-dismiss after 4s
        GLib.timeout_add_seconds(4, lambda: (dialog.close() if dialog.is_visible() else None))

    def on_refresh_clicked(self, _btn):
        if self.active_operations:
            return
        self.btn_refresh.set_sensitive(False)
        self.show_toast("Refreshing store catalog...")

        def _do():
            self.core.refresh_catalog(force=True)
            GLib.idle_add(self._after_refresh)

        threading.Thread(target=_do, daemon=True).start()

    def _after_refresh(self):
        self.btn_refresh.set_sensitive(True)
        self.render_current_view()
        self.show_toast("Catalog updated from repository.")

    def on_search_changed(self, entry):
        query = entry.get_text()
        if query:
            self.btn_back.set_visible(False)
            self.content_stack.set_visible_child_name("browser")
            self.render_search_results(query)
        else:
            self.render_current_view()

    def on_category_selected(self, _listbox, row):
        if not row:
            return
        self.current_category = getattr(row, "cat_id", "discover")
        self.search_entry.set_text("")
        self.btn_back.set_visible(False)
        self.content_stack.set_visible_child_name("browser")
        self.render_current_view()

    def on_back_clicked(self, _btn):
        self.btn_back.set_visible(False)
        self.content_stack.set_visible_child_name("browser")

    def clear_container(self, container):
        while True:
            child = container.get_first_child()
            if not child:
                break
            container.remove(child)

    def get_item_icon_widget(self, item: Dict[str, Any], size: int = 48) -> Gtk.Widget:
        cached = self.core.get_cached_icon(item)
        if cached and os.path.isfile(cached):
            try:
                gfile = Gio.File.new_for_path(cached)
                texture = Gdk.Texture.new_from_file(gfile)
                img = Gtk.Image.new_from_paintable(texture)
                img.set_pixel_size(size)
                img.add_css_class("app-icon" if size <= 52 else "app-icon-detail")
                return img
            except Exception as e:
                print(f"[Icon Error] {e}")

        # Fallback to subtle letter avatar
        name = item.get("name", item.get("id", "?")).strip()
        first_char = (name[0] if name else "?").upper()

        box = Gtk.Box()
        box.set_size_request(size, size)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)
        box.add_css_class("letter-avatar" if size <= 52 else "letter-avatar-detail")

        lbl = Gtk.Label(label=first_char)
        lbl.set_hexpand(True)
        lbl.set_vexpand(True)
        lbl.set_valign(Gtk.Align.CENTER)
        lbl.set_halign(Gtk.Align.CENTER)
        box.append(lbl)
        return box

    def render_current_view(self):
        self.clear_container(self.browser_box)

        if self.current_category == "updates":
            self.render_updates_view()
            return

        items = self.core.get_items_by_category(self.current_category)

        cat_titles = {
            "discover": "Discover & Highlights",
            "apps": "Applications",
            "extensions": "GNOME Shell Extensions",
            "sayri": "Sayri AI Ecosystem",
            "installed": "Installed Components"
        }
        title_text = cat_titles.get(self.current_category, self.current_category.capitalize())

        header_lbl = Gtk.Label(label=title_text)
        header_lbl.add_css_class("title-2")
        header_lbl.set_halign(Gtk.Align.START)
        self.browser_box.append(header_lbl)

        if not items:
            status = Adw.StatusPage()
            status.set_icon_name("emblem-default-symbolic")
            status.set_title("No packages available")
            status.set_description("No items found in this section from the official repository.")
            self.browser_box.append(status)
            return

        grid = Gtk.Grid()
        grid.set_column_spacing(14)
        grid.set_row_spacing(14)
        grid.set_column_homogeneous(True)
        grid.set_row_homogeneous(False)
        grid.set_valign(Gtk.Align.START)
        grid.set_vexpand(False)
        self.browser_box.append(grid)

        col = 0
        row_idx = 0
        for item in items:
            card = self.create_item_tile(item)
            grid.attach(card, col, row_idx, 1, 1)
            col += 1
            if col >= 2:
                col = 0
                row_idx += 1

        # Announcement / Ad Space at bottom of Main / Discover page
        if self.current_category in ("discover", "apps"):
            ann = self.core.get_announcement()
            if ann:
                self.browser_box.append(self.create_announcement_banner(ann))

    def create_announcement_banner(self, ann: Dict[str, Any]) -> Gtk.Widget:
        card = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        card.add_css_class("announcement-card")

        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        vbox.set_hexpand(True)
        vbox.set_valign(Gtk.Align.CENTER)
        card.append(vbox)

        title_lbl = Gtk.Label(label=ann.get("title", "Publish your App"))
        title_lbl.add_css_class("heading")
        title_lbl.set_halign(Gtk.Align.START)
        vbox.append(title_lbl)

        desc_lbl = Gtk.Label(label=ann.get("description", ""))
        desc_lbl.add_css_class("dim-label")
        desc_lbl.add_css_class("caption")
        desc_lbl.set_halign(Gtk.Align.START)
        desc_lbl.set_wrap(True)
        vbox.append(desc_lbl)

        btn = Gtk.Button(label=ann.get("action_label", "Publish"))
        btn.add_css_class("suggested-action")
        btn.add_css_class("pill-action")
        btn.set_valign(Gtk.Align.CENTER)
        url = ann.get("action_url", "https://github.com/Inled-Pulsar-OS/store/issues/new/choose")
        btn.connect("clicked", lambda b: Gio.AppInfo.launch_default_for_uri(url, None))
        card.append(btn)

        return card

    def render_search_results(self, query: str):
        self.clear_container(self.browser_box)

        header_lbl = Gtk.Label(label=f"Results for '{query}'")
        header_lbl.add_css_class("title-2")
        header_lbl.set_halign(Gtk.Align.START)
        self.browser_box.append(header_lbl)

        results = self.core.search(query)
        if not results:
            status = Adw.StatusPage()
            status.set_icon_name("system-search-symbolic")
            status.set_title("No results found")
            status.set_description(f"No packages found matching '{query}'.")
            self.browser_box.append(status)
            return

        grid = Gtk.Grid()
        grid.set_column_spacing(14)
        grid.set_row_spacing(14)
        grid.set_column_homogeneous(True)
        grid.set_row_homogeneous(False)
        grid.set_valign(Gtk.Align.START)
        grid.set_vexpand(False)
        self.browser_box.append(grid)

        col = 0
        row_idx = 0
        for item in results:
            card = self.create_item_tile(item)
            grid.attach(card, col, row_idx, 1, 1)
            col += 1
            if col >= 2:
                col = 0
                row_idx += 1

    def create_item_tile(self, item: Dict[str, Any]) -> Gtk.Widget:
        item_id = item.get("id", "")
        button = Gtk.Button()
        button.add_css_class("store-tile")
        button.set_valign(Gtk.Align.START)
        button.set_vexpand(False)

        card_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=14)
        button.set_child(card_box)

        # Icon or Letter Avatar
        icon_widget = self.get_item_icon_widget(item, size=48)
        icon_widget.set_valign(Gtk.Align.CENTER)
        card_box.append(icon_widget)

        # Details VBox (Spacious Multi-Level Layout)
        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=5)
        vbox.set_hexpand(True)
        vbox.set_valign(Gtk.Align.CENTER)
        card_box.append(vbox)

        # Level 1: App Title
        title_lbl = Gtk.Label(label=item.get("name", "Unnamed"))
        title_lbl.add_css_class("heading")
        title_lbl.set_halign(Gtk.Align.START)
        title_lbl.set_ellipsize(3)
        vbox.append(title_lbl)

        # Level 2: Summary Description
        summary_text = item.get("summary") or item.get("description", "")
        summary_lbl = Gtk.Label(label=summary_text)
        summary_lbl.set_halign(Gtk.Align.START)
        summary_lbl.set_wrap(True)
        summary_lbl.set_lines(2)
        summary_lbl.set_ellipsize(3)
        summary_lbl.add_css_class("dim-label")
        summary_lbl.add_css_class("caption")
        vbox.append(summary_lbl)

        # Level 3: Metadata Badges (Category/Format + Security Score)
        meta_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        vbox.append(meta_row)

        itype = item.get("type", "app").replace("_", " ").upper()
        type_badge = Gtk.Label(label=itype)
        type_badge.add_css_class("tag-badge")
        meta_row.append(type_badge)

        sec_score = item.get("security_report", {}).get("score")
        if sec_score:
            score_badge = Gtk.Label(label=f"{sec_score}/100")
            score_badge.add_css_class("score-badge")
            meta_row.append(score_badge)

        # Action Pill Button / Live Spinner
        if item_id in self.active_operations:
            spin_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
            spin_box.set_valign(Gtk.Align.CENTER)
            spinner = Gtk.Spinner(spinning=True)
            spinner.set_size_request(20, 20)
            spin_box.append(spinner)
            card_box.append(spin_box)
        else:
            is_inst = self.core.is_installed(item)
            action_btn = Gtk.Button()
            action_btn.add_css_class("pill-action")
            action_btn.set_valign(Gtk.Align.CENTER)

            if is_inst:
                action_btn.set_label("Installed")
                action_btn.add_css_class("flat")
                action_btn.set_sensitive(False)
            else:
                action_btn.set_label("GET")
                action_btn.add_css_class("suggested-action")
                action_btn.connect("clicked", lambda b, it=item: self.perform_action(it, "install"))

            card_box.append(action_btn)

        button.connect("clicked", lambda b, it=item: self.open_details(it))
        return button

    def open_details(self, item: Dict[str, Any]):
        self.current_item = item
        item_id = item.get("id", "")
        self.clear_container(self.detail_box)
        self.btn_back.set_visible(True)
        self.content_stack.set_visible_child_name("details")

        # Top Header
        top_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=18)
        self.detail_box.append(top_box)

        icon_widget = self.get_item_icon_widget(item, size=64)
        top_box.append(icon_widget)

        meta_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        meta_box.set_hexpand(True)
        meta_box.set_valign(Gtk.Align.CENTER)
        top_box.append(meta_box)

        name_lbl = Gtk.Label(label=item.get("name", "Unnamed"))
        name_lbl.add_css_class("title-1")
        name_lbl.set_halign(Gtk.Align.START)
        meta_box.append(name_lbl)

        dev_lbl = Gtk.Label(label=f"By @{item.get('author', 'Community')}  •  v{item.get('version', '1.0')}")
        dev_lbl.add_css_class("dim-label")
        dev_lbl.set_halign(Gtk.Align.START)
        meta_box.append(dev_lbl)

        # Action Buttons / Live Spinner in Details
        act_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        act_box.set_valign(Gtk.Align.CENTER)
        top_box.append(act_box)

        if item_id in self.active_operations:
            spin_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
            spinner = Gtk.Spinner(spinning=True)
            spinner.set_size_request(24, 24)
            spin_box.append(spinner)
            op_lbl = Gtk.Label(label=f"{self.active_operations[item_id].capitalize()}ing...")
            op_lbl.add_css_class("dim-label")
            spin_box.append(op_lbl)
            act_box.append(spin_box)
        else:
            is_inst = self.core.is_installed(item)
            if not is_inst:
                btn_install = Gtk.Button(label="Install")
                btn_install.add_css_class("suggested-action")
                btn_install.add_css_class("pill-action")
                btn_install.connect("clicked", lambda b: self.perform_action(item, "install"))
                act_box.append(btn_install)
            else:
                btn_uninstall = Gtk.Button(label="Uninstall")
                btn_uninstall.add_css_class("destructive-action")
                btn_uninstall.add_css_class("pill-action")
                btn_uninstall.connect("clicked", lambda b: self.perform_action(item, "uninstall"))
                act_box.append(btn_uninstall)

                btn_update = Gtk.Button(label="Reinstall")
                btn_update.add_css_class("pill-action")
                btn_update.connect("clicked", lambda b: self.perform_action(item, "update"))
                act_box.append(btn_update)

        # Overview Group
        desc_group = Adw.PreferencesGroup()
        desc_group.set_title("About")
        self.detail_box.append(desc_group)

        desc_row = Adw.ActionRow()
        desc_text = item.get("description", item.get("summary", ""))
        desc_lbl = Gtk.Label(label=desc_text)
        desc_lbl.set_wrap(True)
        desc_lbl.set_halign(Gtk.Align.START)
        desc_lbl.set_margin_top(12)
        desc_lbl.set_margin_bottom(12)
        desc_lbl.set_margin_start(12)
        desc_lbl.set_margin_end(12)
        desc_row.set_child(desc_lbl)
        desc_group.add(desc_row)

        # Security Audit Group
        sec = item.get("security_report")
        if sec:
            sec_group = Adw.PreferencesGroup()
            sec_group.set_title("Security Audit")
            self.detail_box.append(sec_group)

            score_row = Adw.ActionRow(title="Audit Score")
            score_lbl = Gtk.Label(label=f"{sec.get('score', 90)}/100 ({sec.get('status', 'PASSED')})")
            score_lbl.add_css_class("score-badge")
            score_row.add_suffix(score_lbl)
            sec_group.add(score_row)

            if sec.get("audited_by"):
                aud_row = Adw.ActionRow(title="Audited By")
                aud_lbl = Gtk.Label(label=sec.get("audited_by"))
                aud_lbl.add_css_class("dim-label")
                aud_row.add_suffix(aud_lbl)
                sec_group.add(aud_row)

            if sec.get("summary"):
                sum_row = Adw.ActionRow()
                sum_lbl = Gtk.Label(label=sec.get("summary"))
                sum_lbl.set_wrap(True)
                sum_lbl.set_halign(Gtk.Align.START)
                sum_lbl.set_margin_top(10)
                sum_lbl.set_margin_bottom(10)
                sum_lbl.set_margin_start(12)
                sum_lbl.set_margin_end(12)
                sum_row.set_child(sum_lbl)
                sec_group.add(sum_row)

        # Package Files & Destination Group
        files_group = Adw.PreferencesGroup()
        files_group.set_title("Package Files & Destination")
        self.detail_box.append(files_group)

        pkg_files = self.get_package_files(item)
        for f in pkg_files:
            file_row = Adw.ActionRow(title=f["path"], subtitle=f["desc"])
            img = Gtk.Image.new_from_icon_name(f.get("icon", "text-x-generic-symbolic"))
            img.set_pixel_size(20)
            file_row.add_prefix(img)
            files_group.add(file_row)

        # Documentation Preview Group
        raw_content = item.get("raw_content") or item.get("skill_md")
        readme_url = item.get("readme_url")

        if raw_content or readme_url:
            doc_group = Adw.PreferencesGroup()
            doc_group.set_title("Documentation & Preview")
            self.detail_box.append(doc_group)

            doc_row = Adw.ActionRow()
            doc_scroll = Gtk.ScrolledWindow()
            doc_scroll.set_min_content_height(140)
            doc_scroll.set_max_content_height(280)
            doc_scroll.set_hexpand(True)
            doc_scroll.set_vexpand(False)

            text_view = Gtk.TextView()
            text_view.set_editable(False)
            text_view.set_cursor_visible(False)
            text_view.set_wrap_mode(Gtk.WrapMode.WORD_CHAR)
            text_view.set_monospace(True)
            text_view.add_css_class("doc-preview-box")

            text_buffer = text_view.get_buffer()

            if raw_content:
                text_buffer.set_text(raw_content)
            elif readme_url:
                text_buffer.set_text("Loading README documentation from repository...")
                def _fetch_readme(url=readme_url, buf=text_buffer):
                    try:
                        req = urllib.request.Request(url, headers={"User-Agent": "PulsarStore/1.0"})
                        with urllib.request.urlopen(req, timeout=5) as resp:
                            if resp.status == 200:
                                content = resp.read().decode("utf-8")
                                GLib.idle_add(buf.set_text, content)
                    except Exception as e:
                        GLib.idle_add(buf.set_text, f"Could not load README from {url}: {e}")
                threading.Thread(target=_fetch_readme, daemon=True).start()

            doc_scroll.set_child(text_view)
            doc_row.set_child(doc_scroll)
            doc_group.add(doc_row)

        # Package Details Group
        info_group = Adw.PreferencesGroup()
        info_group.set_title("Package Details")
        self.detail_box.append(info_group)

        info_data = [
            ("Package ID", item.get("id", "-")),
            ("Format", item.get("type", "-").replace("_", " ").upper()),
            ("Version", str(item.get("version", "1.0"))),
            ("Developer", item.get("author", "Community")),
        ]

        for title, value in info_data:
            row = Adw.ActionRow(title=title)
            val_lbl = Gtk.Label(label=value)
            val_lbl.add_css_class("dim-label")
            row.add_suffix(val_lbl)
            info_group.add(row)

    def get_package_files(self, item: Dict[str, Any]) -> List[Dict[str, str]]:
        item_id = item.get("id", "")
        item_type = item.get("type", "")
        if item_type == "sayri_skill":
            return [
                {"path": f"~/.local/share/sayri/skills/{item_id}/SKILL.md", "desc": "Skill system prompt, persona & tool definitions", "icon": "text-x-generic-symbolic"},
                {"path": f"~/.local/share/sayri/skills/{item_id}/metadata.json", "desc": "Security audit parameters & manifest", "icon": "document-properties-symbolic"},
            ]
        elif item_type == "sayri_plugin":
            return [
                {"path": f"~/.local/share/sayri/plugins/{item_id}/manifest.json", "desc": "Plugin sandbox manifest & OTP pairing configuration", "icon": "document-properties-symbolic"},
                {"path": f"~/.local/share/sayri/plugins/{item_id}/gateway.py", "desc": "Bridge service executable script", "icon": "system-run-symbolic"},
                {"path": f"~/.local/share/sayri/plugins/{item_id}/README.md", "desc": "Setup guide and token instructions", "icon": "text-x-generic-symbolic"},
            ]
        elif item_type == "gnome_extension":
            uuid = item.get("metadata", {}).get("uuid", item_id)
            return [
                {"path": f"~/.local/share/gnome-shell/extensions/{uuid}/metadata.json", "desc": "GNOME Shell UUID metadata definition", "icon": "document-properties-symbolic"},
                {"path": f"~/.local/share/gnome-shell/extensions/{uuid}/extension.js", "desc": "GNOME JavaScript (GJS) extension code", "icon": "application-x-executable-symbolic"},
                {"path": f"~/.local/share/gnome-shell/extensions/{uuid}/stylesheet.css", "desc": "Extension visual stylesheet", "icon": "image-x-generic-symbolic"},
                {"path": f"~/.local/share/gnome-shell/extensions/{uuid}/schemas/", "desc": "GSettings configuration schemas", "icon": "folder-symbolic"},
            ]
        elif item_type == "flatpak":
            return [
                {"path": f"/var/lib/flatpak/app/{item_id}/", "desc": "Sandboxed OSTree runtime & application binaries", "icon": "package-x-generic-symbolic"},
                {"path": f"/var/lib/flatpak/exports/share/applications/{item_id}.desktop", "desc": "Desktop launch entry", "icon": "preferences-desktop-display-symbolic"},
                {"path": f"/var/lib/flatpak/exports/share/icons/hicolor/128x128/apps/{item_id}.png", "desc": "High-resolution desktop icon", "icon": "image-x-generic-symbolic"},
            ]
        else:
            return [
                {"path": f"/usr/bin/{item_id}", "desc": "Application executable binary", "icon": "system-run-symbolic"},
                {"path": f"/usr/share/applications/{item_id}.desktop", "desc": "Desktop integration entry", "icon": "preferences-desktop-display-symbolic"},
            ]

    def render_updates_view(self):
        header_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        lbl = Gtk.Label(label="Updates")
        lbl.add_css_class("title-2")
        lbl.set_halign(Gtk.Align.START)
        lbl.set_hexpand(True)
        header_box.append(lbl)

        btn_update_all = Gtk.Button(label="Update All")
        btn_update_all.add_css_class("suggested-action")
        btn_update_all.add_css_class("pill-action")
        btn_update_all.connect("clicked", self.on_update_all_clicked)
        header_box.append(btn_update_all)

        self.browser_box.append(header_box)

        updates = self.core.check_updates()
        if not updates:
            btn_update_all.set_sensitive(False)
            status = Adw.StatusPage()
            status.set_icon_name("emblem-ok-symbolic")
            status.set_title("Up to Date")
            status.set_description("All packages, extensions, and AI plugins are up to date.")
            self.browser_box.append(status)
            return

        group = Adw.PreferencesGroup()
        self.browser_box.append(group)

        for update_info in updates:
            item = update_info["item"]
            row = Adw.ActionRow(
                title=item.get("name", "Unknown"),
                subtitle=f"v{update_info['current_version']} -> v{update_info['available_version']}"
            )
            icon_widget = self.get_item_icon_widget(item, size=24)
            row.add_prefix(icon_widget)

            btn = Gtk.Button(label="Update")
            btn.add_css_class("suggested-action")
            btn.add_css_class("pill-action")
            btn.set_valign(Gtk.Align.CENTER)
            btn.connect("clicked", lambda b, it=item: self.perform_action(it, "update"))
            row.add_suffix(btn)

            group.add(row)

    def perform_action(self, item: Dict[str, Any], action: str):
        item_id = item.get("id", "")
        if item_id in self.active_operations:
            self.show_toast("Operation already in progress for this item.")
            return

        self.active_operations[item_id] = action
        item_name = item.get("name", item_id)
        self.show_toast(f"{action.capitalize()}ing {item_name}...")

        # Re-render immediately to show spinner
        if self.current_item and self.current_item.get("id") == item_id:
            self.open_details(item)
        else:
            self.render_current_view()

        def _worker():
            success = False
            if action == "install":
                success = self.core.install(item)
            elif action == "uninstall":
                success = self.core.uninstall(item)
            elif action == "update":
                success = self.core.update_item(item)

            GLib.idle_add(self._after_action, item, action, success)

        threading.Thread(target=_worker, daemon=True).start()

    def _after_action(self, item: Dict[str, Any], action: str, success: bool):
        item_id = item.get("id", "")
        self.active_operations.pop(item_id, None)
        item_name = item.get("name", item_id)

        if success:
            self.show_toast(f"{item_name} completed.")
            self.show_success_dialog(item, action)
        else:
            self.show_toast(f"Failed to {action} {item_name}.")

        if self.current_item and self.current_item.get("id") == item_id:
            self.open_details(item)
        else:
            self.render_current_view()

    def on_update_all_clicked(self, _btn):
        if self.active_operations:
            return
        updates = self.core.check_updates()
        if not updates:
            return

        self.show_toast("Updating all components...")

        def _worker():
            for update_info in updates:
                it = update_info["item"]
                iid = it.get("id", "")
                self.active_operations[iid] = "update"
                GLib.idle_add(self.render_current_view)
                self.core.update_item(it)
                self.active_operations.pop(iid, None)
            GLib.idle_add(self._after_update_all)

        threading.Thread(target=_worker, daemon=True).start()

    def _after_update_all(self):
        self.show_toast("All updates applied.")
        self.render_current_view()


class PulsarStoreApp(Adw.Application):
    def __init__(self):
        super().__init__(
            application_id="es.inled.PulsarStore",
            flags=Gio.ApplicationFlags.FLAGS_NONE
        )

    def do_activate(self):
        win = self.props.active_window
        if not win:
            win = PulsarStoreWindow(self)
        win.present()


def main():
    app = PulsarStoreApp()
    return app.run(sys.argv)


if __name__ == "__main__":
    sys.exit(main())
