import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import Meta from 'gi://Meta';
import St from 'gi://St';
import Shell from 'gi://Shell';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as WindowMenu from 'resource:///org/gnome/shell/ui/windowMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import { Extension, gettext as _ } from 'resource:///org/gnome/shell/extensions/extension.js';

const AppSystem = Shell.AppSystem.get_default();
const WinTracker = Shell.WindowTracker.get_default();

function log(msg) {
    console.log('[FILDEM_MENU] ' + msg);
}

const WindowActions = class WindowActions {
    constructor() {
        this._win = global.display.get_focus_window();
        this.actions = [];
    }

    getActions() {
        if (!this._win) return [];
        let type = this._win.get_window_type();
        let win = this._win;

        if (win.can_minimize())
            this.actions.push('Minimize');

        if (win.can_maximize()) {
            const isMaximized = win.is_maximized ? win.is_maximized() : win.get_maximized();
            this.actions.push(isMaximized ? 'Unmaximize' : 'Maximize');
        }

        if (win.allows_move())
            this.actions.push('Move');

        if (win.allows_resize())
            this.actions.push('Resize');

        if (win.titlebar_is_onscreen() && type != Meta.WindowType.DOCK && type != Meta.WindowType.DESKTOP)
            this.actions.push('Move Titlebar Onscreen')

        if (win.get_maximized() == Meta.MaximizeFlags.BOTH
            || type == Meta.WindowType.DOCK
            || type == Meta.WindowType.DESKTOP
            || type == Meta.WindowType.SPLASHSCREEN) {

            this.actions.push('Always on Top' + (win.is_above() ? ' ✓' : ''));
        }

        if (Main.sessionMode.hasWorkspaces
            && (!Meta.prefs_get_workspaces_only_on_primary() || win.is_on_primary_monitor())) {

            let isSticky = win.is_on_all_workspaces();

            if (win.is_always_on_all_workspaces()) {
                this.actions.push('Always on Visible Workspace' + (isSticky ? ' ✓' : ''));
            }

            if (!isSticky) {
                let workspace = win.get_workspace();
                if (workspace != workspace.get_neighbor(Meta.MotionDirection.LEFT))
                    this.actions.push('Move to Workspace Left');

                if (workspace != workspace.get_neighbor(Meta.MotionDirection.RIGHT))
                    this.actions.push('Move to Workspace Right');

                if (workspace != workspace.get_neighbor(Meta.MotionDirection.UP))
                    this.actions.push('Move to Workspace Up');

                if (workspace != workspace.get_neighbor(Meta.MotionDirection.DOWN))
                    this.actions.push('Move to Workspace Down');
            }
        }

        let display = global.display;
        let nMonitors = display.get_n_monitors();
        let monitorIndex = win.get_monitor();
        if (nMonitors > 1 && monitorIndex >= 0) {
            let dir = Meta.DisplayDirection.UP;
            let upMonitorIndex = display.get_monitor_neighbor_index(monitorIndex, dir);
            if (upMonitorIndex != -1)
                this.actions.push('Move to Monitor Up');

            dir = Meta.DisplayDirection.DOWN;
            let downMonitorIndex = display.get_monitor_neighbor_index(monitorIndex, dir);
            if (downMonitorIndex != -1)
                this.actions.push('Move to Monitor Down');

            dir = Meta.DisplayDirection.LEFT;
            let leftMonitorIndex = display.get_monitor_neighbor_index(monitorIndex, dir);
            if (leftMonitorIndex != -1)
                this.actions.push('Move to Monitor Left');

            dir = Meta.DisplayDirection.RIGHT;
            let rightMonitorIndex = display.get_monitor_neighbor_index(monitorIndex, dir);
            if (rightMonitorIndex != -1)
                this.actions.push('Move to Monitor Right');
        }
        
        if (win.can_close())
            this.actions.push('Close');

        return this.actions;
    }

    _doAction(action) {
        if (!this._win) return;
        if (action.endsWith(' ✓')) {
            action = action.substr(0, action.length - 2);
        }
        let win = this._win;
        switch (action) {
            case 'Minimize':
                win.minimize();
                break;
            case 'Unmaximize':
                if (win.unmaximize && win.unmaximize.length === 0) win.unmaximize();
                else win.unmaximize(Meta.MaximizeFlags.BOTH);
                break;
            case 'Maximize':
                if (win.maximize && win.maximize.length === 0) win.maximize();
                else win.maximize(Meta.MaximizeFlags.BOTH);
                break;
            case 'Move':
                GLib.timeout_add(GLib.PRIORITY_DEFAULT, 100, () => {
                    WindowMenu.WindowMenu.prototype._grabAction(win, Meta.GrabOp.KEYBOARD_MOVING, global.display.get_current_time_roundtrip());
                });
                break;
            case 'Resize':
                GLib.timeout_add(GLib.PRIORITY_DEFAULT, 100, () => {
                    WindowMenu.WindowMenu.prototype._grabAction(win, Meta.GrabOp.KEYBOARD_RESIZING_UNKNOWN, global.display.get_current_time_roundtrip());
                });
                break;
            case 'Move Titlebar Onscreen':
                win.shove_titlebar_onscreen();
                break;
            case 'Always on Top':
                if (win.is_above())
                    win.unmake_above();
                else
                    win.make_above();
                break;
            case 'Always on Visible Workspace':
                if (win.is_on_all_workspaces())
                    win.unstick();
                else
                    win.stick();
                break;
            case 'Move to Workspace Left':
                this._moveToWorkspace(Meta.MotionDirection.LEFT);
                break;
            case 'Move to Workspace Right':
                this._moveToWorkspace(Meta.MotionDirection.RIGHT);
                break;
            case 'Move to Workspace Up':
                this._moveToWorkspace(Meta.MotionDirection.UP);
                break;
            case 'Move to Workspace Down':
                this._moveToWorkspace(Meta.MotionDirection.DOWN);
                break;
            case 'Move to Monitor Up':
                this._moveToMonitor(Meta.DisplayDirection.UP);
                break;
            case 'Move to Monitor Down':
                this._moveToMonitor(Meta.DisplayDirection.DOWN);
                break;
            case 'Move to Monitor Left':
                this._moveToMonitor(Meta.DisplayDirection.LEFT);
                break;
            case 'Move to Monitor Right':
                this._moveToMonitor(Meta.DisplayDirection.RIGHT);
                break;
            case 'Close':
                win.delete(global.get_current_time());
                break;
        }
    }

    _moveToWorkspace(dir) {
        let workspace = this._win.get_workspace();
        this._win.change_workspace(workspace.get_neighbor(dir));
    }

    _moveToMonitor(dir) {
        let monitorIndex = this._win.get_monitor();
        let newMonitorIndex = global.display.get_monitor_neighbor_index(monitorIndex, dir);
        if (newMonitorIndex != -1) {
            this._win.move_to_monitor(newMonitorIndex);
        }
    }
}

