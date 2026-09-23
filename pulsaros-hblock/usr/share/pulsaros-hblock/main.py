#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Ad Block - simple HTML interface for the hblock DNS ad-blocker.

The page is a local WebKitGTK app. JavaScript talks to Python through the
WebKit script-message handler named ``pulsarAction``; Python runs the
root-free status reads and the privileged helper through pkexec.
"""

import json
import os
import sys
import threading
import urllib.parse

import gi

gi.require_version("WebKit", "6.0")
from gi.repository import WebKit, GLib  # noqa: E402

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if SCRIPT_DIR not in sys.path:
    sys.path.insert(0, SCRIPT_DIR)

# ---------------------------------------------------------------------------
# Localization: Spanish source strings; only an English catalog ships.
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
    if lang.lower().startswith("es"):
        try:
            return gettext.translation(
                "pulsaros-hblock", localedir=_LOCALE_DIR, languages=["es"], fallback=True
            ).gettext
        except Exception:
            return gettext.gettext

    candidates = []
    if lang:
        candidates.append(lang)
        short = lang.split("_")[0]
        if short not in candidates:
            candidates.append(short)
    if "en" not in candidates:
        candidates.append("en")
    try:
        return gettext.translation(
            "pulsaros-hblock", localedir=_LOCALE_DIR, languages=candidates, fallback=False
        ).gettext
    except Exception:
        return gettext.gettext


_ = _make_gettext()

from hblock_core import get_status, run_action  # noqa: E402

I18N = {
    "title": "Ad Block",
    "subtitleEnabled": _("Estás protegido"),
    "subtitleDisabled": _("No estás bloqueando"),
    "btnMainOff": _("No estás bloqueando"),
    "btnMainOn": _("Estás protegido"),
    "btnSubOff": _("Toca para protegerte"),
    "btnSubOn": _("Toca para desactivar"),
    "blocked": _("Dominios bloqueados"),
    "sources": _("Fuentes activas"),
    "updated": _("Última actualización"),
    "version": _("Versión de hblock"),
    "hint": _("hblock actualiza /etc/hosts con los dominios de sus fuentes para bloquear anuncios, rastreadores y malware."),
    "installed": _("hblock instalado"),
    "notInstalled": _("hblock no está instalado en el sistema. Instala el paquete 'hblock' para usar esta aplicación."),
    "checking": _("Comprobando…"),
    "updating": _("Actualizando…"),
    "enabled": _("Bloqueo activado"),
    "disabled": _("Bloqueo desactivado"),
    "ok": _("Listo."),
    "error": _("No se pudo cambiar el estado del bloqueo."),
    "updateOk": _("Lista actualizada correctamente."),
    "updateError": _("No se pudo actualizar la lista: {msg}"),
}


def _json_for_js(value):
    """Return a JSON string suitable for direct use in JavaScript."""
    return json.dumps(value, ensure_ascii=False)


class AdBlockWindow:
    def __init__(self):
        self.webview = WebKit.WebView()
        self.manager = self.webview.get_user_content_manager()
        self.manager.register_script_message_handler("pulsarAction")
        self.manager.connect("script-message-received::pulsarAction", self._on_script_message)

        # Inject translations before the local page executes (at START).
        js_i18n = "window.I18N = %s;" % _json_for_js(I18N)
        script = WebKit.UserScript.new(
            js_i18n,
            WebKit.UserContentInjectedFrames.TOP_FRAME,
            WebKit.UserScriptInjectionTime.START,
        )
        self.manager.add_script(script)

        # Load the local HTML file.
        page_uri = "file://" + urllib.parse.quote(SCRIPT_DIR + "/app.html")
        self.webview.load_uri(page_uri)

        # Connect to load-changed to refresh status after load.
        self.webview.connect("load-changed", self._on_load_changed)

        # Initial status.
        self.status = get_status()

    def _on_load_changed(self, webview, load_event):
        """When the page finishes loading, push the current status."""
        if load_event == WebKit.LoadEvent.FINISHED:
            self._push_status()

    def _push_status(self):
        """Push the current status to the HTML page."""
        js = "updateUI(%s);" % _json_for_js(self.status)
        try:
            self.webview.evaluate_javascript(js, len(js), cancellable=None)
        except Exception as exc:
            print("Ad Block: could not push status to page: %s" % exc, file=sys.stderr)

    def _on_script_message(self, _manager, message):
        """Handle a ``pulsar://``-like action sent by JavaScript."""
        try:
            payload = json.loads(message.get_string())
        except Exception:
            return

        action = payload.get("action")
        if action == "status":
            GLib.idle_add(self._apply_status, get_status())
            return

        if action == "toggle":
            self._apply_status(get_status(), busy=True)
            threading.Thread(target=self._toggle_action, daemon=True).start()
            return

        if action == "enable":
            threading.Thread(target=lambda: self._apply_status(run_action("enable")), daemon=True).start()
            return

        if action == "disable":
            threading.Thread(target=lambda: self._apply_status(run_action("disable")), daemon=True).start()
            return

        if action == "update":
            threading.Thread(target=self._update_action, daemon=True).start()
            return

    def _apply_status(self, result, busy=False):
        """Apply a status/action result to the page."""
        if busy:
            try:
                self.webview.evaluate_javascript("setBusy(true);", len("setBusy(true);"), cancellable=None)
            except Exception:
                pass
            return

        if isinstance(result, tuple):
            ok, message = result
            status = get_status()
            status["message"] = message
            status["ok"] = ok
        else:
            status = result
            status.setdefault("ok", True)

        self.status = status
        self._push_status()

    def _toggle_action(self):
        desired = not self.status.get("enabled", False)
        ok, message = run_action("enable" if desired else "disable")
        status = get_status()
        status["message"] = message
        status["ok"] = ok
        GLib.idle_add(self._apply_status, status)

    def _update_action(self):
        try:
            self.webview.evaluate_javascript("setBusy(true);", len("setBusy(true);"), cancellable=None)
        except Exception:
            pass
        ok, message = run_action("update")
        status = get_status()
        status["message"] = message
        status["ok"] = ok
        GLib.idle_add(self._apply_status, status)


def main():
    window = AdBlockWindow()
    # Keep the GTK/WebKit event loop alive.
    GLib.MainLoop().run()


if __name__ == "__main__":
    main()