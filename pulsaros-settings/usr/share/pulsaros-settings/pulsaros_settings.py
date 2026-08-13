#!/usr/bin/env python3
"""
Pulsar OS - System Settings (Ajustes del Sistema)
macOS-styled System Settings application with native Live Wallpaper manager,
Cursor preview, macOS keyboard remap, Liquid Glass, Spotlight selector, and Account management.
"""

import sys
import os
import pwd
import json
import subprocess
import glob
from pathlib import Path

import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Adw, Gio, GLib, Gdk, GdkPixbuf, Pango

APP_ID = "es.inled.PulsarSettings"
ICONS_DIR = Path("/usr/share/pulsaros-settings/icons")
if not ICONS_DIR.exists():
    ICONS_DIR = Path(__file__).resolve().parent / "icons"

LIVE_WALLPAPER_CONFIG = Path.home() / ".config" / "pulsaros" / "live-wallpaper.json"
DEFAULT_WALLPAPER_DIR = Path("/usr/share/backgrounds")

def get_current_user_info():
    username = os.environ.get("USER", "pulsaros")
    try:
        user_entry = pwd.getpwnam(username)
        full_name = user_entry.pw_gecos.split(",")[0] or username.capitalize()
    except Exception:
        full_name = username.capitalize()

    # Look for user avatar
    avatar_path = Path.home() / ".face"
    if not avatar_path.exists():
        avatar_path = Path.home() / ".face.icon"
    if not avatar_path.exists():
        acc_avatar = Path(f"/var/lib/AccountsService/icons/{username}")
        if acc_avatar.exists():
            avatar_path = acc_avatar

    return {
        "username": username,
        "name": full_name,
        "avatar": str(avatar_path) if avatar_path.exists() else None
    }

def get_icon_file(name):
    p = ICONS_DIR / name
    if p.exists():
        return str(p)
    # Check with .svg / .png
    for ext in [".svg", ".png"]:
        p_ext = ICONS_DIR / f"{name}{ext}"
        if p_ext.exists():
            return str(p_ext)
    return "preferences-system"

class PulsarSettingsApp(Adw.Application):
    def __init__(self):
        super().__init__(application_id=APP_ID, flags=Gio.ApplicationFlags.FLAGS_NONE)
        self.settings_bg = Gio.Settings(schema_id="org.gnome.desktop.background")
        self.settings_iface = Gio.Settings(schema_id="org.gnome.desktop.interface")
        self.settings_wm = Gio.Settings(schema_id="org.gnome.desktop.wm.keybindings")

    def do_activate(self):
        win = self.props.active_window
        if not win:
            win = MainWindow(self)
        win.present()

class MainWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app, title="Ajustes del Sistema")
        self.app = app
        self.set_default_size(980, 680)
        self.set_size_request(820, 560)

        # Main Split View (Sidebar + Content Page)
        self.split_view = Adw.NavigationSplitView()
        self.set_content(self.split_view)

        # --- SIDEBAR ---
        self.sidebar_page = Adw.NavigationPage(title="Ajustes")
        self.sidebar_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.sidebar_page.set_child(self.sidebar_box)

        # Header for sidebar
        sidebar_header = Adw.HeaderBar()
        sidebar_header.set_show_end_title_buttons(False)
        self.sidebar_box.append(sidebar_header)

        # User Account Card (Apple ID Style)
        user_info = get_current_user_info()
        user_card = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        user_card.set_margin_start(12)
        user_card.set_margin_end(12)
        user_card.set_margin_top(8)
        user_card.set_margin_bottom(12)
        user_card.add_css_class("card")
        user_card.set_cursor(Gdk.Cursor.new_from_name("pointer", None))

        # Avatar
        avatar = Adw.Avatar(size=48, text=user_info["name"], show_initials=True)
        if user_info["avatar"]:
            try:
                paintable = Gdk.Texture.new_from_filename(user_info["avatar"])
                avatar.set_custom_image(paintable)
            except Exception:
                pass
        user_card.append(avatar)

        # Name and Subtitle
        user_text_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        user_text_box.set_valign(Gtk.Align.CENTER)
        user_name_lbl = Gtk.Label(label=user_info["name"], xalign=0)
        user_name_lbl.add_css_class("heading")
        user_sub_lbl = Gtk.Label(label="ID de Pulsar OS, iCloud y Cuentas", xalign=0)
        user_sub_lbl.add_css_class("caption")
        user_sub_lbl.add_css_class("dim-label")
        user_text_box.append(user_name_lbl)
        user_text_box.append(user_sub_lbl)
        user_card.append(user_text_box)

        # Make user card clickable to open Accounts
        user_click = Gtk.GestureClick()
        user_click.connect("released", lambda *args: self.select_page("users"))
        user_card.add_controller(user_click)

        self.sidebar_box.append(user_card)

        # Sidebar Search
        search_entry = Gtk.SearchEntry()
        search_entry.set_placeholder_text("Buscar")
        search_entry.set_margin_start(12)
        search_entry.set_margin_end(12)
        search_entry.set_margin_bottom(8)
        self.sidebar_box.append(search_entry)

        # Sidebar Menu List
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_vexpand(True)
        self.sidebar_list = Gtk.ListBox()
        self.sidebar_list.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self.sidebar_list.add_css_class("navigation-sidebar")
        self.sidebar_list.connect("row-selected", self.on_sidebar_row_selected)
        scrolled.set_child(self.sidebar_list)
        self.sidebar_box.append(scrolled)

        # --- CONTENT VIEW ---
        self.content_page = Adw.NavigationPage(title="Detalles")
        self.content_stack = Gtk.Stack()
        self.content_stack.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
        
        content_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.content_header = Adw.HeaderBar()
        content_box.append(self.content_header)
        content_box.append(self.content_stack)
        self.content_page.set_child(content_box)

        self.split_view.set_sidebar(self.sidebar_page)
        self.split_view.set_content(self.content_page)

        # Build Sidebar Items & Content Pages
        self.pages = {}
        self.setup_categories()

        # Select first category (Apariencia) by default
        self.select_page("appearance")

    def setup_categories(self):
        categories = [
            ("appearance", "Apariencia", "preferences-desktop-appearance.svg", self.create_appearance_page),
            ("wallpaper", "Fondo de Pantalla & Live", "preferences-wallpaper.svg", self.create_wallpaper_page),
            ("cursors", "Punteros del Ratón", "preferences-desktop-cursors.svg", self.create_cursors_page),
            ("keyboard", "Teclado & Atajos macOS", "preferences-desktop-keyboard.svg", self.create_keyboard_page),
            ("effects", "Efectos & Liquid Glass", "preferences-desktop-effects.svg", self.create_effects_page),
            ("spotlight", "Lanzador Spotlight", "plasma-search.svg", self.create_spotlight_page),
            ("displays", "Pantallas & Monitores", "preferences-desktop-display.svg", self.create_displays_page),
            ("sound", "Sonido & Notificaciones", "preferences-desktop-sound.svg", self.create_sound_page),
            ("users", "Usuarios & Cuentas", "preferences-system-users.svg", self.create_users_page),
            ("about", "Acerca de Pulsar OS", "computer.svg", self.create_about_page),
        ]

        for page_id, title, icon_name, create_func in categories:
            row = Gtk.ListBoxRow()
            row.page_id = page_id
            
            box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
            box.set_margin_start(8)
            box.set_margin_end(8)
            box.set_margin_top(6)
            box.set_margin_bottom(6)

            icon_path = get_icon_file(icon_name)
            img = Gtk.Image.new_from_file(icon_path)
            img.set_pixel_size(24)
            box.append(img)

            lbl = Gtk.Label(label=title, xalign=0)
            lbl.set_hexpand(True)
            box.append(lbl)

            row.set_child(box)
            self.sidebar_list.append(row)

            # Create page widget and add to content stack
            page_widget = create_func()
            self.content_stack.add_named(page_widget, page_id)
            self.pages[page_id] = row

    def on_sidebar_row_selected(self, listbox, row):
        if row and hasattr(row, "page_id"):
            self.content_stack.set_visible_child_name(row.page_id)
            # Update title
            box = row.get_child()
            lbl = box.get_last_child()
            if isinstance(lbl, Gtk.Label):
                self.content_page.set_title(lbl.get_text())

    def select_page(self, page_id):
        if page_id in self.pages:
            self.sidebar_list.select_row(self.pages[page_id])

    # =========================================================================
    # 1. APARIENCIA / APPEARANCE
    # =========================================================================
    def create_appearance_page(self):
        page = Adw.PreferencesPage()
        group = Adw.PreferencesGroup(title="Aspecto Visual")
        page.add(group)

        # Dark / Light mode selector row
        mode_row = Adw.ActionRow(title="Esquema de Color", subtitle="Elige entre modo oscuro, claro o automático")
        mode_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        mode_box.set_valign(Gtk.Align.CENTER)

        dark_btn = Gtk.Button(label="Oscuro")
        dark_btn.connect("clicked", lambda b: self.set_color_scheme("prefer-dark"))
        light_btn = Gtk.Button(label="Claro")
        light_btn.connect("clicked", lambda b: self.set_color_scheme("default"))

        mode_box.append(light_btn)
        mode_box.append(dark_btn)
        mode_row.add_suffix(mode_box)
        group.add(mode_row)

        # Accent colors
        accent_group = Adw.PreferencesGroup(title="Color de Acento macOS")
        page.add(accent_group)
        accent_row = Adw.ActionRow(title="Color Principal", subtitle="Color de resaltado de botones, selecciones y elementos")
        
        accents = [
            ("Azul", "#007aff"), ("Morado", "#af52de"), ("Rosa", "#ff2d55"),
            ("Rojo", "#ff3b30"), ("Naranja", "#ff9500"), ("Amarillo", "#ffcc00"),
            ("Verde", "#34c759"), ("Grafito", "#8e8e93")
        ]
        color_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        color_box.set_valign(Gtk.Align.CENTER)
        for name, hex_code in accents:
            dot_btn = Gtk.Button()
            dot_btn.set_tooltip_text(name)
            dot_btn.set_size_request(24, 24)
            dot_btn.add_css_class("circular")
            dot_btn.set_css_classes(["circular"])
            # Apply colored background
            css = Gtk.CssProvider()
            css.load_from_data(f"button {{ background-color: {hex_code}; min-width: 24px; min-height: 24px; border-radius: 9999px; border: 2px solid rgba(255,255,255,0.2); }}".encode())
            dot_btn.get_style_context().add_provider(css, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)
            dot_btn.connect("clicked", lambda b, c=name: self.set_accent_color(c))
            color_box.append(dot_btn)

        accent_row.add_suffix(color_box)
        accent_group.add(accent_row)

        return page

    def set_color_scheme(self, scheme):
        try:
            self.app.settings_iface.set_string("color-scheme", scheme)
        except Exception as e:
            print(f"Error setting color scheme: {e}")

    def set_accent_color(self, color_name):
        print(f"Set accent color: {color_name}")

    # =========================================================================
    # 2. FONDO DE PANTALLA & LIVE WALLPAPER
    # =========================================================================
    def create_wallpaper_page(self):
        page = Adw.PreferencesPage()
        
        # Live Wallpaper Section
        live_group = Adw.PreferencesGroup(
            title="Motor de Fondo Animado (Live Wallpaper)",
            description="Fondos dinámicos y vídeo (.mp4, .webm, .gif, .mkv) sincronizados con Escritorio, SDDM y Bloqueo"
        )
        page.add(live_group)

        self.live_status_row = Adw.ActionRow(title="Estado del Fondo Animado", subtitle="Detenido")
        live_group.add(self.live_status_row)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        btn_box.set_valign(Gtk.Align.CENTER)

        pick_video_btn = Gtk.Button(label="Elegir Vídeo / GIF...")
        pick_video_btn.add_css_class("suggested-action")
        pick_video_btn.connect("clicked", self.on_pick_live_wallpaper)
        btn_box.append(pick_video_btn)

        stop_video_btn = Gtk.Button(label="Desactivar")
        stop_video_btn.add_css_class("destructive-action")
        stop_video_btn.connect("clicked", self.on_stop_live_wallpaper)
        btn_box.append(stop_video_btn)

        self.live_status_row.add_suffix(btn_box)

        # Static Wallpapers Grid
        static_group = Adw.PreferencesGroup(title="Fondos de Pantalla de Pulsar OS")
        page.add(static_group)

        wallpapers = glob.glob("/usr/share/backgrounds/*.png") + glob.glob("/usr/share/backgrounds/*.jpg")
        if wallpapers:
            flow_box = Gtk.FlowBox()
            flow_box.set_selection_mode(Gtk.SelectionMode.NONE)
            flow_box.set_max_children_per_line(4)
            flow_box.set_min_children_per_line(2)
            flow_box.set_row_spacing(12)
            flow_box.set_column_spacing(12)

            for wp in wallpapers:
                wp_btn = Gtk.Button()
                wp_btn.set_size_request(160, 100)
                wp_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
                
                try:
                    pix = GdkPixbuf.Pixbuf.new_from_file_at_scale(wp, 160, 90, True)
                    tex = Gdk.Texture.new_for_pixbuf(pix)
                    img = Gtk.Image.new_from_paintable(tex)
                    wp_box.append(img)
                except Exception:
                    pass

                lbl = Gtk.Label(label=Path(wp).stem, ellipsize=Pango.EllipsizeMode.END)
                lbl.add_css_class("caption")
                wp_box.append(lbl)
                wp_btn.set_child(wp_box)
                wp_btn.connect("clicked", lambda b, p=wp: self.set_static_wallpaper(p))
                flow_box.append(wp_btn)

            static_group.add(flow_box)

        self.update_live_wallpaper_ui()
        return page

    def update_live_wallpaper_ui(self):
        if LIVE_WALLPAPER_CONFIG.exists():
            try:
                with open(LIVE_WALLPAPER_CONFIG, "r") as f:
                    cfg = json.load(f)
                    if cfg.get("enabled"):
                        fname = Path(cfg.get("file", "")).name
                        self.live_status_row.set_subtitle(f"Activo: {fname} ({cfg.get('type')})")
                        return
            except Exception:
                pass
        self.live_status_row.set_subtitle("Detenido (Usando fondo estático)")

    def on_pick_live_wallpaper(self, btn):
        dialog = Gtk.FileDialog.new()
        dialog.set_title("Seleccionar Vídeo o Fondo Animado")
        
        # File filter
        filter_video = Gtk.FileFilter()
        filter_video.set_name("Vídeos y Animaciones (*.mp4, *.webm, *.gif, *.mkv, *.webp)")
        for pat in ["*.mp4", "*.webm", "*.gif", "*.mkv", "*.mov", "*.webp"]:
            filter_video.add_pattern(pat)
        
        filters = Gio.ListStore.new(Gtk.FileFilter)
        filters.append(filter_video)
        dialog.set_filters(filters)

        dialog.open(self, None, self.on_live_file_selected)

    def on_live_file_selected(self, dialog, result):
        try:
            file = dialog.open_finish(result)
            if file:
                path = file.get_path()
                subprocess.run(["pulsaros-live-wallpaper", "set", path], check=True)
                self.update_live_wallpaper_ui()
        except Exception as e:
            print(f"Error selecting live wallpaper: {e}")

    def on_stop_live_wallpaper(self, btn):
        try:
            subprocess.run(["pulsaros-live-wallpaper", "stop"], check=True)
            self.update_live_wallpaper_ui()
        except Exception as e:
            print(f"Error stopping live wallpaper: {e}")

    def set_static_wallpaper(self, path):
        try:
            # Stop live wallpaper first
            if LIVE_WALLPAPER_CONFIG.exists():
                subprocess.run(["pulsaros-live-wallpaper", "stop"], check=False)
                self.update_live_wallpaper_ui()

            uri = f"file://{Path(path).resolve()}"
            self.app.settings_bg.set_string("picture-uri", uri)
            self.app.settings_bg.set_string("picture-uri-dark", uri)
            subprocess.run(["gsettings", "set", "org.gnome.desktop.screensaver", "picture-uri", uri], check=False)

            # Sync to SDDM
            if Path("/var/lib/pulsar-sddm").exists():
                subprocess.run(["cp", "-f", path, "/var/lib/pulsar-sddm/pulsar-wallpaper.png"], check=False)
        except Exception as e:
            print(f"Error setting static wallpaper: {e}")

    # =========================================================================
    # 3. CURSORES / CURSORS
    # =========================================================================
    def create_cursors_page(self):
        page = Adw.PreferencesPage()
        group = Adw.PreferencesGroup(title="Punteros del Ratón Instalados", description="Selecciona el tema de cursor para previsualizarlo y aplicarlo en tiempo real")
        page.add(group)

        cursor_dirs = ["/usr/share/icons", str(Path.home() / ".icons")]
        found_themes = set()

        for cdir in cursor_dirs:
            if Path(cdir).exists():
                for sub in Path(cdir).iterdir():
                    if sub.is_dir() and (sub / "cursors").exists():
                        found_themes.add(sub.name)

        current_cursor = self.app.settings_iface.get_string("cursor-theme") or "Tahoe"

        for theme in sorted(list(found_themes)):
            row = Adw.ActionRow(title=theme)
            if theme == current_cursor:
                row.set_subtitle("Activo actualmente")
                row.add_css_class("accent")

            btn = Gtk.Button(label="Aplicar")
            if theme == current_cursor:
                btn.set_sensitive(False)
            btn.connect("clicked", lambda b, t=theme: self.apply_cursor_theme(t))
            row.add_suffix(btn)
            group.add(row)

        return page

    def apply_cursor_theme(self, theme_name):
        try:
            self.app.settings_iface.set_string("cursor-theme", theme_name)
            print(f"Applied cursor theme: {theme_name}")
            self.select_page("cursors")
        except Exception as e:
            print(f"Error applying cursor theme: {e}")

    # =========================================================================
    # 4. TECLADO & ATADOS MACOS
    # =========================================================================
    def create_keyboard_page(self):
        page = Adw.PreferencesPage()
        group = Adw.PreferencesGroup(title="Distribución y Mapeo de Teclado macOS")
        page.add(group)

        remap_row = Adw.ActionRow(
            title="Mapeo de Teclas macOS (Cmd como Ctrl)",
            subtitle="Permite usar la tecla Command/Super para copiar (Cmd+C), pegar (Cmd+V) y atajos de macOS en todo el sistema"
        )
        remap_switch = Gtk.Switch()
        remap_switch.set_valign(Gtk.Align.CENTER)
        
        # Check if remap service is active
        try:
            res = subprocess.run(["systemctl", "--user", "is-active", "gnome-macos-remap-wayland.service"], capture_output=True, text=True)
            remap_switch.set_active("active" in res.stdout)
        except Exception:
            remap_switch.set_active(True)

        remap_switch.connect("state-set", self.on_toggle_keyboard_remap)
        remap_row.add_suffix(remap_switch)
        group.add(remap_row)

        return page

    def on_toggle_keyboard_remap(self, switch, state):
        cmd = "start" if state else "stop"
        try:
            subprocess.run(["systemctl", "--user", cmd, "gnome-macos-remap-wayland.service"], check=False)
        except Exception as e:
            print(f"Error toggling remap service: {e}")
        return False

    # =========================================================================
    # 5. EFECTOS & LIQUID GLASS
    # =========================================================================
    def create_effects_page(self):
        page = Adw.PreferencesPage()
        group = Adw.PreferencesGroup(title="Efectos de Ventana y Liquid Glass", description="Controla desenfoques, sombras y efectos de vidrio líquido estilo macOS")
        page.add(group)

        lg_row = Adw.ActionRow(
            title="Activar Efecto Liquid Glass",
            subtitle="Aplica desenfoques translúcidos y bordes reflectantes a las ventanas activas"
        )
        lg_switch = Gtk.Switch()
        lg_switch.set_valign(Gtk.Align.CENTER)

        # Check if extension is enabled
        try:
            res = subprocess.run(["gnome-extensions", "info", "liquid-glass@thinkingcoding1231.gmail.com"], capture_output=True, text=True)
            lg_switch.set_active("State: ENABLED" in res.stdout or "State: ACTIVE" in res.stdout)
        except Exception:
            lg_switch.set_active(True)

        lg_switch.connect("state-set", self.on_toggle_liquid_glass)
        lg_row.add_suffix(lg_switch)
        group.add(lg_row)

        return page

    def on_toggle_liquid_glass(self, switch, state):
        action = "enable" if state else "disable"
        try:
            subprocess.run(["gnome-extensions", action, "liquid-glass@thinkingcoding1231.gmail.com"], check=False)
        except Exception as e:
            print(f"Error toggling Liquid Glass: {e}")
        return False

    # =========================================================================
    # 6. LANZADOR SPOTLIGHT
    # =========================================================================
    def create_spotlight_page(self):
        page = Adw.PreferencesPage()
        group = Adw.PreferencesGroup(
            title="Buscador Spotlight del Sistema",
            description="Configura qué motor de búsqueda se abre al presionar Cmd+Espacio o el botón de la barra superior"
        )
        page.add(group)

        options = [
            ("spotlight-python", "Spotlight macOS (Python)", "Lanzador flotante estilo macOS Tahoe con búsqueda instantánea"),
            ("spotlight-gtk", "Spotlight GTK", "Lanzador rápido en GTK nativo"),
            ("gnome-overview", "Vista de Actividades de GNOME", "Pantalla general de aplicaciones y escritorios de GNOME")
        ]

        for opt_id, opt_name, opt_desc in options:
            row = Adw.ActionRow(title=opt_name, subtitle=opt_desc)
            btn = Gtk.Button(label="Activar")
            btn.connect("clicked", lambda b, o=opt_id: self.set_spotlight_launcher(o))
            row.add_suffix(btn)
            group.add(row)

        return page

    def set_spotlight_launcher(self, launcher_id):
        print(f"Setting default Spotlight launcher to: {launcher_id}")
        if launcher_id == "spotlight-python":
            subprocess.run(["gsettings", "set", "org.gnome.shell.extensions.pulsaros-spotlight-launcher", "launcher-type", "'python'"], check=False)
        elif launcher_id == "spotlight-gtk":
            subprocess.run(["gsettings", "set", "org.gnome.shell.extensions.pulsaros-spotlight-launcher", "launcher-type", "'gtk'"], check=False)

    # =========================================================================
    # 7. PANTALLAS / DISPLAYS
    # =========================================================================
    def create_displays_page(self):
        page = Adw.PreferencesPage()
        group = Adw.PreferencesGroup(title="Monitores y Disposición de Pantalla")
        page.add(group)

        row = Adw.ActionRow(title="Ajustes Avanzados de Pantalla", subtitle="Resolución, tasa de refresco, escala fraccional y multimonitor")
        btn = Gtk.Button(label="Abrir Configuración de Pantallas")
        btn.connect("clicked", lambda b: subprocess.Popen(["gnome-control-center", "display"]))
        row.add_suffix(btn)
        group.add(row)
        return page

    # =========================================================================
    # 8. SONIDO / SOUND
    # =========================================================================
    def create_sound_page(self):
        page = Adw.PreferencesPage()
        group = Adw.PreferencesGroup(title="Sonido y Dispositivos de Audio")
        page.add(group)

        row = Adw.ActionRow(title="Panel de Audio y Salidas", subtitle="Ajustar volumen, micrófonos y sonido de inicio macOS")
        btn = Gtk.Button(label="Abrir Ajustes de Sonido")
        btn.connect("clicked", lambda b: subprocess.Popen(["gnome-control-center", "sound"]))
        row.add_suffix(btn)
        group.add(row)
        return page

    # =========================================================================
    # 9. USUARIOS / USERS
    # =========================================================================
    def create_users_page(self):
        page = Adw.PreferencesPage()
        user_info = get_current_user_info()

        group = Adw.PreferencesGroup(title="Cuenta de Usuario Actual")
        page.add(group)

        row = Adw.ActionRow(title=user_info["name"], subtitle=f"Usuario: {user_info['username']}")
        btn = Gtk.Button(label="Gestionar Usuarios")
        btn.connect("clicked", lambda b: subprocess.Popen(["gnome-control-center", "user-accounts"]))
        row.add_suffix(btn)
        group.add(row)
        return page

    # =========================================================================
    # 10. ACERCA DE PULSAR OS / ABOUT
    # =========================================================================
    def create_about_page(self):
        page = Adw.PreferencesPage()
        group = Adw.PreferencesGroup()
        page.add(group)

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        box.set_margin_top(24)
        box.set_margin_bottom(24)
        box.set_halign(Gtk.Align.CENTER)

        # Official Logo
        logo_path = "/usr/share/pixmaps/pulsar-logo.png"
        if Path(logo_path).exists():
            pix = GdkPixbuf.Pixbuf.new_from_file_at_scale(logo_path, 120, 120, True)
            tex = Gdk.Texture.new_for_pixbuf(pix)
            logo_img = Gtk.Image.new_from_paintable(tex)
            box.append(logo_img)

        title = Gtk.Label(label="Pulsar OS Pear Edition")
        title.add_css_class("title-1")
        box.append(title)

        version_lbl = Gtk.Label(label="Versión Rolling • Basado en Arch Linux")
        version_lbl.add_css_class("body")
        version_lbl.add_css_class("dim-label")
        box.append(version_lbl)

        group.add(box)

        # System Specs Group
        specs_group = Adw.PreferencesGroup(title="Especificaciones del Sistema")
        page.add(specs_group)

        # Kernel
        try:
            kernel_ver = subprocess.run(["uname", "-r"], capture_output=True, text=True).stdout.strip()
        except Exception:
            kernel_ver = "Linux"
        row_kernel = Adw.ActionRow(title="Kernel", subtitle=kernel_ver)
        specs_group.add(row_kernel)

        # Hostname
        try:
            hostname = subprocess.run(["hostname"], capture_output=True, text=True).stdout.strip()
        except Exception:
            hostname = "pulsaros"
        row_host = Adw.ActionRow(title="Nombre del Equipo", subtitle=hostname)
        specs_group.add(row_host)

        # Update button
        update_group = Adw.PreferencesGroup()
        page.add(update_group)
        update_btn = Gtk.Button(label="Buscar Actualizaciones del Sistema...")
        update_btn.add_css_class("suggested-action")
        update_btn.set_halign(Gtk.Align.CENTER)
        update_btn.connect("clicked", lambda b: subprocess.Popen(["appinstall"]))
        update_group.add(update_btn)

        return page

if __name__ == "__main__":
    app = PulsarSettingsApp()
    sys.exit(app.run(sys.argv))