const MenuButton = GObject.registerClass(
class MenuButton extends PanelMenu.Button {
    _init(label, menuBar) {
        label = label.replace('_', '');
        super._init(0.0, label);
        this._label = label;
        this._menuBar = menuBar;

        this.box = new St.BoxLayout({ style_class: 'panel-status-menu-box menubar-button' });
        this.labelWidget = new St.Label({
            text: this._label,
            y_align: Clutter.ActorAlign.CENTER,
            reactive: false
        });
        this.box.add_child(this.labelWidget);
        this.add_child(this.box);
        this.connect('button-release-event', this.onButtonEvent.bind(this));
    }

    _onStyleChanged(actor) {
        super._onStyleChanged(actor);
        let padding = this._menuBar.extension.settings.get_int('min-padding');
        this._minHPadding = padding;
        this._natHPadding = padding;
    }

    onButtonEvent(actor, event) {
        if (event.get_button() !== 1)
            return Clutter.EVENT_PROPAGATE;

        this._menuBar.onButtonClicked(this._label);
        return Clutter.EVENT_STOP;
    }
});

// --- FALLBACK MENU BUTTON WITH POPUP / BOTÓN DE MENÚ DE FALLBACK CON DESPLEGABLE ---
// Used when Fildem's Python backend is not running to render standard macOS-like static menus.
// Se usa cuando el backend de Python de Fildem no está activo para renderizar menús estáticos tipo macOS.
const FallbackMenuButton = GObject.registerClass(
class FallbackMenuButton extends PanelMenu.Button {
    _init(label, items, menuBar) {
        label = label.replace('_', '');
        super._init(0.0, label);
        this._label = label;
        this._menuBar = menuBar;

        this.box = new St.BoxLayout({ style_class: 'panel-status-menu-box menubar-button' });
        this.labelWidget = new St.Label({
            text: this._label,
            y_align: Clutter.ActorAlign.CENTER,
            reactive: false
        });
        this.box.add_child(this.labelWidget);
        this.add_child(this.box);

        // Force menu display toggle on click event for maximum robustness
        // Forzar alternancia de la visualización del menú en el evento de clic para máxima robustez
        this.connect('button-press-event', (actor, event) => {
            if (event.get_button() === 1) {
                this.menu.toggle();
                return Clutter.EVENT_STOP;
            }
            return Clutter.EVENT_PROPAGATE;
        });

        // Add dropdown items for fallback simulation / Añadir elementos del menú desplegable
        for (let itemLabel of items) {
            let menuItem = new PopupMenu.PopupMenuItem(itemLabel);
            menuItem.connect('activate', () => {
                this._onItemActivated(itemLabel);
            });
            this.menu.addMenuItem(menuItem);
        }
    }

    _onStyleChanged(actor) {
        super._onStyleChanged(actor);
        let padding = this._menuBar.extension.settings.get_int('min-padding');
        this._minHPadding = padding;
        this._natHPadding = padding;
    }

    _onItemActivated(itemLabel) {
        log(`Fallback item activado: ${itemLabel}`);
        
        // Handle web navigation for os.inled.es button without needing a window focus
        // Manejar navegación web para el botón os.inled.es sin requerir foco de ventana
        if (itemLabel === 'os.inled.es') {
            try {
                Gio.AppInfo.launch_default_for_uri("https://os.inled.es", null);
            } catch (e) {
                log(`Error al abrir os.inled.es: ${e}`);
            }
            return;
        }

        let win = global.display.get_focus_window();
        if (!win) return;

        switch (itemLabel) {
            case 'Minimize':
                win.minimize();
                break;
            case 'Close Window':
                win.delete(global.get_current_time());
                break;
            case 'Enter Full Screen':
                if (win.maximize) win.maximize(Meta.MaximizeFlags.BOTH);
                break;
            case 'Zoom':
                if (win.is_maximized && win.is_maximized()) {
                    win.unmaximize(Meta.MaximizeFlags.BOTH);
                } else {
                    win.maximize(Meta.MaximizeFlags.BOTH);
                }
                break;
            case 'Resize':
                // Resize active window using keyboard window-management API
                // Redimensionar la ventana activa usando la API de gestión de ventanas por teclado
                GLib.timeout_add(GLib.PRIORITY_DEFAULT, 100, () => {
                    WindowMenu.WindowMenu.prototype._grabAction(win, Meta.GrabOp.KEYBOARD_RESIZING_UNKNOWN, global.display.get_current_time_roundtrip());
                });
                break;
            case 'Copy':
                // Simulate Ctrl+C keypress sequence via virtual keyboard device
                // Simular secuencia de teclas Ctrl+C mediante el dispositivo de teclado virtual
                try {
                    let seat = Clutter.get_default_backend().get_default_seat();
                    let kb = seat.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
                    let time = GLib.get_monotonic_time();
                    kb.notify_keyval(time, Clutter.KEY_Control_L, Clutter.KeyState.PRESSED);
                    kb.notify_keyval(time, Clutter.KEY_c, Clutter.KeyState.PRESSED);
                    kb.notify_keyval(time, Clutter.KEY_c, Clutter.KeyState.RELEASED);
                    kb.notify_keyval(time, Clutter.KEY_Control_L, Clutter.KeyState.RELEASED);
                } catch (e) {
                    log(`Error al simular copiar: ${e}`);
                }
                break;
            case 'Paste':
                // Simulate Ctrl+V keypress sequence via virtual keyboard device
                // Simular secuencia de teclas Ctrl+V mediante el dispositivo de teclado virtual
                try {
                    let seat = Clutter.get_default_backend().get_default_seat();
                    let kb = seat.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
                    let time = GLib.get_monotonic_time();
                    kb.notify_keyval(time, Clutter.KEY_Control_L, Clutter.KeyState.PRESSED);
                    kb.notify_keyval(time, Clutter.KEY_v, Clutter.KeyState.PRESSED);
                    kb.notify_keyval(time, Clutter.KEY_v, Clutter.KeyState.RELEASED);
                    kb.notify_keyval(time, Clutter.KEY_Control_L, Clutter.KeyState.RELEASED);
                } catch (e) {
                    log(`Error al simular pegar: ${e}`);
                }
                break;
        }
    }
});

