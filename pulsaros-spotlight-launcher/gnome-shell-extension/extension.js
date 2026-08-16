/**
 * Pulsar OS - Spotlight Launcher Extension
 * Adds the authentic macOS Spotlight search icon to GNOME top panel that launches spotlight-python.
 * Automatically ensures Spotlight window is always on top (make_above), intercepts demands-attention,
 * and provides native Wayland keystroke injection (Paste) via Clutter VirtualInputDevice over D-Bus.
 * 
 * Compatible with GNOME 45-50 & Wayland.
 */

import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import St from 'gi://St';
import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import Meta from 'gi://Meta';

const DBUS_NAME = 'org.gnome.Shell.Extensions.PulsarSpotlight';
const DBUS_PATH = '/org/gnome/Shell/Extensions/PulsarSpotlight';
const DBUS_NODE = Gio.DBusNodeInfo.new_for_xml(`
<node>
  <interface name="org.gnome.Shell.Extensions.PulsarSpotlight">
    <method name="Paste"/>
  </interface>
</node>
`);
const DBUS_INFO = DBUS_NODE.lookup_interface(DBUS_NAME);

const SpotlightLauncherButton = GObject.registerClass({
    GTypeName: 'PulsarosSpotlightLauncherButton'
}, class SpotlightLauncherButton extends PanelMenu.Button {
    _init(extensionPath) {
        super._init(0.5, "Spotlight Launcher", true);
        
        let gicon = null;
        try {
            if (extensionPath) {
                let iconFile = Gio.File.new_for_path(extensionPath + '/icons/spotlight-symbolic.svg');
                if (iconFile.query_exists(null)) {
                    gicon = new Gio.FileIcon({ file: iconFile });
                }
            }
        } catch (e) {}

        this.icon = new St.Icon({
            gicon: gicon,
            icon_name: gicon ? null : 'system-search-symbolic',
            style_class: 'system-status-icon'
        });
        
        this.add_child(this.icon);
    }
});

export default class PulsarosSpotlightLauncherExtension extends Extension {
    enable() {
        this._button = new SpotlightLauncherButton(this.path);
        this._clickedId = 0;
        this._touchId = 0;
        this._windowCreatedId = 0;
        this._demandsAttentionId = 0;
        this._markedUrgentId = 0;
        this._virtualKeyboard = null;
        this._dbusRegistrationId = 0;
        this._nameId = 0;

        // Initialize virtual keyboard device for injecting keystrokes into any Wayland window
        try {
            const seat = Clutter.get_default_backend().get_default_seat();
            if (seat) {
                this._virtualKeyboard = seat.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
            }
        } catch (e) {
            console.error('[SpotlightLauncher] Failed to create virtual keyboard device:', e);
        }

        // Register D-Bus object on session bus for programmatic auto-pasting
        try {
            this._dbusRegistrationId = Gio.DBus.session.register_object(
                DBUS_PATH,
                DBUS_INFO,
                (connection, sender, objectPath, interfaceName, methodName, parameters, invocation) => {
                    if (methodName === 'Paste') {
                        this.Paste();
                        invocation.return_value(null);
                    }
                },
                null,
                null
            );
            this._nameId = Gio.bus_own_name(
                Gio.BusType.SESSION,
                DBUS_NAME,
                Gio.BusNameOwnerFlags.NONE,
                null,
                null,
                null
            );
        } catch (e) {
            console.error('[SpotlightLauncher] Failed to register D-Bus interface:', e);
        }
        
        // Connect mouse button press event (left click)
        this._clickedId = this._button.connect('button-press-event', (actor, event) => {
            if (event.get_button() === 1) {
                this._launchSpotlight();
                return Clutter.EVENT_STOP;
            }
            return Clutter.EVENT_PROPAGATE;
        });
        
        // Connect touch events for touchscreen support
        this._touchId = this._button.connect('touch-event', (actor, event) => {
            if (event.type() === Clutter.EventType.TOUCH_END) {
                this._launchSpotlight();
                return Clutter.EVENT_STOP;
            }
            return Clutter.EVENT_PROPAGATE;
        });
        
        // Listen to window creation to automatically raise Spotlight above everything
        this._windowCreatedId = global.display.connect('window-created', (display, win) => {
            GLib.idle_add(GLib.PRIORITY_DEFAULT, () => {
                this._ensureSpotlightOnTop(win);
                return GLib.SOURCE_REMOVE;
            });
        });

        // Intercept demands-attention and marked-urgent to prevent "is ready" banner and focus immediately
        this._demandsAttentionId = global.display.connect('window-demands-attention', (display, win) => {
            if (this._isSpotlightWindow(win)) {
                this._ensureSpotlightOnTop(win);
            }
        });

        this._markedUrgentId = global.display.connect('window-marked-urgent', (display, win) => {
            if (this._isSpotlightWindow(win)) {
                this._ensureSpotlightOnTop(win);
            }
        });

        // Add the button to the right box of the panel, next to the system menu (index 0)
        Main.panel.addToStatusArea('pulsaros-spotlight-launcher', this._button, 0, 'right');
    }
    
