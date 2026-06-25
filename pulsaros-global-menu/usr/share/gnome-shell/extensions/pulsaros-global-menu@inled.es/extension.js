/**
 * Pulsar OS - Global Menu Extension
 * Extension to display macOS-like application menus in GNOME top bar.
 * 
 * Extensión de Pulsar OS - Menú Global
 * Extensión para mostrar menús de aplicación al estilo macOS en la barra superior de GNOME.
 * 
 * Compatible with GNOME 45-50 & Wayland.
 * Compatible con GNOME 45-50 y Wayland.
 */

import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import St from 'gi://St';
import Clutter from 'gi://Clutter';
import Shell from 'gi://Shell';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import Meta from 'gi://Meta';

// Custom GObject class to construct the menu buttons in the panel
// Clase GObject personalizada para construir los botones de menú en el panel
const GlobalMenuButton = GObject.registerClass({
    GTypeName: 'PulsarosGlobalMenuButton'
}, class GlobalMenuButton extends PanelMenu.Button {
    _init(title, isAppMenu = false) {
        super._init(0.0, title, false);
        
        // St.Label widget to show the menu text on the top bar
        // Widget St.Label para mostrar el texto del menú en la barra superior
        this.label = new St.Label({
            text: title,
            y_align: Clutter.ActorAlign.CENTER,
            style_class: isAppMenu ? 'global-menu-app-label' : 'global-menu-label'
        });
        
        this.add_child(this.label);
    }
    
    // Updates the label text dynamically (e.g., when active window changes)
    // Actualiza el texto de la etiqueta dinámicamente (p. ej., al cambiar la ventana activa)
    setText(text) {
        this.label.set_text(text);
    }
});

