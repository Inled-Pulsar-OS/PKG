#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar OS - What's New viewer.

A small GTK4 / libadwaita window that renders the release notes with WebKit.
If WebKit is not available it falls back to a Pango-based text view so the
window still works.

Usage:
    pulsaros-whatsnew                 # newest note (plus history picker)
    pulsaros-whatsnew --ids a,b       # show the given notes, newest first
    pulsaros-whatsnew --all           # whole history in one page
    pulsaros-whatsnew --list          # print the installed note ids
"""

import argparse
import html
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import common  # noqa: E402


# ---------------------------------------------------------------------------
# Fallback renderer (no WebKit): a small Markdown -> Pango markup converter.
# ---------------------------------------------------------------------------
def _pango_inline(text):
    text = html.escape(text, quote=False)
    text = re.sub(r"\*\*([^*]+)\*\*", r"<b>\1</b>", text)
    text = re.sub(r"__([^_]+)__", r"<b>\1</b>", text)
    text = re.sub(r"(?<!\*)\*([^*\n]+)\*(?!\*)", r"<i>\1</i>", text)
    text = re.sub(r"`([^`]+)`", r"<tt>\1</tt>", text)
    text = re.sub(r"\[([^\]]+)\]\(([^)\s]+)\)", r"\1 (\2)", text)
    return text


def markdown_to_pango(markdown):
    lines = markdown.replace("\r\n", "\n").split("\n")
    out = []
    in_code = False
    for line in lines:
        if re.match(r"^\s*```", line):
            in_code = not in_code
            out.append("")
            continue
        if in_code:
            out.append("<tt>" + html.escape(line, quote=False) + "</tt>")
            continue
        heading = re.match(r"^(#{1,6})\s+(.*)$", line)
        if heading:
            level = len(heading.group(1))
            size = {1: "xx-large", 2: "x-large", 3: "large"}.get(level, "medium")
            out.append(
                f'<span size="{size}" weight="bold">{_pango_inline(heading.group(2))}</span>'
            )
            continue
        if re.match(r"^\s*([-*_])\s*\1\s*\1", line):
            out.append("────────────────────────")
            continue
        item = re.match(r"^\s*[-*+]\s+(.*)$", line)
        if item:
            out.append("  • " + _pango_inline(item.group(1)))
            continue
        ordered = re.match(r"^\s*\d+[.)]\s+(.*)$", line)
        if ordered:
            out.append("  " + _pango_inline(line.strip()))
            continue
        quote = re.match(r"^\s*>\s?(.*)$", line)
        if quote:
            out.append("<i>" + _pango_inline(quote.group(1)) + "</i>")
            continue
        out.append(_pango_inline(line) if line.strip() else "")
    return "\n".join(out)


# ---------------------------------------------------------------------------
# WebKit renderer
# ---------------------------------------------------------------------------
def _read_asset(assets_dir, name, default=""):
    try:
        with open(os.path.join(assets_dir, name), "r", encoding="utf-8") as fh:
            return fh.read()
    except OSError:
        return default


def build_html(notes, assets_dir, dark=False):
    template = _read_asset(assets_dir, "viewer.html")
    if not template.strip():
        raise RuntimeError(f"viewer.html not found in {assets_dir}")
    css = _read_asset(assets_dir, "style.css")
    js = _read_asset(assets_dir, "render.js")

    if dark:
        template = template.replace('<html lang="en">', '<html lang="en" class="dark">')

    payload = json.dumps(
        {
            "notes": [
                {
                    "id": n["id"],
                    "date": n["date"],
                    "title": n["title"],
                    "channel": n["channel"],
                    "important": n["important"],
                    "packages": n["packages"],
                    "markdown": n["body"],
                }
                for n in notes
            ]
        },
        ensure_ascii=False,
    )
    # Avoid an accidental </script> terminator inside the JSON payload.
    payload = payload.replace("<", "\\u003c")

    return (
        template.replace("/*__CSS__*/", css)
        .replace("/*__JS__*/", js)
        .replace("__PAYLOAD__", payload)
    )


class Viewer:
    def __init__(self, notes, assets_dir):
        import gi

        gi.require_version("Gtk", "4.0")
        gi.require_version("Adw", "1")
        from gi.repository import Adw

        self.notes = notes
        self.assets_dir = assets_dir
        self.adw = Adw
        self.webview = None
        self._dropdown = None
        self.app = Adw.Application(application_id=common.APP_ID)
        self.app.connect("activate", self.on_activate)

    def run(self):
        return self.app.run(["pulsaros-whatsnew"])

    # -- UI ---------------------------------------------------------------
    def on_activate(self, app):
        import gi

        gi.require_version("Gtk", "4.0")
        from gi.repository import Gtk

        window = self.adw.ApplicationWindow(application=app)
        window.set_title("What's New in Pulsar OS")
        window.set_default_size(760, 720)

        toolbar = self.adw.ToolbarView()
        header = self.adw.HeaderBar()
        toolbar.add_top_bar(header)

        header.set_title_widget(
            self.adw.WindowTitle(
                title="What's New in Pulsar OS", subtitle=self._subtitle()
            )
        )

        # History picker (only if there is more than one note installed).
        self.all_notes = common.load_notes()
        if len(self.all_notes) > 1:
            names = [self._label_for(n) for n in self.all_notes]
            dropdown = Gtk.DropDown.new_from_strings(names)
            current_ids = {n["id"] for n in self.notes}
            for idx, n in enumerate(self.all_notes):
                if n["id"] in current_ids:
                    dropdown.set_selected(idx)
                    break
            dropdown.set_tooltip_text("Version history")
            dropdown.connect("notify::selected", self._on_selected)
            header.pack_end(dropdown)
            self._dropdown = dropdown

        header.pack_end(self._build_menu())

        # Content: WebKit when available, Pango text view otherwise.
        webkit_ok = True
        try:
            content = self._build_webview()
        except Exception as exc:  # pragma: no cover - depends on the system
            webkit_ok = False
            content = self._build_fallback()
            print(f"pulsaros-whatsnew: WebKit not available ({exc}); using simple view.",
                  file=sys.stderr)

        scroller = Gtk.ScrolledWindow()
        scroller.set_vexpand(True)
        scroller.set_child(content)
        toolbar.set_content(scroller)

        # Bottom bar.
        bottom = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        bottom.set_margin_top(12)
        bottom.set_margin_bottom(12)
        bottom.set_margin_start(12)
        bottom.set_margin_end(12)
        if not webkit_ok:
            note = Gtk.Label(label="Simplified view")
            note.add_css_class("dim-label")
            bottom.append(note)
        spacer = Gtk.Box()
        spacer.set_hexpand(True)
        bottom.append(spacer)
        close_btn = Gtk.Button(label="Got it")
        close_btn.add_css_class("suggested-action")
        close_btn.connect("clicked", lambda *_: window.close())
        bottom.append(close_btn)
        toolbar.add_bottom_bar(bottom)

        window.set_content(toolbar)
        window.present()

    def _subtitle(self):
        if len(self.notes) == 1:
            n = self.notes[0]
            return " · ".join(x for x in (n["date"], n["channel"] or None) if x)
        return "Accumulated changes"

    def _label_for(self, note):
        label = note["id"]
        if note["title"]:
            label += f" — {note['title']}"
        if note["important"]:
            label = "★ " + label
        return label

    def _build_menu(self):
        import gi

        gi.require_version("Gtk", "4.0")
        from gi.repository import Gtk, Gio

        menu = Gio.Menu()
        menu.append("Disable automatic notifications", "app.disable_autoshow")
        menu.append("About", "app.about")

        button = Gtk.MenuButton()
        button.set_icon_name("open-menu-symbolic")
        button.set_menu_model(menu)
        button.set_tooltip_text("Menu")

        disable = Gio.SimpleAction.new("disable_autoshow", None)
        disable.connect("activate", self._on_disable)
        self.app.add_action(disable)

        about = Gio.SimpleAction.new("about", None)
        about.connect("activate", self._on_about)
        self.app.add_action(about)
        return button

    def _on_disable(self, *_args):
        state = common.load_state()
        state["disabled"] = True
        common.save_state(state)

    def _on_about(self, *_args):
        self.adw.AboutDialog(
            application_name="What's New in Pulsar OS",
            application_icon="pulsaros-whatsnew",
            version="1.0",
            comments="Pulsar OS changelog.",
            website="https://inled.es",
        ).present(self.app.props.active_window)

    def _on_selected(self, dropdown, _param):
        idx = dropdown.get_selected()
        if 0 <= idx < len(getattr(self, "all_notes", [])):
            self.notes = [self.all_notes[idx]]
            self._reload()

    def _is_dark(self):
        try:
            return bool(self.adw.StyleManager.get_default().get_dark())
        except Exception:
            return False

    def _reload(self):
        if self.webview is not None:
            try:
                self.webview.load_html(
                    build_html(self.notes, self.assets_dir, dark=self._is_dark()),
                    "file://" + self.assets_dir + "/",
                )
            except Exception:
                pass

    def _build_webview(self):
        import gi

        gi.require_version("WebKit", "6.0")
        from gi.repository import WebKit

        settings = WebKit.Settings()
        settings.set_enable_developer_extras(False)
        webview = WebKit.WebView()
        webview.set_settings(settings)
        webview.load_html(
            build_html(self.notes, self.assets_dir, dark=self._is_dark()),
            "file://" + self.assets_dir + "/",
        )
        webview.connect("decide-policy", self._on_decide_policy)
        # Re-render when the system switches between light and dark.
        try:
            self.adw.StyleManager.get_default().connect(
                "notify::dark", lambda *_a: self._reload()
            )
        except Exception:
            pass
        self.webview = webview
        return webview

    def _on_decide_policy(self, webview, decision, decision_type):
        from gi.repository import WebKit

        if decision_type == WebKit.PolicyDecisionType.NAVIGATION_ACTION:
            action = decision.get_navigation_action()
            uri = action.get_request().get_uri() or ""
            if uri.startswith("file://") or uri.startswith("about:"):
                return False
            try:
                from gi.repository import Gio

                Gio.AppInfo.launch_default_for_uri(uri, None)
            except Exception:
                pass
            decision.ignore()
            return True
        return False

    def _build_fallback(self):
        import gi

        gi.require_version("Gtk", "4.0")
        from gi.repository import Gtk

        parts = []
        for note in self.notes:
            title = note["title"] or note["id"]
            parts.append(
                f'<span size="x-large" weight="bold">{html.escape(title)}</span>'
            )
            parts.append("")
            parts.append(markdown_to_pango(note["body"]))
            parts.append("")
        view = Gtk.TextView()
        view.set_editable(False)
        view.set_cursor_visible(False)
        view.set_wrap_mode(Gtk.WrapMode.WORD_CHAR)
        view.set_margin_top(18)
        view.set_margin_bottom(18)
        view.set_margin_start(18)
        view.set_margin_end(18)
        view.get_buffer().set_text("\n".join(parts))
        return view


def main():
    if common.is_live_or_pre_setup():
        return 0

    parser = argparse.ArgumentParser(description="Pulsar OS What's New viewer")
    parser.add_argument("--ids", help="Comma-separated note ids to show")
    parser.add_argument("--all", action="store_true", help="Show the whole history")
    parser.add_argument("--list", action="store_true", help="List installed note ids")
    args = parser.parse_args()

    notes = common.load_notes()

    if args.list:
        for note in notes:
            print(note["id"])
        return 0

    if args.all:
        selected = notes
    elif args.ids:
        wanted = [i.strip() for i in args.ids.split(",") if i.strip()]
        by_id = {n["id"]: n for n in notes}
        selected = [by_id[i] for i in wanted if i in by_id]
    else:
        selected = notes[:1]

    if not selected:
        selected = notes[:1]

    if not selected:
        print("pulsaros-whatsnew: no release notes available.", file=sys.stderr)
        return 0

    return Viewer(selected, common.ASSETS_DIR).run()


if __name__ == "__main__":
    sys.exit(main())
