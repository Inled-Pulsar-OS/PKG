// Optimistic controls: a click is shown at once and held until the daemon reports the same value,
// or until it expires (the daemon ignored the verb). Pure: the clock is injected.

const TTL_MS = 4000;
const FIELDS = ['noiseMode', 'conversationalAwareness', 'oneBudAnc', 'adaptiveNoiseLevel', 'earDetection'];

const normalize = (field, value) =>
    (field === 'adaptiveNoiseLevel' ? Math.min(100, Math.max(0, Math.round(value))) : value);

export class Pending {
    constructor(now) {
        this._now = now;
        this._held = new Map(); // field -> {value, at}
    }

    /** Returns false for a field this class does not manage. */
    set(field, value) {
        if (!FIELDS.includes(field))
            return false;
        this._held.set(field, {value: normalize(field, value), at: this._now()});
        return true;
    }

    clear(field) {
        this._held.delete(field);
    }

    /** A copy of status with the held fields applied. Drops a hold the daemon agrees with or that expired. */
    overlay(status) {
        const shown = {...status};
        if (status.state !== 'ok')
            return shown;
        for (const [field, {value, at}] of [...this._held]) {
            if (status[field] === value || this._now() - at > TTL_MS)
                this._held.delete(field);
            else
                shown[field] = value;
        }
        return shown;
    }
}
