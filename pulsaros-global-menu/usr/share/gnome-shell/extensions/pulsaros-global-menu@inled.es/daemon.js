// Lifecycle for global-menu, the companion process that reads an app's
// exported menu off the bus and hands it back as a JSON tree.
//
// The daemon ships separately from the extension, so this only ever launches
// a program already on PATH and bundles nothing. Distributions that install
// the D-Bus activation file get it started on demand instead, in which case
// the name is already owned and nothing is spawned here.

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

const BUS_NAME = 'space.unmade.GlobalMenu';
const PROGRAM = 'global-menu';

// A daemon that takes the name and then dies is a crash loop. One that never
// takes it stops on its own, because the watcher fires only once.
const MAX_RESTARTS = 3;

export class DaemonManager {
	constructor() {
		this._proc = null;
		this._cancellable = null;
		this._nameWatchId = 0;
		this._restarts = 0;
		this._warned = false;
	}

	start() {
		if (this._nameWatchId)
			return;

		this._cancellable = new Gio.Cancellable();
		// Fires vanished() straight away when nobody owns the name, which
		// covers both the initial start and crash recovery
		this._nameWatchId = Gio.bus_watch_name(
			Gio.BusType.SESSION,
			BUS_NAME,
			Gio.BusNameWatcherFlags.NONE,
			() => {
				this._restarts = 0;
			},
			() => this._spawn()
		);
	}

	destroy() {
		if (this._nameWatchId) {
			Gio.bus_unwatch_name(this._nameWatchId);
			this._nameWatchId = 0;
		}
		this._cancellable?.cancel();
		this._cancellable = null;

		// SIGTERM rather than force_exit, because the daemon traps it to take
		// its own children down with it. Only ever our own process: a daemon
		// the user started themselves is left alone.
		if (this._proc) {
			try {
				this._proc.send_signal(15);
			} catch (e) {
				console.warn(`[global-menu] could not stop ${PROGRAM}: ${e.message}`);
			}
			this._proc = null;
		}
	}

	_spawn() {
		if (!this._cancellable || this._cancellable.is_cancelled())
			return;
		// One in flight is enough; wait_async clears it when it exits
		if (this._proc)
			return;

		if (this._restarts >= MAX_RESTARTS) {
			this._warnOnce(`${PROGRAM} keeps exiting; giving up. ` +
				`Run it in a terminal to see why.`);
			return;
		}

		const path = GLib.find_program_in_path(PROGRAM);
		if (!path) {
			this._warnOnce(`${PROGRAM} is not on PATH. Menus exported by ` +
				`applications need it; install the global-menu package.`);
			return;
		}

		this._restarts++;
		try {
			this._proc = Gio.Subprocess.new([path], Gio.SubprocessFlags.NONE);
		} catch (e) {
			console.warn(`[global-menu] could not start ${PROGRAM}: ${e.message}`);
			return;
		}

		const proc = this._proc;
		proc.wait_async(this._cancellable, (p, res) => {
			try {
				p.wait_finish(res);
			} catch (e) {
				// Cancelled on disable
			}
			if (this._proc === proc)
				this._proc = null;
		});
	}

	_warnOnce(message) {
		if (this._warned)
			return;
		this._warned = true;
		console.warn(`[global-menu] ${message}`);
	}
}