const Cache = class Cache {
    constructor() {
        this.N = 10;
        this.lru = [];
        this.entries = {};
        this.lastQueriedKey = '';
    }

    get(key) {
        this.lastQueriedKey = key;
        return this.entries[key];
    }

    _set(key, value) {
        if (this.entries[key]) {
            const index = this.lru.indexOf(key);
            if (index > -1) this.lru.splice(index, 1);
        }
        this.lru.push(key);
        this.entries[key] = value;

        if (this.lru.length > this.N) {
            const toRemove = this.lru.shift();
            delete this.entries[toRemove];
        }
    }

    withCache(f) {
        const self = this;
        const g = (param) => {
            self._set(self.lastQueriedKey, param);
            f(param);
        }
        return g;
    }
}

const MenuBar = class MenuBar {
    constructor(proxy, extension) {
        this._menuButtons = [];
        this._proxy = proxy;
        this.extension = extension;
        this._width_offset = 300;
        this.MARGIN_FIRST_ELEMENT = 4;
        this._isShowingMenu = false;

        this._cache = new Cache();
        this._backendActive = false; // Tracks if Python daemon is running / Rastrea si el demonio Python está corriendo

        this._notifyFocusWinId = global.display.connect('notify::focus-window', this._onWindowSwitched.bind(this));
        this._proxy.listeners['SendTopLevelMenus'].push(this._cache.withCache(this.setMenus.bind(this)));
        this._proxy.listeners['MenuOnOff'].push(this._onMenuOnOff.bind(this));
        
        Main.panel.reactive = true;
        
        this._panelEvHandlers = [];
        this._forceShowMenu = false;
        this._showAppMenuButton = false;
        this.setForceShowMenu();
        this.setHideAppMenuButton();

        this._overviewShowingId = Main.overview.connect('showing', this._onOverviewOpened.bind(this));
        this._overviewHidingId = Main.overview.connect('hiding', this._onOverviewClosed.bind(this));
        
        log('MenuBar inicializado');
    }

    setForceShowMenu() {
        this._forceShowMenu = !this.extension.settings.get_boolean('show-only-when-hover');

        if (!this._forceShowMenu) {
            this._panelEvHandlers.push(Main.panel.connect('enter-event', this._onPanelEnter.bind(this)));
            this._panelEvHandlers.push(Main.panel.connect('leave-event', this._onPanelLeave.bind(this)));
        } else {
            for (let h of this._panelEvHandlers) {
                Main.panel.disconnect(h);
            }
            this._panelEvHandlers = [];
        }
    }

    setHideAppMenuButton() {
        this._showAppMenuButton = !this.extension.settings.get_boolean('hide-app-menu');
        this._restoreLabel();
    }

    addMenuButton(label, setmargin) {
        let menuButton = new MenuButton(label, this);
        this._menuButtons.push(menuButton);
        const nItems = Main.panel._leftBox.get_children().length;
        menuButton.hide();
        if (setmargin)
            menuButton.set_style('margin-left: '+ this.MARGIN_FIRST_ELEMENT + 'px')
        Main.panel.addToStatusArea(label, menuButton, nItems, 'left');
    }

    setFallbackMenus() {
        this.removeAll();
        
        // Get the active window/app name for macOS-like dynamic app menu
        // Obtener el nombre de la aplicación activa para el menú dinámico tipo macOS
        const focusApp = WinTracker.focus_app;
        let appName = 'PulsarOS';
        if (focusApp) {
            appName = focusApp.get_name();
        }

        // Define standard macOS-like static fallback menus
        // Definir menús de fallback estáticos tipo macOS tradicionales
        const fallbackData = {};
        
        // Active Application Menu (Always placed first)
        // Menú de la Aplicación Activa (Siempre va primero)
        fallbackData[appName] = [
            'About ' + appName,
            'Services',
            'Hide ' + appName,
            'Quit ' + appName
        ];

        fallbackData['File'] = ['New Window', 'Open...', 'Save', 'Close Window'];
        fallbackData['Edit'] = ['Undo', 'Redo', 'Cut', 'Copy', 'Paste', 'Select All'];
        fallbackData['View'] = ['Zoom In', 'Zoom Out', 'Actual Size', 'Enter Full Screen'];
        fallbackData['Go'] = ['Back', 'Forward', 'Home'];
        fallbackData['Window'] = ['Minimize', 'Zoom', 'Resize', 'Cycle Through Windows', 'Bring All to Front'];
        fallbackData['Help'] = ['PulsarOS Help', 'os.inled.es', 'About PulsarOS'];

        let first = true;
        for (let label in fallbackData) {
            let items = fallbackData[label];
            let menuButton = new FallbackMenuButton(label, items, this);
            this._menuButtons.push(menuButton);
            const nItems = Main.panel._leftBox.get_children().length;
            menuButton.hide();
            if (first) {
                menuButton.set_style('margin-left: ' + this.MARGIN_FIRST_ELEMENT + 'px');
                first = false;
            }
            Main.panel.addToStatusArea(label, menuButton, nItems, 'left');
        }
        
        if (this._forceShowMenu && !Main.overview.visibleTarget) {
            this._onPanelEnter();
        }
    }

    setMenus(menus) {
        log(`Recibidos ${menus.length} menús: ${menus}`);
        this._backendActive = true; // Mark backend active / Marcar backend como activo
        if (menus.length === 0) {
            this._hideMenu();
        }
        this.removeAll();

        // Get the active window/app name for macOS-like dynamic app menu
        // Obtener el nombre de la aplicación activa para el menú dinámico tipo macOS
        const focusApp = WinTracker.focus_app;
        let appName = 'PulsarOS';
        if (focusApp) {
            appName = focusApp.get_name();
        }

        // Add the application menu button first
        // Añadir primero el botón del menú de la aplicación
        this.addMenuButton(appName, true);

        // Add the rest of the backend-provided menus
        // Añadir el resto de menús provistos por el backend
        for (let menu of menus) {
            if (menu !== appName) {
                this.addMenuButton(menu, false);
            }
        }

        if (this._forceShowMenu && !Main.overview.visibleTarget) {
            this._onPanelEnter();
        }
    }

    _onPanelEnter() {
        if (this._menuButtons.length === 0 || Main.overview.visibleTarget)
            return;

        this._hideAppMenuButton();
        this._showMenu();
    }

    _hideAppMenuButton() {
        let width = 0;
        for (let el of Main.panel._leftBox.get_children()) {
            let firstChild = el.get_first_child();
            if (firstChild === this._menuButtons[0]) {
                this._width_offset = width;
                break;
            }
            if (el.is_visible()) {
                width += el.get_width();
            }
        }
    }

    _showMenu() {
        this._menuButtons.forEach(btn => btn.show());
        this._menuButtons.forEach(btn => btn.ease({
            opacity: 255,
            mode: Clutter.AnimationMode.EASE_OUT_QUART,
            duration: 250
        }));
    }

    _onPanelLeave() {
        if (this._isShowingMenu || this._forceShowMenu)
            return;

        this._hideMenu();
    }

    _hideMenu() {
        this._menuButtons.forEach(btn => btn.ease({
            opacity: 0,
            mode: Clutter.AnimationMode.EASE_OUT_QUART,
            duration: 100,
            onComplete: () => { 
                this._menuButtons.forEach(btn => btn.hide()); 
                this._restoreLabel();
            }
        }));
    }

    _restoreLabel() {
    }

    _onMenuOnOff(on) {
        log(`MenuOnOff recibido: ${on}`);
        if (on) {
            this._backendActive = true;
            this._onPanelEnter();
            this.onButtonClicked('__fildem_move', this._width_offset);
        } else {
            this._isShowingMenu = false;
            this._onPanelLeave();
        }
    }

    onButtonClicked(label) {
        this._isShowingMenu = true;
        this._proxy.EchoSignal(label, this._width_offset);
    }

    removeAll() {
        for (let e of this._menuButtons) {
            e.destroy();
        }
        this._menuButtons = [];
    }

    _onWindowSwitched() {
        this.removeAll();
        this._restoreLabel();
        this._hideMenu();
        const focusApp = WinTracker.focus_app;
        if (focusApp) {
            let appId = focusApp.get_id();
            log(`Ventana cambiada a: ${appId}`);

            // Use cached menus if backend is active
            // Usar menús cacheados si el backend está activo
            if (this._backendActive) {
                let cachedValue = this._cache.get(appId);
                if (cachedValue) {
                    this.setMenus(cachedValue);
                    return;
                }
            }

            // Fallback if backend is inactive
            // Fallback si el backend está inactivo
            if (!this._backendActive) {
                this.setFallbackMenus();
            } else {
                let windowData = {};
                let win = focusApp.get_windows()[0];
                let xid = '';
                if (win) {
                    try {
                        xid = parseInt(win.get_description().match(/0x[0-9a-f]+/)[0]);
                    } catch (e) {}
                    windowData['xid'] = String(xid);
                    for (let p in win) {
                        if (p.startsWith('gtk_') && win[p] != null) {
                            windowData[p] = win[p];
                        }
                    }
                }
                this._proxy.WindowSwitched(windowData);
            }
        }
    }

    _onOverviewOpened() {
        this._hideMenu();
    }

    _onOverviewClosed() {
        if (this._forceShowMenu && this._menuButtons.length) {
            this._hideAppMenuButton();
            this._showMenu();
        }
    }

    _disconnectAll() {
        for (let h of this._panelEvHandlers) {
            Main.panel.disconnect(h);
        }
        if (this._notifyFocusWinId)
            global.display.disconnect(this._notifyFocusWinId);
        
        if (this._overviewShowingId)
            Main.overview.disconnect(this._overviewShowingId);
        if (this._overviewHidingId)
            Main.overview.disconnect(this._overviewHidingId);
    }

    destroy() {
        this._disconnectAll();
        this.removeAll();
        this._restoreLabel();
    }
};