    disable() {
        if (this._button) {
            if (this._clickedId) {
                this._button.disconnect(this._clickedId);
                this._clickedId = 0;
            }
            if (this._touchId) {
                this._button.disconnect(this._touchId);
                this._touchId = 0;
            }
            this._button.destroy();
            this._button = null;
        }

        if (this._windowCreatedId) {
            global.display.disconnect(this._windowCreatedId);
            this._windowCreatedId = 0;
        }
        if (this._demandsAttentionId) {
            global.display.disconnect(this._demandsAttentionId);
            this._demandsAttentionId = 0;
        }
        if (this._markedUrgentId) {
            global.display.disconnect(this._markedUrgentId);
            this._markedUrgentId = 0;
        }

        if (this._dbusRegistrationId) {
            Gio.DBus.session.unregister_object(this._dbusRegistrationId);
            this._dbusRegistrationId = 0;
        }
        if (this._nameId) {
            Gio.bus_unown_name(this._nameId);
            this._nameId = 0;
        }

        this._virtualKeyboard = null;
    }

    /**
     * D-Bus Method: Paste
     * Simulates native Wayland Ctrl+V keypress into active window using Mutter VirtualInputDevice.
     */
    Paste() {
        GLib.timeout_add(GLib.PRIORITY_DEFAULT, 80, () => {
            if (this._virtualKeyboard) {
                const time = global.get_current_time();
                this._virtualKeyboard.notify_keyval(time, Clutter.KEY_Control_L, Clutter.KeyState.PRESSED);
                this._virtualKeyboard.notify_keyval(time, Clutter.KEY_v, Clutter.KeyState.PRESSED);
                this._virtualKeyboard.notify_keyval(time, Clutter.KEY_v, Clutter.KeyState.RELEASED);
                this._virtualKeyboard.notify_keyval(time, Clutter.KEY_Control_L, Clutter.KeyState.RELEASED);
            }
            return GLib.SOURCE_REMOVE;
        });
    }

    _isSpotlightWindow(win) {
        if (!win || win.is_override_redirect()) return false;
        const appId = win.get_gtk_application_id ? win.get_gtk_application_id() : '';
        const title = win.get_title ? win.get_title() : '';
        const wmClass = win.get_wm_class ? win.get_wm_class() : '';

        return appId === 'com.inled.spotlight' || 
               appId === 'org.pulsaros.Spotlight' || 
               title === 'Spotlight' || 
               wmClass === 'pulsaros-spotlight' ||
               wmClass === 'spotlight-gtk';
    }

    _ensureSpotlightOnTop(win) {
        if (this._isSpotlightWindow(win)) {
            try {
                win.make_above();
                win.stick();
                Main.activateWindow(win);
                win.activate(global.get_current_time());
            } catch (e) {
                console.error('[SpotlightLauncher] Error raising spotlight window:', e);
            }
        }
    }

    _launchSpotlight() {
        // If window is already present, raise and focus it immediately
        try {
            const wins = global.display.get_tab_list(Meta.TabList.NORMAL, null);
            for (const win of wins) {
                if (this._isSpotlightWindow(win)) {
                    this._ensureSpotlightOnTop(win);
                    return;
                }
            }
        } catch (e) {}

        const cmd = GLib.find_program_in_path('pulsaros-spotlight') ||
                    GLib.find_program_in_path('spotlight-python') ||
                    GLib.find_program_in_path('spotlight-gtk');
        if (!cmd) {
            console.error('[SpotlightLauncher] No spotlight binary found');
            return;
        }
        try {
            GLib.spawn_command_line_async(GLib.shell_quote(cmd));
        } catch (e) {
            console.error(`[SpotlightLauncher] Failed to launch ${cmd}:`, e);
        }
    }
}
