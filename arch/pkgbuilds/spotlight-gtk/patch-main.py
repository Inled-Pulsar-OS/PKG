#!/usr/bin/env python3
"""Apply the Pulsar OS fixes to spotlight-gtk main.py.

Fixes:
- Focus-leave auto-hide with a 0.5s grace period (shortcut launches race
  with Wayland focus stealing and the window hid instantly).
- Delayed re-grab of search focus after present() so typing works when
  the compositor steals focus right after launch.
- Autostart file uses the actual interpreter/script instead of the
  distro-specific `spotlight-python` binary (absent on Arch).
"""
import sys

path = sys.argv[1]
with open(path) as f:
    src = f.read()

edits = [
    (
        "import sys\nimport os\nimport json\nimport gi\nimport subprocess\n",
        "import sys\nimport os\nimport json\nimport time\nimport gi\nimport subprocess\n",
    ),
    (
        "        self.ensure_autostart()\n        self.indicator_proc = None\n",
        "        self.ensure_autostart()\n        self.indicator_proc = None\n        self.focus_since = None\n",
    ),
    (
        '            desktop_content = """[Desktop Entry]\nName=Spotlight\nExec=spotlight-python --hidden\n',
        '            exec_cmd = f"{sys.executable} {os.path.abspath(__file__)} --hidden"\n'
        '            desktop_content = f"""[Desktop Entry]\nName=Spotlight\nExec={exec_cmd}\n',
    ),
    (
        """            if not is_hidden:
                self.win.present()
        else:
            self.win.present()
            self.search_entry.set_text("")
            self.search_entry.grab_focus()""",
        """            if not is_hidden:
                self.win.present()
                GLib.timeout_add(300, self.grab_search_focus)
        else:
            self.win.present()
            self.search_entry.set_text("")
            self.search_entry.grab_focus()
            GLib.timeout_add(300, self.grab_search_focus)""",
    ),
    (
        """        focus_ctrl = Gtk.EventControllerFocus()
        focus_ctrl.connect("leave", lambda _: self.win.set_visible(False))
        self.win.add_controller(focus_ctrl)""",
        """        focus_ctrl = Gtk.EventControllerFocus()
        focus_ctrl.connect("enter", self.on_focus_enter)
        focus_ctrl.connect("leave", self.on_focus_leave)
        self.win.add_controller(focus_ctrl)""",
    ),
    (
        """    def on_close_request(self, win):
        win.set_visible(False)
        return True
""",
        """    def on_close_request(self, win):
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
""",
    ),
]

for old, new in edits:
    if new in src:
        continue
    if old in src:
        src = src.replace(old, new, 1)
        continue
    print(f"ERROR: pattern not found (upstream may have changed): {old[:60]!r}")
    sys.exit(1)

with open(path, "w") as f:
    f.write(src)
print("main.py patched")
