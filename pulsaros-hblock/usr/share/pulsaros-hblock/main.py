#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar HBlock - visual switch and statistics for hblock.
Native Libadwaita / GTK4 interface.
"""

import os
import sys
import threading

import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Adw, GLib, Gdk  # noqa: E402

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if SCRIPT_DIR not in sys.path:
    sys.path.insert(0, SCRIPT_DIR)

# ---------------------------------------------------------------------------
# Localization: sources are Spanish; the en catalog ships the English texts.
# ---------------------------------------------------------------------------
import gettext  # noqa: E402
import locale  # noqa: E402


def _setup_lang():
    try:
        locale.setlocale(locale.LC_ALL, "")
    except Exception:
        pass
    env = (
        os.environ.get("LANGUAGE")
        or os.environ.get("LC_ALL")
        or os.environ.get("LC_MESSAGES")
        or os.environ.get("LANG")
        or ""
    )
    lang = env.split(".")[0].split(":")[0].strip()
    if lang.lower() in ("c", "posix", ""):
        lang = "en"
    return lang


lang = _setup_lang()
_LOCALE_DIR = os.path.join(SCRIPT_DIR, "locale")


def _make_gettext():
    # Spanish systems: the source strings are already Spanish, so no catalog
    # lookup (the en catalog would otherwise translate Spanish users to English).
    if lang.lower().startswith("es"):
        try:
            trans = gettext.translation(
                "pulsaros-hblock", localedir=_LOCALE_DIR, languages=["es"], fallback=True
            )
            return trans.gettext
        except Exception:
            return gettext.gettext

    # Any other language: prefer the specific catalog, always fall back to 'en'.
    candidates = []
    if lang:
        candidates.append(lang)
        short = lang.split("_")[0]
        if short not in candidates:
            candidates.append(short)
    if "en" not in candidates:
        candidates.append("en")
    try:
        trans = gettext.translation(
            "pulsaros-hblock", localedir=_LOCALE_DIR, languages=candidates, fallback=False
        )
        return trans.gettext
    except Exception:
        return gettext.gettext


_ = _make_gettext()

from hblock_core import get_status, run_action  # noqa: E402


class PulsarHBlockWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app, title="HBlock")
        self.set_default_size(560, 640)
        self.set_size_request(440, 560)

        self.toast_overlay = Adw.ToastOverlay()
        self.set_content(self.toast_overlay)

        self.load_css()
        self.build_ui()
        self.refresh_async()

    # ------------------------------------------------------------------ UI
    def load_css(self):
        css_file = os.path.join(SCRIPT_DIR, "styles.css")
        if os.path.exists(css_file):
            provider = Gtk.CssProvider()
            provider.load_from_path(css_file)
            Gtk.StyleContext.add_provider_for_display(
                Gdk.Display.get_default(), provider,
                Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION,
            )

    def build_ui(self):
        toolbar = Adw.ToolbarView()
        self.toast_overlay.set_child(toolbar)

        header = Adw.HeaderBar()
        header.set_title_widget(Adw.WindowTitle(title=_("HBlock"), subtitle=_("Bloqueo de anuncios y rastreadores")))
        header.add_css_class("header-bar")

        refresh_btn = Gtk.Button(icon_name="view-refresh-symbolic")
        refresh_btn.set_tooltip_text(_("Recargar"))
        refresh_btn.connect("clicked", lambda *_: self.refresh_async())
        header.pack_end(refresh_btn)
        toolbar.add_top_bar(header)

        clamp = Adw.Clamp()
        clamp.set_maximum_size(560)
        clamp.set_tightening_threshold(420)
        toolbar.set_content(clamp)

        content = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        content.set_margin_top(24)
        content.set_margin_bottom(24)
        content.set_margin_start(24)
        content.set_margin_end(24)
        clamp.set_child(content)

        # --- Status card -------------------------------------------------
        status_card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        status_card.add_css_class("card")
        status_card.add_css_class("hblock-status-card")

        status_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=14)
        icon_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.status_icon = Gtk.Image.new_from_icon_name("pulsaros-hblock")
        self.status_icon.set_pixel_size(56)
        icon_box.append(self.status_icon)

        texts = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        self.status_title = Gtk.Label(label=_("Bloqueo de anuncios y rastreadores"), xalign=0)
        self.status_title.add_css_class("title-label")
        self.status_subtitle = Gtk.Label(label=_("Comprobando…"), xalign=0)
        self.status_subtitle.add_css_class("dim-label")
        texts.append(self.status_title)
        texts.append(self.status_subtitle)

        status_row.append(icon_box)
        status_row.append(texts)

        switch_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        switch_box.set_valign(Gtk.Align.CENTER)
        self.switch = Gtk.Switch()
        self.switch.set_valign(Gtk.Align.CENTER)
        self.switch.connect("state-set", self.on_switch_toggled)
        switch_box.append(self.switch)

        status_row.append(switch_box)
        status_card.append(status_row)

        self.not_installed_label = Gtk.Label(
            label=_("hblock no está instalado en el sistema. Instala el paquete 'hblock' para usar esta aplicación."),
            wrap=True, xalign=0,
        )
        self.not_installed_label.add_css_class("error")
        self.not_installed_label.set_visible(False)
        status_card.append(self.not_installed_label)

        content.append(status_card)

        # --- Stats card ---------------------------------------------------
        stats_card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        stats_card.add_css_class("card")

        stats_title = Gtk.Label(label=_("Estadísticas"), xalign=0)
        stats_title.add_css_class("heading")
        stats_card.append(stats_title)

        self.blocked_num = self._stat_label()
        self.blocked_caption = self._dim_label(_("Dominios bloqueados"))
        self.blocked_col = self._stat_column(self.blocked_num, self.blocked_caption)

        self.sources_num = self._stat_label()
        self.sources_caption = self._dim_label(_("Fuentes activas"))
        self.sources_col = self._stat_column(self.sources_num, self.sources_caption)

        stats_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=24)
        stats_row.set_halign(Gtk.Align.FILL)
        stats_row.append(self.blocked_col)
        stats_row.append(self.sources_col)
        stats_card.append(stats_row)

        details = Gtk.Grid(column_spacing=12, row_spacing=8)
        details.set_halign(Gtk.Align.FILL)

        self.updated_value = self._detail_row(details, _("Última actualización"), 0)
        self.version_value = self._detail_row(details, _("Versión de hblock"), 1)
        stats_card.append(details)

        content.append(stats_card)

        # --- Actions -------------------------------------------------------
        actions_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        self.update_btn = Gtk.Button(label=_("Actualizar ahora"))
        self.update_btn.add_css_class("suggested-action")
        self.update_btn.connect("clicked", self.on_update_clicked)
        actions_box.append(self.update_btn)

        self.update_spinner = Gtk.Spinner()
        self.update_spinner.set_visible(False)
        actions_box.append(self.update_spinner)

        note = Gtk.Label(
            label=_("hblock actualiza /etc/hosts con los dominios de sus fuentes para bloquear anuncios, rastreadores y malware."),
            wrap=True, xalign=0,
        )
        note.add_css_class("dim-label")
        actions_box.set_halign(Gtk.Align.START)
        actions_box.set_valign(Gtk.Align.CENTER)
        content.append(actions_box)
        content.append(note)

    def _stat_label(self):
        lbl = Gtk.Label(label="—")
        lbl.add_css_class("hblock-big-number")
        lbl.set_hexpand(True)
        return lbl

    def _dim_label(self, text):
        lbl = Gtk.Label(label=text, xalign=0)
        lbl.add_css_class("dim-label")
        return lbl

    def _stat_column(self, number, caption):
        col = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        col.set_hexpand(True)
        num_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        num_box.set_halign(Gtk.Align.START)
        num_box.append(number)
        col.append(num_box)
        cap_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        cap_box.set_halign(Gtk.Align.START)
        cap_box.append(caption)
        col.append(cap_box)
        return col

    def _detail_row(self, grid, label_text, row):
        lbl = Gtk.Label(label=label_text, xalign=0)
        lbl.add_css_class("dim-label")
        grid.attach(lbl, 0, row, 1, 1)
        value = Gtk.Label(label="—", xalign=0)
        grid.attach(value, 1, row, 1, 1)
        return value

    # -------------------------------------------------------------- state
    def refresh_async(self):
        def work():
            state = get_status()
            GLib.idle_add(self._apply_status, state)

        threading.Thread(target=work, daemon=True).start()

    def _apply_status(self, state):
        installed = state.get("installed", False)
        enabled = state.get("enabled", False)

        self.switch.set_state(enabled)
        self.switch.set_sensitive(installed)
        self.update_btn.set_sensitive(installed)
        self.not_installed_label.set_visible(not installed)
        self.status_icon.set_from_icon_name("pulsaros-hblock")

        if enabled:
            self.status_subtitle.set_label(_("Activado"))
            self.blocked_num.set_label(f"{state.get('blocked', 0):,}" if state.get("blocked") else "—")
        else:
            self.status_subtitle.set_label(_("Desactivado"))
            self.blocked_num.set_label("—")

        srcs = state.get("sources")
        self.sources_num.set_label(str(srcs) if srcs else "—")

        self.updated_value.set_label(state.get("last_update") or "—")
        self.version_value.set_label(state.get("version") or "—")

    # ------------------------------------------------------------ actions
    def on_switch_toggled(self, switch, new_state):
        desired = bool(new_state)
        switch.set_state(not desired)  # optimistic flip only after success
        switch.set_sensitive(False)

        def work():
            ok, msg = run_action("enable" if desired else "disable")
            GLib.idle_add(self._switch_done, switch, desired, ok, msg)

        threading.Thread(target=work, daemon=True).start()
        return True

    def _switch_done(self, switch, desired, ok, msg):
        switch.set_sensitive(True)
        if ok:
            switch.set_state(desired)
            self._toast(_("Bloqueo activado") if desired else _("Bloqueo desactivado"))
            self.refresh_async()
        else:
            switch.set_state(not desired)
            self._toast(_("No se pudo cambiar el estado del bloqueo."))

    def on_update_clicked(self, btn):
        btn.set_sensitive(False)
        self.update_spinner.set_visible(True)
        self.update_spinner.start()

        def work():
            ok, msg = run_action("update")
            GLib.idle_add(self._update_done, ok, msg)

        threading.Thread(target=work, daemon=True).start()

    def _update_done(self, ok, msg):
        self.update_spinner.stop()
        self.update_spinner.set_visible(False)
        self.update_btn.set_sensitive(True)
        if ok:
            self._toast(_("Lista actualizada correctamente."))
        else:
            self._toast(_("No se pudo actualizar la lista: {msg}").format(msg=msg))
        self.refresh_async()

    def _toast(self, message):
        toast = Adw.Toast.new(message)
        toast.set_timeout(4)
        self.toast_overlay.add_toast(toast)


class PulsarHBlockApp(Adw.Application):
    def __init__(self):
        super().__init__(application_id="es.inled.PulsarHBlock")

    def do_activate(self):
        win = PulsarHBlockWindow(self)
        win.present()


def main():
    app = PulsarHBlockApp()
    app.run(sys.argv)


if __name__ == "__main__":
    main()