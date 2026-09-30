// Styling for the menu bar and its popups: translucency, rounding, button
// spacing, font size, and optional blur behind the popups.
//
// Every MenuBarButton registers itself here from menuBarButton.js, so the
// three menu sources get styled without knowing that this module exists.
// Nothing here touches shared shell UI: styling is applied as inline styles
// and global-menu-* CSS classes on our own actors. The one exception is the
// workspace indicator, which we hide on request and restore on disable.
//
// Blur goes through Blur My Shell, which publishes itself as
// global.blur_my_shell for exactly this. We take one of its native dynamic
// gaussian blur effects per popup (a BACKGROUND-mode Shell.BlurEffect,
// re-blurred every frame) from its EffectsManager. Without Blur My Shell
// there is no blur and everything else still works.

import Clutter from 'gi://Clutter';

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

// Buttons register even while no manager exists, so a manager created later
// can still style them.
const buttons = new Set();
let manager = null;

/** Called by MenuBarButton._init. Unregisters itself when the actor dies. */
export function registerButton(btn) {
	buttons.add(btn);
	btn.connect('destroy', () => buttons.delete(btn));
	manager?.applyToButton(btn);
}

/**
 * Called by MenuBarButton when its menu opens. The popup's theme background
 * color can only be read once the actor is on stage, so translucency is
 * reapplied here. The blur effect is created here too rather than at button
 * creation: buttons are rebuilt on every menu-tree change, and taking an
 * effect out of Blur My Shell's pool for each short-lived button leaks
 * handler connections inside it.
 */
export function onMenuOpened(btn) {
	manager?.applyPopupBackground(btn);
	manager?.applyBlur(btn);
}

/**
 * GNOME's Activities button doubles as the workspace pill at the far left of
 * the panel.
 *
 * Hiding it is not a one-off. Panel._updatePanel() re-adds every indicator
 * and shows its container again, and it runs after extensions are enabled at
 * login, so a lone hide() in enable() is undone before the panel is ever
 * painted. That is why the pill used to reappear until the extension was
 * toggled by hand. Tracking the container's visibility, and the session mode
 * that rebuilds the panel, makes the hide stick from the first enable.
 */
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
		// Disconnect before showing. show() emits notify::visible, and the
		// handler below would read a setting that is still true and hide the
		// pill straight back again.
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
		// A panel rebuild shows the container again without any signal of
		// its own to hang off, so re-hide whenever it comes back.
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
		this._settingsIds = APPEARANCE_KEYS.map(key =>
			settings.connect(`changed::${key}`, () => this.applyAll()));

		// Blur My Shell does not detach effects it made for foreign actors,
		// so follow its enabled state and detach them ourselves.
		this._extStateId = Main.extensionManager.connect(
			'extension-state-changed', (_mgr, extension) => {
				if (extension.uuid === BMS_UUID)
					this.applyAll();
			});

		this._workspaceIndicator = new WorkspaceIndicator(settings);
		this.applyAll();
	}

	applyAll() {
		for (const btn of buttons)
			this.applyToButton(btn);
		this._workspaceIndicator?.sync();
	}

	applyToButton(btn) {
		const s = this._settings;

		// panelMenu reads -natural/-minimum-hpadding off the button's theme
		// node and the label inherits the font, so both are set inline here.
		// The shell theme bolds every panel button, which would drown the
		// app name it is meant to distinguish, so weight is forced to normal
		// and only the app-name button gets bold back.
		const spacing = s.get_int('button-spacing');
		const fontSize = s.get_int('menu-font-size');
		const bold = btn._globalMenuAppName && s.get_boolean('bold-app-name');
		let style = `-natural-hpadding: ${spacing}px; ` +
			`-minimum-hpadding: ${Math.min(spacing, 6)}px; ` +
			`font-weight: ${bold ? 'bold' : 'normal'};`;
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
		const radius = s.get_int('popup-corner-radius');
		const opacity = s.get_int('popup-opacity');
		const fontSize = s.get_int('menu-font-size');

		let style = `border-radius: ${radius}px;`;
		if (fontSize > 0)
			style += ` font-size: ${fontSize}pt;`;
		const base = this._popupBaseColor(btn);
		if (base !== null) {
			const alpha = (opacity / 100).toFixed(2);
			style += ` background-color: rgba(${base.red}, ${base.green}, ${base.blue}, ${alpha});`;
		}
		btn.menu.box.set_style(style);
	}

	applyBlur(btn) {
		const s = this._settings;
		const effects = this._bmsEffects();
		const wanted = s.get_boolean('blur-popups') && effects !== null;

		if (!wanted) {
			this._removeBlur(btn);
			return;
		}

		const radius = 2 * s.get_int('popup-blur-sigma');
		const brightness = s.get_double('popup-blur-brightness');
		const cornerRadius = s.get_int('popup-corner-radius');

		if (btn._globalMenuBlur) {
			btn._globalMenuBlur.unscaled_radius = radius;
			btn._globalMenuBlur.brightness = brightness;
			if (btn._globalMenuBlur.corner_radius !== undefined)
				btn._globalMenuBlur.corner_radius = cornerRadius;
			return;
		}

		// Deferred to the first open, see onMenuOpened
		if (!btn.menu.isOpen)
			return;

		btn._globalMenuBlur = effects.new_native_dynamic_gaussian_blur_effect({
			unscaled_radius: radius,
			brightness,
			corner_radius: cornerRadius,
		});
		btn.menu.box.add_effect(btn._globalMenuBlur);

		// BoxPointer always forces offscreen redirect, which makes a
		// BACKGROUND blur sample the popup's own empty offscreen buffer
		// instead of the screen: translucent, never blurred. Relaxing it to
		// the stock actor default only flattens the popup during its
		// open/close fade, where the blur cannot be seen anyway.
		btn.menu.actor.set_offscreen_redirect(Clutter.OffscreenRedirect.AUTOMATIC_FOR_OPACITY);
	}

	destroy() {
		for (const id of this._settingsIds)
			this._settings.disconnect(id);
		this._settingsIds = [];
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

	// The popup's theme background color, captured once per button before our
	// translucent inline style overrides it. An off-stage theme node yields a
	// bogus color, so return null and let the next menu open retry.
	_popupBaseColor(btn) {
		if (btn._globalMenuBaseBg !== undefined)
			return btn._globalMenuBaseBg;
		if (!btn.menu.box.get_stage())
			return null;
		try {
			const color = btn.menu.box.get_theme_node().get_background_color();
			btn._globalMenuBaseBg = {red: color.red, green: color.green, blue: color.blue};
		} catch (_e) {
			return null;
		}
		return btn._globalMenuBaseBg;
	}

	_removeBlur(btn) {
		if (btn.menu?.actor)
			btn.menu.actor.set_offscreen_redirect(Clutter.OffscreenRedirect.ALWAYS);
		if (!btn._globalMenuBlur)
			return;
		const effects = this._bmsEffects();
		if (effects)
			effects.remove(btn._globalMenuBlur);
		else
			btn.menu.box.remove_effect(btn._globalMenuBlur);
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