// --- Tahoe Theme Screen Locker ---
// --- Bloqueador de pantalla con tema Tahoe ---
const LockScreen = GObject.registerClass({
    GTypeName: 'PulsarosLockScreen'
}, class LockScreen extends St.BoxLayout {
    _init(extension) {
        super._init({
            name: 'pulsaros-lockscreen',
            style_class: 'pulsaros-lockscreen-root',
            visible: false,
            clip_to_allocation: true,
            reactive: true,
            orientation: Clutter.Orientation.VERTICAL,
            x_expand: true,
            y_expand: true
        });
        
        this._extension = extension;
        this._isLocked = false;
        this._hasGrab = false;
        this._authenticating = false;
        this._timerId = 0;
        this._sizeChangedId = 0;
        this._sizeChangedId2 = 0;
        
        // Dynamically reference extension path for background asset
        // Referenciar dinámicamente la ruta de la extensión para el fondo de pantalla
        this.style = `background-image: url("file://${this._extension.path}/background.webp"); background-size: cover;`;
        
        // Monitor screen size changes to remain fullscreen
        // Monitorear cambios de resolución de pantalla para mantenerse en pantalla completa
        this._sizeChangedId = global.stage.connect('notify::width', () => this._onSizeChanged());
        this._sizeChangedId2 = global.stage.connect('notify::height', () => this._onSizeChanged());
        this._onSizeChanged();
        
        this._buildUI();
    }
    
    _onSizeChanged() {
        this.set_position(0, 0);
        this.set_size(global.stage.width, global.stage.height);
    }
    
    _buildUI() {
        // 1. Top bar for power actions (Shutdown, Reboot)
        // 1. Barra superior para acciones de energía (Apagar, Reiniciar)
        let topBar = new St.BoxLayout({
            style_class: 'pulsaros-lockscreen-topbar',
            x_align: Clutter.ActorAlign.END,
            y_align: Clutter.ActorAlign.START
        });
        this.add_child(topBar);
        
        // Reboot Button
        // Botón de reiniciar
        this._rebootBtn = new St.Button({
            style_class: 'pulsaros-lockscreen-power-button',
            reactive: true,
            can_focus: true,
            child: new St.Icon({
                icon_name: 'system-restart-symbolic',
                icon_size: 20
            })
        });
        this._rebootBtn.connect('clicked', () => {
            GLib.spawn_command_line_async("systemctl reboot");
        });
        topBar.add_child(this._rebootBtn);
        
        // Shutdown Button
        // Botón de apagar
        this._shutdownBtn = new St.Button({
            style_class: 'pulsaros-lockscreen-power-button',
            reactive: true,
            can_focus: true,
            child: new St.Icon({
                icon_name: 'system-shutdown-symbolic',
                icon_size: 20
            })
        });
        this._shutdownBtn.connect('clicked', () => {
            GLib.spawn_command_line_async("systemctl poweroff");
        });
        topBar.add_child(this._shutdownBtn);
        
        // Fixed top spacer to push clock down slightly (like macOS)
        // Espaciador superior fijo para empujar el reloj un poco hacia abajo (como en macOS)
        let spacer = new St.Widget({
            style_class: 'pulsaros-lockscreen-spacer',
            height: 60
        });
        this.add_child(spacer);
        
        // 2. Central Clock displays
        // 2. Reloj central en pantalla
        this._clockBox = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.START,
            style_class: 'pulsaros-lockscreen-clock-box'
        });
        this.add_child(this._clockBox);
        
        this._timeLabel = new St.Label({
            style_class: 'pulsaros-lockscreen-time-label',
            text: '00:00'
        });
        this._clockBox.add_child(this._timeLabel);
        
        this._dateLabel = new St.Label({
            style_class: 'pulsaros-lockscreen-date-label',
            text: ''
        });
        this._clockBox.add_child(this._dateLabel);
        
        // Middle spacer
        // Espaciador medio
        let middleSpacer = new St.Widget({
            y_expand: true,
            style_class: 'pulsaros-lockscreen-middle-spacer'
        });
        this.add_child(middleSpacer);
        
        // 3. User Credentials login card
        // 3. Tarjeta de inicio de sesión de usuario
        this._userCard = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.END,
            style_class: 'pulsaros-lockscreen-user-card'
        });
        this.add_child(this._userCard);
        
        // Circular Avatar
        // Avatar circular
        let username = GLib.get_user_name();
        
        this._avatarWidget = new St.Widget({
            style_class: 'pulsaros-lockscreen-avatar',
            x_align: Clutter.ActorAlign.CENTER
        });
        
        try {
            let avatarPath = `/var/lib/AccountsService/icons/${username}`;
            let avatarFile = Gio.File.new_for_path(avatarPath);
            
            if (avatarFile.query_exists(null)) {
                this._avatarWidget.style = `background-image: url("file://${avatarPath}"); background-size: cover; border-radius: 55px; width: 110px; height: 110px; border: 2px solid rgba(255, 255, 255, 0.9);`;
            } else {
                let faceFile = Gio.File.new_for_path(GLib.get_home_dir() + '/.face');
                if (faceFile.query_exists(null)) {
                    this._avatarWidget.style = `background-image: url("file://${GLib.get_home_dir()}/.face"); background-size: cover; border-radius: 55px; width: 110px; height: 110px; border: 2px solid rgba(255, 255, 255, 0.9);`;
                } else {
                    this._avatarWidget.style = `border-radius: 55px; width: 110px; height: 110px; border: 2px solid rgba(255, 255, 255, 0.9); background-color: rgba(255, 255, 255, 0.15);`;
                    let defaultIcon = new St.Icon({
                        icon_name: 'avatar-default-symbolic',
                        icon_size: 64,
                        style_class: 'pulsaros-lockscreen-avatar-default',
                        x_align: Clutter.ActorAlign.CENTER,
                        y_align: Clutter.ActorAlign.CENTER
                    });
                    this._avatarWidget.add_child(defaultIcon);
                }
            }
        } catch (e) {
            console.error("[LockScreen] Failed to load avatar:", e);
            // Default styling fallback / Alternativa de estilo por defecto
            this._avatarWidget.style = `border-radius: 55px; width: 110px; height: 110px; border: 2px solid rgba(255, 255, 255, 0.9); background-color: rgba(255, 255, 255, 0.15);`;
            let defaultIcon = new St.Icon({
                icon_name: 'avatar-default-symbolic',
                icon_size: 64,
                style_class: 'pulsaros-lockscreen-avatar-default',
                x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.CENTER
            });
            this._avatarWidget.add_child(defaultIcon);
        }
        this._userCard.add_child(this._avatarWidget);
        
        // Display Name (Real Name or Username fallback)
        // Nombre para mostrar (Nombre real o usuario como alternativa)
        let realName = username;
        try {
            let gn = GLib.get_real_name();
            if (gn && gn !== 'Unknown' && gn.trim() !== '') {
                realName = gn;
            }
        } catch (e) {
            // ignore
        }
        realName = realName.charAt(0).toUpperCase() + realName.slice(1);
        
        this._nameLabel = new St.Label({
            style_class: 'pulsaros-lockscreen-name-label',
            x_align: Clutter.ActorAlign.CENTER,
            text: realName
        });
        this._userCard.add_child(this._nameLabel);
        
        // Password Input Entry
        // Entrada de contraseña
        this._passwordEntry = new St.Entry({
            style_class: 'pulsaros-lockscreen-entry',
            x_align: Clutter.ActorAlign.CENTER,
            hint_text: 'Enter Password',
            can_focus: true,
            reactive: true
        });
        
        // Safe check and resolution for Clutter.Text inside St.Entry across GNOME versions
        // Comprobación segura y resolución para Clutter.Text dentro de St.Entry en distintas versiones de GNOME
        let clutterText = this._passwordEntry.clutter_text || this._passwordEntry.clutterText;
        if (!clutterText && typeof this._passwordEntry.get_clutter_text === 'function') {
            clutterText = this._passwordEntry.get_clutter_text();
        }
        
        if (clutterText) {
            clutterText.set_password_char('●');
            
            // Trigger verification when pressing Enter
            // Lanzar verificación al pulsar Enter
            clutterText.connect('activate', () => {
                let password = this._passwordEntry.get_text();
                if (password && password.length > 0) {
                    this._authenticate(password);
                }
            });
            
            // Reset text style upon typing
            // Restablecer el estilo al escribir
            clutterText.connect('text-changed', () => {
                this._passwordEntry.style_class = 'pulsaros-lockscreen-entry';
                this._passwordEntry.set_hint_text('Enter Password');
            });
            
            // Clear text on Escape key
            // Limpiar el texto al pulsar Escape
            clutterText.connect('key-press-event', (actor, event) => {
                let symbol = event.get_key_symbol();
                if (symbol === Clutter.KEY_Escape) {
                    this._passwordEntry.set_text('');
                    return Clutter.EVENT_STOP;
                }
                return Clutter.EVENT_PROPAGATE;
            });
        }
        
        this._userCard.add_child(this._passwordEntry);
        
        // Grab keyboard focus on any background click (safely targeting actual Clutter.Text)
        // Forzar foco en la entrada de contraseña al hacer clic en el fondo (apuntando con seguridad a Clutter.Text)
        this.connect('button-press-event', () => {
            let activeText = this._passwordEntry.clutter_text || this._passwordEntry.clutterText || this._passwordEntry;
            activeText.grab_key_focus();
            return Clutter.EVENT_PROPAGATE;
        });
        
        // Bottom spacer to offset card
        // Espaciador inferior para compensar la tarjeta
        let bottomSpacer = new St.Widget({
            style_class: 'pulsaros-lockscreen-bottom-spacer',
            height: 40
        });
        this.add_child(bottomSpacer);
    }
    
    _updateClock() {
        let now = GLib.DateTime.new_now_local();
        this._timeLabel.set_text(now.format('%H:%M'));
        this._dateLabel.set_text(now.format('%A, %B %d'));
    }
    
    lock() {
        if (this._isLocked) return;
        this._isLocked = true;
        this.visible = true;
        this._passwordEntry.set_text('');
        this._passwordEntry.set_reactive(true);
        this._passwordEntry.style_class = 'pulsaros-lockscreen-entry';
        this._passwordEntry.set_hint_text('Enter Password');
        
        // Put lockscreen overlay on the absolute top of the uiGroup stack
        // Poner la pantalla de bloqueo en el tope absoluto del uiGroup
        try {
            let parent = this.get_parent();
            if (parent) {
                parent.set_child_at_index(this, -1);
            }
        } catch (e) {
            console.error("[LockScreen] Failed to raise lockscreen overlay via set_child_at_index:", e);
            try {
                Main.uiGroup.set_child_above_sibling(this, null);
            } catch (e2) {
                console.error("[LockScreen] Fallback set_child_above_sibling failed too:", e2);
            }
        }
        
        // Defer input grab and key focus to the next main loop cycle to guarantee the actor is mapped
        // Diferir la captura de entrada y el foco de teclado al siguiente ciclo del bucle principal para garantizar que el actor esté mapeado
        GLib.idle_add(GLib.PRIORITY_DEFAULT, () => {
            if (!this._isLocked) return GLib.SOURCE_REMOVE;
            
            if (Main.pushModal(this)) {
                this._hasGrab = true;
            } else {
                console.error("[LockScreen] Failed to acquire input grab");
                this._hasGrab = false;
            }
            
            let activeText = this._passwordEntry.clutter_text || this._passwordEntry.clutterText || this._passwordEntry;
            activeText.grab_key_focus();
            return GLib.SOURCE_REMOVE;
        });
        
        // Start live clock updates
        // Iniciar actualizaciones en vivo del reloj
        this._updateClock();
        this._timerId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 1, () => {
            this._updateClock();
            return GLib.SOURCE_CONTINUE;
        });
        
        this.opacity = 255;
    }
    
    unlock() {
        if (!this._isLocked) return;
        this._isLocked = false;
        this.visible = false;
        
        // Release modal input grab
        // Liberar la captura modal de entrada
        if (this._hasGrab) {
            Main.popModal(this);
            this._hasGrab = false;
        }
        
        // Clean clock timer
        // Detener el temporizador del reloj
        if (this._timerId) {
            GLib.source_remove(this._timerId);
            this._timerId = 0;
        }
    }
    
    _authenticate(password) {
        if (this._authenticating) return;
        this._authenticating = true;
        
        this._passwordEntry.set_reactive(false);
        this._passwordEntry.style_class = 'pulsaros-lockscreen-entry-authenticating';
        
        let username = GLib.get_user_name();
        
        // Check if the PAM service file is present. If not, fallback to developer passwords for local testing on host
        // Comprobar si el archivo de servicio PAM existe. Si no, usar contraseñas de desarrollo para pruebas locales en el host
        let pamFile = Gio.File.new_for_path('/etc/pam.d/pulsaros-lock');
        if (!pamFile.query_exists(null)) {
            console.warn("[LockScreen] PAM service '/etc/pam.d/pulsaros-lock' is missing. Falling back to developer passwords.");
            if (password === 'pulsar' || password === 'live' || password === 'jaime') {
                this._onAuthSuccess();
            } else {
                this._onAuthFailure();
            }
            return;
        }
        
        try {
            // Run pamtester asynchronously using stdin piping
            // Ejecutar pamtester asíncronamente enviando la contraseña por stdin
            let proc = new Gio.Subprocess({
                argv: ['/usr/bin/pamtester', 'pulsaros-lock', username, 'authenticate'],
                flags: Gio.SubprocessFlags.STDIN_PIPE | Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_PIPE
            });
            proc.init(null);
            
            let stdinStream = proc.get_stdin_pipe();
            if (stdinStream) {
                let bytes = GLib.Bytes.new(password + '\n');
                stdinStream.write_bytes(bytes, null);
                stdinStream.close(null);
            }
            
            proc.wait_async(null, (obj, res) => {
                try {
                    proc.wait_finish(res);
                    let success = proc.get_successful();
                    if (success) {
                        this._onAuthSuccess();
                    } else {
                        this._onAuthFailure();
                    }
                } catch (e) {
                    console.error("[LockScreen] pamtester wait error:", e);
                    this._onAuthFailure();
                }
            });
        } catch (e) {
            console.error("[LockScreen] pamtester launch failed:", e);
            // Local fallback for testing under host (without pamtester)
            // Alternativa local para pruebas en el host (sin pamtester)
            if (password === 'pulsar' || password === 'live' || password === 'jaime') {
                this._onAuthSuccess();
            } else {
                this._onAuthFailure();
            }
        }
    }
    
    _onAuthSuccess() {
        this._authenticating = false;
        this.unlock();
    }
    
    _onAuthFailure() {
        this._authenticating = false;
        this._passwordEntry.set_reactive(true);
        this._passwordEntry.set_text('');
        this._passwordEntry.set_hint_text('Incorrect Password');
        this._passwordEntry.style_class = 'pulsaros-lockscreen-entry-failed';
        this._passwordEntry.grab_key_focus();
        
        // Shake animation
        // Animación de agitación de contraseña incorrecta
        let originalX = this._passwordEntry.translation_x;
        let shakeOffset = 10;
        let step = 0;
        let shakeInterval = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 50, () => {
            if (step >= 6) {
                this._passwordEntry.translation_x = originalX;
                return GLib.SOURCE_REMOVE;
            }
            this._passwordEntry.translation_x = originalX + (step % 2 === 0 ? shakeOffset : -shakeOffset);
            step++;
            return GLib.SOURCE_CONTINUE;
        });
    }
    
    destroy() {
        if (this._sizeChangedId) {
            global.stage.disconnect(this._sizeChangedId);
            this._sizeChangedId = 0;
        }
        if (this._sizeChangedId2) {
            global.stage.disconnect(this._sizeChangedId2);
            this._sizeChangedId2 = 0;
        }
        if (this._timerId) {
            GLib.source_remove(this._timerId);
            this._timerId = 0;
        }
        if (this._isLocked && this._hasGrab) {
            Main.popModal(this);
        }
        super.destroy();
    }
});

