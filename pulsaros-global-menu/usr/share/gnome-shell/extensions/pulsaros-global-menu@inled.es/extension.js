// Pulsar OS Global Menu Extension
// Extensión para mostrar menús de aplicación reales nativos (DBusMenu) y curados al estilo macOS.
// Compatible with GNOME 45-50 & Wayland.

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Shell from 'gi://Shell';
import Meta from 'gi://Meta';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as SystemActions from 'resource:///org/gnome/shell/misc/systemActions.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

import {createManager as createAppearanceManager} from './appearance.js';
import {DaemonManager} from './daemon.js';
import {FallbackMenu} from './fallbackMenu.js';
import {NativeMenu} from './nativeMenu.js';
import {ShortcutMenu} from './shortcutMenu.js';
import {ShortcutStore} from './shortcutStore.js';
import {WindowActions, cancelPendingActions} from './windowActions.js';
import * as KeySynth from './keySynth.js';

import {PulsarLogoButton} from './pulsarAppleMenu.js';
import {LockScreen} from './pulsarLockScreen.js';
import {DesktopLiveWallpaperManager} from './pulsarWallpaper.js';
import {MacOSFullscreenManager} from './pulsarFullscreen.js';

const BUS_NAME = 'space.unmade.GlobalMenu';
const BUS_PATH = '/space/unmade/GlobalMenu';

class MenuTreeCache {
	constructor(size = 10) {
		this._size = size;
		this._lru = [];
		this._entries = new Map();
		this._lastQueried = '';
	}

	get(appId) {
		this._lastQueried = appId;
		return this._entries.get(appId);
	}

	storeForLastQuery(items) {
		const key = this._lastQueried;
		if (this._entries.has(key))
			this._lru.splice(this._lru.indexOf(key), 1);
		this._lru.push(key);
		this._entries.set(key, items);

		if (this._lru.length > this._size)
			this._entries.delete(this._lru.shift());
	}
}

class MenuBar {
	constructor(proxy, fallbackMenu, uuid, store, settings) {
		this._winTracker = Shell.WindowTracker.get_default();
		this._proxy = proxy;
		this._fallback = fallbackMenu ?? null;
		this._native = new NativeMenu(proxy, uuid);
		this._shortcuts = new ShortcutMenu(uuid);
		this._store = store;
		this._settings = settings;
		this._shortcutMapping = null;
		this._cache = new MenuTreeCache();

		this._notifyFocusWinId = global.display.connect('notify::focus-window',
			() => this.refresh());
		this._onTreeReceived = items => {
			this._cache.storeForLastQuery(items);
			this.setMenuTree(items);
		};
		this._proxy.listeners['SendMenuTree'].push(this._onTreeReceived);
		this._overviewHandler = Main.overview.connect('showing', () => this._closeAllMenus());
	}

	setFallback(fallbackMenu) {
		this._fallback = fallbackMenu;
		this._fallback?.setNativeMenusPresent(this._native.hasMenus() || this._shortcuts.hasMenus());
	}

	setMenuTree(items) {
		this._native.setItems(items);
		if (items.length > 0) {
			this._shortcuts.setMapping(null);
			this._fallback?.setNativeMenusPresent(true);
		} else if (this._shortcutMapping) {
			this._shortcuts.setMapping(this._shortcutMapping);
			this._fallback?.setNativeMenusPresent(true);
		} else {
			this._shortcuts.setMapping(null);
			this._fallback?.setNativeMenusPresent(false);
		}
	}

	refresh() {
		this._fallback?.updateAppName();
		this._shortcutMapping = null;

		const focusApp = this._winTracker.focus_app;
		const win = focusApp?.get_windows()[0];
		if (!win) {
			this._showNothing();
			return;
		}

		const candidates = this._shortcutCandidates(focusApp, win);
		const mapping = this._store.resolveMapping(candidates);
		this._shortcutMapping = mapping && this._shortcutsEnabledForApp(mapping) ? mapping : null;
		if (this._settings.get_boolean('debug-shortcut-menus')) {
			console.log(`[global-menu-shortcuts] resolution: candidates=[${candidates}] ` +
				`matched ${this._shortcutMapping?.app ?? 'nothing'}`);
		}

		const appId = focusApp.get_id();
		const cached = this._cache.get(appId);
		if (cached) {
			this.setMenuTree(cached);
		} else if (this._shortcutMapping) {
			this._native.clear();
			this._shortcuts.setMapping(this._shortcutMapping);
			this._fallback?.setNativeMenusPresent(true);
		} else {
			this._showNothing();
		}

		this._proxy.WindowSwitched(this._windowData(win));
	}

