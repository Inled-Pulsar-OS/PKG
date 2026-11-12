// Run with: gjs -m tests/model_check.js
// Specification of gnome-shell-extension/model.js, the librepods status.json parser.
// Exit 1 with the list of failures. Expected values are written out by hand from
// the daemon's schema (omarchy-pods knowledge/librepods-status-schema.md), not computed.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import {availableModes, controlVerb, parseStatus} from '../gnome-shell-extension/model.js';

const failures = [];
const same = (what, got, want) => {
    const g = JSON.stringify(got), w = JSON.stringify(want);
    if (g !== w)
        failures.push(`${what}\n    got  ${g}\n    want ${w}`);
};

const line = o => JSON.stringify(o);
const pod = (level, charging = false, in_ear = false) => ({available: true, level, charging, in_ear});

const pro3 = {
    schema_version: 1, connected: true, device_name: "Victor's AirPods Pro", model_name: 'AirPods Pro 3',
    noise_mode: 3, left: pod(80, false, true), right: pod(70, false, true),
    case: {available: true, level: 50, charging: true},
    conversational_awareness: true, adaptive_noise_level: 40, one_bud_anc_mode: false,
    is_pro_series: true, supports_noise_control: true, supports_noise_off: false, supports_adaptive: true,
    supports_conversational_awareness: true, supports_one_bud_anc: true,
    ear_detection_behavior: 0, lid_state: 0,
};

// -- state of the file ------------------------------------------------------------------------
same('missing file means the daemon is down', parseStatus(null).state, 'daemon-down');
same('empty file is invalid', parseStatus('').state, 'invalid');
same('not JSON is invalid', parseStatus('{oops').state, 'invalid');
same('JSON that is not an object is invalid', parseStatus('[1,2]').state, 'invalid');
same('newer schema is flagged', parseStatus(line({schema_version: 2, connected: true})).state, 'newer-schema');
same('missing schema_version reads as 1', parseStatus(line({connected: true})).state, 'ok');
same('good line', parseStatus(line(pro3)).state, 'ok');

// -- a fresh daemon: left, right, case absent --------------------------------------------------
const fresh = parseStatus(line({schema_version: 1, connected: false}));
same('fresh: pods unavailable', [fresh.left.available, fresh.right.available, fresh.case.available], [false, false, false]);
same('fresh: levels are -1, never 0', [fresh.left.level, fresh.right.level, fresh.case.level], [-1, -1, -1]);
same('fresh: noise mode unknown', fresh.noiseMode, -1);
same('fresh: not connected', fresh.connected, false);
same('fresh: names are empty strings', [fresh.deviceName, fresh.modelName], ['', '']);

// -- Pro 3 -------------------------------------------------------------------------------------
const p = parseStatus(line(pro3));
same('pro3 identity', [p.deviceName, p.modelName, p.connected], ["Victor's AirPods Pro", 'AirPods Pro 3', true]);
same('pro3 left', p.left, {available: true, level: 80, charging: false, inEar: true});
same('pro3 case', p.case, {available: true, level: 50, charging: true});
same('pro3 mode and levels', [p.noiseMode, p.adaptiveNoiseLevel, p.conversationalAwareness, p.oneBudAnc], [3, 40, true, false]);
same('pro3 capabilities', [p.supportsNoiseControl, p.supportsNoiseOff, p.supportsAdaptive, p.supportsConversationalAwareness, p.supportsOneBudAnc],
    [true, false, true, true, true]);
same('pro3 has no Off', availableModes(p).map(m => m.verb), ['noise:anc', 'noise:transparency', 'noise:adaptive']);
same('pro3 ear detection and lid', [p.earDetection, p.lidState], [0, 0]);

// -- a pod not heard from, odd values ------------------------------------------------------------
const stale = parseStatus(line({...pro3, right: {available: false}}));
same('pod not heard from', stale.right, {available: false, level: -1, charging: false, inEar: false});
same('level clamps to 0..100', parseStatus(line({...pro3, left: pod(250)})).left.level, 100);
same('negative level is clamped to 0', parseStatus(line({...pro3, left: pod(-5)})).left.level, 0);
same('level that is not a number is unavailable', parseStatus(line({...pro3, left: {available: true, level: 'x'}})).left.available, false);
same('adaptive level is clamped', parseStatus(line({...pro3, adaptive_noise_level: 900})).adaptiveNoiseLevel, 100);
same('unknown noise mode value is -1', parseStatus(line({...pro3, noise_mode: 9})).noiseMode, -1);

// -- capability keys are additive: absence is not false -----------------------------------------
const old = {schema_version: 1, connected: true, noise_mode: 1, is_pro_series: true};
const o = parseStatus(line(old));
same('old daemon, Pro: adaptive/CA/one-bud follow is_pro_series',
    [o.supportsAdaptive, o.supportsConversationalAwareness, o.supportsOneBudAnc], [true, true, true]);