const ifaceXml = `
<node>
  <interface name="es.inled.fildem">
    <method name="EchoSignal">
      <arg type="s" direction="in" name="menu"/>
      <arg type="u" direction="in" name="x"/>
    </method>
    <method name="WindowSwitched">
      <arg name="win_data" type="a{ss}" direction="in"/>
    </method>

    <signal name="WindowSwitchedSignal">
      <arg name="win_data" type="a{ss}"/>
    </signal>
    <signal name="MenuActivated">
      <arg name="menu" type="s"/>
      <arg name="x" type="u"/>
    </signal>

    <method name="EchoMenuOnOff">
      <arg name="on" type="b" direction="in"/>
    </method>
    <signal name="MenuOnOff">
      <arg name="on" type="b"/>
    </signal>

    <method name="SendTopLevelMenus">
      <arg name="top_level_menus" type="as" direction="in"/>
    </method>
    <signal name="SendTopLevelMenusSignal">
      <arg name="top_level_menus" type="as"/>
    </signal>


    <method name="RequestWindowActions"/>
    <signal name="RequestWindowActionsSignal"/>

    <method name="ListWindowActions">
      <arg name="actions" type="as" direction="in"/>
    </method>
    <signal name="ListWindowActionsSignal">
      <arg name="actions" type="as"/>
    </signal>

    <method name="ActivateWindowAction">
      <arg name="action" type="s" direction="in"/>
    </method>
    <signal name="ActivateWindowActionSignal">
      <arg name="action" type="s"/>
    </signal>
  </interface>
</node>`;

