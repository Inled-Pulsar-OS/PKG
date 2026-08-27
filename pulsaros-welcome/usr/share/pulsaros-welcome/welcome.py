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

import os
import shutil
import subprocess
import sys

import cairo
import gi

# Requerir versiones específicas de GTK, Gdk, GdkPixbuf y WebKit2
# Require specific versions of GTK, Gdk, GdkPixbuf, and WebKit2
gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
gi.require_version("GdkPixbuf", "2.0")
gi.require_version("WebKit2", "4.1")

from gi.repository import Gdk, GdkPixbuf, Gio, GLib, Gtk, WebKit2


def is_arch_system():
    """
    English: Detects if the running Pulsar OS edition is Arch-based (pacman)
             rather than Debian-based (apt-get), so platform-specific launcher
             paths (e.g. Winboat at /opt/winboat/winboat on Arch) can be used.
    Español: Detecta si la edición de Pulsar OS en ejecución está basada en
             Arch (pacman) en lugar de Debian (apt-get), para usar rutas de
             lanzamiento específicas de la plataforma (p. ej. Winboat en
             /opt/winboat/winboat en Arch).
    """
    try:
        with open("/etc/os-release", "r") as f:
            rel = f.read().lower()
        if "id_like=arch" in rel or "id=arch" in rel:
            return True
        if "id_like=debian" in rel or "id=debian" in rel or "id=ubuntu" in rel:
            return False
    except Exception:
        pass
    if shutil.which("pacman"):
        return True
    if shutil.which("apt-get"):
        return False
    return False


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

