/**
 * Pulsar OS - Idle Guard (pulsaros-idle-guard@inled.es)
 *
 * Why
 * ---
 * Pulsar OS turns GNOME's inactivity auto-suspend on (see
 * org.gnome.settings-daemon.plugins.power / sleep-inactive-*-timeout). But
 * gsd-power only knows about user input and D-Bus idle inhibitors: it cannot
 * tell whether the *display* is changing. So watching a video, an animated
 * page, a remote desktop or a game with no keyboard/mouse input used to let
 * the machine suspend in the middle of the action.
 *
 * What
 * ----
 * This extension counts compositor frames via Clutter's `after-paint` signal on
 * the stage. On Wayland the stage is only repainted when something on screen
 * changes, so a static screen produces no paints while a video produces tens of
 * paints per second. While the frame rate stays above a small threshold, the
 * extension holds a gnome-session inhibitor (idle + suspend) that makes
 * gsd-power keep the session awake. When the screen goes static the inhibitor
 * is released and normal idle/suspend behaviour resumes.
 *
 * A blinking text caret (~1 paint/s) and the panel clock (~1 paint/min) stay
 * below the threshold on purpose, so reading a document does not keep the
 * machine powered forever. The threshold must also be cleared on consecutive
 * samples (MIN_ACTIVE_SAMPLES): a single one-second blip — a window opening, a
 * notification, the clock redrawing — must never hold the session awake on its
 * own for a whole grace period.
 *
 * The guard does nothing while the session is locked, so the lock screen can
 * still blank and suspend normally.
 */

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const BUS_NAME = 'org.gnome.SessionManager';
const OBJ_PATH = '/org/gnome/SessionManager';
const IFACE = 'org.gnome.SessionManager';

// gnome-session inhibitor flags: 4 = suspend, 8 = "session is idle".
// Holding both keeps the screen from blanking *and* the machine from sleeping.
const INHIBIT_SUSPEND = 4;
const INHIBIT_IDLE = 8;
const INHIBIT_FLAGS = INHIBIT_IDLE | INHIBIT_SUSPEND;

const APP_ID = 'pulsaros-idle-guard';
const REASON = 'Pulsar OS: the screen is actively changing';

const SAMPLE_MS = 1000; // evaluate the frame rate once per second
const WINDOW = 3;       // one-second samples kept in the sliding window
// How many of those samples must clear the threshold before the screen counts
// as "changing". One isolated burst (a window opening, a notification, the panel
// clock redrawing) is not enough; real video and animations clear it on
// consecutive samples. This keeps a single blip from holding the session awake
// for the whole grace period.
const MIN_ACTIVE_SAMPLES = 2;

export default class PulsarIdleGuardExtension extends Extension {
    enable() {
        this._settings = this.getSettings();
        this._frames = 0;
        this._history = [];
        this._lastActive = 0;
        this._held = false;
        this._cookie = 0;
        this._pending = false;
        this._generation = 0;
        this._cancellable = new Gio.Cancellable();

        // The stage is repainted only when something on screen changes.
        this._paintId = global.stage.connect('after-paint', () => {
            this._frames++;
        });

        this._sampleId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, SAMPLE_MS, () => {
            this._tick();
            return GLib.SOURCE_CONTINUE;
        });
    }

    disable() {
        this._generation++;
        if (this._sampleId) {
            GLib.source_remove(this._sampleId);
            this._sampleId = 0;
        }
        if (this._paintId) {
            global.stage.disconnect(this._paintId);
            this._paintId = 0;
        }
        if (this._cancellable) {
            this._cancellable.cancel();
            this._cancellable = null;
        }
        this._history = [];
        // Never leave an inhibitor behind, or the session could not sleep again.
        this._releaseInhibitor();
        this._settings = null;
    }

    _tick() {
        const frames = this._frames;
        this._frames = 0;

        if (!this._settings)
            return;

        const active = this._settings.get_boolean('active');
        const minFps = this._settings.get_int('minimum-fps');
        const grace = this._settings.get_int('grace-seconds');

        if (!active || this._isLocked()) {
            this._history = [];
            this._lastActive = 0;
            this._releaseInhibitor();
            return;
        }

        this._history.push(frames);
        if (this._history.length > WINDOW)
            this._history.shift();

        const avg = this._history.reduce((a, b) => a + b, 0) / this._history.length;
        // Sustain the motion: require MIN_ACTIVE_SAMPLES samples above the
        // threshold within the window, so a single blip cannot set _lastActive
        // (and with it, a whole grace period of inhibition).
        const activeSamples = this._history.filter(f => f >= minFps).length;
        const now = GLib.get_monotonic_time() / 1e6;
        if (activeSamples >= MIN_ACTIVE_SAMPLES)
            this._lastActive = now;

        const changing = this._lastActive > 0 && (now - this._lastActive) <= grace;
        if (changing) {
            if (this._settings.get_boolean('debug'))
                this._log(`frames/s=${frames} avg=${avg.toFixed(1)} active=${activeSamples}/${this._history.length} -> keep awake`);
            this._ensureInhibitor();
        } else {
            if (this._held && this._settings.get_boolean('debug'))
                this._log(`frames/s=${frames} avg=${avg.toFixed(1)} active=${activeSamples}/${this._history.length} -> allow idle`);
            this._releaseInhibitor();
        }
    }

    _isLocked() {
        try {
            if (Main.sessionMode?.isLocked)
                return true;
        } catch (e) { /* property may not exist */ }
        try {
            if (Main.screenShield?.active || Main.screenShield?.locked || Main.screenShield?._isLocked)
                return true;
        } catch (e) { /* shield may not exist */ }
        return false;
    }

    _ensureInhibitor() {
        if (this._held || this._pending)
            return;
        this._pending = true;
        const generation = this._generation;

        Gio.DBus.session.call(
            BUS_NAME, OBJ_PATH, IFACE, 'Inhibit',
            new GLib.Variant('(susu)', [APP_ID, 0, REASON, INHIBIT_FLAGS]),
            new GLib.VariantType('(u)'),
            Gio.DBusCallFlags.NONE, -1, this._cancellable,
            (conn, res) => {
                this._pending = false;
                let cookie;
                try {
                    [cookie] = conn.call_finish(res).deepUnpack();
                } catch (e) {
                    // Cancelled on disable, or gnome-session refused it.
                    return;
                }
                if (generation !== this._generation || !this._settings || this._isLocked()) {
                    // Disabled or locked while the call was in flight: release immediately.
                    this._uninhibit(cookie);
                    return;
                }
                this._held = true;
                this._cookie = cookie;
                this._log(`awake: idle + suspend inhibited (cookie ${cookie})`);
            });
    }

    _releaseInhibitor() {
        if (!this._held)
            return;
        const cookie = this._cookie;
        this._held = false;
        this._cookie = 0;
        this._uninhibit(cookie);
        this._log(`idle resumed: inhibitor released (cookie ${cookie})`);
    }

    _uninhibit(cookie) {
        try {
            Gio.DBus.session.call(
                BUS_NAME, OBJ_PATH, IFACE, 'Uninhibit',
                new GLib.Variant('(u)', [cookie]),
                null, Gio.DBusCallFlags.NONE, -1, null, null);
        } catch (e) { /* nothing we can do */ }
    }

    _log(msg) {
        console.log(`[pulsaros-idle-guard] ${msg}`);
    }
}