export default class PulsarosGlobalMenuExtension extends Extension {
    enable() {
        this._menuButtons = [];
        this._focusNotifyId = 0;
        this._virtualKeyboard = null;
        this._origCanLock = null;
        this._origLock = null;
        this._origUnlock = null;
        
        // Initialize the virtual keyboard device for injecting keystrokes (Wayland native)
        // Inicializar el dispositivo de teclado virtual para inyectar pulsaciones de tecla (nativo en Wayland)
        try {
            let seat = Clutter.get_default_backend().get_default_seat();
            if (seat) {
                this._virtualKeyboard = seat.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
            }
        } catch (e) {
            console.error("[GlobalMenu] Failed to create virtual keyboard device:", e);
        }
        
        // Create lockscreen overlay instance and add it to top of uiGroup
        // Crear la instancia de pantalla de bloqueo y añadirla a uiGroup
        this._lockScreenOverlay = new LockScreen(this);
        Main.uiGroup.add_child(this._lockScreenOverlay);
        
        // Bind the native lock-screen media key shortcut (Super+L) to our custom LockScreen overlay
        // Vincular el atajo de teclado nativo de bloqueo (Super+L) a nuestra pantalla de bloqueo
        try {
            Main.wm.addKeybinding(
                'screensaver',
                new Gio.Settings({ schema_id: 'org.gnome.settings-daemon.plugins.media-keys' }),
                Meta.KeyBindingFlags.NONE,
                Shell.ActionMode.ALL,
                () => {
                    this._lockScreenOverlay.lock();
                }
            );
        } catch (e) {
            console.error("[GlobalMenu] Failed to bind screensaver keybinding:", e);
        }
        
        // Create each of the menu buttons
        // Crear cada uno de los botones de menú
        this._createLogoMenu();
        this._createAppMenu();
        this._createFileMenu();
        this._createEditMenu();
        this._createGoMenu();
        this._createWindowMenu();
        this._createHelpMenu();
        
        // Add menu buttons to GNOME top panel (Left Box)
        // Añadir botones de menú al panel superior de GNOME (caja izquierda)
        let pos = 1; 
        for (let button of this._menuButtons) {
            Main.panel.addToStatusArea(`global-menu-${button.uuid_suffix}`, button, pos++, 'left');
        }
        
        // Listen to window focus change notifications from display
        // Escuchar notificaciones de cambio de foco de ventana de la pantalla
        this._focusNotifyId = global.display.connect('notify::focus-window', () => {
            this._onFocusWindowChanged();
        });
        
        // Trigger initial focus state setup
        // Lanzar configuración inicial del estado de foco
        this._onFocusWindowChanged();
        
        // Force the native GNOME Shell lock button in Quick Settings to be always visible
        // Forzar a que el botón de bloqueo nativo en los Quick Settings de GNOME Shell sea siempre visible
        try {
            this._origCanLock = Object.getOwnPropertyDescriptor(Shell.SystemActions.prototype, 'can_lock') || 
                                Object.getOwnPropertyDescriptor(Shell.SystemActions.get_default(), 'can_lock');
            Object.defineProperty(Shell.SystemActions.get_default(), 'can_lock', {
                get: () => true,
                configurable: true
            });
            Shell.SystemActions.get_default().notify('can-lock');
        } catch (e) {
            console.error("[GlobalMenu] Failed to override can_lock:", e);
        }

        // Intercept native screenShield lock and unlock events to use our custom overlay
        // Interceptar eventos de bloqueo y desbloqueo nativos del screenshield para usar nuestra pantalla
        try {
            if (Main.screenShield) {
                this._origLock = Main.screenShield.lock;
                let self = this;
                Main.screenShield.lock = function(animate) {
                    try {
                        self._lockScreenOverlay.lock();
                    } catch (e) {
                        console.error("[GlobalMenu] Failed to trigger custom lock:", e);
                    }
                    return true;
                };
                
                this._origUnlock = Main.screenShield.unlock;
                Main.screenShield.unlock = function(animate) {
                    try {
                        self._lockScreenOverlay.unlock();
                    } catch (e) {
                        console.error("[GlobalMenu] Failed to trigger custom unlock:", e);
                    }
                    if (self._origUnlock) {
                        self._origUnlock.call(Main.screenShield, animate);
                    }
                };
            }
        } catch (e) {
            console.error("[GlobalMenu] Failed to override Main.screenShield:", e);
        }
    }
    
