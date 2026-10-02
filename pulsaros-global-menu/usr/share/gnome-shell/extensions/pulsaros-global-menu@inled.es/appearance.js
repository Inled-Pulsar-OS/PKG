// Styling for the menu bar and its popups: translucency, rounding, button
// spacing, font size, Liquid Glass specular refraction, and blur behind popups.

import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import Shell from 'gi://Shell';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const BMS_UUID = 'blur-my-shell@aunetx';

const APPEARANCE_KEYS = [
	'blur-popups',
	'popup-blur-sigma',
	'popup-blur-brightness',
	'popup-opacity',
	'popup-corner-radius',
	'hide-popup-arrow',
	'button-spacing',
	'menu-font-size',
	'bold-app-name',
	'hide-workspace-indicator',
];

const buttons = new Set();
let manager = null;

export function registerButton(btn) {
	buttons.add(btn);
	btn.connect('destroy', () => buttons.delete(btn));
	manager?.applyToButton(btn);
}

export function onMenuOpened(btn) {
	manager?.applyPopupBackground(btn);
	manager?.applyBlur(btn);
}

class WorkspaceIndicator {
	constructor(settings) {
		this._settings = settings;
		this._indicator = null;
		this._visibleId = 0;
		this._destroyId = 0;
		this._hiddenByUs = false;
		this._sessionModeId = Main.sessionMode.connect('updated', () => this.sync());
		this.sync();
	}

	sync() {
		this._track(Main.panel.statusArea.activities ?? null);
		const container = this._indicator?.container;
		if (!container)
			return;

		if (this._settings.get_boolean('hide-workspace-indicator')) {
			container.hide();
			this._hiddenByUs = true;
		} else if (this._hiddenByUs) {
			container.show();
			this._hiddenByUs = false;
		}
	}

	destroy() {
		Main.sessionMode.disconnect(this._sessionModeId);
		this._sessionModeId = 0;

		const container = this._indicator?.container;
		const restore = this._hiddenByUs;
		this._untrack();
		if (restore)
			container?.show();

		this._hiddenByUs = false;
		this._settings = null;
	}

	_track(indicator) {
		if (indicator === this._indicator)
			return;
		this._untrack();
		this._indicator = indicator;
		if (!indicator)
			return;

		const container = indicator.container;
		this._visibleId = container.connect('notify::visible', () => {
			if (container.visible && this._settings.get_boolean('hide-workspace-indicator'))
				container.hide();
		});
		this._destroyId = container.connect('destroy', () => this._untrack());
	}

	_untrack() {
		const container = this._indicator?.container;
		if (container) {
			if (this._visibleId)
				container.disconnect(this._visibleId);
			if (this._destroyId)
				container.disconnect(this._destroyId);
		}
		this._visibleId = 0;
		this._destroyId = 0;
		this._indicator = null;
	}
}

export class AppearanceManager {
	constructor(settings) {
		this._settings = settings;
		this._ifaceSettings = new Gio.Settings({ schema_id: 'org.gnome.desktop.interface' });
		
		this._settingsIds = APPEARANCE_KEYS.map(key =>
			settings.connect(`changed::${key}`, () => this.applyAll()));

		this._colorSchemeId = this._ifaceSettings.connect('changed::color-scheme', () => {
			this.applyAll();
		});

		this._extStateId = Main.extensionManager.connect(
			'extension-state-changed', (_mgr, extension) => {
				if (extension.uuid === BMS_UUID)
					this.applyAll();
			});

		this._workspaceIndicator = new WorkspaceIndicator(settings);
		this.applyAll();
		this._hookGenericPanelMenus();
	}

	_hookGenericPanelMenus() {
		if (!Main.panel?.statusArea) return;
		for (let key in Main.panel.statusArea) {
			let item = Main.panel.statusArea[key];
			if (item?.menu && !item._pulsarMenuBlurHooked) {
				item._pulsarMenuBlurHooked = true;
				item.menu.connect('open-state-changed', (menu, open) => {
					if (open) {
						this._applyGenericBlur(menu);
					}
				});
			}
		}
	}

	_applyGenericBlur(menu) {
		if (!this._settings || !this._settings.get_boolean('blur-popups')) return;
		let actor = menu.actor || menu._boxPointer;
		if (!actor) return;
		if (!actor._pulsarBlurEffect && Shell.BlurEffect) {
			try {
				let sigma = this._settings.get_int('popup-blur-sigma') || 28;
				let brightness = this._settings.get_double('popup-blur-brightness') || 0.85;
				actor._pulsarBlurEffect = new Shell.BlurEffect({
					mode: Shell.BlurMode.BACKGROUND,
					sigma: sigma,
					brightness: brightness,
				});
				actor.add_effect(actor._pulsarBlurEffect);
				actor.set_offscreen_redirect(Clutter.OffscreenRedirect.AUTOMATIC_FOR_OPACITY);
			} catch (e) {
				console.error("[GlobalMenu] Generic BlurEffect failed:", e);
			}
		}
	}

	applyAll() {
		for (const btn of buttons)
			this.applyToButton(btn);
		this._workspaceIndicator?.sync();
		this._hookGenericPanelMenus();
	}

