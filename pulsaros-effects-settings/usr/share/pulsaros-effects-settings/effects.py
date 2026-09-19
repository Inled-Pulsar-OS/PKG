#!/usr/bin/env python3
# ==============================================================================
# Pulsar OS - Desktop Effects Settings App
# ==============================================================================
# English: This application allows users to toggle desktop special effects
#          between standard Blur my Shell and premium Liquid Glass, managing
#          extension states and adjusting Dash to Dock transparency presets.
# Español: Esta aplicación permite a los usuarios alternar los efectos especiales
#          del escritorio entre el Blur my Shell estándar y el Liquid Glass premium,
#          gestionando los estados de las extensiones y ajustando la transparencia de Dash to Dock.
# ==============================================================================

import gi
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk, Gdk, Gio, GLib
import os
import sys
import subprocess

class EffectsSettingsWindow(Gtk.Window):
    def __init__(self):
        super().__init__(title="Desktop Effects / Efectos de Escritorio")
        self.set_default_size(480, 360)
        self.set_position(Gtk.WindowPosition.CENTER)
        self.schema_source = self.load_custom_schemas()
        self.set_resizable(False)

        # Apply dark mode style using custom CSS
        # Aplicar estilo de modo oscuro usando CSS personalizado
        style_provider = Gtk.CssProvider()
        css = """
        window {
            background-color: #1e1e1e;
            color: #ffffff;
        }
        .header-bar {
            background: #2d2d2d;
            border-bottom: 1px solid #3d3d3d;
            padding: 6px;
        }
        .main-container {
            padding: 24px;
        }
        .pref-box {
            background-color: #2b2b2b;
            border: 1px solid #3d3d3d;
            border-radius: 8px;
            padding: 16px;
        }
        .title-label {
            font-size: 16px;
            font-weight: bold;
            color: #ffffff;
        }
        .desc-label {
            font-size: 12px;
            color: #b3b3b3;
        }
        .warning-box {
            background-color: #3a1a1a;
            border: 1px solid #662222;
            border-radius: 6px;
            padding: 12px;
            margin-top: 16px;
        }
        .warning-text {
            font-size: 11px;
            color: #ff9999;
        }
        .action-btn {
            font-weight: bold;
            padding: 8px 16px;
            border-radius: 6px;
        }
        """
        style_provider.load_from_data(css.encode())
        Gtk.StyleContext.add_provider_for_screen(
            Gdk.Screen.get_default(),
            style_provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

        # Headerbar styling (macOS style window header)
        # Estilo Headerbar (cabecera de ventana estilo macOS)
        header = Gtk.HeaderBar()
        header.set_show_close_button(True)
        header.get_style_context().add_class("header-bar")
        header.set_title("Desktop Effects Settings")
        header.set_subtitle("Pulsar OS Visual Effects")
        self.set_titlebar(header)

        # Main Layout
        # Distribución principal
        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        main_box.get_style_context().add_class("main-container")
        self.add(main_box)

        # Intro text
        # Texto de introducción
        lbl_intro = Gtk.Label()
        lbl_intro.set_markup("Configure the desktop design style and performance preference.\nConfigura el estilo de diseño del escritorio y la preferencia de rendimiento.")
        lbl_intro.set_line_wrap(True)
        lbl_intro.set_justify(Gtk.Justification.CENTER)
        lbl_intro.get_style_context().add_class("desc-label")
        main_box.pack_start(lbl_intro, False, False, 0)

        # Preference Box (Card)
        # Caja de Preferencias (Tarjeta)
        card_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        card_box.get_style_context().add_class("pref-box")
        main_box.pack_start(card_box, False, False, 4)

        # Options selection using radio buttons
        # Selección de opciones usando botones de radio
        self.radio_blur = Gtk.RadioButton.new_with_label_from_widget(None, "Standard Blur my Shell (Recommended / Recomendado)")
        self.radio_blur.connect("toggled", self.on_effects_toggled)
        card_box.pack_start(self.radio_blur, False, False, 4)

        self.radio_glass = Gtk.RadioButton.new_with_label_from_widget(self.radio_blur, "Premium Liquid Glass (Glassmorphism / Efecto Cristal)")
        card_box.pack_start(self.radio_glass, False, False, 4)

        # Warning panel (only visible when Liquid Glass is selected)
        # Panel de advertencia (solo visible cuando se selecciona Liquid Glass)
        self.warning_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        self.warning_box.get_style_context().add_class("warning-box")
        card_box.pack_start(self.warning_box, False, False, 4)

        lbl_warn = Gtk.Label()
        lbl_warn.set_markup("<span weight='bold'>⚠️ PERFORMANCE WARNING / ADVERTENCIA DE RENDIMIENTO:</span>\n"
                            "Liquid Glass consumes significant GPU and CPU resources. It might cause lag on older graphic cards or virtualized environments.\n"
                            "Liquid Glass consume bastantes recursos de GPU y CPU. Puede causar lag en tarjetas gráficas antiguas o entornos virtualizados.")
        lbl_warn.set_line_wrap(True)
        lbl_warn.set_max_width_chars(50)
        lbl_warn.set_justify(Gtk.Justification.LEFT)
        lbl_warn.get_style_context().add_class("warning-text")
        self.warning_box.pack_start(lbl_warn, True, True, 0)

        # Separator line / Línea divisoria
        separator = Gtk.Separator(orientation=Gtk.Orientation.HORIZONTAL)
        card_box.pack_start(separator, False, False, 8)

        # Show apps dock switch row / Fila con interruptor para mostrar el botón de apps del dock
        apps_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        card_box.pack_start(apps_row, False, False, 4)

        lbl_apps = Gtk.Label()
        lbl_apps.set_markup("<b>Show applications icon in Dock / Mostrar aplicaciones en el Dock</b>\n"
                            "<span size='small' foreground='#888888'>Adds GNOME app grid launcher to the dock panels.\nAñade el lanzador de cuadrícula de apps a los paneles del dock.</span>")
        lbl_apps.set_xalign(0.0)
        lbl_apps.set_line_wrap(True)
        lbl_apps.set_max_width_chars(45)
        apps_row.pack_start(lbl_apps, True, True, 0)

        self.switch_show_apps = Gtk.Switch()
        self.switch_show_apps.set_valign(Gtk.Align.CENTER)
        apps_row.pack_end(self.switch_show_apps, False, False, 0)

        # Load current system state to set UI switches
        # Cargar el estado actual del sistema para configurar los interruptores de la UI
        is_glass_active = self.get_current_effects_state()
        if is_glass_active:
            self.radio_glass.set_active(True)
            self.warning_box.set_visible(True)
        else:
            self.radio_blur.set_active(True)
            self.warning_box.set_visible(False)

        # Load current show-apps state
        # Cargar el estado actual de show-apps
        try:
            settings_dock = self.get_safe_settings("org.gnome.shell.extensions.dash-to-dock")
            if settings_dock:
                self.switch_show_apps.set_active(settings_dock.get_boolean("show-show-apps-button"))
        except Exception as e:
            print("Error loading show-apps state:", e)
            self.switch_show_apps.set_active(False)

        self.switch_show_apps.connect("notify::active", self.on_show_apps_toggled)

        # Add button bar
        # Añadir barra de botones
        btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        btn_box.set_halign(Gtk.Align.CENTER)
        main_box.pack_end(btn_box, False, False, 0)

        btn_close = Gtk.Button(label="Close / Cerrar")
        btn_close.get_style_context().add_class("action-btn")
        btn_close.connect("clicked", lambda w: self.close())
        btn_box.pack_start(btn_close, False, False, 0)

        self.show_all()

    def get_safe_settings(self, schema_id):
        """
        English: Instantiates Gio.Settings safely by verifying if the schema exists first,
                 preventing GLib-GIO-ERROR crashes and core dumps.
        Español: Instancia Gio.Settings de forma segura verificando primero si el esquema existe,
                 evitando caídas GLib-GIO-ERROR y volcados de memoria (core dumps).
        """
        try:
            schema = self.schema_source.lookup(schema_id, True)
            if schema:
                return Gio.Settings.new_full(schema, None, None)
            print(f"Warning: GSettings schema '{schema_id}' is not found in the custom schema sources.")
        except Exception as e:
            print(f"Error checking schema '{schema_id}':", e)
        return None

    def load_custom_schemas(self):
        """
        English: Checks for local extension schema paths and links them to a custom SettingsSchemaSource.
        Español: Comprueba las rutas de esquemas de extensiones locales y las vincula a un SettingsSchemaSource personalizado.
        """
        default_source = Gio.SettingsSchemaSource.get_default()
        
        # Diagnostics to inspect installed extensions on developer host
        # Diagnósticos para inspeccionar extensiones instaladas en el host del desarrollador
        try:
            local_exts = os.path.expanduser("~/.local/share/gnome-shell/extensions")
            if os.path.isdir(local_exts):
                print("[Diag] Installed user extensions:", os.listdir(local_exts))
            global_exts = "/usr/share/gnome-shell/extensions"
            if os.path.isdir(global_exts):
                print("[Diag] Installed system extensions:", os.listdir(global_exts))
        except Exception as diag_err:
            print("[Diag] Error listing extensions:", diag_err)
            
        # Determine the base PKG directory relative to this script
        # Determinar el directorio base PKG relativo a este script
        script_dir = os.path.dirname(os.path.realpath(__file__))
        pkg_dir = os.path.abspath(os.path.join(script_dir, "../../../../"))
        
        # Candidate directories for GSettings schemas (global and local)
        # Directorios candidatos para esquemas GSettings (globales y locales)
        candidate_paths = [
            os.path.expanduser("~/.local/share/gnome-shell/extensions/liquid-glass@thinkingcoding1231.gmail.com/schemas"),
            "/usr/share/gnome-shell/extensions/liquid-glass@thinkingcoding1231.gmail.com/schemas",
            os.path.expanduser("~/.local/share/gnome-shell/extensions/blur-my-shell@aunetx/schemas"),
            "/usr/share/gnome-shell/extensions/blur-my-shell@aunetx/schemas",
            os.path.expanduser("~/.local/share/gnome-shell/extensions/pulsar-dock@inled.es/schemas"),
            "/usr/share/gnome-shell/extensions/pulsar-dock@inled.es/schemas",
            os.path.expanduser("~/.local/share/gnome-shell/extensions/dash-to-dock@micxgx.gmail.com/schemas"),
            "/usr/share/gnome-shell/extensions/dash-to-dock@micxgx.gmail.com/schemas",
            "/usr/share/glib-2.0/schemas",
            os.path.join(pkg_dir, "build/pkg-staging/pulsaros-gnome/usr/share/glib-2.0/schemas"),
            os.path.join(pkg_dir, "pulsaros-gnome/usr/share/glib-2.0/schemas")
        ]
        
        current_source = default_source
        for path in candidate_paths:
            if os.path.isdir(path):
                try:
                    files = os.listdir(path)
                    has_xml = any(f.endswith(".gschema.xml") for f in files)
                    has_compiled = "gschemas.compiled" in files
                    
                    if not (has_xml or has_compiled):
                        # Silently skip folder if it doesn't contain schema definitions
                        continue
                    
                    # Chain pre-compiled schema source if gschemas.compiled is available
                    if has_compiled:
                        current_source = Gio.SettingsSchemaSource.new_from_directory(path, current_source, False)
                        print(f"[Schema] Loaded schemas directory successfully: {path}")
                except Exception as e:
                    print(f"[Schema] Error loading custom schema path {path}: {e}")
                    
        return current_source

    def get_current_effects_state(self):
        """
        Reads GSettings to check if Liquid Glass is currently enabled.
        Lee GSettings para comprobar si Liquid Glass está habilitado actualmente.
        """
        try:
            settings = self.get_safe_settings("org.gnome.shell")
            if settings:
                enabled = settings.get_strv("enabled-extensions")
                return "liquid-glass@thinkingcoding1231.gmail.com" in enabled
        except Exception as e:
            print("Error reading current effects state:", e)
        return False

    def on_effects_toggled(self, radio):
        """
        Triggered when switching options. Applies extension states and dock transparency presets.
        Se ejecuta al cambiar opciones. Aplica estados de extensiones y preajustes de transparencia del dock.
        """
        if self.radio_blur.get_active():
            self.warning_box.set_visible(False)
            self.apply_mode("blur")
        else:
            self.warning_box.set_visible(True)
            self.apply_mode("glass")

    def apply_mode(self, mode):
        """
        Applies either 'glass' or 'blur' preset.
        Aplica el perfil de efectos 'glass' o 'blur'.
        """
        if mode == "glass":
            self.set_extension_state("blur-my-shell@aunetx", False)
            self.set_extension_state("liquid-glass@thinkingcoding1231.gmail.com", True)
            self.apply_liquid_glass_settings()
        else:
            self.set_extension_state("liquid-glass@thinkingcoding1231.gmail.com", False)
            self.set_extension_state("blur-my-shell@aunetx", True)
            self.apply_blur_myshell_dock_settings()

    def apply_blur_myshell_dock_settings(self):
        """
        Restores Dash to Dock settings for standard Blur my Shell rendering.
        Restaura la configuración de Dash to Dock para el renderizado estándar de Blur my Shell.
        """
        try:
            settings = self.get_safe_settings("org.gnome.shell.extensions.dash-to-dock")
            if settings:
                settings.set_double("background-opacity", 0.15)
                settings.set_boolean("custom-theme-shrink", False)
                settings.set_double("height-fraction", 0.9)
                settings.set_boolean("apply-custom-theme", False)
                settings.set_string("transparency-mode", "FIXED")
                settings.set_boolean("customize-alphas", False)
                print("Effects App: Restored Dash to Dock for Blur my Shell (apply-custom-theme=False).")
        except Exception as e:
            print("Error restoring Dash to Dock settings:", e)

    def apply_liquid_glass_settings(self):
        """
        Sets Dash to Dock with system theme enabled and applies Liquid Glass host presets.
        Establece Dash to Dock con 'usar tema del sistema' activado y aplica los preajustes de Liquid Glass del host.
        """
        try:
            # 1. Configurar Dash to Dock (Pulsar Dock) con 'Usar tema del sistema' activado
            settings_dock = self.get_safe_settings("org.gnome.shell.extensions.dash-to-dock")
            if settings_dock:
                settings_dock.set_double("background-opacity", 0.15)
                settings_dock.set_boolean("custom-theme-shrink", False)
                settings_dock.set_double("height-fraction", 0.9)
                settings_dock.set_boolean("apply-custom-theme", True)
                settings_dock.set_string("transparency-mode", "FIXED")
                settings_dock.set_boolean("customize-alphas", False)
                print("Effects App: Configured Dash to Dock for Liquid Glass (apply-custom-theme=True).")
        except Exception as e:
            print("Error configuring Dash to Dock for Liquid Glass:", e)

        try:
            # 2. Configurar Liquid Glass según los ajustes extraídos del host
            settings_glass = self.get_safe_settings("org.gnome.shell.extensions.liquid-glass")
            if not settings_glass:
                settings_glass = self.get_safe_settings("org.gnome.shell.extensions.liquid-glass@thinkingcoding1231.gmail.com")

            if settings_glass:
                def safe_set(setter, key, val):
                    try:
                        setter(key, val)
                    except Exception as err:
                        print(f"Skipping key {key}: {err}")

                safe_set(settings_glass.set_int, "application-blur-radius", 5)
                safe_set(settings_glass.set_double, "application-content-opacity", 1.0)
                safe_set(settings_glass.set_double, "application-corner-radius", 20.689655172413794)
                safe_set(settings_glass.set_boolean, "application-glass-all-windows", True)
                safe_set(settings_glass.set_double, "application-saturation", 2.0)
                safe_set(settings_glass.set_string, "application-tint-color", "#000000")
                safe_set(settings_glass.set_double, "application-tint-strength", 0.0)
                safe_set(settings_glass.set_strv, "application-window-whitelist", [])
                safe_set(settings_glass.set_int, "blur-method", 0)
                safe_set(settings_glass.set_double, "dock-corner-radius", 24.0)
                safe_set(settings_glass.set_int, "dock-glass-expand", 3)
                safe_set(settings_glass.set_string, "dock-tint-color", "#000000")
                safe_set(settings_glass.set_boolean, "enable-application-glass", True)
                safe_set(settings_glass.set_boolean, "enable-menu-glass", True)
                safe_set(settings_glass.set_boolean, "enable-quick-settings-glass", False)
                safe_set(settings_glass.set_double, "glass-chroma-strength", 0.0)
                safe_set(settings_glass.set_double, "glass-displacement-scale", 188.37209302325581)
                safe_set(settings_glass.set_double, "glass-edge-smoothing", 0.0)
                safe_set(settings_glass.set_double, "glass-ior", 2.0175438596491229)
                safe_set(settings_glass.set_double, "glass-max-z", 16.981132075471699)
                safe_set(settings_glass.set_double, "glass-profile-shape-n", 20.0)
                safe_set(settings_glass.set_double, "glass-rim-width", 4.8000000000000007)
                safe_set(settings_glass.set_double, "glass-specular-intensity", 0.0)
                safe_set(settings_glass.set_double, "menu-corner-radius", 14.0)
                safe_set(settings_glass.set_int, "menu-glass-expand", 4)
                safe_set(settings_glass.set_double, "panel-menu-corner-radius", 14.0)
                safe_set(settings_glass.set_int, "panel-menu-glass-expand", 4)
                safe_set(settings_glass.set_double, "desktop-menu-corner-radius", 14.0)
                safe_set(settings_glass.set_string, "menu-tint-color", "#000000")
                safe_set(settings_glass.set_double, "notification-corner-radius", 16.0)
                safe_set(settings_glass.set_string, "notification-tint-color", "#000000")
                safe_set(settings_glass.set_double, "osd-corner-radius", 16.0)
                safe_set(settings_glass.set_string, "osd-tint-color", "#000000")
                safe_set(settings_glass.set_boolean, "output-logs", False)
                safe_set(settings_glass.set_int, "quick-settings-apply-to", 1)
                safe_set(settings_glass.set_double, "quick-settings-corner-radius", 18.0)
                safe_set(settings_glass.set_boolean, "quick-settings-enable-adaptive-text-color", False)
                print("Effects App: Applied Liquid Glass GSettings from host profile.")
            else:
                # Fallback via dconf command if GSettings schema is not loaded in current glib source
                dconf_commands = [
                    ("/org/gnome/shell/extensions/liquid-glass/application-blur-radius", "5"),
                    ("/org/gnome/shell/extensions/liquid-glass/application-content-opacity", "1.0"),
                    ("/org/gnome/shell/extensions/liquid-glass/application-corner-radius", "20.689655172413794"),
                    ("/org/gnome/shell/extensions/liquid-glass/application-glass-all-windows", "true"),
                    ("/org/gnome/shell/extensions/liquid-glass/application-saturation", "2.0"),
                    ("/org/gnome/shell/extensions/liquid-glass/application-tint-color", "'#000000'"),
                    ("/org/gnome/shell/extensions/liquid-glass/application-tint-strength", "0.0"),
                    ("/org/gnome/shell/extensions/liquid-glass/application-window-whitelist", "@as []"),
                    ("/org/gnome/shell/extensions/liquid-glass/blur-method", "0"),
                    ("/org/gnome/shell/extensions/liquid-glass/dock-corner-radius", "24.0"),
                    ("/org/gnome/shell/extensions/liquid-glass/dock-glass-expand", "3"),
                    ("/org/gnome/shell/extensions/liquid-glass/dock-tint-color", "'#000000'"),
                    ("/org/gnome/shell/extensions/liquid-glass/enable-application-glass", "true"),
                    ("/org/gnome/shell/extensions/liquid-glass/enable-menu-glass", "true"),
                    ("/org/gnome/shell/extensions/liquid-glass/enable-quick-settings-glass", "false"),
                    ("/org/gnome/shell/extensions/liquid-glass/glass-chroma-strength", "0.0"),
                    ("/org/gnome/shell/extensions/liquid-glass/glass-displacement-scale", "188.37209302325581"),
                    ("/org/gnome/shell/extensions/liquid-glass/glass-edge-smoothing", "0.0"),
                    ("/org/gnome/shell/extensions/liquid-glass/glass-ior", "2.0175438596491229"),
                    ("/org/gnome/shell/extensions/liquid-glass/glass-max-z", "16.981132075471699"),
                    ("/org/gnome/shell/extensions/liquid-glass/glass-profile-shape-n", "20.0"),
                    ("/org/gnome/shell/extensions/liquid-glass/glass-rim-width", "4.8000000000000007"),
                    ("/org/gnome/shell/extensions/liquid-glass/glass-specular-intensity", "0.0"),
                    ("/org/gnome/shell/extensions/liquid-glass/menu-corner-radius", "14.0"),
                    ("/org/gnome/shell/extensions/liquid-glass/menu-glass-expand", "4"),
                    ("/org/gnome/shell/extensions/liquid-glass/panel-menu-corner-radius", "14.0"),
                    ("/org/gnome/shell/extensions/liquid-glass/panel-menu-glass-expand", "4"),
                    ("/org/gnome/shell/extensions/liquid-glass/desktop-menu-corner-radius", "14.0"),
                    ("/org/gnome/shell/extensions/liquid-glass/menu-tint-color", "'#000000'"),
                    ("/org/gnome/shell/extensions/liquid-glass/notification-corner-radius", "16.0"),
                    ("/org/gnome/shell/extensions/liquid-glass/notification-tint-color", "'#000000'"),
                    ("/org/gnome/shell/extensions/liquid-glass/osd-corner-radius", "16.0"),
                    ("/org/gnome/shell/extensions/liquid-glass/osd-tint-color", "'#000000'"),
                    ("/org/gnome/shell/extensions/liquid-glass/output-logs", "false"),
                    ("/org/gnome/shell/extensions/liquid-glass/quick-settings-apply-to", "1"),
                    ("/org/gnome/shell/extensions/liquid-glass/quick-settings-corner-radius", "18.0"),
                    ("/org/gnome/shell/extensions/liquid-glass/quick-settings-enable-adaptive-text-color", "false"),
                ]
                for path, val in dconf_commands:
                    subprocess.run(["dconf", "write", path, val], capture_output=True)
                print("Effects App: Applied Liquid Glass settings via dconf fallback.")
        except Exception as e:
            print("Error applying Liquid Glass settings:", e)

    def on_show_apps_toggled(self, switch, gparam):
        """
        Triggered when toggling the 'Show applications' switch.
        Se ejecuta al alternar el interruptor de 'Mostrar aplicaciones'.
        """
        show_apps = switch.get_active()
        try:
            settings = self.get_safe_settings("org.gnome.shell.extensions.dash-to-dock")
            if settings:
                settings.set_boolean("show-show-apps-button", show_apps)
                print(f"Effects App: show-show-apps-button set to {show_apps}")
        except Exception as e:
            print("Error toggling show-apps setting:", e)

    def set_extension_state(self, uuid, enable):
        """
        Enables or disables a GNOME Shell extension by UUID.
        Habilita o deshabilita una extensión de GNOME Shell por su UUID.
        """
        try:
            settings = self.get_safe_settings("org.gnome.shell")
            if settings:
                enabled = list(settings.get_strv("enabled-extensions"))
                if enable:
                    if uuid not in enabled:
                        enabled.append(uuid)
                else:
                    if uuid in enabled:
                        enabled.remove(uuid)
                settings.set_strv("enabled-extensions", enabled)
        except Exception as e:
            print(f"Error setting extension {uuid} state via GSettings: {e}")
        
        # Also run gnome-extensions command for immediate Shell reload
        cmd = "enable" if enable else "disable"
        subprocess.run(["gnome-extensions", cmd, uuid], capture_output=True)

def apply_headless(mode):
    """Headless application of effects mode without opening GTK window."""
    app = EffectsSettingsWindow.__new__(EffectsSettingsWindow)
    app.schema_source = app.load_custom_schemas()
    if mode in ["glass", "liquid", "--glass", "-g"]:
        app.apply_mode("glass")
        print("✅ Liquid Glass mode enabled with host presets and system theme dock.")
    elif mode in ["blur", "standard", "--blur", "-b"]:
        app.apply_mode("blur")
        print("✅ Standard Blur my Shell mode enabled.")
    elif mode in ["toggle", "-t"]:
        is_active = app.get_current_effects_state()
        new_mode = "blur" if is_active else "glass"
        app.apply_mode(new_mode)
        print(f"✅ Toggled effects mode to: {new_mode}")
    elif mode in ["status", "-s"]:
        is_active = app.get_current_effects_state()
        print("glass" if is_active else "blur")
    else:
        print(f"Unknown mode: {mode}. Use 'glass', 'blur', 'toggle' or 'status'.")
        sys.exit(1)

def main():
    if len(sys.argv) > 1:
        apply_headless(sys.argv[1].lower())
        return

    app = EffectsSettingsWindow()
    app.connect("destroy", Gtk.main_quit)
    Gtk.main()

if __name__ == "__main__":
    main()
