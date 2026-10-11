import GLib from 'gi://GLib';

class PointerWatch {
    constructor(watcher, interval, callback) {
        this.watcher = watcher;
        this.interval = interval;
        this.callback = callback;
        this.paused = false;
    }

    remove() {
        this.watcher._removeWatch(this);
    }
}

class PointerWatcher {
    constructor() {
        this._watches = [];
        this._idleId = 0;
        this._lastX = -1;
        this._lastY = -1;
    }

    addWatch(interval, callback) {
        const watch = new PointerWatch(this, interval, callback);
        this._watches.push(watch);
        this._updateTimer();
        return watch;
    }

    _removeWatch(watch) {
        const idx = this._watches.indexOf(watch);
        if (idx !== -1) {
            this._watches.splice(idx, 1);
            this._updateTimer();
        }
    }

    _updateTimer() {
        if (this._watches.length > 0 && !this._idleId) {
            this._idleId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 40, () => {
                try {
                    const [x, y] = global.get_pointer();
                    if (x !== this._lastX || y !== this._lastY) {
                        this._lastX = x;
                        this._lastY = y;
                        for (const watch of this._watches) {
                            if (!watch.paused) {
                                try {
                                    watch.callback(x, y);
                                } catch (e) {
                                    console.error('[PointerWatcher] callback error:', e);
                                }
                            }
                        }
                    }
                } catch (e) {}
                return GLib.SOURCE_CONTINUE;
            });
        } else if (this._watches.length === 0 && this._idleId) {
            GLib.source_remove(this._idleId);
            this._idleId = 0;
        }
    }
}

let _pointerWatcherInstance = null;
export function getPointerWatcher() {
    if (!_pointerWatcherInstance)
        _pointerWatcherInstance = new PointerWatcher();
    return _pointerWatcherInstance;
}
