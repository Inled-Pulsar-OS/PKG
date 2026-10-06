// Styling for the menu bar and its popups: translucency, rounding, button
// spacing, font size, Liquid Glass specular refraction, and blur behind popups.

import Gio from 'gi://Gio';
import Shell from 'gi://Shell';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const BMS_UUID = 'blur-my-shell@aunetx';

// libblur-effect is the same effect family the shell uses, and it additionally
// rounds the blurred area so it matches the rounded popups. It is optional
// though, so Shell.BlurEffect stays as the always-available fallback.
let BlurNamespace = null;
try {
	BlurNamespace = (await import('gi://Blur')).default;
} catch (e) {
	BlurNamespace = null;
}

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

function stageScale() {
	return St.ThemeContext.get_for_stage(global.stage)?.scale_factor ?? 1;
}

/// Blur behind a popup menu.
///
/// `Shell.BlurEffect` in `BlurMode.BACKGROUND` samples the pixels painted under
/// the actor, so it needs an actor of its own covering exactly the popup. It
/// cannot be attached to the popup itself, because the popup paints its own
/// background and its children on top of whatever the effect blurs.
///
/// Where that actor goes is the whole trick, and getting it wrong looks like
/// "no blur at all" rather than like an error:
///
///  - It cannot be a child of `.popup-menu-content`. That box is a vertical
///    `StBoxLayout`, and a layout manager overwrites the size of its children,
///    so the actor gets flattened on the next relayout.
///
///  - It must not be wrapped in a `Meta.BackgroundGroup` either, which is what
///    Blur my Shell does for the panel and the dock. A background group is not a
///    backdrop provider: mutter describes it as a container that "only draws the
///    parts of the backgrounds not occluded by opaque windows", which is to say
///    it exists for wallpaper actors. Blur my Shell's own popup blur does not
///    use one, and wrapping the popup's blur in one is what left this popup flat
///    while the dock kept logging that its group "needs an allocation".
///
/// So the actor goes straight into `Main.uiGroup`, below the popup, which is
/// where Blur my Shell puts it too. `Main.uiGroup` runs no layout manager, so
/// the size we set survives, and sitting below the popup is what lets the blur
/// be seen at all. Its geometry comes from the popup's transformed extents, so
/// it keeps up with the pointer walking the popup around as well as with its
/// contents changing size.
///
/// GNOME 50's effect has no `sigma` property: the radius is `radius`, in device
/// pixels, so it must be scaled by the stage scale factor the same way the shell
/// scales its own BLUR_RADIUS. Passing `sigma` makes the constructor throw
/// "No property sigma on ShellBlurEffect" and leaves the popup unblurred.
class PopupBlur {
	constructor(menu) {
		this._menu = menu;
		this._container = menu.box;
		this._actor = menu.actor;
		this._ids = [];
		this._effect = null;
		this._parent = null;
		this._cornerRadius = 0;
		this._fallbackCornerRadius = 0;

		// `reactive: false` keeps the surface from swallowing clicks meant for
		// the menu underneath it.
		this._widget = new St.Widget({
			name: 'pulsar-popup-blur-widget', reactive: false,
		});

		this._attach();
		this._sync();

		// The popup moves as the pointer tracks it and resizes as its contents
		// change; neither shows up on the container alone.
		this._watch(this._actor, 'notify::position');
		this._watch(this._container, 'notify::width');
		this._watch(this._container, 'notify::height');
		this._watch(menu, 'open-state-changed', () => {
			// `isOpen` is a plain boolean property on PopupMenuBase, and it is
			// already updated by the time this signal fires.
			this._widget.visible = menu.isOpen;
			if (menu.isOpen)
				this._sync();
		});
		this._widget.visible = menu.isOpen;
	}

	/// Walk up from the popup to the child of `Main.uiGroup` that contains it,
	/// so the surface can sit directly below that. Blur my Shell picks the same
	/// parent: a submenu lives deeper than the popup root, and dropping its
	/// surface into `Main.uiGroup` regardless would paint it over the menu it
	/// belongs to.
	_overlayParent() {
		let actor = this._actor;
		while (actor) {
			const parent = actor.get_parent();
			if (!parent)
				break;
			if (parent === Main.uiGroup)
				return { parent, sibling: actor };
			actor = parent;
		}
		return { parent: Main.uiGroup, sibling: null };
	}

	/// Put the surface under the popup. Anywhere else it either gets resized by
	/// a layout manager or painted over.
	_attach() {
		const { parent, sibling } = this._overlayParent();
		this._parent = parent;
		try {
			parent.add_child(this._widget);
			if (sibling)
				parent.set_child_below_sibling(this._widget, sibling);
		} catch (e) {
			console.error('[GlobalMenu] could not place popup blur:', e);
		}
	}

	_watch(target, signal, callback) {
		if (!target)
			return;
		try {
			this._ids.push([
				target, signal,
				target.connect(signal, callback ?? (() => this._sync())),
			]);
		} catch (e) {
			// A target can already be gone by the time we get here.
		}
	}

