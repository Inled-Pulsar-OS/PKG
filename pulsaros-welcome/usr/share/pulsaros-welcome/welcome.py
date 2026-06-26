#!/usr/bin/env python3
# ==============================================================================
# Pulsar OS - Welcome Application Core (GTK3 Native Apple-like Setup Assistant)
# ==============================================================================
# English: Python script that manages the lifecycle of the welcome application.
#          Shows a transparent Apple-like hello animation first (via WebKit2),
#          then launches a native GTK3 window designed after the Apple macOS Setup
#          Assistant interface: light-gray backgrounds, white central configurations,
#          navigation buttons at the bottom-right (with blue continue button), and
#          monochromatic GNOME symbolic icons tinted in Apple blue without background circles.
#          Detects available resolutions dynamically to prevent invalid mode selection errors.
# Español: Script en Python que gestiona el ciclo de vida de la aplicación de bienvenida.
#          Muestra primero una animación transparente estilo Apple hello (mediante WebKit2)
#          y luego lanza una ventana nativa de GTK3 diseñada a imagen del Asistente
#          de Configuración de Apple macOS: fondos gris claro, configuraciones centrales
#          blancas, botones de navegación abajo a la derecha (con botón de continuar azul) e
#          iconos simbólicos monocromáticos de GNOME teñidos de azul Apple sin círculos de fondo.
#          Detecta dinámicamente las resoluciones disponibles para evitar errores de modos no soportados.

import sys
import os
import subprocess
import gi
import cairo

# Requerir versiones específicas de GTK y WebKit2
# Require specific versions of GTK and WebKit2
gi.require_version('Gtk', '3.0')
gi.require_version('WebKit2', '4.1')

from gi.repository import Gtk, WebKit2, Gdk, GLib, GdkPixbuf

CSS_DATA = """
window {
    background-color: #e3e3e6;
}

.main-container {
    padding: 40px 60px 24px 60px;
}

.icon-wrapper {
    margin-bottom: 20px;
}

.icon-style {
    color: #0066cc;
}

.title-label {
    font-size: 25px;
    font-weight: 700;
    color: #1d1d1f;
    margin-bottom: 20px;
    text-shadow: none;
}

.desc-text {
    font-size: 13px;
    color: #515154;
    margin-bottom: 24px;
}

.central-card {
    background-color: #ffffff;
    border: 1px solid #d2d2d7;
    border-radius: 12px;
    padding: 24px;
    margin-bottom: 12px;
}

.action-button {
    background-image: none;
    background-color: #ffffff;
    border: 1px solid #d2d2d7;
    border-radius: 8px;
    color: #0066cc;
    font-weight: 600;
    font-size: 13px;
    padding: 8px 20px;
    box-shadow: 0 1px 2px rgba(0,0,0,0.02);
}

.action-button:hover {
    background-color: #f5f5f7;
    border-color: #c5c5ca;
}

.action-button:active {
    background-color: #e3e3e6;
    box-shadow: none;
}

.resolution-scroll {
    background-color: #ffffff;
    border: 1px solid #d2d2d7;
    border-radius: 10px;
}

.resolution-list {
    background-color: transparent;
}

.resolution-row {
    padding: 10px 14px;
    color: #1d1d1f;
    font-size: 13px;
    border-bottom: 1px solid #f5f5f7;
}

.resolution-row:last-child {
    border-bottom: none;
}

.resolution-row:selected {
    background-color: #0066cc;
    color: #ffffff;
}

.nav-bar {
    margin-top: 24px;
}

.btn-nav {
    background-image: none;
    background-color: #ffffff;
    border: 1px solid #d2d2d7;
    border-radius: 8px;
    color: #1d1d1f;
    font-weight: 500;
    font-size: 13px;
    padding: 6px 20px;
    box-shadow: 0 1px 2px rgba(0,0,0,0.03);
}

.btn-nav:hover {
    background-color: #f5f5f7;
    border-color: #c5c5ca;
}

.btn-nav:active {
    background-color: #e3e3e6;
    box-shadow: none;
}

.btn-nav:disabled {
    color: #aeaeae;
    border-color: #e3e3e6;
    background-color: #f5f5f7;
    box-shadow: none;
}

.btn-continue {
    background-image: none;
    background-color: #0066cc;
    border: none;
    color: #ffffff;
    font-weight: 700;
    font-size: 13px;
    padding: 6px 20px;
    box-shadow: 0 1px 2px rgba(0,0,0,0.1);
}

.btn-continue:hover {
    background-color: #0077ed;
}

.btn-continue:active {
    background-color: #005bb5;
    box-shadow: none;
}

.qr-image-style {
    border: 1px solid #d2d2d7;
    border-radius: 8px;
    background-color: #ffffff;
    padding: 6px;
}

.qr-caption {
    font-size: 10px;
    font-weight: 700;
    color: #86868b;
    margin-top: 4px;
}
"""