	destroy() {
		global.display.disconnect(this._notifyFocusWinId);
		Main.overview.disconnect(this._overviewHandler);
		const treeListeners = this._proxy.listeners['SendMenuTree'];
		const idx = treeListeners.indexOf(this._onTreeReceived);
		if (idx !== -1)
			treeListeners.splice(idx, 1);
		this._native.destroy();
		this._native = null;
		this._shortcuts.destroy();
		this._shortcuts = null;
		this._shortcutMapping = null;
		this._store = null;
		this._settings = null;
		this._winTracker = null;
	}

	_closeAllMenus() {
		this._native.closeAllMenus();
		this._shortcuts.closeAllMenus();
		this._fallback?.closeAllMenus();
	}

	_showNothing() {
		this._native.clear();
		this._shortcuts.setMapping(null);
		this._fallback?.setNativeMenusPresent(false);
	}

	_shortcutsEnabledForApp(mapping) {
		return this._settings.get_boolean('shortcut-menus-enabled') &&
			!this._settings.get_strv('shortcut-disabled-apps').includes(mapping.app);
	}

	_shortcutCandidates(focusApp, win) {
		const candidates = new Set();
		const add = value => {
			if (value)
				candidates.add(String(value).toLowerCase());
		};
		const appId = focusApp.get_id();
		add(appId);
		if (appId?.endsWith('.desktop'))
			add(appId.slice(0, -'.desktop'.length));
		add(win.get_wm_class());
		add(win.get_wm_class_instance());
		add(focusApp.get_name());
		return [...candidates];
	}

	_windowData(win) {
		const data = {xid: '0'};
		const description = win.get_description()?.match(/0x[0-9a-f]+/);
		if (description)
			data.xid = String(parseInt(description[0]));

		const gtkProps = [
			'gtk_unique_bus_name',
			'gtk_application_id',
			'gtk_application_object_path',
			'gtk_window_object_path',
			'gtk_app_menu_object_path',
			'gtk_menubar_object_path',
		];
		for (const prop of gtkProps) {
			if (win[prop] != null)
				data[prop] = win[prop];
		}
		return data;
	}
}

const ifaceXml = `
<node>
  <interface name="space.unmade.GlobalMenu">
	<method name="WindowSwitched">
	  <arg name="win_data" type="a{ss}" direction="in"/>
	</method>

	<signal name="SendMenuTree">
	  <arg name="tree_json" type="s"/>
	</signal>
	<method name="ActivateMenuItem">
	  <arg name="item_path" type="s" direction="in"/>
	</method>
	<method name="RequestMenuTree"/>

	<signal name="RequestWindowActionsSignal"/>
	<method name="ListWindowActions">
	  <arg name="actions" type="as" direction="in"/>
	</method>
	<signal name="ActivateWindowActionSignal">
	  <arg name="action" type="s"/>
	</signal>
  </interface>
</node>`;

const MenuProxy = Gio.DBusProxy.makeProxyWrapper(ifaceXml);

class DaemonProxy {
	constructor() {
		this._handlerIds = [];
		this._destroyed = false;
		this._currentWindow = null;
		this.listeners = {
			'SendMenuTree': [],
		};
		this._proxy = new MenuProxy(
			Gio.DBus.session,
			BUS_NAME,
			BUS_PATH,
			this._onProxyReady.bind(this)
		);
	}

	WindowSwitched(windowData) {
		this._proxy.WindowSwitchedRemote(windowData);
	}

	ActivateMenuItem(itemPath) {
		this._proxy.ActivateMenuItemRemote(itemPath);
	}

	RequestMenuTree() {
		this._proxy.RequestMenuTreeRemote();
	}

	destroy() {
		this._destroyed = true;
		for (const id of this._handlerIds)
			this._proxy.disconnectSignal(id);
		this._handlerIds = [];
		this._currentWindow = null;
	}

	_onProxyReady(result, error) {
		if (this._destroyed)
			return;
		if (error)
			return;

		this._handlerIds.push(
			this._proxy.connectSignal('SendMenuTree', this._onSendMenuTree.bind(this)),
			this._proxy.connectSignal('RequestWindowActionsSignal', this._onRequestWindowActions.bind(this)),
			this._proxy.connectSignal('ActivateWindowActionSignal', this._onActivateWindowAction.bind(this))
		);
	}

	_onSendMenuTree(proxy, nameOwner, args) {
		let items = [];
		try {
			items = JSON.parse(args[0]);
		} catch (e) {
			console.warn(`[global-menu] daemon sent an unreadable menu tree: ${e.message}`);
		}
		for (const callback of this.listeners['SendMenuTree'])
			callback(items);
	}