	applyToButton(btn) {
		const s = this._settings;
		const colorScheme = this._ifaceSettings.get_string('color-scheme');
		const isDark = (colorScheme === 'prefer-dark');

		const spacing = s.get_int('button-spacing');
		const fontSize = s.get_int('menu-font-size');
		const bold = btn._globalMenuAppName && s.get_boolean('bold-app-name');
		
		// The top bar panel always requires high-contrast white/light text against the desktop panel
		let textColor = '#ffffff';
		let labelColor = 'rgba(255, 255, 255, 0.90)';

		let style = `-natural-hpadding: ${spacing}px; ` +
			`-minimum-hpadding: ${Math.min(spacing, 6)}px; ` +
			`font-weight: ${bold ? 'bold' : 'normal'}; ` +
			`color: ${bold ? textColor : labelColor}; ` +
			`text-shadow: 0 1px 2px rgba(0, 0, 0, 0.35);`;
		if (fontSize > 0)
			style += ` font-size: ${fontSize}pt;`;
		btn.set_style(style);

		if (s.get_boolean('hide-popup-arrow'))
			btn.menu.actor.add_style_class_name('global-menu-no-arrow');
		else
			btn.menu.actor.remove_style_class_name('global-menu-no-arrow');

		this.applyPopupBackground(btn);
		this.applyBlur(btn);
	}

	applyPopupBackground(btn) {
		const s = this._settings;
		const radius = s.get_int('popup-corner-radius') || 14;
		const opacity = s.get_int('popup-opacity') || 97;
		const fontSize = s.get_int('menu-font-size');
		const colorScheme = this._ifaceSettings.get_string('color-scheme');
		const isDark = (colorScheme === 'prefer-dark');

		const alpha = (opacity / 100).toFixed(2);
		let style = `border-radius: ${radius}px;`;
		if (fontSize > 0)
			style += ` font-size: ${fontSize}pt;`;

		if (isDark) {
			style += ` background-color: rgba(30, 30, 34, ${alpha});` +
				` border: 1px solid rgba(255, 255, 255, 0.18);` +
				` box-shadow: inset 0 1px 0 0 rgba(255, 255, 255, 0.22), 0 16px 40px rgba(0, 0, 0, 0.65), 0 2px 8px rgba(0, 0, 0, 0.35);` +
				` color: #ffffff;`;
		} else {
			style += ` background-color: rgba(246, 246, 248, ${alpha});` +
				` border: 1px solid rgba(0, 0, 0, 0.12);` +
				` box-shadow: inset 0 1px 0 0 rgba(255, 255, 255, 0.60), 0 12px 32px rgba(0, 0, 0, 0.15), 0 2px 6px rgba(0, 0, 0, 0.08);` +
				` color: #1d1d1f;`;
		}

		btn.menu.box.set_style(style);
	}

	applyBlur(btn) {
		const s = this._settings;
		const wanted = s.get_boolean('blur-popups');

		if (!wanted) {
			this._removeBlur(btn);
			return;
		}

		const sigma = s.get_int('popup-blur-sigma') || 28;
		const brightness = s.get_double('popup-blur-brightness') || 0.85;

		if (btn._globalMenuBlur) {
			btn._globalMenuBlur.sigma = sigma;
			btn._globalMenuBlur.brightness = brightness;
			return;
		}

		if (!btn.menu.isOpen)
			return;

		try {
			if (Shell.BlurEffect) {
				btn._globalMenuBlur = new Shell.BlurEffect({
					mode: Shell.BlurMode.BACKGROUND,
					sigma: sigma,
					brightness: brightness,
				});
				let targetActor = btn.menu.actor || btn.menu.box;
				targetActor.add_effect(btn._globalMenuBlur);
				targetActor.set_offscreen_redirect(Clutter.OffscreenRedirect.AUTOMATIC_FOR_OPACITY);
			}
		} catch (e) {
			console.error("[GlobalMenu] BlurEffect failed:", e);
		}
	}

	destroy() {
		for (const id of this._settingsIds)
			this._settings.disconnect(id);
		this._settingsIds = [];
		if (this._colorSchemeId && this._ifaceSettings) {
			this._ifaceSettings.disconnect(this._colorSchemeId);
			this._colorSchemeId = 0;
		}
		this._ifaceSettings = null;
		Main.extensionManager.disconnect(this._extStateId);
		this._extStateId = 0;
		this._workspaceIndicator.destroy();
		this._workspaceIndicator = null;
		for (const btn of buttons)
			this._resetButton(btn);
		this._settings = null;
		manager = null;
	}

	_bmsEffects() {
		return global.blur_my_shell?._effects_manager ?? null;
	}

	_removeBlur(btn) {
		if (!btn._globalMenuBlur)
			return;
		try {
			let targetActor = btn.menu.actor || btn.menu.box;
			targetActor.remove_effect(btn._globalMenuBlur);
		} catch (e) {}
		btn._globalMenuBlur = null;
	}

	_resetButton(btn) {
		btn.set_style(null);
		btn.menu.actor.remove_style_class_name('global-menu-no-arrow');
		btn.menu.box.set_style(null);
		this._removeBlur(btn);
	}
}

export function createManager(settings) {
	manager = new AppearanceManager(settings);
	return manager;
}