    disable() {
        // Disconnect display focus notification signal
        // Desconectar la señal de notificación de foco de la pantalla
        if (this._focusNotifyId) {
            global.display.disconnect(this._focusNotifyId);
            this._focusNotifyId = 0;
        }
        
        // Unbind the screensaver keybinding
        // Desvincular el atajo de teclado de screensaver
        try {
            Main.wm.removeKeybinding('screensaver');
        } catch (e) {
            // ignore
        }
        
        // Destroy custom lockscreen overlay
        // Destruir la pantalla de bloqueo personalizada
        if (this._lockScreenOverlay) {
            Main.uiGroup.remove_child(this._lockScreenOverlay);
            this._lockScreenOverlay.destroy();
            this._lockScreenOverlay = null;
        }
        
        // Safely destroy and remove all panel buttons
        // Destruir y quitar todos los botones del panel de forma segura
        for (let button of this._menuButtons) {
            button.destroy();
        }
        this._menuButtons = [];
        
        // Restore the original can_lock descriptor
        // Restaurar el descriptor de can_lock original
        try {
            if (this._origCanLock) {
                Object.defineProperty(Shell.SystemActions.get_default(), 'can_lock', this._origCanLock);
                Shell.SystemActions.get_default().notify('can-lock');
            }
        } catch (e) {
            // ignore
        }

        // Restore original lock and unlock functions
        // Restaurar las funciones de bloqueo y desbloqueo originales
        try {
            if (Main.screenShield) {
                if (this._origLock) {
                    Main.screenShield.lock = this._origLock;
                }
                if (this._origUnlock) {
                    Main.screenShield.unlock = this._origUnlock;
                }
            }
        } catch (e) {
            // ignore
        }
        
        // Nullify the virtual keyboard device reference
        // Anular la referencia del dispositivo de teclado virtual
        this._virtualKeyboard = null;
    }
    