const TestProxy = Gio.DBusProxy.makeProxyWrapper(ifaceXml);

const BUS_NAME = 'es.inled.fildem';
const BUS_PATH = '/es/inled/fildem';

class MyProxy {
    constructor() {
        this._createProxy();
        this._handlerIds = [];
    }

    async _createProxy() {
        log('Creando Proxy D-Bus...');
        this._proxy = new TestProxy(
            Gio.DBus.session,
            BUS_NAME,
            BUS_PATH,
            this._onProxyReady.bind(this)
        );
        this.listeners = {
            'MenuActivated': [],
            'SendTopLevelMenus': [],
            'MenuOnOff': []
        }
    }

    async _onProxyReady(result, error) {
        if (error) {
            console.error('[FILDEM_MENU] Error al conectar Proxy D-Bus:', error);
            return;
        }
        log('Proxy D-Bus listo. Conectando señales...');
        let id = undefined;
        id = this._proxy.connectSignal('SendTopLevelMenus', this._onSendTopLevelMenus.bind(this));
        this._handlerIds.push(id);
        id = this._proxy.connectSignal('RequestWindowActionsSignal', this._onRequestWindowActionsSignal.bind(this));
        this._handlerIds.push(id);
        id = this._proxy.connectSignal('ActivateWindowActionSignal', this._onActivateWindowActionSignal.bind(this));
        this._handlerIds.push(id);
        id = this._proxy.connectSignal('MenuOnOff', this._onMenuOnOff.bind(this));
        this._handlerIds.push(id);
    }