def get_available_resolutions():
    """
    English: Runs xrandr to dynamically query and parse supported monitor resolutions.
             Flags the currently active resolution.
    Español: Ejecuta xrandr para consultar y parsear dinámicamente las resoluciones soportadas.
             Marca la resolución que esté actualmente activa.
    """
    try:
        output = subprocess.check_output("xrandr", shell=True, text=True)
        resolutions = []
        for line in output.splitlines():
            parts = line.strip().split()
            if len(parts) >= 1 and 'x' in parts[0]:
                res_str = parts[0]
                # Detect the active mode (usually marked with * or *+)
                is_active = False
                for p in parts[1:]:
                    if '*' in p:
                        is_active = True
                        break
                
                # Avoid duplicates
                if res_str not in [r[0] for r in resolutions]:
                    resolutions.append((res_str, is_active))
        if resolutions:
            return resolutions
    except Exception as e:
        print("xrandr parsing error (fallback to defaults):", e)
    
    # Static Fallback if xrandr fails or is not present
    return [
        ("1920x1080", False),
        ("1600x900", False),
        ("1440x900", False),
        ("1366x768", False),
        ("1280x800", False),
        ("1280x720", True),
        ("1024x768", False)
    ]


class HelloWindow(Gtk.Window):
    """
    English: Transparent window that plays the Apple hello SVG animation indefinitely
             until the white pill "Continue" button at the bottom is clicked.
    Español: Ventana transparente que reproduce la animación SVG de hello de Apple indefinidamente
             hasta que se pulsa el botón pill blanco "Continue" de la parte inferior.
    """
    def __init__(self, on_finish_callback):
        super().__init__(type=Gtk.WindowType.TOPLEVEL)
        self.on_finish_callback = on_finish_callback
        self.completed = False

        self.set_title("Hello")
        self.set_decorated(False)
        self.set_app_paintable(True)
        self.set_keep_above(True)
        self.set_position(Gtk.WindowPosition.CENTER)

        screen = self.get_screen()
        visual = screen.get_rgba_visual()
        if visual:
            self.set_visual(visual)

        self.maximize()

        # Cargar estilos CSS personalizados para el botón Pill Blanco
        style_provider = Gtk.CssProvider()
        hello_css = """
        button.hello-continue-btn {
            background-color: #ffffff;
            border: none;
            color: #1d1d1f;
            font-size: 15px;
            font-weight: 600;
            border-radius: 22px;
            padding: 10px 42px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.3);
            transition: all 0.2s ease;
        }
        button.hello-continue-btn:hover {
            background-color: #f5f5f7;
            box-shadow: 0 6px 16px rgba(0,0,0,0.4);
        }
        button.hello-continue-btn:active {
            background-color: #e3e3e6;
        }
        """
        style_provider.load_from_data(hello_css.encode())
        Gtk.StyleContext.add_provider_for_screen(
            Gdk.Screen.get_default(),
            style_provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

        # Contenedor principal vertical
        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=20)
        vbox.set_margin_bottom(80) # Espacio para empujar el botón arriba del dock/fondo
        self.add(vbox)

        # Webview para la animación SVG
        self.webview = WebKit2.WebView()
        bg_color = Gdk.RGBA()
        bg_color.alpha = 0.0
        self.webview.set_background_color(bg_color)
        vbox.pack_start(self.webview, True, True, 0)

        # Contenedor horizontal para centrar el botón "Continue"
        btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=0)
        btn_box.set_halign(Gtk.Align.CENTER)
        vbox.pack_end(btn_box, False, False, 0)

        # Botón Continue Pill Blanco
        btn_continue = Gtk.Button(label="Continue")
        btn_continue.get_style_context().add_class("hello-continue-btn")
        btn_continue.connect("clicked", lambda b: self.finish_animation())
        btn_box.pack_start(btn_continue, False, False, 0)

        curr_dir = os.path.dirname(os.path.abspath(__file__))
        hello_html_path = os.path.join(curr_dir, "hello", "index.html")
        self.webview.load_uri("file://" + hello_html_path)

        self.connect("draw", self.on_draw)
        self.show_all()

    def on_draw(self, widget, context):
        context.set_source_rgba(0.0, 0.0, 0.0, 0.0)
        context.set_operator(cairo.OPERATOR_CLEAR)
        context.paint()
        context.set_operator(cairo.OPERATOR_OVER)
        return False

    def finish_animation(self):
        if not self.completed:
            self.completed = True
            self.destroy()
            self.on_finish_callback()
        return False