    // --- Logo Menu Button (Pulsar OS logo) ---
    // --- Botón de menú con Logo (logo de Pulsar OS) ---
    _createLogoMenu() {
        this.logoMenuButton = new GlobalMenuButton("");
        this.logoMenuButton.uuid_suffix = "logo";
        
        this.logoMenuButton.label.destroy();
        
        // Load custom Pulsar OS PNG logo icon from the extension directory path
        // Cargar el icono del logo PNG personalizado de Pulsar OS desde la ruta del directorio de la extensión
        let iconFile = Gio.File.new_for_path(this.path + '/pulsar-white-sf.png');
        let fileIcon = new Gio.FileIcon({ file: iconFile });
        this.logoMenuButton.icon = new St.Icon({
            gicon: fileIcon,
            style_class: 'global-menu-logo-icon'
        });
        this.logoMenuButton.add_child(this.logoMenuButton.icon);
        
        // About Pulsar OS
        let aboutItem = new PopupMenu.PopupMenuItem("About Pulsar OS");
        aboutItem.connect('activate', () => {
            Main.notify("Pulsar OS", "Pulsar OS Tahoe Edition\nBasing on Debian GNU/Linux");
        });
        this.logoMenuButton.menu.addMenuItem(aboutItem);
        
        // System Settings
        let settingsItem = new PopupMenu.PopupMenuItem("System Settings...");
        settingsItem.connect('activate', () => {
            this._openUri("settings://");
        });
        this.logoMenuButton.menu.addMenuItem(settingsItem);
        
        // App Store
        let appStoreItem = new PopupMenu.PopupMenuItem("App Store...");
        appStoreItem.connect('activate', () => {
            this._openUri("appstream://");
        });
        this.logoMenuButton.menu.addMenuItem(appStoreItem);
        
        this.logoMenuButton.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        
        // Lock Screen (Solves missing lock option in system menu when not using GDM)
        // Bloquear Pantalla (Soluciona la falta de opción de bloqueo en el menú del sistema al no usar GDM)
        let lockItem = new PopupMenu.PopupMenuItem("Lock Screen");
        lockItem.connect('activate', () => {
            this._lockScreen();
        });
        this.logoMenuButton.menu.addMenuItem(lockItem);
        
        // Log Out
        let logoutItem = new PopupMenu.PopupMenuItem("Log Out...");
        logoutItem.connect('activate', () => {
            this._runCommand("gnome-session-quit --logout");
        });
        this.logoMenuButton.menu.addMenuItem(logoutItem);
        
        this.logoMenuButton.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        
        // Restart
        let restartItem = new PopupMenu.PopupMenuItem("Restart...");
        restartItem.connect('activate', () => {
            this._runCommand("gnome-session-quit --reboot");
        });
        this.logoMenuButton.menu.addMenuItem(restartItem);
        
        // Shut Down
        let shutdownItem = new PopupMenu.PopupMenuItem("Shut Down...");
        shutdownItem.connect('activate', () => {
            this._runCommand("gnome-session-quit --power-off");
        });
        this.logoMenuButton.menu.addMenuItem(shutdownItem);
        
        this._menuButtons.push(this.logoMenuButton);
    }
    
