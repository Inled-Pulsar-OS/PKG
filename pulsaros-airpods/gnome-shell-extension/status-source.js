// Watches the librepods status file and reports its text. No polling: a Gio.FileMonitor wakes us.
// onText(text) gets the file content, or null when the file does not exist (the daemon is not running).
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

const DEBOUNCE_MS = 50; // a rewrite is several events (create, change, done); read once

export function defaultStatusPath() {
    return GLib.build_filenamev([GLib.get_user_state_dir(), 'librepods', 'status.json']);
}

export class StatusSource {
    constructor(path, onText) {
        this._file = Gio.File.new_for_path(path);
        this._onText = onText;
        this._monitor = null;
        this._watchedDir = null;
        this._timeoutId = 0;
        this._cancellable = null;
    }

    start() {
        if (this._cancellable)
            return;
        this._cancellable = new Gio.Cancellable();
        this._attach();
        this._read();
    }

    stop() {
        if (this._timeoutId) {
            GLib.source_remove(this._timeoutId);
            this._timeoutId = 0;
        }
        this._cancellable?.cancel();
        this._cancellable = null;
        this._detach();
    }

    // A monitor on the file itself misses its creation when the directory is not there yet (the
    // daemon creates ~/.local/state/librepods late). So watch the nearest directory that exists,
    // and move down to the real one when it appears.
    _nearestExistingDir() {
        let dir = this._file.get_parent();
        while (dir && !dir.query_exists(null))
            dir = dir.get_parent();
        return dir;
    }

    _attach() {
        const dir = this._nearestExistingDir();
        if (!dir)
            return;
        this._watchedDir = dir.get_path();
        this._monitor = dir.monitor_directory(Gio.FileMonitorFlags.NONE, null);
        this._monitor.connect('changed', () => {
            const nearest = this._nearestExistingDir();
            if (nearest && nearest.get_path() !== this._watchedDir) {
                this._detach();
                this._attach();
            }
            this._schedule();
        });
    }

    _detach() {
        this._monitor?.cancel();
        this._monitor = null;
        this._watchedDir = null;
    }

    _schedule() {
        if (this._timeoutId || !this._cancellable)
            return;
        this._timeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, DEBOUNCE_MS, () => {
            this._timeoutId = 0;
            this._read();
            return GLib.SOURCE_REMOVE;
        });
    }

    _read() {
        const cancellable = this._cancellable;
        if (!cancellable)
            return;
        this._file.load_contents_async(cancellable, (file, result) => {
            if (cancellable.is_cancelled())
                return;
            try {
                const [, bytes] = file.load_contents_finish(result);
                this._onText(new TextDecoder().decode(bytes));
            } catch (e) {
                if (e.matches(Gio.IOErrorEnum, Gio.IOErrorEnum.NOT_FOUND))
                    this._onText(null);
                else if (!e.matches(Gio.IOErrorEnum, Gio.IOErrorEnum.CANCELLED))
                    this._onText(''); // unreadable counts as invalid, never as "daemon stopped"
            }
        });
    }
}