.page-dots {
    margin-top: 2px;
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
            if len(parts) >= 1 and "x" in parts[0]:
                res_str = parts[0]
                # Detect the active mode (usually marked with * or *+)
                is_active = False
                for p in parts[1:]:
                    if "*" in p:
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
        ("1024x768", False),
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
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION,
        )

        # Contenedor principal vertical
        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=20)
        vbox.set_margin_bottom(
            80
        )  # Espacio para empujar el botón arriba del dock/fondo
        self.add(vbox)

        # Webview para la animación SVG
        self.webview = WebKit2.WebView()
        webview_settings = self.webview.get_settings()
        webview_settings.set_allow_file_access_from_file_urls(True)
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
        lang = (os.environ.get("LANG") or os.environ.get("LANGUAGE") or "").split(".")[0].replace("-", "_")
        hello_uri = "file://" + hello_html_path + (f"?lang={lang}" if lang else "")
        self.webview.load_uri(hello_uri)

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

    def is_live_system(self):
        """
        English: Checks if the system is running in live-boot mode or QEMU test mode (rootfstype=9p)
                 so developers can test the recovery slide during development.
        Español: Comprueba si el sistema se está ejecutando en modo live-boot o en modo de prueba QEMU (rootfstype=9p)
                 para que los desarrolladores puedan probar la diapositiva de recuperación durante el desarrollo.
        """
        try:
            if os.path.exists("/lib/live/mount"):
                return True
            if os.getenv("USER") == "live" or os.getlogin() == "live":
                return True
            if os.path.exists("/proc/cmdline"):
                with open("/proc/cmdline", "r") as f:
                    cmdline = f.read()
                    if "boot=live" in cmdline or "rootfstype=9p" in cmdline:
                        return True
        except Exception as e:
            print("Error checking live system status:", e)
        return False

    def __init__(self):
        super().__init__(type=Gtk.WindowType.TOPLEVEL)
        self.set_title("Setup Assistant")
        self.set_default_size(780, 560)
        self.set_position(Gtk.WindowPosition.CENTER)
        self.set_resizable(False)

        self.current_step = 0
        self.is_live = self.is_live_system()
        self.steps_count = 14 if self.is_live else 13
        self.schema_source = self.load_custom_schemas()

        # Cargar estilos CSS personalizados de GTK
        style_provider = Gtk.CssProvider()
        style_provider.load_from_data(CSS_DATA.encode())
        Gtk.StyleContext.add_provider_for_screen(
            Gdk.Screen.get_default(),
            style_provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION,
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

        # Indicador de página (puntos estilo Apple) centrado en la barra inferior
        # English: Apple-style page indicator dots, centered on the bottom bar
        self.dots_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        self.dots_box.set_halign(Gtk.Align.CENTER)
        self.dots_box.set_hexpand(True)
        self.dots_box.get_style_context().add_class("page-dots")
        nav_bar.pack_start(self.dots_box, True, True, 0)

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

    def launch_app(self, command_name, fallback_command=None):
        """
        English: Launches an external application safely as a detached background process.
        Español: Lanza una aplicación externa de forma segura como un proceso en segundo plano independiente.
        """
        try:
            if shutil.which(command_name):
                subprocess.Popen([command_name], start_new_session=True)
                print(f"Welcome App: Launched {command_name}")
            elif fallback_command and shutil.which(fallback_command):
                subprocess.Popen([fallback_command], start_new_session=True)
                print(f"Welcome App: Launched fallback {fallback_command}")
            else:
                print(f"Welcome App Error: command {command_name} not found in path.")
        except Exception as e:
            print(f"Error launching {command_name}: {e}")

    def on_adb_check_clicked(self, button):
        """
        English: Runs `adb devices` and displays the connected mobile devices in a native GTK Dialog.
        Español: Ejecuta `adb devices` y muestra los dispositivos móviles conectados en un diálogo nativo de GTK.
        """
        try:
            result = subprocess.run(
                ["adb", "devices"], capture_output=True, text=True, timeout=5
            )
            output = result.stdout
        except Exception as e:
            output = (
                f"Error running adb: {e}\n\nMake sure adb is installed and running."
            )

        dialog = Gtk.MessageDialog(
            transient_for=self,
            flags=0,
            message_type=Gtk.MessageType.INFO,
            buttons=Gtk.ButtonsType.OK,
            text="ADB Connected Devices / Dispositivos Conectados",
        )
        dialog.format_secondary_text(output)
        dialog.run()
        dialog.destroy()

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

    def create_image_icon(self, file_name, fallback_icon):
        """
        English: Creates an icon widget from a custom image file, with a symbolic fallback icon.
        Español: Crea un widget de icono a partir de un archivo de imagen personalizado, con un icono simbólico de respaldo.
        """
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        box.get_style_context().add_class("icon-wrapper")
        box.set_halign(Gtk.Align.CENTER)

        curr_dir = os.path.dirname(os.path.abspath(__file__))
        path = os.path.join(curr_dir, "ui", file_name)
        if not os.path.exists(path):
            path = f"/usr/share/pulsaros-welcome/ui/{file_name}"

        if os.path.exists(path):
            try:
                pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(path, 80, 80, True)
                img = Gtk.Image.new_from_pixbuf(pixbuf)
            except Exception as e:
                print(f"Error loading custom image {file_name}: {e}")
                img = Gtk.Image.new_from_icon_name(fallback_icon, Gtk.IconSize.DIALOG)
        else:
            img = Gtk.Image.new_from_icon_name(fallback_icon, Gtk.IconSize.DIALOG)

        box.pack_start(img, True, True, 0)
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

        # Iconos simbólicos e imágenes para las diapositivas
        self.icon_stack.add_named(
            self.create_symbolic_icon("video-display-symbolic"), "icon1"
        )
        self.icon_stack.add_named(
            self.create_symbolic_icon("network-wireless-symbolic"), "icon2"
        )
        self.icon_stack.add_named(
            self.create_symbolic_icon("bluetooth-symbolic"), "icon3"
        )
        self.icon_stack.add_named(self.create_symbolic_icon("phone-symbolic"), "icon4")
        self.icon_stack.add_named(
            self.create_symbolic_icon("media-removable-symbolic"), "icon5"
        )
        self.icon_stack.add_named(
            self.create_image_icon("droidtux.png", "phone-symbolic"), "icon6"
        )
        self.icon_stack.add_named(
            self.create_image_icon("macboat.png", "computer-symbolic"), "icon7"
        )
        self.icon_stack.add_named(
            self.create_image_icon("winboat.svg", "computer-symbolic"), "icon8"
        )
        self.icon_stack.add_named(
            self.create_symbolic_icon("system-software-install-symbolic"), "icon9"
        )
        self.icon_stack.add_named(
            self.create_symbolic_icon("preferences-desktop-theme-symbolic"), "icon10"
        )
        self.icon_stack.add_named(
            self.create_symbolic_icon("applications-system-symbolic"), "icon11"
        )
        self.icon_stack.add_named(
            self.create_symbolic_icon("help-browser-symbolic"), "icon12"
        )
        if self.is_live:
            # English: Setup Recovery icon for the live recovery installation slide
            # Español: Configurar icono de Recovery para la diapositiva de recuperación en vivo
            self.icon_stack.add_named(
                self.create_symbolic_icon("system-run-symbolic"), "icon13"
            )

    def init_slides(self):
        self.titles = [
            "Welcome to Pulsar OS",
            "Select Screen Resolution",
            "Connect to Wi-Fi",
            "Set Up Bluetooth Connection",
            "Sync Mobile with GSConnect",
            "USB Debugging Setup",
            "Phone Integration with Droidtux",
            "Run macOS with Macboat",
            "Run Windows Apps with Winboat",
            "Installing Applications",
            "Desktop Special Effects",
            "GPU Driver Manager",
            "Beta Feedback & Support",
        ]
        if self.is_live:
            # English: Setup additional title for the recovery installer slide
            # Español: Configurar título adicional para la diapositiva del instalador recovery
            self.titles.append("Pulsar OS Recovery")

        # ----------------------------------------------------------------------
        # Slide 0: Welcome Screen
        # ----------------------------------------------------------------------
        slide_0 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_0.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(
            label="Pulsar OS combines speed, beauty, and design into a powerful and modern operating system. This Setup Assistant will guide you through the essential configurations to customize your experience on first boot.PulsarOS combines macOS design with Linux technology. A useful and functional distribution, ready for anything. This wizard will guide you through the initial setup, show you the essentials, and allow you to run the installer."
        )
        lbl_desc.get_style_context().add_class("desc-text")
        lbl_desc.set_max_width_chars(65)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_justify(Gtk.Justification.CENTER)
        slide_0.pack_start(lbl_desc, False, False, 10)

        self.content_stack.add_named(slide_0, "slide0")

        # ----------------------------------------------------------------------
        # Slide 1: Resolution Screen (Simplified Display Settings Button)
        # ----------------------------------------------------------------------
        slide_1 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_1.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(
            label="Adjust the desktop screen resolution to best fit your monitor. In virtual machine environments, opening system display settings will allow you to configure the ideal size."
        )
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
        btn_display.connect(
            "clicked", lambda b: os.system("gnome-control-center display &")
        )
        btn_box.pack_start(btn_display, False, False, 0)

        self.content_stack.add_named(slide_1, "slide1")

        # ----------------------------------------------------------------------
        # Slide 2: Wi-Fi Configuration
        # ----------------------------------------------------------------------
        slide_wifi = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_wifi.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(
            label="Connect to your Wi-Fi network to access online services, software updates, and app downloads. Pulsar OS uses NetworkManager, which manages wireless, Ethernet and mobile broadband connections automatically."
        )
        lbl_desc.get_style_context().add_class("desc-text")
        lbl_desc.set_max_width_chars(65)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_justify(Gtk.Justification.CENTER)
        slide_wifi.pack_start(lbl_desc, False, False, 10)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        btn_box.set_halign(Gtk.Align.CENTER)
        slide_wifi.pack_start(btn_box, True, True, 10)

        btn_wifi = Gtk.Button(label="Open Wi-Fi Settings")
        btn_wifi.get_style_context().add_class("action-button")
        btn_wifi.connect(
            "clicked", lambda b: os.system("gnome-control-center wifi &")
        )
        btn_box.pack_start(btn_wifi, False, False, 0)

        self.content_stack.add_named(slide_wifi, "slide2")

        # ----------------------------------------------------------------------
        # Slide 3: Bluetooth Devices
        # ----------------------------------------------------------------------
        slide_3 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_3.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(
            label="Connect wireless controllers, headphones, keyboards, or mice. Pulsar OS scans and listens for both Bluetooth Low Energy (BLE) and classic Bluetooth devices simultaneously for maximum hardware compatibility."
        )
        lbl_desc.get_style_context().add_class("desc-text")
        lbl_desc.set_max_width_chars(65)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_justify(Gtk.Justification.CENTER)
        slide_3.pack_start(lbl_desc, False, False, 10)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        btn_box.set_halign(Gtk.Align.CENTER)
        slide_3.pack_start(btn_box, True, True, 10)

        btn_bluetooth = Gtk.Button(label="Configure Bluetooth Devices")
        btn_bluetooth.get_style_context().add_class("action-button")
        btn_bluetooth.connect(
            "clicked", lambda b: os.system("gnome-control-center bluetooth &")
        )
        btn_box.pack_start(btn_bluetooth, False, False, 0)

        self.content_stack.add_named(slide_3, "slide3")

        # ----------------------------------------------------------------------
        # Slide 4: GSConnect Screen
        # ----------------------------------------------------------------------
        slide_4 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        slide_4.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(
            label="Link your Android or iOS device to share files, synchronize the clipboard, manage notifications, and control media features wirelessly. Keep both devices on the same Wi-Fi network."
        )
        lbl_desc.get_style_context().add_class("desc-text")
        lbl_desc.set_max_width_chars(65)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_justify(Gtk.Justification.CENTER)
        slide_4.pack_start(lbl_desc, False, False, 0)

        layout_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=24)
        layout_box.set_halign(Gtk.Align.CENTER)
        slide_4.pack_start(layout_box, True, True, 10)

        col_left = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        col_left.set_valign(Gtk.Align.CENTER)
        layout_box.pack_start(col_left, True, True, 0)

        btn_gsconnect = Gtk.Button(label="GSConnect Settings")
        btn_gsconnect.get_style_context().add_class("action-button")
        btn_gsconnect.connect(
            "clicked",
            lambda b: os.system(
                "gnome-extensions prefs gsconnect@andyholmes.github.io || gnome-shell-extension-prefs gsconnect@andyholmes.github.io &"
            ),
        )
        btn_gsconnect.set_halign(Gtk.Align.START)
        col_left.pack_start(btn_gsconnect, False, False, 0)

        lbl_col_text = Gtk.Label(
            label="Make sure the mobile app is open on your phone to initiate pairing."
        )
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
            qr_image = Gtk.Image.new_from_icon_name(
                "image-missing", Gtk.IconSize.DIALOG
            )

        col_right.pack_start(qr_image, False, False, 0)

        lbl_qr = Gtk.Label(label="Scan to download app")
        lbl_qr.get_style_context().add_class("qr-caption")
        col_right.pack_start(lbl_qr, False, False, 0)

        self.content_stack.add_named(slide_4, "slide4")

        # ----------------------------------------------------------------------
        # Slide 5: USB Debugging Setup (ADB)
        # ----------------------------------------------------------------------
        slide_5 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_5.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(
            label="Enable USB debugging in your mobile device's developer options. This allows Pulsar OS to communicate with your phone via ADB so that the integration of Android apps as native apps works."
        )
        lbl_desc.get_style_context().add_class("desc-text")
        lbl_desc.set_max_width_chars(65)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_justify(Gtk.Justification.CENTER)
        slide_5.pack_start(lbl_desc, False, False, 10)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        btn_box.set_halign(Gtk.Align.CENTER)
        slide_5.pack_start(btn_box, True, True, 10)

        btn_adb = Gtk.Button(label="Check Connected Devices (ADB)")
        btn_adb.get_style_context().add_class("action-button")
        btn_adb.connect("clicked", self.on_adb_check_clicked)
        btn_box.pack_start(btn_adb, False, False, 0)

        self.content_stack.add_named(slide_5, "slide5")

        # ----------------------------------------------------------------------
        # Slide 6: Phone Integration with Droidtux
        # ----------------------------------------------------------------------
        slide_6 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_6.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(
            label="Connect your Android device to your computer via USB, and Droidtux will integrate your phone's applications as native Pulsar OS apps. It offers the same experience as a Googlebook."
        )
        lbl_desc.get_style_context().add_class("desc-text")
        lbl_desc.set_max_width_chars(65)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_justify(Gtk.Justification.CENTER)
        slide_6.pack_start(lbl_desc, False, False, 10)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        btn_box.set_halign(Gtk.Align.CENTER)
        slide_6.pack_start(btn_box, True, True, 10)

        btn_droidtux = Gtk.Button(label="Launch Droidtux Sync")
        btn_droidtux.get_style_context().add_class("action-button")
        btn_droidtux.connect(
            "clicked", lambda b: self.launch_app("droidtux-sync", "scrcpy")
        )
        btn_box.pack_start(btn_droidtux, False, False, 0)

        self.content_stack.add_named(slide_6, "slide6")

        # ----------------------------------------------------------------------
        # Slide 7: Run macOS with Macboat
        # ----------------------------------------------------------------------
        slide_7 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_7.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(
            label="PulsarOS comes with Macboat pre-installed, a powerful application that allows you to run macOS within Linux. macOS is downloaded from Apple's official recovery servers. Please read the macOS EULA before using Macboat and accept the terms."
        )
        lbl_desc.get_style_context().add_class("desc-text")
        lbl_desc.set_max_width_chars(65)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_justify(Gtk.Justification.CENTER)
        slide_7.pack_start(lbl_desc, False, False, 10)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        btn_box.set_halign(Gtk.Align.CENTER)
        slide_7.pack_start(btn_box, True, True, 10)

        btn_macboat = Gtk.Button(label="Launch Macboat Setup")
        btn_macboat.get_style_context().add_class("action-button")
        btn_macboat.connect("clicked", lambda b: self.launch_app("macboat"))
        btn_box.pack_start(btn_macboat, False, False, 0)

        self.content_stack.add_named(slide_7, "slide7")

        # ----------------------------------------------------------------------
        # Slide 8: Run Windows Apps with Winboat
        # ----------------------------------------------------------------------
        slide_8 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_8.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(
            label="PulsarOS can run Windows apps without compatibility issues because it comes with Winboat built-in by default; this powerful application virtualizes Windows and integrates Windows apps as native Linux applications, ensuring a seamless experience."
        )
        lbl_desc.get_style_context().add_class("desc-text")
        lbl_desc.set_max_width_chars(65)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_justify(Gtk.Justification.CENTER)
        slide_8.pack_start(lbl_desc, False, False, 10)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        btn_box.set_halign(Gtk.Align.CENTER)
        slide_8.pack_start(btn_box, True, True, 10)

        btn_winboat = Gtk.Button(label="Launch Winboat Setup")
        btn_winboat.get_style_context().add_class("action-button")
        # English: On Arch the Winboat binary lives at /opt/winboat/winboat;
        #          on Debian the `winboat` command in PATH is used instead.
        # Español: En Arch el binario de Winboat está en /opt/winboat/winboat;
        #          en Debian se usa el comando `winboat` del PATH.
        btn_winboat.connect(
            "clicked",
            lambda b: self.launch_app(
                "/opt/winboat/winboat" if is_arch_system() else "winboat"
            ),
        )
        btn_box.pack_start(btn_winboat, False, False, 0)

        self.content_stack.add_named(slide_8, "slide8")

        # ----------------------------------------------------------------------
        # Slide 9: Installing Applications (New!)
        # ----------------------------------------------------------------------
        slide_9 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_9.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(
            label="Pulsar OS Software Center allows you to easily install Linux applications from both the official repositories and Flathub. Additionally, if you need other software, you can download standard .deb installation files (which act similarly to macOS .pkg or .dmg packages) from your web browser and install them by simply double-clicking on them."
        )
        lbl_desc.get_style_context().add_class("desc-text")
        lbl_desc.set_max_width_chars(65)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_justify(Gtk.Justification.CENTER)
        slide_9.pack_start(lbl_desc, False, False, 10)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        btn_box.set_halign(Gtk.Align.CENTER)
        slide_9.pack_start(btn_box, True, True, 10)

        btn_software = Gtk.Button(label="Open Software Center")
        btn_software.get_style_context().add_class("action-button")
        btn_software.connect("clicked", lambda b: self.launch_app("appinstall"))
        btn_box.pack_start(btn_software, False, False, 0)

        self.content_stack.add_named(slide_9, "slide9")

        # ----------------------------------------------------------------------
        # Slide 10: Desktop Special Effects (Liquid Glass vs Blur)
        # ----------------------------------------------------------------------
        slide_10 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_10.set_halign(Gtk.Align.CENTER)

        lbl_desc = Gtk.Label(
            label='Choose the desktop effect you like best. The basic "blur-my-shell" effect is perfect for older computers or those with mid-range hardware. The "Liquid Glass" effect consumes more resources because it is complex to render, but if you have a powerful computer, you will certainly enjoy it.'
        )
        lbl_desc.get_style_context().add_class("desc-text")
        lbl_desc.set_max_width_chars(65)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_justify(Gtk.Justification.CENTER)
        slide_10.pack_start(lbl_desc, False, False, 10)

        card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        card.get_style_context().add_class("central-card")
        card.set_halign(Gtk.Align.CENTER)
        slide_10.pack_start(card, True, True, 0)

        self.radio_blur = Gtk.RadioButton.new_with_label_from_widget(
            None, "Enable Blur my Shell (Standard performance - Recommended)"
        )
        self.radio_blur.connect("toggled", self.on_effects_toggled)
        card.pack_start(self.radio_blur, False, False, 4)

        self.radio_glass = Gtk.RadioButton.new_with_label_from_widget(
            self.radio_blur,
            "Enable Liquid Glass (Premium Apple look - High Resource Consumption!)",
        )
        card.pack_start(self.radio_glass, False, False, 4)

        lbl_warning = Gtk.Label()
        lbl_warning.set_markup(
            "<span foreground='#cc0000'><b>⚠️ Warning / Advertencia:</b> Liquid Glass consumes significant system resources. May cause lag on older GPUs or virtual machines.</span>"
        )
        lbl_warning.set_line_wrap(True)
        lbl_warning.set_max_width_chars(60)
        lbl_warning.set_justify(Gtk.Justification.CENTER)
        card.pack_start(lbl_warning, False, False, 8)

        is_glass = self.get_current_effects_state()
        if is_glass:
            self.radio_glass.set_active(True)
        else:
            self.radio_blur.set_active(True)

        self.content_stack.add_named(slide_10, "slide10")

        # ----------------------------------------------------------------------
        # Slide 11: GPU Driver Manager (driverman)
        # ----------------------------------------------------------------------
        slide_11 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_11.set_halign(Gtk.Align.CENTER)

        lbl_desc_10 = Gtk.Label(
            label="Pulsar OS detects your GPU automatically and recommends the best open-source or proprietary driver for it. Use Driver Manager to install, switch, or remove GPU drivers. Package conflicts can be resolved directly from the app."
        )
        lbl_desc_10.get_style_context().add_class("desc-text")
        lbl_desc_10.set_max_width_chars(65)
        lbl_desc_10.set_line_wrap(True)
        lbl_desc_10.set_justify(Gtk.Justification.CENTER)
        slide_11.pack_start(lbl_desc_10, False, False, 10)

        btn_box_10 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        btn_box_10.set_halign(Gtk.Align.CENTER)
        slide_11.pack_start(btn_box_10, True, True, 10)

        btn_drivers = Gtk.Button(label="Open Driver Manager / Abrir Gestor de Controladores")
        btn_drivers.get_style_context().add_class("action-button")
        btn_drivers.connect(
            "clicked",
            lambda b: self.launch_app("driverman-gui", "driverman"),
        )
        btn_box_10.pack_start(btn_drivers, False, False, 0)

        self.content_stack.add_named(slide_11, "slide11")

        # ----------------------------------------------------------------------
        # Slide 12: Support Channel
        # ----------------------------------------------------------------------
        slide_12 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_12.set_halign(Gtk.Align.CENTER)

        lbl_desc_11 = Gtk.Label(
            label="Pulsar OS is in active development. Help us improve stability by reporting installation bugs, hardware issues or user interface feedback directly to our official issue tracker on GitHub."
        )
        lbl_desc_11.get_style_context().add_class("desc-text")
        lbl_desc_11.set_max_width_chars(65)
        lbl_desc_11.set_line_wrap(True)
        lbl_desc_11.set_justify(Gtk.Justification.CENTER)
        slide_12.pack_start(lbl_desc_11, False, False, 10)

        # English: Container for the support buttons with a clean vertical layout and 8px spacing
        # Español: Contenedor para los botones de soporte con un diseño vertical limpio y espaciado de 8px
        btn_box_11 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        btn_box_11.set_halign(Gtk.Align.CENTER)
        slide_12.pack_start(btn_box_11, True, True, 10)

        # English: Button to open GitHub issues page
        # Español: Botón para abrir la página de incidencias en GitHub
        btn_issue_11 = Gtk.Button(label="Open Issues Page on GitHub / Reportar Incidencias")
        btn_issue_11.get_style_context().add_class("action-button")
        btn_issue_11.connect(
            "clicked",
            lambda b: os.system(
                "xdg-open https://github.com/Inled-Pulsar-OS/ISO/issues &"
            ),
        )
        btn_box_11.pack_start(btn_issue_11, False, False, 0)

        # English: Button to open the official Wiki documentation
        # Español: Botón para abrir la documentación Wiki oficial
        btn_wiki_11 = Gtk.Button(label="Open Official Wiki / Abrir Wiki Oficial")
        btn_wiki_11.get_style_context().add_class("action-button")
        btn_wiki_11.connect(
            "clicked",
            lambda b: os.system(
                "xdg-open https://github.com/Inled-Pulsar-OS/DOCS/wiki &"
            ),
        )
        btn_box_11.pack_start(btn_wiki_11, False, False, 0)

        # English: Button to visit the official website
        # Español: Botón para visitar la web oficial
        btn_web_11 = Gtk.Button(label="Visit Official Website / Visitar Web Oficial")
        btn_web_11.get_style_context().add_class("action-button")
        btn_web_11.connect(
            "clicked",
            lambda b: os.system(
                "xdg-open https://os.inled.es &"
            ),
        )
        btn_box_11.pack_start(btn_web_11, False, False, 0)

        # English: Button to join the official Discord server
        # Español: Botón para unirse al servidor oficial de Discord
        btn_discord_11 = Gtk.Button(label="Join Discord / Unirse a Discord")
        btn_discord_11.get_style_context().add_class("action-button")
        btn_discord_11.connect(
            "clicked",
            lambda b: os.system(
                "xdg-open https://discord.gg/PSeTkDMnr &"
            ),
        )
        btn_box_11.pack_start(btn_discord_11, False, False, 0)

        self.content_stack.add_named(slide_12, "slide12")

        # ----------------------------------------------------------------------
        # Slide 13: Pulsar OS Recovery (Only in Live System / Solo en Sistema Live)
        # ----------------------------------------------------------------------
        if self.is_live:
            slide_13 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
            slide_13.set_halign(Gtk.Align.CENTER)

            lbl_desc_12 = Gtk.Label(
                label="Pulsar OS is currently running in a Live Session. You can start the system installation on your computer or launch the backup recovery, browser or disk partitioner."
            )
            lbl_desc_12.get_style_context().add_class("desc-text")
            lbl_desc_12.set_max_width_chars(65)
            lbl_desc_12.set_line_wrap(True)
            lbl_desc_12.set_justify(Gtk.Justification.CENTER)
            slide_13.pack_start(lbl_desc_12, False, False, 10)

            btn_box_12 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
            btn_box_12.set_halign(Gtk.Align.CENTER)
            slide_13.pack_start(btn_box_12, True, True, 10)

            # English: Create button to launch recovery installer
            # Español: Crear botón para lanzar el instalador recovery
            btn_recovery = Gtk.Button(label="Launch Pulsar OS Recovery")
            btn_recovery.get_style_context().add_class("action-button")
            btn_recovery.connect(
                "clicked", lambda b: self.launch_app("pulsaros-recovery")
            )
            btn_box_12.pack_start(btn_recovery, False, False, 0)

            self.content_stack.add_named(slide_13, "slide13")

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

    def update_dots(self):
        """
        English: Rebuilds the Apple-style page indicator dots below the content,
                 highlighting the current step in Apple blue.
        Español: Reconstruye los puntos indicadores de página estilo Apple,
                 resaltando la diapositiva actual en azul Apple.
        """
        for child in self.dots_box.get_children():
            self.dots_box.remove(child)

        for i in range(self.steps_count):
            dot = Gtk.Label()
            if i == self.current_step:
                dot.set_markup("<span size='13000' foreground='#0066cc'>●</span>")
            else:
                dot.set_markup("<span size='9000' foreground='#c7c7cc'>●</span>")
            self.dots_box.pack_start(dot, False, False, 0)

        self.dots_box.show_all()

    def update_ui(self):
        # Cambiar icono
        self.icon_stack.set_visible_child_name(f"icon{self.current_step}")

        # Cambiar titulo
        self.title_label.set_text(self.titles[self.current_step])

        # Cambiar slide
        self.content_stack.set_visible_child_name(f"slide{self.current_step}")

        # Actualizar indicador de página
        self.update_dots()

        # Configurar botones
        if self.current_step == 0:
            self.btn_back.set_sensitive(False)
        else:
            self.btn_back.set_sensitive(True)

        if self.current_step == self.steps_count - 1:
            self.btn_next.set_label("Start")
        else:
            self.btn_next.set_label("Continue")

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
            print(
                f"Warning: GSettings schema '{schema_id}' is not found in welcome custom sources."
            )
        except Exception as e:
            print(f"Error checking schema '{schema_id}':", e)
        return None

    def load_custom_schemas(self):
        """
        English: Checks for local extension schema paths and links them to a custom SettingsSchemaSource.
        Español: Comprueba las rutas de esquemas de extensiones locales y las vincula a un SettingsSchemaSource personalizado.
        """
        default_source = Gio.SettingsSchemaSource.get_default()

        # Determine the base PKG directory relative to this script
        # Determinar el directorio base PKG relativo a este script
        script_dir = os.path.dirname(os.path.realpath(__file__))
        pkg_dir = os.path.abspath(os.path.join(script_dir, "../../../../"))

        # Candidate directories for GSettings schemas (global and local)
        # Directorios candidatos para esquemas GSettings (globales y locales)
        candidate_paths = [
            os.path.expanduser(
                "~/.local/share/gnome-shell/extensions/liquid-glass@thinkingcoding1231.gmail.com/schemas"
            ),
            "/usr/share/gnome-shell/extensions/liquid-glass@thinkingcoding1231.gmail.com/schemas",
            os.path.expanduser(
                "~/.local/share/gnome-shell/extensions/blur-my-shell@aunetx/schemas"
            ),
            "/usr/share/gnome-shell/extensions/blur-my-shell@aunetx/schemas",
            os.path.expanduser(
                "~/.local/share/gnome-shell/extensions/pulsar-dock@inled.es/schemas"
            ),
            "/usr/share/gnome-shell/extensions/pulsar-dock@inled.es/schemas",
            os.path.expanduser(
                "~/.local/share/gnome-shell/extensions/dash-to-dock@micxgx.gmail.com/schemas"
            ),
            "/usr/share/gnome-shell/extensions/dash-to-dock@micxgx.gmail.com/schemas",
            os.path.join(
                pkg_dir, "build/pkg-staging/pulsaros-gnome/usr/share/glib-2.0/schemas"
            ),
            os.path.join(pkg_dir, "pulsaros-gnome/usr/share/glib-2.0/schemas"),
        ]

        current_source = default_source
        for path in candidate_paths:
            if os.path.isdir(path):
                try:
                    files = os.listdir(path)
                    has_xml = any(f.endswith(".gschema.xml") for f in files)
                    has_compiled = "gschemas.compiled" in files

                    if not (has_xml or has_compiled):
                        continue

                    if has_xml and not has_compiled:
                        subprocess.run(
                            ["glib-compile-schemas", path], capture_output=True
                        )

                    current_source = Gio.SettingsSchemaSource.new_from_directory(
                        path, current_source, False
                    )
                except Exception as e:
                    print(
                        f"[Schema Welcome] Error loading custom schema path {path}: {e}"
                    )

        return current_source

    def get_current_effects_state(self):
        """
        English: Reads the currently enabled extensions from GNOME shell to set initial radio state.
        Español: Lee las extensiones actualmente habilitadas en GNOME shell para establecer el estado de radio inicial.
        """
        try:
            settings = self.get_safe_settings("org.gnome.shell")
            if settings:
                enabled = settings.get_strv("enabled-extensions")
                return "liquid-glass@thinkingcoding1231.gmail.com" in enabled
        except Exception as e:
            print("Error reading extensions state:", e)
        return False

    def on_effects_toggled(self, radio):
        """
        English: Callback triggered when the effects radio buttons are toggled.
                 Switches between Blur my Shell and Liquid Glass, and applies configurations.
        Español: Callback ejecutado cuando se pulsan los botones de radio de efectos.
                 Intercambia entre Blur my Shell y Liquid Glass, y aplica las configuraciones.
        """
        if self.radio_blur.get_active():
            # Activar Blur my Shell / Desactivar Liquid Glass
            # Activate Blur my Shell / Deactivate Liquid Glass
            self.set_extension_state("blur-my-shell@aunetx", True)
            self.set_extension_state("liquid-glass@thinkingcoding1231.gmail.com", False)
            self.apply_blur_myshell_dock_settings()
        else:
            # Activar Liquid Glass / Desactivar Blur my Shell
            # Activate Liquid Glass / Deactivate Blur my Shell
            self.set_extension_state("blur-my-shell@aunetx", False)
            self.set_extension_state("liquid-glass@thinkingcoding1231.gmail.com", True)
            self.apply_liquid_glass_settings()

    def apply_blur_myshell_dock_settings(self):
        """
        English: Applies Dash to Dock settings optimized for Blur my Shell mode.
        Español: Aplica la configuración de Dash to Dock optimizada para el modo Blur my Shell.
        """
        try:
            settings = self.get_safe_settings("org.gnome.shell.extensions.dash-to-dock")
            if settings:
                settings.set_double("background-opacity", 0.8)
                settings.set_boolean("custom-theme-shrink", False)
                settings.set_boolean("show-show-apps-button", False)
                settings.set_double("height-fraction", 0.9)
                settings.set_boolean("apply-custom-theme", True)
                settings.set_string("transparency-mode", "FIXED")
                settings.set_boolean("customize-alphas", False)
                print(
                    "GNOME Settings: Restored Dash to Dock settings for Blur my Shell."
                )
        except Exception as e:
            print("Error restoring Dash to Dock for Blur my Shell:", e)

    def apply_liquid_glass_settings(self):
        """
        English: Applies Liquid Glass settings and Dash to Dock settings optimized for glassmorphism.
        Español: Aplica las configuraciones de Liquid Glass y de Dash to Dock optimizadas para glassmorphism.
        """
        try:
            # 1. Configurar Dash to Dock para Liquid Glass (Opacidad 0.0 y alphas a 0 para dejar ver el cristal de fondo de forma 100% transparente)
            settings_dock = self.get_safe_settings(
                "org.gnome.shell.extensions.dash-to-dock"
            )
            if settings_dock:
                settings_dock.set_double("background-opacity", 0.0)
                settings_dock.set_boolean("custom-theme-shrink", False)
                settings_dock.set_boolean("show-show-apps-button", False)
                settings_dock.set_double("height-fraction", 0.9)
                settings_dock.set_boolean("apply-custom-theme", False)
                settings_dock.set_string("transparency-mode", "FIXED")
                settings_dock.set_boolean("customize-alphas", True)
                settings_dock.set_double("min-alpha", 0.0)
                settings_dock.set_double("max-alpha", 0.0)
        except Exception as e:
            print("Error configuring Dash to Dock for Liquid Glass:", e)

        try:
            # 2. Configurar Liquid Glass según los ajustes extraídos del host
            settings_glass = self.get_safe_settings(
                "org.gnome.shell.extensions.liquid-glass"
            )
            if settings_glass:
                settings_glass.set_int("application-blur-radius", 9)
                settings_glass.set_double("application-content-opacity", 1.0)
                settings_glass.set_double("application-corner-radius", 17.0)
                settings_glass.set_boolean("application-glass-all-windows", False)
                settings_glass.set_string("application-tint-color", "#000000")
                settings_glass.set_double("application-tint-strength", 0.06)
                settings_glass.set_strv("application-window-whitelist", [])
                settings_glass.set_double("dock-corner-radius", 24.0)
                settings_glass.set_int("dock-glass-expand", 3)
                settings_glass.set_string("dock-tint-color", "#000000")
                settings_glass.set_boolean("enable-application-glass", False)
                settings_glass.set_boolean("enable-menu-glass", True)
                settings_glass.set_boolean("enable-quick-settings-glass", False)
                settings_glass.set_string("menu-tint-color", "#000000")
                settings_glass.set_string("notification-tint-color", "#000000")
                settings_glass.set_string("osd-tint-color", "#000000")
                settings_glass.set_boolean("output-logs", False)
                print(
                    "GNOME Settings: Applied Liquid Glass and transparent dock settings."
                )
        except Exception as e:
            print("Error configuring Liquid Glass settings:", e)

    def set_extension_state(self, uuid, enable):
        """
        English: Enables or disables a GNOME Shell extension by its UUID.
                 Uses GSettings as primary API and falls back to gnome-extensions command.
        Español: Habilita o deshabilita una extensión de GNOME Shell por su UUID.
                 Usa GSettings como API principal y cae en el comando gnome-extensions de fallback.
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
            print(f"Error setting extension {uuid} state to {enable}: {e}")
            cmd = "enable" if enable else "disable"
            subprocess.run(["gnome-extensions", cmd, uuid], capture_output=True)


def main():
    first_boot = "--first-boot" in sys.argv

    def start_assistant():
        if first_boot:
            subprocess.Popen(["sudo", "-E", "/usr/bin/python3", "/usr/share/pulsaros/welcome_ootb.py"])
            Gtk.main_quit()
        else:
            AssistantWindow()

    done_file = os.path.expanduser("~/.config/pulsaros-welcome.done")
    force = "--force" in sys.argv or "-f" in sys.argv

    if not os.path.exists(done_file) or force or first_boot:
        HelloWindow(start_assistant)
        Gtk.main()
    else:
        print("Welcome app already completed. Use --force to run again.")


if __name__ == "__main__":
    main()