    // --- App Menu Button (Finder / Active App name) ---
    // --- Botón de menú de App (Finder / Nombre de App activa) ---
    _createAppMenu() {
        this.appMenuButton = new GlobalMenuButton("Finder", true);
        this.appMenuButton.uuid_suffix = "app";
        
        // Dynamic items that will adapt their labels to the active window title
        // Elementos dinámicos que adaptarán sus etiquetas al título de la ventana activa
        this.aboutItem = new PopupMenu.PopupMenuItem("About Finder");
        this.aboutItem.connect('activate', () => {
            let activeApp = this._getAppName(global.display.focus_window);
            Main.notify("Pulsar OS", `Active App: ${activeApp}`);
        });
        this.appMenuButton.menu.addMenuItem(this.aboutItem);
        
        this.hideItem = new PopupMenu.PopupMenuItem("Hide Finder");
        this.hideItem.connect('activate', () => {
            let window = global.display.focus_window;
            if (window) {
                window.minimize();
            }
        });
        this.appMenuButton.menu.addMenuItem(this.hideItem);
        
        this.quitItem = new PopupMenu.PopupMenuItem("Quit Finder");
        this.quitItem.connect('activate', () => {
            let window = global.display.focus_window;
            if (window) {
                window.delete(global.get_current_time());
            }
        });
        this.appMenuButton.menu.addMenuItem(this.quitItem);
        
        this._menuButtons.push(this.appMenuButton);
    }
    
    // --- File Menu Button ---
    // --- Botón de menú de Archivo ---
    _createFileMenu() {
        let fileBtn = new GlobalMenuButton("File");
        fileBtn.uuid_suffix = "file";
        
        let newWindowItem = new PopupMenu.PopupMenuItem("New Window");
        newWindowItem.connect('activate', () => {
            let window = global.display.focus_window;
            if (window) {
                let tracker = Shell.WindowTracker.get_default();
                let app = tracker.get_window_app(window);
                if (app) {
                    app.open_new_window(-1);
                    return;
                }
            }
            // If no application has focus (Finder state), launch the default file manager (Home)
            // Si ninguna aplicación tiene el foco (estado Finder), abrir gestor de archivos predeterminado (Home)
            this._openUri("file://" + GLib.get_home_dir());
        });
        fileBtn.menu.addMenuItem(newWindowItem);
        
        let closeItem = new PopupMenu.PopupMenuItem("Close Window");
        closeItem.connect('activate', () => {
            let window = global.display.focus_window;
            if (window) {
                window.delete(global.get_current_time());
            }
        });
        fileBtn.menu.addMenuItem(closeItem);
        
        this._menuButtons.push(fileBtn);
    }
    