	/// The surface covers the popup's on-screen rect. Taking it from the
	/// transformed extents rather than from the container's own size means the
	/// theme's padding, border and the scale factor are already accounted for,
	/// so the blur reaches the rounded edge instead of stopping short of it.
	_sync() {
		if (!this._widget)
			return;

		let x, y, w, h;
		try {
			const extents = this._container.get_transformed_extents();
			const topLeft = extents.get_top_left();
			const bottomRight = extents.get_bottom_right();
			x = topLeft.x;
			y = topLeft.y;
			w = bottomRight.x - topLeft.x;
			h = bottomRight.y - topLeft.y;
		} catch (e) {
			[x, y] = this._container.get_transformed_position();
			[w, h] = this._container.get_transformed_size();
		}

		if (!(w > 0) || !(h > 0))
			return;

		// The extents are in stage coordinates, while the surface is positioned
		// inside the overlay parent, so the parent's own origin comes off again.
		let parentX = 0, parentY = 0;
		try {
			[parentX, parentY] = this._parent.get_transformed_position();
		} catch (e) {
			// A parent that has gone away leaves us at the stage origin.
		}

		this._widget.set_position(
			Math.round(x - parentX), Math.round(y - parentY));
		this._widget.set_size(Math.ceil(w), Math.ceil(h));

		// The theme's radius only becomes readable once the popup is mapped,
		// which is after the effect has been built, so push it out here.
		try {
			const node = this._container.get_theme_node();
			if (node)
				this._cornerRadius = node.get_border_radius(St.Corner.TOPLEFT);
		} catch (e) {
			// get_theme_node() throws while unmapped.
		}

		if (this._effect && BlurNamespace) {
			const corner = this._cornerRadius > 0
				? this._cornerRadius : this._fallbackCornerRadius;
			if (corner > 0)
				this._effect.corner_radius = Math.round(corner * stageScale());
		}

		// A resized effect keeps the backdrop it already sampled until something
		// asks for a new one, which on a popup that resizes with its contents
		// shows up as the blur lagging behind the menu.
		this._effect?.queue_repaint();
	}

	setEffect(radius, cornerRadius, brightness) {
		if (!this._widget)
			return;

		const scale = stageScale();
		const ns = BlurNamespace ?? Shell;
		this._fallbackCornerRadius = cornerRadius;

		const params = {
			mode: ns.BlurMode.BACKGROUND,
			radius: Math.round(radius * scale),
			brightness,
		};
		if (BlurNamespace) {
			const corner = this._cornerRadius > 0
				? this._cornerRadius : cornerRadius;
			params.corner_radius = Math.round(corner * scale);
		}

		if (this._effect) {
			this._effect.radius = params.radius;
			this._effect.brightness = brightness;
			if (BlurNamespace)
				this._effect.corner_radius = params.corner_radius;
			return;
		}

		try {
			this._effect = new ns.BlurEffect(params);
			this._widget.add_effect(this._effect);
			// An effect on a surface that has never been shown has nothing
			// sampled yet.
			this._effect.queue_repaint();
		} catch (e) {
			console.error('[GlobalMenu] could not create blur effect:', e);
			this._effect = null;
		}
	}

	destroy() {
		for (const [target, signal, id] of this._ids) {
			try {
				target.disconnect(id);
			} catch (e) {
				// Already disposed along with the popup.
			}
		}
		this._ids = [];

		// Just destroy the surface. Removing the effect first only produces a
		// stream of "already disposed" warnings in the journal, because the
		// popup is normally being torn down at the same moment.
		this._widget?.destroy();
		this._widget = null;
		this._effect = null;
		this._menu = null;
		this._container = null;
		this._actor = null;
		this._parent = null;
	}
}

