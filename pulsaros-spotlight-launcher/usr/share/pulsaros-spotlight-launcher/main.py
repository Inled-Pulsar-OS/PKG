import sys
import os
import json
import time
import gi
import subprocess
gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
from gi.repository import Gtk, Gdk, Gio, GLib, Adw

class SpotlightApp(Adw.Application):
    def __init__(self):
        super().__init__(application_id='com.jaime.spotlight',
                         flags=Gio.ApplicationFlags.FLAGS_NONE)
        self.all_apps = []
        self.filtered_apps = []
        self.win = None
        self.config_dir = os.path.join(GLib.get_user_config_dir(), 'spotlight-python')
        self.config_file = os.path.join(self.config_dir, 'config.json')
        self.is_grid_view = self.load_config()
        self.ensure_autostart()
        self.indicator_proc = None
        self.focus_since = None

    def load_config(self):
        if os.path.exists(self.config_file):
            try:
                with open(self.config_file, 'r') as f:
                    config = json.load(f)
                    return config.get('is_grid_view', False)
            except:
                pass
        return False

    def save_config(self):
        if not os.path.exists(self.config_dir):
            os.makedirs(self.config_dir)
        try:
            with open(self.config_file, 'w') as f:
                json.dump({'is_grid_view': self.is_grid_view}, f)
        except Exception as e:
            print(f"Error saving config: {e}")

    def ensure_autostart(self):
        autostart_dir = os.path.join(GLib.get_user_config_dir(), 'autostart')
        if not os.path.exists(autostart_dir):
            try:
                os.makedirs(autostart_dir)
            except:
                return
        
        autostart_file = os.path.join(autostart_dir, 'spotlight-python.desktop')
        if not os.path.exists(autostart_file):
            exec_cmd = f"{sys.executable} {os.path.abspath(__file__)} --hidden"
            desktop_content = f"""[Desktop Entry]
Name=Spotlight
Exec={exec_cmd}
Icon=view-app-grid-symbolic
Type=Application
Categories=Utility;
Comment=Apple-like Spotlight launcher
Terminal=false
X-GNOME-Autostart-enabled=true
"""
            try:
                with open(autostart_file, 'w') as f:
                    f.write(desktop_content)
            except Exception as e:
                print(f"Error creating autostart file: {e}")

    def start_tray(self):
        if self.indicator_proc is not None:
            return
        
        script_dir = os.path.dirname(os.path.abspath(__file__))
        tray_script = os.path.join(script_dir, 'tray.py')
        
        if os.path.exists(tray_script):
            try:
                self.indicator_proc = subprocess.Popen([sys.executable, tray_script, str(os.getpid())])
            except Exception as e:
                print(f"Failed to start tray: {e}")
        else:
            sys_tray = "/usr/share/spotlight-python/tray.py"
            if os.path.exists(sys_tray):
                try:
                    self.indicator_proc = subprocess.Popen([sys.executable, sys_tray, str(os.getpid())])
                except Exception as e:
                    print(f"Failed to start system tray: {e}")

    def do_activate(self):
        self.hold()
        self.start_tray()
        if not self.win:
            self.load_apps()
            self.build_ui()
            
            is_hidden = False
            for arg in sys.argv:
                if arg == "--hidden":
                    is_hidden = True
                    break
            
            if not is_hidden:
                self.win.present()
                GLib.timeout_add(300, self.grab_search_focus)
        else:
            self.win.present()
            self.search_entry.set_text("")
            self.search_entry.grab_focus()
            GLib.timeout_add(300, self.grab_search_focus)
            # We don't reload apps here to be instant. 
            # Apps are already loaded in __init__ or first activation.

    def build_ui(self):
        style_manager = Adw.StyleManager.get_default()
        style_manager.set_color_scheme(Adw.ColorScheme.PREFER_DARK)
        
        self.win = Gtk.ApplicationWindow(application=self)
        self.win.set_default_size(680, 500)
        self.win.set_decorated(False)
        self.win.set_resizable(False)
        self.win.set_title("Spotlight")

        script_dir = os.path.dirname(os.path.abspath(__file__))
        css_path = os.path.join(script_dir, 'style.css')
        css_provider = Gtk.CssProvider()
        if os.path.exists(css_path):
            css_provider.load_from_path(css_path)
        Gtk.StyleContext.add_provider_for_display(Gdk.Display.get_default(), css_provider, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)

        self.main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.main_box.add_css_class("spotlight-main")
        self.win.set_child(self.main_box)

        search_container = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=15)
        search_container.add_css_class("search-header")
        
        search_icon = Gtk.Image.new_from_icon_name("system-search-symbolic")
        search_icon.add_css_class("search-icon")
        
        self.search_entry = Gtk.Entry()
        self.search_entry.set_placeholder_text("Search applications...")
        self.search_entry.set_hexpand(True)
        self.search_entry.add_css_class("search-input")
        self.search_entry.connect("changed", self.on_search_changed)
        self.search_entry.connect("activate", self.on_enter_pressed)
        
        self.view_toggle_btn = Gtk.Button()
        self.view_toggle_btn.set_icon_name("view-grid-symbolic")
        self.view_toggle_btn.add_css_class("view-toggle")
        self.view_toggle_btn.connect("clicked", self.toggle_view)

        search_container.append(search_icon)
        search_container.append(self.search_entry)
        search_container.append(self.view_toggle_btn)
        self.main_box.append(search_container)

        self.scroll = Gtk.ScrolledWindow()
        self.scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        self.scroll.set_vexpand(True)
        self.scroll.add_css_class("results-area")
        
        self.stack = Gtk.Stack()
        self.stack.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
        
        self.list_box = Gtk.ListBox()
        self.list_box.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self.list_box.connect("row-activated", self.on_row_activated)
        
        self.grid = Gtk.FlowBox()
        self.grid.set_valign(Gtk.Align.START)
        self.grid.set_max_children_per_line(6)
        self.grid.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self.grid.connect("child-activated", self.on_child_activated)

        self.stack.add_named(self.list_box, "list")
        self.stack.add_named(self.grid, "grid")
        self.stack.set_visible_child_name("grid" if self.is_grid_view else "list")
        self.view_toggle_btn.set_icon_name("view-list-symbolic" if self.is_grid_view else "view-grid-symbolic")
        
        self.scroll.set_child(self.stack)
        self.main_box.append(self.scroll)

        focus_ctrl = Gtk.EventControllerFocus()
        focus_ctrl.connect("enter", self.on_focus_enter)
        focus_ctrl.connect("leave", self.on_focus_leave)
        self.win.add_controller(focus_ctrl)

        key_ctrl = Gtk.EventControllerKey()
        key_ctrl.connect("key-pressed", self.on_key_pressed)
        self.win.add_controller(key_ctrl)

        self.render_results()
        self.search_entry.grab_focus()
        self.win.connect("close-request", self.on_close_request)

    def on_close_request(self, win):
        win.set_visible(False)
        return True

    def grab_search_focus(self):
        if self.win is not None and self.win.get_visible():
            self.search_entry.grab_focus()
        return False

    def on_focus_enter(self, ctrl):
        self.focus_since = time.time()

    def on_focus_leave(self, ctrl):
        if self.focus_since is not None and (time.time() - self.focus_since) > 0.5:
            self.win.set_visible(False)
        self.focus_since = None

    def toggle_view(self, btn):
        self.is_grid_view = not self.is_grid_view
        self.stack.set_visible_child_name("grid" if self.is_grid_view else "list")
        btn.set_icon_name("view-list-symbolic" if self.is_grid_view else "view-grid-symbolic")
        self.save_config()
        self.render_results()

    def on_search_changed(self, entry):
        query = entry.get_text().lower()
        self.filtered_apps = [a for a in self.all_apps if query in a['name'].lower() or query in a['comment'].lower()]
        self.render_results()

    def render_results(self):
        while (child := self.list_box.get_first_child()): self.list_box.remove(child)
        while (child := self.grid.get_first_child()): self.grid.remove(child)

        for app in self.filtered_apps[:200]:
            if self.is_grid_view:
                self.grid.append(self.create_grid_item(app))
            else:
                self.list_box.append(self.create_list_item(app))
        
        if not self.is_grid_view:
            first_row = self.list_box.get_row_at_index(0)
            if first_row: self.list_box.select_row(first_row)

    def create_list_item(self, app):
        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=15)
        box.add_css_class("app-item-list")
        if app['icon']:
            icon = Gtk.Image.new_from_gicon(app['icon'])
        else:
            icon = Gtk.Image.new_from_icon_name("application-x-executable-symbolic")
        icon.set_pixel_size(32)
        icon.add_css_class("app-icon")
        info_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        name_label = Gtk.Label(label=app['name'], xalign=0)
        name_label.add_css_class("app-name")
        desc_label = Gtk.Label(label=app['comment'], xalign=0)
        desc_label.add_css_class("app-desc")
        desc_label.set_ellipsize(True)
        desc_label.set_max_width_chars(60)
        info_box.append(name_label)
        info_box.append(desc_label)
        box.append(icon)
        box.append(info_box)
        return box

    def create_grid_item(self, app):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        box.add_css_class("app-item-grid")
        box.set_size_request(100, -1)
        if app['icon']:
            icon = Gtk.Image.new_from_gicon(app['icon'])
        else:
            icon = Gtk.Image.new_from_icon_name("application-x-executable-symbolic")
        icon.set_pixel_size(64)
        icon.add_css_class("app-icon-grid")
        name_label = Gtk.Label(label=app['name'])
        name_label.add_css_class("app-name-grid")
        name_label.set_wrap(True)
        name_label.set_justify(Gtk.Justification.CENTER)
        name_label.set_max_width_chars(15)
        name_label.set_halign(Gtk.Align.CENTER)
        box.append(icon)
        box.append(name_label)
        return box

    def on_enter_pressed(self, entry):
        if self.filtered_apps:
            self.launch_app(self.filtered_apps[0])

    def on_row_activated(self, listbox, row):
        index = row.get_index()
        self.launch_app(self.filtered_apps[index])

    def on_child_activated(self, flowbox, child):
        index = child.get_index()
        self.launch_app(self.filtered_apps[index])

    def launch_app(self, app):
        app['app_info'].launch(None, None)
        self.win.set_visible(False)

    def on_key_pressed(self, ctrl, keyval, keycode, state):
        if keyval == Gdk.KEY_Escape:
            self.win.set_visible(False)
            return True
        if (state & Gdk.ModifierType.CONTROL_MASK) and keyval == Gdk.KEY_q:
            self.quit()
            return True
        return False

    def load_apps(self):
        app_dict = {}
        all_app_infos = Gio.AppInfo.get_all()
        for app in all_app_infos:
            if app.get_name() and app.should_show():
                app_id = app.get_id() or app.get_name()
                app_dict[app_id] = {
                    'name': app.get_name(),
                    'comment': app.get_description() or "",
                    'icon': app.get_icon(),
                    'app_info': app
                }
        paths = ["/usr/share/applications", "/usr/local/share/applications", "/var/lib/flatpak/exports/share/applications", "/var/lib/snapd/desktop/applications"]
        home = GLib.get_home_dir()
        if home:
            paths.append(home + "/.local/share/applications")
            paths.append(home + "/.local/share/flatpak/exports/share/applications")
        for path in paths:
            if not os.path.exists(path): continue
            for filename in os.listdir(path):
                if filename.endswith(".desktop"):
                    file_path = os.path.join(path, filename)
                    try:
                        app = Gio.DesktopAppInfo.new_from_filename(file_path)
                        if app and app.get_name() and app.should_show():
                            app_id = app.get_id() or filename
                            if app_id not in app_dict:
                                app_dict[app_id] = {
                                    'name': app.get_name(),
                                    'comment': app.get_description() or "",
                                    'icon': app.get_icon(),
                                    'app_info': app
                                }
                    except:
                        continue
        self.all_apps = list(app_dict.values())
        self.all_apps.sort(key=lambda x: x['name'].lower())
        self.filtered_apps = self.all_apps[:]

if __name__ == "__main__":
    app = SpotlightApp()
    clean_argv = [arg for arg in sys.argv if arg != "--hidden"]
    app.run(clean_argv)