	_onRequestWindowActions() {
		this._currentWindow = new WindowActions();
		this._proxy.ListWindowActionsRemote(this._currentWindow.getActions());
	}

	_onActivateWindowAction(proxy, nameOwner, args) {
		this._currentWindow?.doAction(args[0]);
	}
}

export default class PulsarosGlobalMenuExtension extends Extension {
	enable() {
		this._settings = this.getSettings();
		this._settingsHandlerIds = [];
		this._activePowerDialog = null;

		// 1. Pulsar Apple Logo Button at the leftmost panel position
		try {
			this._appleLogoButton = new PulsarLogoButton(this);
			Main.panel.addToStatusArea('pulsaros-apple-menu', this._appleLogoButton, 0, 'left');
		} catch (e) {
			console.error("[GlobalMenu] Failed to create Apple logo button:", e);
		}

		// 2. LockScreen Overlay
		try {
			this._lockScreenOverlay = new LockScreen(this);
			Main.uiGroup.add_child(this._lockScreenOverlay);

			// Super+L lock screen shortcut
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
			} catch (e) {}

			// Native screenShield lock & unlock interception
			if (Main.screenShield) {
				this._origLock = Main.screenShield.lock;
				let self = this;
				Main.screenShield.lock = function(animate) {
					try {
						self._lockScreenOverlay.lock();
					} catch (err) {}
					return true;
				};

				this._origUnlock = Main.screenShield.unlock;
				Main.screenShield.unlock = function(animate) {
					try {
						self._lockScreenOverlay.unlock();
					} catch (err) {}
					if (self._origUnlock) {
						self._origUnlock.call(Main.screenShield, animate);
					}
				};

				this._activeChangedId = Main.screenShield.connect('active-changed', () => {
					if (!Main.screenShield.active && this._lockScreenOverlay) {
						this._lockScreenOverlay.unlock();
					}
				});
			}

			// Login1 Session Lock signal
			this._login1SessionLockId = Gio.DBus.system.signal_subscribe(
				'org.freedesktop.login1',
				'org.freedesktop.login1.Session',
				'Lock',
				null,
				null,
				Gio.DBusSignalFlags.NONE,
				() => {
					if (this._lockScreenOverlay) {
						this._lockScreenOverlay.lock();
					}
				}
			);
		} catch (e) {
			console.error("[GlobalMenu] Failed to initialize lock screen:", e);
		}

		// 3. Prepare for sleep signal (cleanup dialogs)
		try {
			this._prepareForSleepId = Gio.DBus.system.signal_subscribe(
				'org.freedesktop.login1',
				'org.freedesktop.login1.Manager',
				'PrepareForSleep',
				'/org/freedesktop/login1',
				null,
				Gio.DBusSignalFlags.NONE,
				(connection, senderName, objectPath, interfaceName, signalName, parameters) => {
					if (this._activePowerDialog) {
						this._activePowerDialog._cleanup();
						this._activePowerDialog.close();
						this._activePowerDialog = null;
					}
				}
			);
		} catch (e) {}

		// 4. Live Wallpaper Manager
		try {
			this._desktopLiveWallpaperManager = new DesktopLiveWallpaperManager(this);
		} catch (e) {
			console.error("[GlobalMenu] Failed to start live wallpaper manager:", e);
		}

		// 5. macOS Fullscreen Spaces Manager
		try {
			this._macOSFullscreenManager = new MacOSFullscreenManager(this);
		} catch (e) {
			console.error("[GlobalMenu] Failed to start fullscreen manager:", e);
		}

		// 6. Native Menus & Shortcuts Manager
		this._daemon = new DaemonManager();
		this._daemon.start();
		this._proxy = new DaemonProxy();

		KeySynth.setDebug(this._settings.get_boolean('debug-shortcut-menus'));
		this._store = new ShortcutStore(
			this.dir.get_child('shortcuts'),
			Gio.File.new_for_path(GLib.build_filenamev(
				[GLib.get_user_config_dir(), 'global-menu', 'shortcuts']))
		);
		this._store.load();

		this._appearance = createAppearanceManager(this._settings);
		this._fallback = this._settings.get_boolean('enable-fallback-menu')
			? new FallbackMenu(this.uuid) : null;
		this._menubar = new MenuBar(this._proxy, this._fallback, this.uuid, this._store, this._settings);