export function registerButton(btn) {
	buttons.add(btn);
	// Our own popups are handled by applyBlur/applyPopupBackground. Without this
	// marker the generic uiGroup hook would give them a second, unstyled blur.
	if (btn.menu)
		btn.menu._pulsarOwnMenu = true;
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
		this._watchUiGroup();
		this.applyAll();
	}

	/// Every popup ends up parented to `Main.uiGroup` when it opens, so watching
	/// that catches all of them: panel menus, quick settings, and the menus that
	/// extensions add later. Walking `Main.panel.statusArea` instead only finds
	/// the indicators that already existed at startup, which is why some popups
	/// were left unblurred.
	_watchUiGroup() {
		this._uiGroupId = Main.uiGroup.connect('child-added', (_group, actor) => {
			const menu = actor._delegate;
			if (!menu?.box || menu._pulsarOwnMenu || menu._pulsarMenuBlurHooked)
				return;
			menu._pulsarMenuBlurHooked = true;
			this._applyGenericBlur(menu);
		});
	}

	_applyGenericBlur(menu) {
		if (!this._settings || !this._settings.get_boolean('blur-popups'))
			return;

		// A blur is invisible behind an opaque background, and the shell themes
		// paint `.popup-menu-content` with a solid colour, so these menus need the
		// same translucent backdrop the global menu already applies.
		this._applyTranslucentBackdrop(menu);

		if (!menu.box)
			return;

		if (!menu._pulsarBlur) {
			menu._pulsarBlur = new PopupBlur(menu);
			menu.connect('destroy', () => {
				menu._pulsarBlur?.destroy();
				menu._pulsarBlur = null;
			});
		}

		const s = this._settings;
		menu._pulsarBlur.setEffect(
			s.get_int('popup-blur-sigma') || 28,
			s.get_int('popup-corner-radius') || 14,
			s.get_double('popup-blur-brightness') || 0.85);
	}

	/// The shell themes paint the popup background fully opaque, which hides the
	/// blur underneath. Overriding it inline is the only way to let it through.
	_applyTranslucentBackdrop(menu) {
		if (!menu.box || menu._pulsarTranslucentBackdrop)
			return;
		menu._pulsarTranslucentBackdrop = true;

		const isDark = this._ifaceSettings.get_string('color-scheme') === 'prefer-dark';
		const bg = isDark ? 'rgba(28, 28, 32, 0.62)' : 'rgba(246, 246, 248, 0.66)';
		menu.box.set_style(`background-color: ${bg};`);

		menu.connect('destroy', () => {
			menu._pulsarTranslucentBackdrop = false;
		});
	}

	applyAll() {
		for (const btn of buttons)
			this.applyToButton(btn);
		this._workspaceIndicator?.sync();
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
		const opacity = s.get_int('popup-opacity') || 78;
		const fontSize = s.get_int('menu-font-size');
		const colorScheme = this._ifaceSettings.get_string('color-scheme');
		const isDark = (colorScheme === 'prefer-dark');
		const blurEnabled = s.get_boolean('blur-popups');
		const hasLiquidGlass = !!(global.blur_my_shell || Main.extensionManager.lookup('blur-my-shell@aunetx') || Main.extensionManager.lookup('pulsaros-liquid-glass@inled.es'));

		let style = `border-radius: ${radius}px;`;
		if (fontSize > 0)
			style += ` font-size: ${fontSize}pt;`;

		if (hasLiquidGlass || blurEnabled) {
			// When Liquid Glass or Blur is active, use translucent glass styling so refraction shines through
			if (isDark) {
				style += ` background-color: rgba(28, 28, 32, 0.62);` +
					` border: 1px solid rgba(255, 255, 255, 0.14);` +
					` box-shadow: inset 0 1px 0 0 rgba(255, 255, 255, 0.20), 0 12px 32px rgba(0, 0, 0, 0.35);` +
					` color: #ffffff;`;
			} else {
				style += ` background-color: rgba(246, 246, 248, 0.66);` +
					` border: 1px solid rgba(255, 255, 255, 0.40);` +
					` box-shadow: inset 0 1px 0 0 rgba(255, 255, 255, 0.60), 0 12px 28px rgba(0, 0, 0, 0.12);` +
					` color: #1d1d1f;`;
			}
		} else {
			const alpha = (Math.min(opacity, 82) / 100).toFixed(2);
			if (isDark) {
				style += ` background-color: rgba(28, 32, 44, ${alpha});` +
					` border: 1px solid rgba(255, 255, 255, 0.18);` +
					` box-shadow: inset 0 1px 0 0 rgba(255, 255, 255, 0.28), 0 16px 44px rgba(0, 0, 0, 0.60), 0 2px 8px rgba(0, 0, 0, 0.35);` +
					` color: #ffffff;`;
			} else {
				style += ` background-color: rgba(235, 242, 252, ${alpha});` +
					` border: 1px solid rgba(255, 255, 255, 0.65);` +
					` box-shadow: inset 0 1px 0 0 rgba(255, 255, 255, 0.90), 0 16px 36px rgba(0, 0, 0, 0.15), 0 2px 6px rgba(0, 0, 0, 0.08);` +
					` color: #1d1d1f;`;
			}
		}

		btn.menu.box.set_style(style);
	}

	applyBlur(btn) {
		const s = this._settings;

		if (!s.get_boolean('blur-popups')) {
			this._removeBlur(btn);
			return;
		}

		const radius = s.get_int('popup-blur-sigma') || 28;
		const cornerRadius = s.get_int('popup-corner-radius') || 14;
		const brightness = s.get_double('popup-blur-brightness') || 0.85;

		// `menu.box` is the `.popup-menu-content` box: the blur group has to go
		// in there so it paints below the menu items but above the backdrop.
		if (!btn._globalMenuBlur) {
			if (!btn.menu.box)
				return;
			btn._globalMenuBlur = new PopupBlur(btn.menu);
			btn.menu.connect('destroy', () => this._removeBlur(btn));
		}

		btn._globalMenuBlur.setEffect(radius, cornerRadius, brightness);
	}

	destroy() {
		if (this._uiGroupId) {
			Main.uiGroup.disconnect(this._uiGroupId);
			this._uiGroupId = 0;
		}
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
		btn._globalMenuBlur.destroy();
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

