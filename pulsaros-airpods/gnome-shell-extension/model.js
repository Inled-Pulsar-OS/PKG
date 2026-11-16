// Parser for the librepods daemon's status.json, and the control verbs it accepts.
// Pure module: no Shell, GLib or Gio imports, so it runs under gjs and node.
//
// Schema notes (see the daemon's UPSTREAM.md and docs): the daemon omits left, right and case until a
// battery packet arrives; capability keys (supports_*) are additive, so an absent key is not false.

export const SCHEMA_VERSION = 1;

// noise_mode values as the daemon reports them, and the verb that sets each one.
const MODES = [
    {id: 0, verb: 'noise:off'},
    {id: 1, verb: 'noise:anc'},
    {id: 2, verb: 'noise:transparency'},
    {id: 3, verb: 'noise:adaptive'},
];

const EAR_VERBS = ['ear:one', 'ear:both', 'ear:off']; // pause when one is out, both are out, never

const clampLevel = n => Math.min(100, Math.max(0, Math.round(n)));
const isNumber = v => typeof v === 'number' && Number.isFinite(v);

function emptyStatus(state) {
    const none = () => ({available: false, level: -1, charging: false, inEar: false});
    return {
        state,
        connected: false,
        deviceName: '',
        modelName: '',
        noiseMode: -1,
        left: none(),
        right: none(),
        case: {available: false, level: -1, charging: false},
        headsetLevel: -1,
        isHeadset: false,
        isPro: false,
        conversationalAwareness: false,
        adaptiveNoiseLevel: 0,
        oneBudAnc: false,
        supportsNoiseControl: true,
        supportsNoiseOff: true,
        supportsAdaptive: false,
        supportsConversationalAwareness: false,
        supportsOneBudAnc: false,
        earDetection: 0,
        lidState: 2,
    };
}

function readPod(raw) {
    const pod = {available: false, level: -1, charging: false, inEar: false};
    if (!raw || typeof raw !== 'object' || raw.available !== true || !isNumber(raw.level))
        return pod;
    return {available: true, level: clampLevel(raw.level), charging: raw.charging === true, inEar: raw.in_ear === true};
}

// An absent capability key falls back; an explicit boolean always wins.
const capability = (raw, key, fallback) => (typeof raw[key] === 'boolean' ? raw[key] : fallback);

/**
 * text: the file's content, or null when the file does not exist.
 * Returns a complete status object on every path. status.state is one of
 * 'ok', 'daemon-down' (no file), 'invalid' (empty or unreadable) and 'newer-schema'.
 */
export function parseStatus(text) {
    if (text === null || text === undefined)
        return emptyStatus('daemon-down');

    let raw;
    try {
        raw = JSON.parse(text);
    } catch (_e) {
        return emptyStatus('invalid');
    }
    if (!raw || typeof raw !== 'object' || Array.isArray(raw))
        return emptyStatus('invalid');

    const version = isNumber(raw.schema_version) ? raw.schema_version : 1;
    if (version > SCHEMA_VERSION)
        return emptyStatus('newer-schema');

    const status = emptyStatus('ok');
    const pro = raw.is_pro_series === true;

    status.connected = raw.connected === true;
    status.deviceName = typeof raw.device_name === 'string' ? raw.device_name : '';
    status.modelName = typeof raw.model_name === 'string' ? raw.model_name : '';
    status.noiseMode = MODES.some(m => m.id === raw.noise_mode) ? raw.noise_mode : -1;
    status.left = readPod(raw.left);
    status.right = readPod(raw.right);

    const box = raw.case && typeof raw.case === 'object' ? raw.case : null;
    if (box && box.available === true && isNumber(box.level))
        status.case = {available: true, level: clampLevel(box.level), charging: box.charging === true};

    status.isHeadset = raw.is_headset === true;
    if (status.isHeadset) {
        const side = status.left.available ? status.left : status.right;
        status.headsetLevel = side.available ? side.level : -1;
        status.case = {available: false, level: -1, charging: false};
    }

    status.isPro = pro;
    status.conversationalAwareness = raw.conversational_awareness === true;
    status.oneBudAnc = raw.one_bud_anc_mode === true;
    status.adaptiveNoiseLevel = isNumber(raw.adaptive_noise_level) ? clampLevel(raw.adaptive_noise_level) : 0;

    status.supportsNoiseControl = capability(raw, 'supports_noise_control', true);
    status.supportsNoiseOff = capability(raw, 'supports_noise_off', true);
    status.supportsAdaptive = capability(raw, 'supports_adaptive', pro);
    status.supportsConversationalAwareness = capability(raw, 'supports_conversational_awareness', pro);
    status.supportsOneBudAnc = capability(raw, 'supports_one_bud_anc', pro);

    status.earDetection = [0, 1, 2].includes(raw.ear_detection_behavior) ? raw.ear_detection_behavior : 0;
    status.lidState = [0, 1, 2].includes(raw.lid_state) ? raw.lid_state : 2;
    return status;
}

/** The listening modes this device really has, in display order: [{id, verb}]. */
export function availableModes(status) {
    if (!status.supportsNoiseControl)
        return [];
    return MODES.filter(m => {
        if (m.id === 0)
            return status.supportsNoiseOff;
        if (m.id === 3)
            return status.supportsAdaptive;
        return true;
    }).map(m => ({...m}));
}

/**
 * The only way a command line is built. kind is one of noise, ca, onebud, ear, adaptive;
 * value is validated per kind. Returns null for anything else, never a partial verb.
 */
export function controlVerb(kind, value) {
    switch (kind) {
    case 'noise':
        return Number.isInteger(value) && value >= 0 && value < MODES.length ? MODES[value].verb : null;
    case 'ca':
        return typeof value === 'boolean' ? (value ? 'ca:on' : 'ca:off') : null;
    case 'onebud':
        return typeof value === 'boolean' ? (value ? 'onebud:on' : 'onebud:off') : null;
    case 'ear':
        return Number.isInteger(value) && value >= 0 && value < EAR_VERBS.length ? EAR_VERBS[value] : null;
    case 'adaptive':
        return isNumber(value) ? `adaptive:${clampLevel(value)}` : null;
    default:
        return null;
    }
}
