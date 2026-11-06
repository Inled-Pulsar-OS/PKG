// Low-battery alerts with macOS-like thresholds. Pure logic over parsed status objects (model.js).
//
// The daemon has its own low-battery latch, but it notifies through the omarchy CLI and has no listener
// when it runs headless on another desktop. Doing it here keeps the vendored daemon unmodified.

const THRESHOLDS = [5, 10, 20]; // percent, ascending

const bucket = level => THRESHOLDS.find(t => level <= t) ?? null;

export class BatteryAlerts {
    constructor() {
        this._notified = new Map(); // which -> lowest threshold already announced, or null
    }

    /** Feed every status. Returns [{which: 'left'|'right'|'case'|'headphones', level}] to announce now. */
    update(status) {
        if (status.state !== 'ok' || !status.connected) {
            this._notified.clear();
            return [];
        }

        const readings = status.isHeadset
            ? {headphones: [status.headsetLevel, status.left.charging]}
            : {
                left: [status.left.level, status.left.charging],
                right: [status.right.level, status.right.charging],
                case: [status.case.level, status.case.charging],
            };

        const alerts = [];
        for (const [which, [level, charging]] of Object.entries(readings)) {
            if (charging) {
                this._notified.set(which, null);
            } else if (level >= 0) {
                const now = bucket(level);
                const before = this._notified.get(which) ?? null;
                this._notified.set(which, now);
                if (now !== null && (before === null || now < before))
                    alerts.push({which, level});
            }
        }
        return alerts;
    }
}

// One "connected" notice per connection, with the model and every battery that has a reading.
export class ConnectNotice {
    constructor() {
        this._connected = false;
    }

    /** Feed every status. Returns {name, levels: [[which, level]]} on the transition to connected, else null. */
    update(status) {
        // A transient bad read ('invalid', 'newer-schema') must not re-arm the notice.
        if (status.state !== 'ok' && status.state !== 'daemon-down')
            return null;
        const connected = status.state === 'ok' && status.connected;
        const fresh = connected && !this._connected;
        this._connected = connected;
        if (!fresh)
            return null;

        const readings = status.isHeadset
            ? [['headphones', status.headsetLevel]]
            : [['left', status.left.level], ['right', status.right.level], ['case', status.case.level]];
        return {
            name: status.modelName || status.deviceName || '',
            levels: readings.filter(([, level]) => level >= 0),
        };
    }
}