    async _onMenuActivated(proxy, nameOwner, args) {
        log(`TestSignal: ${args[0]}`);
    }

    async _onSendTopLevelMenus(proxy, nameOwner, args) {
        let topLevelMenus = args[0];
        log(`Señal SendTopLevelMenus recibida: ${topLevelMenus}`);
        for (let callback of this.listeners['SendTopLevelMenus']) {
            callback(topLevelMenus);
        }
    }

    async _onRequestWindowActionsSignal(proxy, nameOwner, args) {
        this._currentWindow = new WindowActions();
        let actions = this._currentWindow.getActions();
        this._proxy.ListWindowActionsRemote(actions);
    }

    async _onActivateWindowActionSignal(proxy, nameOwner, args) {
        if (this._currentWindow)
            this._currentWindow._doAction(args[0]);
    }

    async _onMenuOnOff(proxy, nameOwner, args) {
        for (let callback of this.listeners['MenuOnOff']) {
            callback(args[0]);
        }
    }

    _onNameOwnerChanged(proxy, sender, [name, oldOwner, newOwner]) {
        log(`Name owner changed: ${name} ${oldOwner} -> ${newOwner}`)
    }

    async WindowSwitched(windowData) {
        if (this._proxy) {
            log('Enviando WindowSwitched a Python...');
            this._proxy.WindowSwitchedRemote(windowData);
        }
    }