    // --- Edit Menu Button ---
    // --- Botón de menú de Edición ---
    _createEditMenu() {
        let editBtn = new GlobalMenuButton("Edit");
        editBtn.uuid_suffix = "edit";
        
        let undoItem = new PopupMenu.PopupMenuItem("Undo");
        undoItem.connect('activate', () => {
            this._sendKeyStroke([Clutter.KEY_Control_L], Clutter.KEY_z);
        });
        editBtn.menu.addMenuItem(undoItem);
        
        let redoItem = new PopupMenu.PopupMenuItem("Redo");
        redoItem.connect('activate', () => {
            // CTRL + Y is widely supported on Debian Gtk applications for Redo
            // CTRL + Y tiene soporte generalizado en aplicaciones Gtk de Debian para Rehacer
            this._sendKeyStroke([Clutter.KEY_Control_L], Clutter.KEY_y);
        });
        editBtn.menu.addMenuItem(redoItem);
        
        editBtn.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        
        let cutItem = new PopupMenu.PopupMenuItem("Cut");
        cutItem.connect('activate', () => {
            this._sendKeyStroke([Clutter.KEY_Control_L], Clutter.KEY_x);
        });
        editBtn.menu.addMenuItem(cutItem);
        
        let copyItem = new PopupMenu.PopupMenuItem("Copy");
        copyItem.connect('activate', () => {
            this._sendKeyStroke([Clutter.KEY_Control_L], Clutter.KEY_c);
        });
        editBtn.menu.addMenuItem(copyItem);
        
        let pasteItem = new PopupMenu.PopupMenuItem("Paste");
        pasteItem.connect('activate', () => {
            this._sendKeyStroke([Clutter.KEY_Control_L], Clutter.KEY_v);
        });
        editBtn.menu.addMenuItem(pasteItem);
        
        this._menuButtons.push(editBtn);
    }
    
    // --- Go Menu Button ---
    // --- Botón de menú de Ir ---
    _createGoMenu() {
        let goBtn = new GlobalMenuButton("Go");
        goBtn.uuid_suffix = "go";
        
        let backItem = new PopupMenu.PopupMenuItem("Back");
        backItem.connect('activate', () => {
            this._sendKeyStroke([Clutter.KEY_Alt_L], Clutter.KEY_Left);
        });
        goBtn.menu.addMenuItem(backItem);
        
        goBtn.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        
        let homeItem = new PopupMenu.PopupMenuItem("Home");
        homeItem.connect('activate', () => {
            this._openUri("file://" + GLib.get_home_dir());
        });
        goBtn.menu.addMenuItem(homeItem);
        
        let docsItem = new PopupMenu.PopupMenuItem("Documents");
        docsItem.connect('activate', () => {
            let docsPath = GLib.get_user_special_dir(GLib.UserDirectory.DIRECTORY_DOCUMENTS) || (GLib.get_home_dir() + "/Documents");
            this._openUri("file://" + docsPath);
        });
        goBtn.menu.addMenuItem(docsItem);
        
        let downloadsItem = new PopupMenu.PopupMenuItem("Downloads");
        downloadsItem.connect('activate', () => {
            let dlPath = GLib.get_user_special_dir(GLib.UserDirectory.DIRECTORY_DOWNLOAD) || (GLib.get_home_dir() + "/Downloads");
            this._openUri("file://" + dlPath);
        });
        goBtn.menu.addMenuItem(downloadsItem);
        
        let picsItem = new PopupMenu.PopupMenuItem("Pictures");
        picsItem.connect('activate', () => {
            let picsPath = GLib.get_user_special_dir(GLib.UserDirectory.DIRECTORY_PICTURES) || (GLib.get_home_dir() + "/Pictures");
            this._openUri("file://" + picsPath);
        });
        goBtn.menu.addMenuItem(picsItem);
        
        this._menuButtons.push(goBtn);
    }
    
    // --- Window Menu Button ---
    // --- Botón de menú de Ventana ---
    _createWindowMenu() {
        let winBtn = new GlobalMenuButton("Window");
        winBtn.uuid_suffix = "window";
        
        let minimizeItem = new PopupMenu.PopupMenuItem("Minimize");
        minimizeItem.connect('activate', () => {
            let window = global.display.focus_window;
            if (window) {
                window.minimize();
            }
        });
        winBtn.menu.addMenuItem(minimizeItem);
        
        let maximizeItem = new PopupMenu.PopupMenuItem("Maximize");
        maximizeItem.connect('activate', () => {
            let window = global.display.focus_window;
            if (window) {
                if (window.get_maximized()) {
                    window.unmaximize(Meta.MaximizeFlags.BOTH);
                } else {
                    window.maximize(Meta.MaximizeFlags.BOTH);
                }
            }
        });
        winBtn.menu.addMenuItem(maximizeItem);
        
        let closeItem = new PopupMenu.PopupMenuItem("Close");
        closeItem.connect('activate', () => {
            let window = global.display.focus_window;
            if (window) {
                window.delete(global.get_current_time());
            }
        });
        winBtn.menu.addMenuItem(closeItem);
        
        this._menuButtons.push(winBtn);
    }
    
