#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar OS - Feedback & System Logs Reporter
Opens a pre-formatted GitHub issue in the Inled-Pulsar-OS/PKG repository
with user comments and system hardware specs, and displays a prominent
file hand-off screen to easily attach the complete boot logs via Nautilus.

Supports English and Spanish with automatic English fallback when system locale
is not Spanish.
"""

import datetime
import os
import platform
import subprocess
import sys
import threading
import urllib.parse

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Adw, Gdk, Gio, GLib, Gtk

APP_ID = "es.inled.PulsarFeedback"
GITHUB_REPO_URL = "https://github.com/Inled-Pulsar-OS/PKG"
GITHUB_NEW_ISSUE_URL = f"{GITHUB_REPO_URL}/issues/new"


def get_current_language():
    """Detects system language. Returns 'es' if Spanish, otherwise 'en' (fallback)."""
    lang = (
        os.environ.get("LC_ALL")
        or os.environ.get("LC_MESSAGES")
        or os.environ.get("LANG")
        or ""
    ).lower()
    return "es" if lang.startswith("es") else "en"


STRINGS = {
    "es": {
        "app_title": "Feedback y Diagnóstico - Pulsar OS",
        "banner_title": "Pulsar OS Feedback",
        "banner_desc": "Envía tus comentarios, sugerencias o reportes. La información del sistema y los logs del arranque se incluirán automáticamente.",
        "subject_group": "Asunto (opcional)",
        "subject_row": "Título del reporte",
        "subject_placeholder": "Ej: Problema al conectar Bluetooth, sugerencia...",
        "desc_group": "Descripción",
        "desc_instructions": "Escribe aquí tu mensaje, feedback o lo que estabas haciendo cuando ocurrió el problema:",
        "data_group": "Datos que se enviarán",
        "expander_title": "📋 Ver todo lo que se mandará",
        "expander_sub": "Información del sistema y logs desde que arrancó el PC",
        "loading_logs": "Cargando logs del sistema desde el arranque...",
        "reading_journal": "Recopilando logs de journalctl...",
        "copy_all": "Copiar todo al portapapeles",
        "submit_btn": "Enviar Feedback a GitHub",
        "github_notice": "Se requiere tener cuenta de GitHub para publicar el issue en el repositorio PKG.",
        "logs_gathered": "✓ Logs recopilados ({:,} líneas).",
        "full_file": " Archivo completo: {}",
        "report_copied": "📋 Reporte y logs completos copiados al portapapeles",
        "toast_opened": "🚀 ¡Issue abierta en GitHub! Adjunta el archivo indicado.",
        "open_browser_fallback": "🚀 Abriendo navegador...",
        "default_title": "Reporte de Feedback / Diagnóstico",
        "sec_user_desc": "### Descripción del usuario",
        "no_user_desc": "_Sin descripción escrita._",
        "sec_sys_info": "### Información del Sistema",
        "sec_boot_errors": "### Errores Detectados en el Arranque",
        "sec_full_logs": "### Logs Completos (journalctl -b)",
        "details_summary": "Desplegar registro de logs",
        "full_log_note": "> 📄 Log completo del sistema generado en: `{}`",
        "sys_label": "Sistema",
        "kernel_label": "Kernel",
        "desktop_label": "Entorno de escritorio",
        "uptime_label": "Tiempo encendido",
        "ram_label": "Memoria RAM",
        "cpu_label": "Procesador",
        "gpu_label": "Gráficos",
        "no_errors": "No se detectaron errores críticos en journalctl.",
        "error_reading": "Error leyendo logs: {}",
        # Success Screen Strings
        "success_title": "¡Issue abierta en GitHub!",
        "success_desc": "GitHub limita el texto de los issues a 6.000 caracteres. Para incluir todos los logs del sistema, debes adjuntar el archivo generado.",
        "file_card_title": "Archivo de logs generado listo para adjuntar:",
        "btn_open_nautilus": "Ver archivo en Nautilus",
        "btn_back_form": "Volver al formulario",
        "step_instructions_title": "💡 ¿Cómo adjuntarlo a GitHub?",
        "step_instruction_1": "1. Pulsa el botón 'Ver archivo en Nautilus' abajo.",
        "step_instruction_2": "2. Arrastra el archivo seleccionado directamente a la caja de texto del issue en GitHub (o usa el botón 'Attach files' abajo a la izquierda).",
        "body_attached_note": "> 📎 **Archivo de logs:** Adjuntando `{}` a este reporte.",
    },
    "en": {
        "app_title": "Feedback & Diagnostics - Pulsar OS",
        "banner_title": "Pulsar OS Feedback",
        "banner_desc": "Submit your feedback, suggestions, or bug reports. System specifications and boot logs will be automatically included.",
        "subject_group": "Subject (optional)",
        "subject_row": "Report title",
        "subject_placeholder": "E.g.: Issue connecting Bluetooth, suggestion...",
        "desc_group": "Description",
        "desc_instructions": "Enter your message, feedback, or what you were doing when the issue occurred:",
        "data_group": "Data to be submitted",
        "expander_title": "📋 View all data to be submitted",
        "expander_sub": "System specifications and logs since PC boot",
        "loading_logs": "Loading system logs since boot...",
        "reading_journal": "Gathering journalctl logs...",
        "copy_all": "Copy all to clipboard",
        "submit_btn": "Submit Feedback to GitHub",
        "github_notice": "A GitHub account is required to publish the issue in the PKG repository.",
        "logs_gathered": "✓ Boot logs gathered ({:,} lines).",
        "full_file": " Full log file: {}",
        "report_copied": "📋 Full report and logs copied to clipboard",
        "toast_opened": "🚀 Issue opened on GitHub! Please attach the logs file.",
        "open_browser_fallback": "🚀 Opening browser...",
        "default_title": "Feedback / Diagnostic Report",
        "sec_user_desc": "### User Description",
        "no_user_desc": "_No description provided._",
        "sec_sys_info": "### System Information",
        "sec_boot_errors": "### Detected Boot Errors",
        "sec_full_logs": "### Full Boot Logs (journalctl -b)",
        "details_summary": "Click to expand boot logs",
        "full_log_note": "> 📄 Full system log generated at: `{}`",
        "sys_label": "System",
        "kernel_label": "Kernel",
        "desktop_label": "Desktop Environment",
        "uptime_label": "Uptime",
        "ram_label": "RAM Memory",
        "cpu_label": "Processor",
        "gpu_label": "Graphics",
        "no_errors": "No critical errors detected in journalctl.",
        "error_reading": "Error reading logs: {}",
        # Success Screen Strings
        "success_title": "Issue opened on GitHub!",
        "success_desc": "GitHub limits issue text to 6,000 characters. To include the full system logs, please attach the generated logs file.",
        "file_card_title": "Generated logs file ready to attach:",
        "btn_open_nautilus": "Show file in Nautilus",
        "btn_back_form": "Back to form",
        "step_instructions_title": "💡 How to attach it to GitHub:",
        "step_instruction_1": "1. Click 'Show file in Nautilus' below.",
        "step_instruction_2": "2. Drag and drop the selected file into the GitHub issue box in your browser (or click 'Attach files' at the bottom left).",
        "body_attached_note": "> 📎 **Logs file:** Attaching `{}` to this report.",
    },
}

CURRENT_LANG = get_current_language()


def t(key):
    """Returns localized string with fallback to English."""
    lang_dict = STRINGS.get(CURRENT_LANG, STRINGS["en"])
    return lang_dict.get(key, STRINGS["en"].get(key, key))


def setup_icon_theme():
    """Ensures local icons are discovered if running uninstalled from source."""
    try:
        base_dir = os.path.dirname(os.path.abspath(__file__))
        local_icons = os.path.abspath(os.path.join(base_dir, "../../share/icons"))
        display = Gdk.Display.get_default()
        if display:
            theme = Gtk.IconTheme.get_for_display(display)
            if os.path.isdir(local_icons):
                theme.add_search_path(local_icons)
    except Exception:
        pass


def get_default_log_dir():
    """Returns a user-friendly directory for saving the log file."""
    home = os.path.expanduser("~")
    for folder in ["Descargas", "Downloads", "Documentos", "Documents"]:
        p = os.path.join(home, folder)
        if os.path.isdir(p):
            return p
    return "/tmp"


def get_system_info():
    """Gathers technical system specifications."""
    info = []

    # OS Version
    os_name = "Pulsar OS"
    if os.path.exists("/etc/os-release"):
        try:
            with open("/etc/os-release", "r", encoding="utf-8") as f:
                for line in f:
                    if line.startswith("PRETTY_NAME="):
                        os_name = line.split("=", 1)[1].strip().strip('"')
                        break
        except Exception:
            pass
    info.append(f"**{t('sys_label')}:** {os_name}")

    # Kernel & Arch
    kernel = f"{platform.system()} {platform.release()} ({platform.machine()})"
    info.append(f"**{t('kernel_label')}:** {kernel}")

    # Desktop Environment & Session
    desktop = os.environ.get("XDG_CURRENT_DESKTOP", "GNOME / Unknown")
    session_type = os.environ.get("XDG_SESSION_TYPE", "Unknown")
    info.append(f"**{t('desktop_label')}:** {desktop} ({session_type})")

    # Uptime
    try:
        res = subprocess.run(["uptime", "-p"], capture_output=True, text=True, check=False)
        uptime = res.stdout.strip() if res.returncode == 0 else "N/A"
    except Exception:
        uptime = "N/A"
    info.append(f"**{t('uptime_label')}:** {uptime}")

    # RAM
    try:
        with open("/proc/meminfo", "r", encoding="utf-8") as f:
            for line in f:
                if line.startswith("MemTotal:"):
                    mem_kb = int(line.split()[1])
                    mem_gb = round(mem_kb / (1024 * 1024), 2)
                    info.append(f"**{t('ram_label')}:** {mem_gb} GB")
                    break
    except Exception:
        pass

    # CPU
    try:
        with open("/proc/cpuinfo", "r", encoding="utf-8") as f:
            for line in f:
                if "model name" in line:
                    cpu_name = line.split(":", 1)[1].strip()
                    info.append(f"**{t('cpu_label')}:** {cpu_name}")
                    break
    except Exception:
        pass

    # GPU
    try:
        lspci = subprocess.run(["lspci"], capture_output=True, text=True, check=False)
        gpus = []
        for line in lspci.stdout.splitlines():
            if any(term in line.lower() for term in ["vga compatible controller", "3d controller", "display controller"]):
                parts = line.split(":", 2)
                if len(parts) >= 3:
                    gpus.append(parts[2].strip())
        if gpus:
            info.append(f"**{t('gpu_label')}:** {', '.join(gpus)}")
    except Exception:
        pass

    return "\n".join(f"- {line}" for line in info)


def get_boot_logs():
    """Gathers all logs since current system boot (journalctl -b)."""
    try:
        proc = subprocess.run(
            ["journalctl", "-b", "--no-pager"],
            capture_output=True,
            text=True,
            errors="replace",
            check=False,
        )
        if proc.returncode == 0 and proc.stdout:
            return proc.stdout
        elif proc.stderr:
            return t("error_reading").format(proc.stderr)
    except Exception as e:
        return f"Error executing journalctl: {e}"
    return "Could not retrieve journalctl logs."


def get_boot_errors():
    """Gathers high-priority error logs from current boot."""
    try:
        proc = subprocess.run(
            ["journalctl", "-b", "-p", "3", "-n", "35", "--no-pager"],
            capture_output=True,
            text=True,
            errors="replace",
            check=False,
        )
        if proc.returncode == 0 and proc.stdout.strip():
            return proc.stdout.strip()
    except Exception:
        pass
    return t("no_errors")


class FeedbackWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app, title=t("app_title"))
        self.set_default_size(680, 720)
        self.set_size_request(440, 520)

        self.system_info_text = ""
        self.boot_logs_text = ""
        self.boot_errors_text = ""
        self.saved_log_file = ""
        self.log_file_size_str = ""

        self._build_ui()
        self._load_data_async()

    def _build_ui(self):
        # Toast overlay for notifications
        self.toast_overlay = Adw.ToastOverlay()
        self.set_content(self.toast_overlay)

        # Main vertical box
        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.toast_overlay.set_child(main_box)

        # HeaderBar
        self.header = Adw.HeaderBar()
        main_box.append(self.header)

        # Stack to switch between Form and File Confirmation view
        self.stack = Gtk.Stack()
        self.stack.set_transition_type(Gtk.StackTransitionType.SLIDE_LEFT_RIGHT)
        self.stack.set_vexpand(True)
        self.stack.set_hexpand(True)
        main_box.append(self.stack)

        # Build Views
        form_view = self._build_form_view()
        self.stack.add_named(form_view, "form")

        success_view = self._build_success_view()
        self.stack.add_named(success_view, "success")

        self.stack.set_visible_child_name("form")

    def _build_form_view(self):
        scroller = Gtk.ScrolledWindow()
        scroller.set_vexpand(True)
        scroller.set_hexpand(True)
        scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)

        content = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=18)
        content.set_margin_top(16)
        content.set_margin_bottom(24)
        content.set_margin_start(24)
        content.set_margin_end(24)
        scroller.set_child(content)

        clamp = Adw.Clamp()
        clamp.set_maximum_size(620)
        content.append(clamp)

        form_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        clamp.set_child(form_box)

        # --- Header Card con Logo y Descripción ---
        banner_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        banner_box.add_css_class("card")
        banner_box.set_size_request(-1, 80)

        # App Icon
        icon_img = Gtk.Image.new_from_icon_name("pulsaros-feedback")
        icon_img.set_pixel_size(56)
        icon_img.set_margin_start(16)
        icon_img.set_margin_top(14)
        icon_img.set_margin_bottom(14)
        banner_box.append(icon_img)

        # App Title & Subtitle
        header_text_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=3)
        header_text_box.set_valign(Gtk.Align.CENTER)
        header_text_box.set_margin_end(16)

        title_lbl = Gtk.Label(label=t("banner_title"))
        title_lbl.add_css_class("title-3")
        title_lbl.set_xalign(0)
        header_text_box.append(title_lbl)

        desc_lbl = Gtk.Label(label=t("banner_desc"))
        desc_lbl.set_wrap(True)
        desc_lbl.add_css_class("dim-label")
        desc_lbl.set_xalign(0)
        header_text_box.append(desc_lbl)

        banner_box.append(header_text_box)
        form_box.append(banner_box)

        # --- Subject / Title ---
        subject_group = Adw.PreferencesGroup()
        subject_group.set_title(t("subject_group"))
        self.entry_subject = Adw.EntryRow()
        self.entry_subject.set_title(t("subject_row"))
        if hasattr(self.entry_subject, "set_placeholder_text"):
            self.entry_subject.set_placeholder_text(t("subject_placeholder"))
        subject_group.add(self.entry_subject)
        form_box.append(subject_group)

        # --- Main Description Text Box ---
        msg_group = Adw.PreferencesGroup()
        msg_group.set_title(t("desc_group"))
        msg_group.set_description(t("desc_instructions"))

        text_frame = Gtk.Frame()
        text_frame.add_css_class("card")

        text_scroller = Gtk.ScrolledWindow()
        text_scroller.set_min_content_height(140)
        text_scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)

        self.text_view = Gtk.TextView()
        self.text_view.set_wrap_mode(Gtk.WrapMode.WORD_CHAR)
        self.text_view.set_margin_top(12)
        self.text_view.set_margin_bottom(12)
        self.text_view.set_margin_start(12)
        self.text_view.set_margin_end(12)
        self.text_buffer = self.text_view.get_buffer()

        text_scroller.set_child(self.text_view)
        text_frame.set_child(text_scroller)
        msg_group.add(text_frame)
        form_box.append(msg_group)

        # --- Dropdown / Expander to preview all data (internal scroll) ---
        preview_group = Adw.PreferencesGroup()
        preview_group.set_title(t("data_group"))

        self.expander = Adw.ExpanderRow()
        self.expander.set_title(t("expander_title"))
        self.expander.set_subtitle(t("expander_sub"))
        self.expander.set_expanded(False)

        expander_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        expander_box.set_margin_top(10)
        expander_box.set_margin_bottom(10)
        expander_box.set_margin_start(10)
        expander_box.set_margin_end(10)

        # Loading status indicator
        self.status_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        self.spinner = Gtk.Spinner()
        self.spinner.start()
        self.status_lbl = Gtk.Label(label=t("loading_logs"))
        self.status_lbl.add_css_class("dim-label")
        self.status_box.append(self.spinner)
        self.status_box.append(self.status_lbl)
        expander_box.append(self.status_box)

        # Internal scrolled window
        logs_scroller = Gtk.ScrolledWindow()
        logs_scroller.set_min_content_height(240)
        logs_scroller.set_max_content_height(300)
        logs_scroller.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
        logs_scroller.add_css_class("card")

        self.logs_view = Gtk.TextView()
        self.logs_view.set_editable(False)
        self.logs_view.set_cursor_visible(False)
        self.logs_view.set_monospace(True)
        self.logs_view.set_wrap_mode(Gtk.WrapMode.NONE)
        self.logs_view.set_margin_top(8)
        self.logs_view.set_margin_bottom(8)
        self.logs_view.set_margin_start(8)
        self.logs_view.set_margin_end(8)
        self.logs_buffer = self.logs_view.get_buffer()
        self.logs_buffer.set_text(t("reading_journal"))

        logs_scroller.set_child(self.logs_view)
        expander_box.append(logs_scroller)

        # Action bar with Copy button
        actions_bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        actions_bar.set_halign(Gtk.Align.END)
        actions_bar.set_margin_top(4)

        btn_copy = Gtk.Button(label=t("copy_all"))
        btn_copy.set_icon_name("edit-copy-symbolic")
        btn_copy.connect("clicked", self._on_copy_clicked)
        actions_bar.append(btn_copy)

        expander_box.append(actions_bar)
        self.expander.add_row(expander_box)
        preview_group.add(self.expander)
        form_box.append(preview_group)

        # --- Submit Button & GitHub Account Notice ---
        submit_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        submit_box.set_margin_top(8)
        submit_box.set_halign(Gtk.Align.FILL)

        self.btn_submit = Gtk.Button()
        self.btn_submit.add_css_class("suggested-action")
        self.btn_submit.add_css_class("pill")
        self.btn_submit.set_size_request(-1, 50)

        btn_content = Adw.ButtonContent()
        btn_content.set_icon_name("mail-send-symbolic")
        btn_content.set_label(t("submit_btn"))
        self.btn_submit.set_child(btn_content)
        self.btn_submit.connect("clicked", self._on_submit_clicked)
        submit_box.append(self.btn_submit)

        # GitHub notice
        notice_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        notice_box.set_halign(Gtk.Align.CENTER)

        notice_icon = Gtk.Image.new_from_icon_name("dialog-information-symbolic")
        notice_icon.add_css_class("dim-label")
        notice_box.append(notice_icon)

        notice_lbl = Gtk.Label(label=t("github_notice"))
        notice_lbl.add_css_class("dim-label")
        notice_lbl.add_css_class("caption")
        notice_box.append(notice_lbl)

        submit_box.append(notice_box)
        form_box.append(submit_box)

        return scroller

    def _build_success_view(self):
        """Vista grande post-envío para guiar al usuario a adjuntar el archivo vía Nautilus."""
        scroller = Gtk.ScrolledWindow()
        scroller.set_vexpand(True)
        scroller.set_hexpand(True)
        scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)

        clamp = Adw.Clamp()
        clamp.set_maximum_size(620)
        clamp.set_margin_top(28)
        clamp.set_margin_bottom(36)
        clamp.set_margin_start(24)
        clamp.set_margin_end(24)
        scroller.set_child(clamp)

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=24)
        box.set_valign(Gtk.Align.CENTER)
        clamp.set_child(box)

        # Big Hero Icon
        hero_icon = Gtk.Image.new_from_icon_name("emblem-ok-symbolic")
        hero_icon.set_pixel_size(84)
        hero_icon.add_css_class("accent")
        box.append(hero_icon)

        # Title & Subtitle
        title_lbl = Gtk.Label(label=t("success_title"))
        title_lbl.add_css_class("title-1")
        title_lbl.set_justify(Gtk.Justification.CENTER)
        box.append(title_lbl)

        desc_lbl = Gtk.Label(label=t("success_desc"))
        desc_lbl.set_wrap(True)
        desc_lbl.set_justify(Gtk.Justification.CENTER)
        desc_lbl.add_css_class("dim-label")
        box.append(desc_lbl)

        # --- Tarjeta del archivo generado ---
        file_card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        file_card.add_css_class("card")
        file_card.set_margin_top(6)
        file_card.set_margin_bottom(6)
        file_card.set_margin_start(8)
        file_card.set_margin_end(8)

        card_header = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=14)
        card_header.set_margin_top(16)
        card_header.set_margin_start(16)
        card_header.set_margin_end(16)

        doc_icon = Gtk.Image.new_from_icon_name("text-x-generic")
        doc_icon.set_pixel_size(48)
        card_header.append(doc_icon)

        file_info_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        file_info_box.set_valign(Gtk.Align.CENTER)

        self.success_filename_lbl = Gtk.Label(label="pulsar_boot_logs.txt")
        self.success_filename_lbl.add_css_class("heading")
        self.success_filename_lbl.set_xalign(0)
        file_info_box.append(self.success_filename_lbl)

        self.success_filepath_lbl = Gtk.Label(label="/path/to/log.txt")
        self.success_filepath_lbl.add_css_class("dim-label")
        self.success_filepath_lbl.add_css_class("caption")
        self.success_filepath_lbl.set_wrap(True)
        self.success_filepath_lbl.set_xalign(0)
        file_info_box.append(self.success_filepath_lbl)

        card_header.append(file_info_box)
        file_card.append(card_header)

        # Instructions inside card
        inst_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        inst_box.set_margin_start(16)
        inst_box.set_margin_end(16)
        inst_box.set_margin_top(8)
        inst_box.set_margin_bottom(16)

        inst_title = Gtk.Label(label=t("step_instructions_title"))
        inst_title.add_css_class("heading")
        inst_title.set_xalign(0)
        inst_box.append(inst_title)

        inst_step1 = Gtk.Label(label=t("step_instruction_1"))
        inst_step1.set_xalign(0)
        inst_step1.set_wrap(True)
        inst_box.append(inst_step1)

        inst_step2 = Gtk.Label(label=t("step_instruction_2"))
        inst_step2.set_xalign(0)
        inst_step2.set_wrap(True)
        inst_box.append(inst_step2)

        file_card.append(inst_box)
        box.append(file_card)

        # --- Action Buttons ---
        buttons_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        buttons_box.set_margin_top(8)

        # Big Nautilus Button
        btn_nautilus = Gtk.Button()
        btn_nautilus.add_css_class("suggested-action")
        btn_nautilus.add_css_class("pill")
        btn_nautilus.set_size_request(-1, 52)
        
        nautilus_content = Adw.ButtonContent()
        nautilus_content.set_icon_name("system-file-manager-symbolic")
        nautilus_content.set_label(t("btn_open_nautilus"))
        btn_nautilus.set_child(nautilus_content)
        btn_nautilus.connect("clicked", self._on_open_nautilus_clicked)
        buttons_box.append(btn_nautilus)

        # Back Button
        btn_back = Gtk.Button(label=t("btn_back_form"))
        btn_back.add_css_class("flat")
        btn_back.connect("clicked", lambda _b: self.stack.set_visible_child_name("form"))
        buttons_box.append(btn_back)

        box.append(buttons_box)

        return scroller

    def _load_data_async(self):
        def worker():
            sys_info = get_system_info()
            boot_logs = get_boot_logs()
            boot_errors = get_boot_errors()

            log_dir = get_default_log_dir()
            timestamp = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
            log_path = os.path.join(log_dir, f"pulsar_boot_logs_{timestamp}.txt")
            
            try:
                with open(log_path, "w", encoding="utf-8") as f:
                    f.write(f"=== PULSAR OS SYSTEM INFO ===\n{sys_info}\n\n")
                    f.write(f"=== BOOT ERRORS ===\n{boot_errors}\n\n")
                    f.write(f"=== BOOT LOGS (journalctl -b) ===\n{boot_logs}\n")
            except Exception:
                log_path = f"/tmp/pulsar_boot_logs_{timestamp}.txt"
                try:
                    with open(log_path, "w", encoding="utf-8") as f:
                        f.write(f"=== PULSAR OS SYSTEM INFO ===\n{sys_info}\n\n")
                        f.write(f"=== BOOT LOGS (journalctl -b) ===\n{boot_logs}\n")
                except Exception:
                    log_path = ""

            GLib.idle_add(self._on_data_loaded, sys_info, boot_logs, boot_errors, log_path)

        threading.Thread(target=worker, daemon=True).start()

    def _on_data_loaded(self, sys_info, boot_logs, boot_errors, log_path):
        self.system_info_text = sys_info
        self.boot_logs_text = boot_logs
        self.boot_errors_text = boot_errors
        self.saved_log_file = log_path

        preview = (
            f"=== {t('sec_sys_info').replace('#', '').strip()} ===\n"
            f"{sys_info}\n\n"
            f"=== {t('sec_boot_errors').replace('#', '').strip()} ===\n"
            f"{boot_errors}\n\n"
            f"=== {t('sec_full_logs').replace('#', '').strip()} ===\n"
            f"{boot_logs}"
        )
        self.logs_buffer.set_text(preview)

        self.spinner.stop()
        self.spinner.set_visible(False)
        lines = len(boot_logs.splitlines())
        status = t("logs_gathered").format(lines)
        if log_path:
            status += t("full_file").format(log_path)
            # Update success view widgets
            self.success_filename_lbl.set_text(os.path.basename(log_path))
            try:
                size_mb = os.path.getsize(log_path) / (1024 * 1024)
                self.success_filepath_lbl.set_text(f"{log_path} ({size_mb:.1f} MB - {lines:,} líneas)")
            except Exception:
                self.success_filepath_lbl.set_text(log_path)

        self.status_lbl.set_text(status)

    def _build_full_report(self, user_msg):
        """Genera el reporte completo en Markdown."""
        parts = []
        parts.append(t("sec_user_desc"))
        parts.append(user_msg if user_msg.strip() else t("no_user_desc"))
        parts.append("")
        parts.append(t("sec_sys_info"))
        parts.append(self.system_info_text)
        parts.append("")
        parts.append(t("sec_boot_errors"))
        parts.append("```")
        parts.append(self.boot_errors_text)
        parts.append("```")
        if self.saved_log_file:
            parts.append("")
            parts.append(t("full_log_note").format(self.saved_log_file))
        return "\n".join(parts)

    def _build_url_body(self, user_msg):
        """Genera un body compacto para la URL de GitHub (máximo ~500 caracteres)."""
        filename = os.path.basename(self.saved_log_file) if self.saved_log_file else "pulsar_boot_logs.txt"
        parts = []
        parts.append(t("sec_user_desc"))
        parts.append(user_msg if user_msg.strip() else t("no_user_desc"))
        parts.append("")
        parts.append(t("sec_sys_info"))
        parts.append(self.system_info_text)
        parts.append("")
        parts.append(t("body_attached_note").format(filename))
        return "\n".join(parts)

    def _on_copy_clicked(self, _btn):
        user_msg = self.text_buffer.get_text(
            self.text_buffer.get_start_iter(), self.text_buffer.get_end_iter(), True
        )
        full = self._build_full_report(user_msg)
        full += f"\n\n{t('sec_full_logs')}\n<details><summary>{t('details_summary')}</summary>\n\n```\n{self.boot_logs_text}\n```\n</details>"

        display = Gdk.Display.get_default()
        if display:
            display.get_clipboard().set(full)
            self.toast_overlay.add_toast(Adw.Toast(title=t("report_copied")))

    def _on_open_nautilus_clicked(self, _btn):
        """Abre Nautilus seleccionando el archivo de logs."""
        if self.saved_log_file and os.path.exists(self.saved_log_file):
            try:
                subprocess.Popen(["nautilus", "--select", self.saved_log_file])
            except Exception:
                # Fallback opening directory
                dirname = os.path.dirname(self.saved_log_file)
                try:
                    subprocess.Popen(["xdg-open", dirname])
                except Exception:
                    pass
        elif self.saved_log_file:
            dirname = os.path.dirname(self.saved_log_file)
            try:
                subprocess.Popen(["xdg-open", dirname])
            except Exception:
                pass

    def _on_submit_clicked(self, _btn):
        subj = self.entry_subject.get_text().strip()
        user_msg = self.text_buffer.get_text(
            self.text_buffer.get_start_iter(), self.text_buffer.get_end_iter(), True
        ).strip()

        if subj:
            title = subj
        elif user_msg:
            first_line = user_msg.splitlines()[0]
            title = (first_line[:50] + "...") if len(first_line) > 50 else first_line
        else:
            title = t("default_title")

        issue_title = f"[Feedback] {title}"

        # 1. Copiar reporte al portapapeles
        full_report = self._build_full_report(user_msg)
        display = Gdk.Display.get_default()
        if display:
            display.get_clipboard().set(full_report)

        # 2. Generar URL compacta para GitHub (cuerpo ligero, < 600 caracteres)
        url_body = self._build_url_body(user_msg)
        params = {
            "title": issue_title,
            "body": url_body,
            "labels": "feedback",
        }
        query_string = urllib.parse.urlencode(params)
        final_url = f"{GITHUB_NEW_ISSUE_URL}?{query_string}"

        # 3. Abrir en el navegador predeterminado
        opened = False
        try:
            opened = Gio.AppInfo.launch_default_for_uri(final_url, None)
        except Exception:
            pass

        if not opened:
            try:
                subprocess.Popen(["xdg-open", final_url])
                opened = True
            except Exception:
                pass

        # 4. Cambiar a la vista grande con el archivo y botón de Nautilus
        self.stack.set_visible_child_name("success")
        toast = Adw.Toast(title=t("toast_opened"))
        toast.set_timeout(5)
        self.toast_overlay.add_toast(toast)


class PulsarFeedbackApp(Adw.Application):
    def __init__(self):
        super().__init__(
            application_id=APP_ID,
            flags=Gio.ApplicationFlags.FLAGS_NONE,
        )

    def do_activate(self):
        setup_icon_theme()
        win = self.props.active_window
        if not win:
            win = FeedbackWindow(self)
        win.present()


def main():
    app = PulsarFeedbackApp()
    return app.run(sys.argv)


if __name__ == "__main__":
    sys.exit(main())