same('old daemon: noise control and Off default to true', [o.supportsNoiseControl, o.supportsNoiseOff], [true, true]);
const oldPlain = parseStatus(line({schema_version: 1, connected: true, is_pro_series: false}));
same('old daemon, not Pro: no adaptive', [oldPlain.supportsAdaptive, oldPlain.supportsConversationalAwareness, oldPlain.supportsOneBudAnc], [false, false, false]);
same('explicit false beats the Pro fallback', parseStatus(line({...old, supports_adaptive: false})).supportsAdaptive, false);
same('explicit true beats the non-Pro fallback', parseStatus(line({schema_version: 1, is_pro_series: false, supports_adaptive: true})).supportsAdaptive, true);

// -- models --------------------------------------------------------------------------------------
const airpods4anc = parseStatus(line({...pro3, model_name: 'AirPods 4', is_pro_series: false, supports_noise_off: true}));
same('AirPods 4 ANC gets all four modes', availableModes(airpods4anc).map(m => m.verb),
    ['noise:off', 'noise:anc', 'noise:transparency', 'noise:adaptive']);
const airpods3 = parseStatus(line({...pro3, model_name: 'AirPods 3', supports_noise_control: false, supports_adaptive: false,
    supports_conversational_awareness: false, supports_one_bud_anc: false}));
same('AirPods 1-3 and plain 4 have no modes', availableModes(airpods3), []);
const pro2 = parseStatus(line({...pro3, model_name: 'AirPods Pro 2', supports_noise_off: true}));
same('Pro 2 labels', availableModes(pro2).map(m => m.id), [0, 1, 2, 3]);
const noAdaptive = parseStatus(line({...pro3, model_name: 'AirPods Pro', supports_adaptive: false, supports_noise_off: true}));
same('Pro 1 has no adaptive', availableModes(noAdaptive).map(m => m.verb), ['noise:off', 'noise:anc', 'noise:transparency']);

// Max: one battery, no case
const max = parseStatus(line({schema_version: 1, connected: true, model_name: 'AirPods Max 2', is_headset: true,
    left: pod(64), supports_one_bud_anc: false, noise_mode: 1}));
same('Max is a headset', max.isHeadset, true);
same('Max single battery comes from whichever side reports', max.headsetLevel, 64);
same('Max has no case', max.case.available, false);
same('non-headsets have no headset level', p.headsetLevel, -1);

// -- control verbs: a whitelist, nothing user-typed reaches the command line -------------------
same('mode verbs', [controlVerb('noise', 0), controlVerb('noise', 1), controlVerb('noise', 2), controlVerb('noise', 3)],
    ['noise:off', 'noise:anc', 'noise:transparency', 'noise:adaptive']);
same('conversation awareness', [controlVerb('ca', true), controlVerb('ca', false)], ['ca:on', 'ca:off']);
same('one-bud anc', [controlVerb('onebud', true), controlVerb('onebud', false)], ['onebud:on', 'onebud:off']);
same('ear detection', [controlVerb('ear', 0), controlVerb('ear', 1), controlVerb('ear', 2)], ['ear:one', 'ear:both', 'ear:off']);
same('adaptive level', [controlVerb('adaptive', 0), controlVerb('adaptive', 55), controlVerb('adaptive', 100)], ['adaptive:0', 'adaptive:55', 'adaptive:100']);
same('adaptive level is clamped and rounded', [controlVerb('adaptive', 140), controlVerb('adaptive', -3), controlVerb('adaptive', 41.6)], ['adaptive:100', 'adaptive:0', 'adaptive:42']);
same('unknown kind is refused', controlVerb('rm', 1), null);
same('unknown mode is refused', controlVerb('noise', 7), null);
same('string payload is refused', controlVerb('noise', '1; reboot'), null);
same('non-boolean for a switch is refused', controlVerb('ca', 'on'), null);

// -- fixtures shipped for the extension and docs ----------------------------------------------------
const here = GLib.path_get_dirname(Gio.File.new_for_uri(import.meta.url).get_path());
for (const name of ['airpods4-anc', 'pro3', 'max2', 'in-case']) {
    const path = GLib.build_filenamev([here, 'fixtures', `${name}.json`]);
    try {
        const [ok, bytes] = GLib.file_get_contents(path);
        const s = parseStatus(new TextDecoder().decode(bytes));
        same(`fixture ${name} parses`, ok && s.state, 'ok');
    } catch (e) {
        failures.push(`fixture ${name}: ${e.message}`);
    }
}

if (failures.length > 0) {
    print(failures.join('\n'));
    imports.system.exit(1);
}
print('ok');
