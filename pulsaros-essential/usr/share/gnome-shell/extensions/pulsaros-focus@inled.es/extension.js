/**
 * Pulsar OS - Focus Reporter (pulsaros-focus@inled.es)
 *
 * Writes the PID (and app identity) of the window that currently has focus to
 * $XDG_RUNTIME_DIR/pulsaros-focused.json using an atomic replace, and removes
 * the file when the extension is disabled (including screen lock, so the
 * optimizer never boosts a stale app while the session is locked).
 *
 * Why an extension?
 *  - Since GNOME 45 `org.gnome.Shell.Eval` returns (false, '') unless the
 *    shell runs in unsafe mode (the key no longer even exists), so a daemon
 *    cannot ask the shell for the focused window any more.
 *  - pulsaros-optimizer is a root *system* service: it has no session bus and
 *    no display, so gdbus/xdotool/xprop never worked from there either.
 *  - X11-only fallbacks miss native Wayland apps, which is where Pulsar OS runs.
 *
 * Cost: one tiny file write per focus change. Compatible with GNOME 45-50 (ESM).
 */

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';

const STATE_FILENAME = 'pulsaros-focused.json';

export default class PulsarosFocusExtension extends Extension {
    enable() {
        this._path = GLib.build_filenamev([GLib.get_user_runtime_dir(), STATE_FILENAME]);
        this._file = Gio.File.new_for_path(this._path);
        this._signalId = global.display.connect('focus-window', () => this._report());
        this._report();
        // Heartbeat: pulsaros-optimizer discards reports older than 60s, and
        // focus does not change when the user stays in the same window, so the
        // report must be refreshed even when nothing happens.
        this._timeoutId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 15, () => {
            this._report();
            return GLib.SOURCE_CONTINUE;
        });
    }

    disable() {
        if (this._timeoutId) {
            GLib.source_remove(this._timeoutId);
            this._timeoutId = 0;
        }
        if (this._signalId) {
            global.display.disconnect(this._signalId);
            this._signalId = 0;
        }
        this._file = null;

        // Do not leave a stale focus report behind (also runs on screen lock).
        try {
            const file = Gio.File.new_for_path(this._path);
            if (file.query_exists(null))
                file.delete(null);
        } catch (e) {
            // Never break the shell over a telemetry file.
        }
        this._path = null;
    }

    _report() {
        try {
            const win = global.display.focus_window;
            let pid = 0;
            let wmClass = '';
            let appId = '';

            if (win) {
                pid = win.get_pid() || 0;
                wmClass = win.get_wm_class() ?? '';
                if (typeof win.get_gtk_application_id === 'function')
                    appId = win.get_gtk_application_id() ?? '';
                if (!appId && typeof win.get_sandboxed_app_id === 'function')
                    appId = win.get_sandboxed_app_id() ?? '';
            }

            const payload = `${JSON.stringify({
                pid,
                wm_class: wmClass,
                app_id: appId,
                ts: GLib.get_real_time() / 1e6,
            })}\n`;

            // replace_contents() writes through a temp file + rename: the
            // daemon can never read a half-written JSON.
            this._file.replace_contents(
                new TextEncoder().encode(payload),
                null,
                false,
                Gio.FileCreateFlags.REPLACE_DESTINATION,
                null);
        } catch (e) {
            // Ignore: a failed report only costs one optimization cycle.
        }
    }
}