    // --- Help Menu Button ---
    // --- Botón de menú de Ayuda ---
    _createHelpMenu() {
        let helpBtn = new GlobalMenuButton("Help");
        helpBtn.uuid_suffix = "help";
        
        let inledItem = new PopupMenu.PopupMenuItem("Inled Website");
        inledItem.connect('activate', () => {
            this._openUri("https://inled.es");
        });
        helpBtn.menu.addMenuItem(inledItem);
        
        let debianHelpItem = new PopupMenu.PopupMenuItem("Debian Help");
        debianHelpItem.connect('activate', () => {
            this._openUri("https://www.debian.org/support");
        });
        helpBtn.menu.addMenuItem(debianHelpItem);
        
        this._menuButtons.push(helpBtn);
    }
    
    // --- Focus Event Handler ---
    // --- Manejador del Evento de Foco ---
    _onFocusWindowChanged() {
        let window = global.display.focus_window;
        let appName = this._getAppName(window);
        
        // Update panel text representation for the focused application
        // Actualizar la representación de texto en el panel para la aplicación enfocada
        this.appMenuButton.setText(appName);
        
        // Dynamically update the app menu items
        // Actualizar dinámicamente los elementos del menú de la aplicación
        this.aboutItem.label.set_text(`About ${appName}`);
        this.hideItem.label.set_text(`Hide ${appName}`);
        this.quitItem.label.set_text(`Quit ${appName}`);
        
        // Deactivate Hide/Quit if no app is open (Finder mode)
        // Desactivar Ocultar/Salir si no hay app abierta (modo Finder)
        if (appName === "Finder") {
            this.hideItem.setSensitive(false);
            this.quitItem.setSensitive(false);
        } else {
            this.hideItem.setSensitive(true);
            this.quitItem.setSensitive(true);
        }
    }
    
    // --- Helper to open URI links ---
    // --- Utilidad para abrir enlaces URI ---
    _openUri(uri) {
        try {
            Gio.AppInfo.launch_default_for_uri(uri, null);
        } catch (e) {
            console.error(`[GlobalMenu] Failed to open URI: ${uri}`, e);
        }
    }
    
    // --- Helper to lock screen ---
    // --- Utilidad para bloquear la pantalla ---
    _lockScreen() {
        try {
            if (this._lockScreenOverlay) {
                this._lockScreenOverlay.lock();
            } else if (Main.screenShield) {
                Main.screenShield.lock(true);
            } else {
                GLib.spawn_command_line_async("loginctl lock-session");
            }
        } catch (e) {
            console.error("[GlobalMenu] Failed to lock screen:", e);
            GLib.spawn_command_line_async("loginctl lock-session");
        }
    }
    
    // --- Helper to run command ---
    // --- Utilidad para ejecutar comandos ---
    _runCommand(cmd) {
        try {
            GLib.spawn_command_line_async(cmd);
        } catch (e) {
            console.error(`[GlobalMenu] Failed to run command: ${cmd}`, e);
        }
    }
    
    // --- Helper to get active application name ---
    // --- Utilidad para obtener el nombre de la app activa ---
    _getAppName(window) {
        if (!window) {
            return "Finder";
        }
        
        // Attempt to trace through window tracker app registry
        // Intentar rastrear mediante el registro de apps del WindowTracker
        let tracker = Shell.WindowTracker.get_default();
        let app = tracker.get_window_app(window);
        if (app) {
            let name = app.get_name();
            if (name) {
                return name;
            }
        }
        
        // Fallback to window WM_CLASS name (typically app-id/process type)
        // Alternativa al nombre WM_CLASS de la ventana
        let wmClass = window.get_wm_class();
        if (wmClass) {
            return wmClass.charAt(0).toUpperCase() + wmClass.slice(1);
        }
        
        // Ultimate fallback to window title
        // Última alternativa al título de la ventana
        let title = window.get_title();
        if (title) {
            return title;
        }
        
        return "Finder";
    }
    
    // --- Helper to simulate virtual keypress events ---
    // --- Utilidad para simular eventos de pulsación de teclas virtuales ---
    _sendKeyStroke(modifiers, targetKey) {
        if (!this._virtualKeyboard) {
            console.warn("[GlobalMenu] Virtual keyboard device not available");
            return;
        }
        
        let time = global.get_current_time();
        
        // 1. Press all modifiers (e.g. CTRL, ALT, SHIFT)
        // 1. Presionar todos los modificadores (p. ej., CTRL, ALT, SHIFT)
        for (let key of modifiers) {
            this._virtualKeyboard.notify_keyval(time, key, Clutter.KeyState.PRESSED);
        }
        
        // 2. Press target key
        // 2. Presionar tecla objetivo
        this._virtualKeyboard.notify_keyval(time, targetKey, Clutter.KeyState.PRESSED);
        
        // 3. Release target key
        // 3. Soltar tecla objetivo
        this._virtualKeyboard.notify_keyval(time, targetKey, Clutter.KeyState.RELEASED);
        
        // 4. Release modifiers in reverse order
        // 4. Soltar modificadores en orden inverso
        for (let i = modifiers.length - 1; i >= 0; i--) {
            this._virtualKeyboard.notify_keyval(time, modifiers[i], Clutter.KeyState.RELEASED);
        }
    }
}
