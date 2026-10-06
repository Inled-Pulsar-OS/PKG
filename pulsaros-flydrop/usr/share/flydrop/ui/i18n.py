"""
Internationalization (i18n) module for FlyDrop.
Default: English.
Fallback to English if system locale is not Spanish ('es').
"""

import os
import locale

def get_locale():
    try:
        lang = os.environ.get("LC_ALL") or os.environ.get("LC_MESSAGES") or os.environ.get("LANG") or locale.getdefaultlocale()[0] or "en"
        return lang.lower()
    except Exception:
        return "en"

IS_SPANISH = get_locale().startswith("es")

STRINGS = {
    "en": {
        "app_name": "FlyDrop",
        "settings_title": "FlyDrop Settings",
        "general_section": "General",
        "device_name": "Device Name",
        "device_name_subtitle": "Visible to other devices on the local network",
        "save_name": "Save Name",
        "name_saved": "Name updated successfully",
        "receiving_section": "Receiving",
        "auto_accept": "Receive automatically",
        "auto_accept_subtitle": "Accept incoming files without asking for confirmation",
        "save_destination": "Save destination",
        "save_destination_subtitle": "Downloads folder",
        "nearby_devices": "Nearby Devices",
        "nearby_devices_subtitle": "LocalSend & FlyDrop devices detected on Wi-Fi",
        "scanning": "Scanning local network...",
        "no_devices": "No devices found nearby",
        "send_file": "Send File",
        "incoming_title": "Incoming Transfer",
        "wants_to_send": "wants to send you",
        "file_singular": "file",
        "file_plural": "files",
        "decline": "Decline",
        "accept": "Accept",
        "receiving": "Receiving...",
        "sending": "Sending to",
        "completed": "Completed",
        "failed": "Transfer failed",
        "cancel": "Cancel",
        "open_downloads": "Open Downloads",
        "receive_without_asking": "Receive without asking",
        "settings": "Settings...",
        "quit": "Quit",
        "dynamic_shelf_hint": "Drop files here to send via FlyDrop"
    },
    "es": {
        "app_name": "FlyDrop",
        "settings_title": "Ajustes de FlyDrop",
        "general_section": "General",
        "device_name": "Nombre del equipo",
        "device_name_subtitle": "Visible para otros dispositivos en la red local",
        "save_name": "Guardar",
        "name_saved": "Nombre actualizado correctamente",
        "receiving_section": "Recepción",
        "auto_accept": "Recibir automáticamente",
        "auto_accept_subtitle": "Aceptar archivos entrantes sin pedir confirmación",
        "save_destination": "Carpeta de descarga",
        "save_destination_subtitle": "Carpeta Descargas",
        "nearby_devices": "Dispositivos cercanos",
        "nearby_devices_subtitle": "Dispositivos LocalSend y FlyDrop en la red Wi-Fi",
        "scanning": "Buscando dispositivos en la red...",
        "no_devices": "No se encontraron dispositivos cercanos",
        "send_file": "Enviar archivo",
        "incoming_title": "Transferencia entrante",
        "wants_to_send": "quiere enviarte",
        "file_singular": "archivo",
        "file_plural": "archivos",
        "decline": "Rechazar",
        "accept": "Aceptar",
        "receiving": "Recibiendo...",
        "sending": "Enviando a",
        "completed": "Completado",
        "failed": "Error en la transferencia",
        "cancel": "Cancelar",
        "open_downloads": "Abrir Descargas",
        "receive_without_asking": "Recibir sin preguntar",
        "settings": "Ajustes...",
        "quit": "Salir",
        "dynamic_shelf_hint": "Suelta archivos aquí para enviar con FlyDrop"
    }
}

def _(key: str) -> str:
    lang = "es" if IS_SPANISH else "en"
    return STRINGS.get(lang, STRINGS["en"]).get(key, STRINGS["en"].get(key, key))
