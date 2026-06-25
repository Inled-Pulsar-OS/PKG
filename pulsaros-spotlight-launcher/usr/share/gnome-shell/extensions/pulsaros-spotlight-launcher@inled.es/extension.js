/**
 * Pulsar OS - Spotlight Launcher Extension
 * Adds a macOS-like Spotlight search icon to GNOME top panel that launches spotlight-python.
 * 
 * Extensión de Pulsar OS - Lanzador de Spotlight
 * Añade un icono de búsqueda al estilo macOS Spotlight en la barra superior de GNOME que ejecuta spotlight-python.
 * 
 * Compatible with GNOME 45-50 & Wayland.
 * Compatible con GNOME 45-50 y Wayland.
 */

import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import St from 'gi://St';
import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';

// Custom button class for our spotlight launcher
// Clase de botón personalizada para nuestro lanzador de Spotlight
const SpotlightLauncherButton = GObject.registerClass(
class SpotlightLauncherButton extends PanelMenu.Button {
    _init() {
        // Alignment 0.5, unique identifier, dontCreateMenu = true (do not open a dropdown submenu)
        // Alineación 0.5, identificador único, dontCreateMenu = true (no abre submenú desplegable al clicar)
        super._init(0.5, "Spotlight Launcher", true);
        
        // St.Icon widget to display the system-wide search symbolic icon
        // Widget St.Icon para mostrar el icono simbólico de búsqueda del sistema
        this.icon = new St.Icon({
            icon_name: 'edit-find-symbolic',
            style_class: 'system-status-icon'
        });
        
        this.add_child(this.icon);
    }
});

export default class PulsarosSpotlightLauncherExtension extends Extension {
    enable() {
        this._button = new SpotlightLauncherButton();
        this._clickedId = 0;
        this._touchId = 0;
        
        // Connect mouse button press event (left click)
        // Conectar el evento de pulsación de botón de ratón (clic izquierdo)
        this._clickedId = this._button.connect('button-press-event', (actor, event) => {
            if (event.get_button() === 1) { // 1 is left click
                this._launchSpotlight();
                return Clutter.EVENT_STOP;
            }
            return Clutter.EVENT_PROPAGATE;
        });
        
        // Connect touch events for touchscreen support
        // Conectar eventos táctiles para soporte con pantallas táctiles
        this._touchId = this._button.connect('touch-event', (actor, event) => {
            if (event.type() === Clutter.EventType.TOUCH_END) {
                this._launchSpotlight();
                return Clutter.EVENT_STOP;
            }
            return Clutter.EVENT_PROPAGATE;
        });
        
        // Add the button to the right box of the panel, next to the system menu (index 0)
        // Añadir el botón al lado derecho del panel, junto al menú del sistema (índice 0)
        Main.panel.addToStatusArea('pulsaros-spotlight-launcher', this._button, 0, 'right');
    }
    
    disable() {
        // Disconnect events
        // Desconectar eventos
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
    }
    
    // Spawns the command-line application asynchronously to prevent panel freezing
    // Ejecuta el comando de consola de forma asíncrona para evitar que el panel se congele
    _launchSpotlight() {
        try {
            GLib.spawn_command_line_async('spotlight-python');
        } catch (e) {
            console.error("[SpotlightLauncher] Failed to launch spotlight-python:", e);
        }
    }
}