		this._watch('enable-fallback-menu', () => this._toggleFallback());
		this._watch('shortcuts-revision', () => {
			this._store.load();
			this._menubar.refresh();
		});
		this._watch('debug-shortcut-menus', () => {
			KeySynth.setDebug(this._settings.get_boolean('debug-shortcut-menus'));
		});

		// 7. Auto-sync MacTahoe Light / Dark theme
		try {
			this._ifaceSettings = new Gio.Settings({ schema_id: 'org.gnome.desktop.interface' });
			let userThemeSource = Gio.SettingsSchemaSource.get_default();
			this._userThemeSettings = (userThemeSource && userThemeSource.lookup('org.gnome.shell.extensions.user-theme', true))
				? new Gio.Settings({ schema_id: 'org.gnome.shell.extensions.user-theme' })
				: null;

			this._syncThemes = () => {
				let colorScheme = this._ifaceSettings.get_string('color-scheme');
				let targetTheme = (colorScheme === 'prefer-dark') ? 'MacTahoe-Dark' : 'MacTahoe-Light';
				
				let currentGtk = this._ifaceSettings.get_string('gtk-theme');
				if (currentGtk !== targetTheme) {
					this._ifaceSettings.set_string('gtk-theme', targetTheme);
				}

				if (this._userThemeSettings) {
					let currentShell = this._userThemeSettings.get_string('name');
					if (currentShell !== targetTheme) {
						this._userThemeSettings.set_string('name', targetTheme);
					}
				}
			};

			this._colorSchemeChangeId = this._ifaceSettings.connect('changed::color-scheme', () => {
				this._syncThemes();
			});
			this._syncThemes();
		} catch (e) {
			console.error("[GlobalMenu] Theme sync error:", e);
		}
	}

	disable() {
		if (this._ifaceSettings && this._colorSchemeChangeId) {
			this._ifaceSettings.disconnect(this._colorSchemeChangeId);
			this._colorSchemeChangeId = 0;
		}

		if (this._appleLogoButton) {
			this._appleLogoButton.destroy();
			this._appleLogoButton = null;
		}

		if (this._activeChangedId && Main.screenShield) {
			Main.screenShield.disconnect(this._activeChangedId);
			this._activeChangedId = 0;
		}

		if (this._origLock && Main.screenShield) {
			Main.screenShield.lock = this._origLock;
			this._origLock = null;
		}
		if (this._origUnlock && Main.screenShield) {
			Main.screenShield.unlock = this._origUnlock;
			this._origUnlock = null;
		}

		try {
			Main.wm.removeKeybinding('screensaver');
		} catch (e) {}

		if (this._prepareForSleepId) {
			try { Gio.DBus.system.signal_unsubscribe(this._prepareForSleepId); } catch (e) {}
			this._prepareForSleepId = 0;
		}
		if (this._login1SessionLockId) {
			try { Gio.DBus.system.signal_unsubscribe(this._login1SessionLockId); } catch (e) {}
			this._login1SessionLockId = 0;
		}

		if (this._activePowerDialog) {
			try {
				this._activePowerDialog._cleanup();
				this._activePowerDialog.close();
			} catch (e) {}
			this._activePowerDialog = null;
		}

		if (this._lockScreenOverlay) {
			Main.uiGroup.remove_child(this._lockScreenOverlay);
			this._lockScreenOverlay.destroy();
			this._lockScreenOverlay = null;
		}

		if (this._desktopLiveWallpaperManager) {
			this._desktopLiveWallpaperManager.destroy();
			this._desktopLiveWallpaperManager = null;
		}

		if (this._macOSFullscreenManager) {
			this._macOSFullscreenManager.destroy();
			this._macOSFullscreenManager = null;
		}

		for (const id of this._settingsHandlerIds)
			this._settings.disconnect(id);
		this._settingsHandlerIds = [];
		this._settings = null;

		this._appearance?.destroy();
		this._appearance = null;
		this._menubar?.destroy();
		this._menubar = null;
		this._fallback?.destroy();
		this._fallback = null;
		this._store = null;
		this._proxy?.destroy();
		this._proxy = null;
		this._daemon?.destroy();
		this._daemon = null;
		cancelPendingActions();
	}

	_toggleFallback() {
		if (this._settings.get_boolean('enable-fallback-menu')) {
			this._fallback = new FallbackMenu(this.uuid);
			this._menubar.setFallback(this._fallback);
		} else {
			this._menubar.setFallback(null);
			this._fallback?.destroy();
			this._fallback = null;
		}
	}

	_watch(key, callback) {
		this._settingsHandlerIds.push(
			this._settings.connect(`changed::${key}`, callback));
	}
}
