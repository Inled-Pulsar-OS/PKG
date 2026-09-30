// Parses GTK-style accelerator strings ("<Ctrl><Shift>N") and sends them to
// the focused window through a Clutter virtual keyboard device. Keyvals, not
// raw evdev scancodes, so a shortcut lands on the right key whatever the
// keyboard layout is.
//
// parseAccelerator() is shared with shortcutStore.js, which also runs in the
// preferences process, and extensions must not import Clutter there. So the
// keyvals below are numeric literals (the same values as Clutter.KEY_*, which
// are X11 keysyms) and Clutter itself is imported lazily on the first
// synthesis, which only ever happens in the shell process.

import GLib from 'gi://GLib';

let debug = false;

export function setDebug(enabled) {
	debug = enabled;
}

function logDebug(msg) {
	if (debug)
		console.log(`[global-menu-keysynth] ${msg}`);
}

// Modifier token (lowercased, without <>) to left-modifier keyval. <Meta> is
// deliberately not accepted: it means different things on different setups.
const MODIFIER_KEYVALS = {
	'ctrl': 0xffe3,    // Control_L
	'control': 0xffe3,
	'primary': 0xffe3,
	'shift': 0xffe1,   // Shift_L
	'alt': 0xffe9,     // Alt_L
	'super': 0xffeb,   // Super_L
};

const NAMED_KEYVALS = {
	'f1': 0xffbe, 'f2': 0xffbf, 'f3': 0xffc0, 'f4': 0xffc1,
	'f5': 0xffc2, 'f6': 0xffc3, 'f7': 0xffc4, 'f8': 0xffc5,
	'f9': 0xffc6, 'f10': 0xffc7, 'f11': 0xffc8, 'f12': 0xffc9,
	'return': 0xff0d,
	'escape': 0xff1b,
	'tab': 0xff09,
	'backspace': 0xff08,
	'delete': 0xffff,
	'home': 0xff50,
	'end': 0xff57,
	'page_up': 0xff55,
	'page_down': 0xff56,
	'left': 0xff51,
	'up': 0xff52,
	'right': 0xff53,
	'down': 0xff54,
	'space': 0x020,
	'exclam': 0x021,
	'quotedbl': 0x022,
	'numbersign': 0x023,
	'dollar': 0x024,
	'percent': 0x025,
	'ampersand': 0x026,
	'apostrophe': 0x027,
	'parenleft': 0x028,
	'parenright': 0x029,
	'asterisk': 0x02a,
	'plus': 0x02b,
	'comma': 0x02c,
	'minus': 0x02d,
	'period': 0x02e,
	'slash': 0x02f,
	'colon': 0x03a,
	'semicolon': 0x03b,
	'less': 0x03c,
	'equal': 0x03d,
	'greater': 0x03e,
	'question': 0x03f,
	'at': 0x040,
	'bracketleft': 0x05b,
	'backslash': 0x05c,
	'bracketright': 0x05d,
	'asciicircum': 0x05e,
	'underscore': 0x05f,
	'grave': 0x060,
	'braceleft': 0x07b,
	'bar': 0x07c,
	'braceright': 0x07d,
	'asciitilde': 0x07e,
	'print': 0xff61,
	'insert': 0xff63,
	'menu': 0xff67,
	'kp_0': 0xffb0, 'kp_1': 0xffb1, 'kp_2': 0xffb2, 'kp_3': 0xffb3,
	'kp_4': 0xffb4, 'kp_5': 0xffb5, 'kp_6': 0xffb6, 'kp_7': 0xffb7,
	'kp_8': 0xffb8, 'kp_9': 0xffb9,
	'kp_decimal': 0xffae, 'kp_add': 0xffab, 'kp_subtract': 0xffad,
	'kp_multiply': 0xffaa, 'kp_divide': 0xffaf, 'kp_enter': 0xff8d,
};

// Chords land as one burst otherwise, and an app waiting on a prefix key
// cannot tell them apart.
const CHORD_GAP_US = 1000;

/**
 * "<Ctrl><Shift>N" to [{modifiers: [keyval, ...], keyval}], or null if the
 * string is not a valid accelerator.
 *
 * Space-separated chords are a sequence: "<Ctrl>x s" is Emacs' C-x s and
 * ": w Return" is Vim's :w, for apps whose menus live behind a prefix key
 * rather than a single accelerator. The usual one-chord accelerator is just a
 * one-element sequence.
 */
export function parseAccelerator(accel) {
	if (typeof accel !== 'string' || accel === '')
		return null;

	const chords = [];
	for (const part of accel.split(' ')) {
		const chord = parseChord(part);
		if (!chord)
			return null;
		chords.push(chord);
	}
	return chords;
}

function parseChord(accel) {
	if (accel === '')
		return null;

	const modifiers = [];
	let rest = accel;
	let match;
	while ((match = rest.match(/^<([^<>]+)>/))) {
		const keyval = MODIFIER_KEYVALS[match[1].toLowerCase()];
		if (keyval === undefined)
			return null;
		if (!modifiers.includes(keyval))
			modifiers.push(keyval);
		rest = rest.slice(match[0].length);
	}

	let keyval;
	if (rest.length === 1) {
		// Shift is expressed by the <Shift> modifier, never by an uppercase
		// keyval, so a printable character is always lowercased here
		const code = rest.toLowerCase().charCodeAt(0);
		if (code < 0x21 || code > 0x7e)
			return null;
		keyval = code;
	} else {
		keyval = NAMED_KEYVALS[rest.toLowerCase()];
		if (keyval === undefined)
			return null;
	}
	return {modifiers, keyval};
}

// Imported on first synthesis, shell process only. See the module comment.
let clutterPromise = null;

function getClutter() {
	clutterPromise ??= import('gi://Clutter').then(mod => mod.default);
	return clutterPromise;
}

/**
 * Parse and synthesize an accelerator. Returns false and logs a warning if
 * the accelerator is unparseable, true if synthesis was dispatched.
 */
export function sendShortcut(accel) {
	const chords = parseAccelerator(accel);
	if (!chords) {
		console.warn(`[global-menu-keysynth] Unparseable accelerator: ${accel}`);
		return false;
	}
	logDebug(`Synthesizing ${accel}`);
	getClutter()
		.then(Clutter => synthesize(Clutter, chords))
		.catch(e => console.error(`[global-menu-keysynth] Synthesis failed for "${accel}": ${e}`));
	return true;
}

function synthesize(Clutter, chords) {
	const seat = Clutter.get_default_backend().get_default_seat();
	const dev = seat.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
	let t = GLib.get_monotonic_time();
	for (const {modifiers, keyval} of chords) {
		for (const mod of modifiers)
			dev.notify_keyval(t += 10, mod, Clutter.KeyState.PRESSED);
		dev.notify_keyval(t += 10, keyval, Clutter.KeyState.PRESSED);
		dev.notify_keyval(t += 10, keyval, Clutter.KeyState.RELEASED);
		for (const mod of [...modifiers].reverse())
			dev.notify_keyval(t += 10, mod, Clutter.KeyState.RELEASED);
		t += CHORD_GAP_US;
	}
}