class AssistantWindow(Gtk.Window):
    """
    English: Native GTK3 welcome assistant window styled after Apple macOS Setup Assistant.
             Centered layouts, titles, central configs, bottom right navigation.
             Uses tinted symbolic icons instead of background circles.
    Español: Ventana del asistente de bienvenida nativa de GTK3 inspirada en el Setup Assistant de Apple macOS.
             Layouts limpios y centrados, título, controles centrales, navegación inferior derecha.
             Usa iconos simbólicos coloreados sin círculos de fondo.
    """
    def __init__(self):
        super().__init__(type=Gtk.WindowType.TOPLEVEL)
        self.set_title("Setup Assistant")
        self.set_default_size(780, 560)
        self.set_position(Gtk.WindowPosition.CENTER)
        self.set_resizable(False)

        self.current_step = 0
        self.steps_count = 7

        # Cargar estilos CSS personalizados de GTK
        style_provider = Gtk.CssProvider()
        style_provider.load_from_data(CSS_DATA.encode())
        Gtk.StyleContext.add_provider_for_screen(
            Gdk.Screen.get_default(),
            style_provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        main_box.get_style_context().add_class("main-container")
        self.add(main_box)

        # ==============================================================================
        # PARTE SUPERIOR: ICONO O LOGOTIPO CENTRADO
        # ==============================================================================
        self.icon_stack = Gtk.Stack()
        self.icon_stack.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
        self.icon_stack.set_transition_duration(200)
        main_box.pack_start(self.icon_stack, False, False, 0)

        self.init_icons()

        # ==============================================================================
        # PARTE INTERMEDIA: TÍTULO CENTRADO
        # ==============================================================================
        self.title_label = Gtk.Label()
        self.title_label.get_style_context().add_class("title-label")
        self.title_label.set_justify(Gtk.Justification.CENTER)
        self.title_label.set_halign(Gtk.Align.CENTER)
        main_box.pack_start(self.title_label, False, False, 0)

        # ==============================================================================
        # PARTE CENTRAL: CONTENIDOS EN UN STACK
        # ==============================================================================
        self.content_stack = Gtk.Stack()
        self.content_stack.set_transition_type(Gtk.StackTransitionType.SLIDE_LEFT_RIGHT)
        self.content_stack.set_transition_duration(350)
        main_box.pack_start(self.content_stack, True, True, 0)

        self.init_slides()

        # ==============================================================================
        # PARTE INFERIOR: BARRA DE NAVEGACIÓN
        # ==============================================================================
        nav_bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        nav_bar.get_style_context().add_class("nav-bar")
        main_box.pack_end(nav_bar, False, False, 0)

        self.btn_back = Gtk.Button(label="Back")
        self.btn_back.get_style_context().add_class("btn-nav")
        self.btn_back.connect("clicked", self.on_back_clicked)
        nav_bar.pack_end(self.btn_back, False, False, 0)

        self.btn_next = Gtk.Button(label="Continue")
        self.btn_next.get_style_context().add_class("btn-continue")
        self.btn_next.connect("clicked", self.on_next_clicked)
        nav_bar.pack_end(self.btn_next, False, False, 0)

        self.update_ui()
        self.connect("destroy", Gtk.main_quit)
        self.show_all()

    def create_symbolic_icon(self, icon_name):
        """
        English: Creates an icon widget using a GNOME symbolic icon, styled and tinted in Apple blue.
        Español: Crea un widget de icono usando un icono simbólico de GNOME, estilizado y teñido en azul Apple.
        """
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        box.get_style_context().add_class("icon-wrapper")
        box.set_halign(Gtk.Align.CENTER)
        
        image = Gtk.Image.new_from_icon_name(icon_name, Gtk.IconSize.DIALOG)
        image.set_pixel_size(80)
        image.get_style_context().add_class("icon-style")
        box.pack_start(image, True, True, 0)
        return box

    def init_icons(self):
        # Icono 0: Logo de Pulsar OS (Imagen local logo.png)
        logo_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        logo_box.get_style_context().add_class("icon-wrapper")
        logo_box.set_halign(Gtk.Align.CENTER)
        
        curr_dir = os.path.dirname(os.path.abspath(__file__))
        logo_path = os.path.join(curr_dir, "logo.png")
        if not os.path.exists(logo_path):
            logo_path = "/usr/share/pulsaros-welcome/logo.png"

        if os.path.exists(logo_path):
            pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(logo_path, 80, 80, True)
            logo_img = Gtk.Image.new_from_pixbuf(pixbuf)
        else:
            logo_img = Gtk.Image.new_from_icon_name("computer", Gtk.IconSize.DIALOG)
        
        logo_box.pack_start(logo_img, True, True, 0)
        self.icon_stack.add_named(logo_box, "icon0")

        # Iconos simbólicos teñidos de azul sin círculo para los siguientes pasos
        self.icon_stack.add_named(self.create_symbolic_icon("video-display-symbolic"), "icon1")
        self.icon_stack.add_named(self.create_symbolic_icon("bluetooth-symbolic"), "icon2")
        self.icon_stack.add_named(self.create_symbolic_icon("phone-symbolic"), "icon3")
        self.icon_stack.add_named(self.create_symbolic_icon("usb-symbolic"), "icon4")
        self.icon_stack.add_named(self.create_symbolic_icon("computer-symbolic"), "icon5")
        self.icon_stack.add_named(self.create_symbolic_icon("help-browser-symbolic"), "icon6")

    def init_slides(self):
        self.titles = [
            "Welcome to Pulsar OS",
            "Select Screen Resolution",
            "Set Up Bluetooth Connection",
            "Sync Mobile with GSConnect",
            "USB Debugging & Integration",
            "Run macOS with Macboat",
            "Beta Feedback & Support"
        ]

        # ----------------------------------------------------------------------
        # Slide 0: Welcome Screen
        # ----------------------------------------------------------------------
        slide_0 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_0.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(label="Pulsar OS combines speed, beauty, and design into a powerful and modern operating system. This Setup Assistant will guide you through the essential configurations to customize your experience on first boot.")
        lbl_desc.get_style_context().add_class("desc-text")
        lbl_desc.set_max_width_chars(60)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_justify(Gtk.Justification.CENTER)
        slide_0.pack_start(lbl_desc, False, False, 10)

        self.content_stack.add_named(slide_0, "slide0")

        # ----------------------------------------------------------------------
        # Slide 1: Resolution Screen (Simplified Display Settings Button)
        # ----------------------------------------------------------------------
        slide_1 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_1.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(label="Adjust the desktop screen resolution to best fit your monitor. In virtual machine environments, opening system display settings will allow you to configure the ideal size.")
        lbl_desc.get_style_context().add_class("desc-text")
        lbl_desc.set_max_width_chars(65)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_justify(Gtk.Justification.CENTER)
        slide_1.pack_start(lbl_desc, False, False, 10)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        btn_box.set_halign(Gtk.Align.CENTER)
        slide_1.pack_start(btn_box, True, True, 10)

        btn_display = Gtk.Button(label="Open Display Settings")
        btn_display.get_style_context().add_class("action-button")
        btn_display.connect("clicked", lambda b: os.system("gnome-control-center display &"))
        btn_box.pack_start(btn_display, False, False, 0)

        self.content_stack.add_named(slide_1, "slide1")

        # ----------------------------------------------------------------------
        # Slide 2: Bluetooth Devices
        # ----------------------------------------------------------------------
        slide_2 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_2.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(label="Connect wireless controllers, headphones, keyboards, or mice. Pulsar OS scans and listens for both Bluetooth Low Energy (BLE) and classic Bluetooth devices simultaneously for maximum hardware compatibility.")
        lbl_desc.get_style_context().add_class("desc-text")
        lbl_desc.set_max_width_chars(65)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_justify(Gtk.Justification.CENTER)
        slide_2.pack_start(lbl_desc, False, False, 10)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        btn_box.set_halign(Gtk.Align.CENTER)
        slide_2.pack_start(btn_box, True, True, 10)

        btn_bluetooth = Gtk.Button(label="Configure Bluetooth Devices")
        btn_bluetooth.get_style_context().add_class("action-button")
        btn_bluetooth.connect("clicked", lambda b: os.system("gnome-control-center bluetooth &"))
        btn_box.pack_start(btn_bluetooth, False, False, 0)

        self.content_stack.add_named(slide_2, "slide2")

        # ----------------------------------------------------------------------
        # Slide 3: GSConnect Screen
        # ----------------------------------------------------------------------
        slide_3 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        slide_3.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(label="Link your Android or iOS device to share files, synchronize the clipboard, manage notifications, and control media features wirelessly. Keep both devices on the same Wi-Fi network.")
        lbl_desc.get_style_context().add_class("desc-text")
        lbl_desc.set_max_width_chars(65)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_justify(Gtk.Justification.CENTER)
        slide_3.pack_start(lbl_desc, False, False, 0)

        layout_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=24)
        layout_box.set_halign(Gtk.Align.CENTER)
        slide_3.pack_start(layout_box, True, True, 10)

        col_left = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        col_left.set_valign(Gtk.Align.CENTER)
        layout_box.pack_start(col_left, True, True, 0)

        btn_gsconnect = Gtk.Button(label="GSConnect Settings")
        btn_gsconnect.get_style_context().add_class("action-button")
        btn_gsconnect.connect("clicked", lambda b: os.system("gnome-extensions prefs gsconnect@andyholmes.github.io || gnome-shell-extension-prefs gsconnect@andyholmes.github.io &"))
        btn_gsconnect.set_halign(Gtk.Align.START)
        col_left.pack_start(btn_gsconnect, False, False, 0)

        lbl_col_text = Gtk.Label(label="Make sure the mobile app is open on your phone to initiate pairing.")
        lbl_col_text.get_style_context().add_class("card-desc")
        lbl_col_text.set_line_wrap(True)
        lbl_col_text.set_max_width_chars(30)
        lbl_col_text.set_xalign(0.0)
        col_left.pack_start(lbl_col_text, False, False, 0)

        col_right = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        col_right.set_valign(Gtk.Align.CENTER)
        layout_box.pack_end(col_right, False, False, 0)

        # Cargar QR
        curr_dir = os.path.dirname(os.path.abspath(__file__))
        qr_path = os.path.join(curr_dir, "ui", "kdeconnect-qr.png")
        if not os.path.exists(qr_path):
            qr_path = "/usr/share/pulsaros-welcome/ui/kdeconnect-qr.png"

        if os.path.exists(qr_path):
            pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(qr_path, 110, 110, True)
            qr_image = Gtk.Image.new_from_pixbuf(pixbuf)
            qr_image.get_style_context().add_class("qr-image-style")
        else:
            qr_image = Gtk.Image.new_from_icon_name("image-missing", Gtk.IconSize.DIALOG)

        col_right.pack_start(qr_image, False, False, 0)

        lbl_qr = Gtk.Label(label="Scan to download app")
        lbl_qr.get_style_context().add_class("qr-caption")
        col_right.pack_start(lbl_qr, False, False, 0)

        self.content_stack.add_named(slide_3, "slide3")

        # ----------------------------------------------------------------------
        # Slide 4: USB Integration
        # ----------------------------------------------------------------------
        slide_4 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_4.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(label="Enable USB Debugging in Developer Options on your mobile. Once connected by cable, launch Droidtux to cast, mirror, control and integrate your phone layout as native windows on the desktop.")
        lbl_desc.get_style_context().add_class("desc-text")
        lbl_desc.set_max_width_chars(65)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_justify(Gtk.Justification.CENTER)
        slide_4.pack_start(lbl_desc, False, False, 10)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        btn_box.set_halign(Gtk.Align.CENTER)
        slide_4.pack_start(btn_box, True, True, 10)

        btn_droidtux = Gtk.Button(label="Launch Droidtux Sync")
        btn_droidtux.get_style_context().add_class("action-button")
        btn_droidtux.connect("clicked", lambda b: os.system("droidtux || scrcpy &"))
        btn_box.pack_start(btn_droidtux, False, False, 0)

        self.content_stack.add_named(slide_4, "slide4")

        # ----------------------------------------------------------------------
        # Slide 5: Run macOS
        # ----------------------------------------------------------------------
        slide_5 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_5.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(label="Pulsar OS preinstalls Macboat, allowing you to configure and run macOS virtual machines. KVM configurations are optimized dynamically for accelerated virtual graphics and native performance.")
        lbl_desc.get_style_context().add_class("desc-text")
        lbl_desc.set_max_width_chars(65)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_justify(Gtk.Justification.CENTER)
        slide_5.pack_start(lbl_desc, False, False, 10)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        btn_box.set_halign(Gtk.Align.CENTER)
        slide_5.pack_start(btn_box, True, True, 10)

        btn_macboat = Gtk.Button(label="Launch Macboat Setup")
        btn_macboat.get_style_context().add_class("action-button")
        btn_macboat.connect("clicked", lambda b: os.system("macboat &"))
        btn_box.pack_start(btn_macboat, False, False, 0)

        self.content_stack.add_named(slide_5, "slide5")

        # ----------------------------------------------------------------------
        # Slide 6: Support Channel
        # ----------------------------------------------------------------------
        slide_6 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_6.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(label="Pulsar OS is in active development. Help us improve stability by reporting installation bugs, hardware issues or user interface feedback directly to our official issue tracker on GitHub.")
        lbl_desc.get_style_context().add_class("desc-text")
        lbl_desc.set_max_width_chars(65)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_justify(Gtk.Justification.CENTER)
        slide_6.pack_start(lbl_desc, False, False, 10)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        btn_box.set_halign(Gtk.Align.CENTER)
        slide_6.pack_start(btn_box, True, True, 10)

        btn_issue = Gtk.Button(label="Open Issues Page on GitHub")
        btn_issue.get_style_context().add_class("action-button")
        btn_issue.connect("clicked", lambda b: os.system("xdg-open https://github.com/Inled-Pulsar-OS/ISO/issues &"))
        btn_box.pack_start(btn_issue, False, False, 0)

        self.content_stack.add_named(slide_6, "slide6")

    def on_back_clicked(self, button):
        if self.current_step > 0:
            self.current_step -= 1
            self.update_ui()

    def on_next_clicked(self, button):
        if self.current_step < self.steps_count - 1:
            self.current_step += 1
            self.update_ui()
        else:
            # Crear marca de finalización y cerrar
            done_file = os.path.expanduser("~/.config/pulsaros-welcome.done")
            os.makedirs(os.path.dirname(done_file), exist_ok=True)
            with open(done_file, "w") as f:
                f.write("done")
            Gtk.main_quit()

    def update_ui(self):
        # Cambiar icono
        self.icon_stack.set_visible_child_name(f"icon{self.current_step}")

        # Cambiar titulo
        self.title_label.set_text(self.titles[self.current_step])

        # Cambiar slide
        self.content_stack.set_visible_child_name(f"slide{self.current_step}")

        # Configurar botones
        if self.current_step == 0:
            self.btn_back.set_sensitive(False)
        else:
            self.btn_back.set_sensitive(True)

        if self.current_step == self.steps_count - 1:
            self.btn_next.set_label("Start")
        else:
            self.btn_next.set_label("Continue")


def main():
    def start_assistant():
        AssistantWindow()

    done_file = os.path.expanduser("~/.config/pulsaros-welcome.done")
    force = "--force" in sys.argv or "-f" in sys.argv
    
    if not os.path.exists(done_file) or force:
        HelloWindow(start_assistant)
        Gtk.main()
    else:
        print("Welcome app already completed. Use --force to run again.")

if __name__ == "__main__":
    main()