    async EchoSignal(menu, x) {
        if (this._proxy)
            this._proxy.EchoSignalRemote(menu, x);
    }

    destroy() {
        if (this._proxy) {
            for (let id of this._handlerIds) {
                this._proxy.disconnectSignal(id);
            }
        }
        this._handlerIds = [];
    }
}

export default class FildemExtension extends Extension {
    enable() {
        log('Habilitando extensión...');
        this.settings = this.getSettings();
        this._handlerIds = [];
        this.myProxy = new MyProxy();
        this.menubar = new MenuBar(this.myProxy, this);

        this._connectSettings();
    }

    _connectSettings() {
        this._handlerIds.push(this.settings.connect(
            'changed::show-only-when-hover',
            () => { this.menubar.setForceShowMenu(); }
        ));
        this._handlerIds.push(this.settings.connect(
            'changed::hide-app-menu',
            () => { this.menubar.setHideAppMenuButton(); }
        ));
    }

    disable() {
        log('Deshabilitando extensión...');
        this._disconnectSettings();
        if (this.menubar)
            this.menubar.destroy();
        if (this.myProxy)
            this.myProxy.destroy();
        
        this.menubar = null;
        this.myProxy = null;
        this.settings = null;
    }

    _disconnectSettings() {
        if (this.settings) {
            for (let h of this._handlerIds) {
                this.settings.disconnect(h);
            }
        }
        this._handlerIds = [];
    }
}
