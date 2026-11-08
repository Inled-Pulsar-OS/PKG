/* Pulsar OS - menu bar status icons: thin entry point.
 *
 * GNOME Shell caches this file for the whole session, so everything lives in
 * bar.js, imported with a cache-busting query on every enable: editing bar.js
 * only needs a disable/enable, never a logout.
 */
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

export default class PulsarBarIconsExtension extends Extension {
    async enable() {
        let mod = null;
        try {
            const uri = Gio.File.new_for_path(
                GLib.build_filenamev([this.path, 'bar.js'])).get_uri();
            mod = await import(`${uri}?v=${Date.now()}`);
        } catch (e) {
            console.error(`pulsar-bar: bar.js could not be loaded: ${e}`);
            return;
        }

        try {
            this._bar = mod.crea(this);
            this._bar.enable();
        } catch (e) {
            console.error(`pulsar-bar: enable() failed: ${e}`);
            this._bar = null;
        }
    }

    disable() {
        try {
            this._bar?.disable();
        } catch (e) {
            console.error(`pulsar-bar: disable() failed: ${e}`);
        }
        this._bar = null;
    }
}
