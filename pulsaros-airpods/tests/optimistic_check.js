// Run with: gjs -m tests/optimistic_check.js
// Specification of gnome-shell-extension/optimistic.js: a click moves the control at once, and an
// incoming read cannot overwrite that field until the daemon agrees (or the click expires).
import {Pending} from '../gnome-shell-extension/optimistic.js';
import {parseStatus} from '../gnome-shell-extension/model.js';

const failures = [];
const same = (what, got, want) => {
    const g = JSON.stringify(got), w = JSON.stringify(want);
    if (g !== w)
        failures.push(`${what}\n    got  ${g}\n    want ${w}`);
};

const status = (over = {}) => parseStatus(JSON.stringify({
    schema_version: 1, connected: true, model_name: 'AirPods Pro 3', is_pro_series: true,
    noise_mode: 1, conversational_awareness: false, one_bud_anc_mode: false, adaptive_noise_level: 50,
    ear_detection_behavior: 0, ...over,
}));

let clock = 1000;
const make = () => new Pending(() => clock);

// nothing pending: the status passes through
{
    const p = make(), s = status();
    same('passthrough', [p.overlay(s).noiseMode, p.overlay(s).adaptiveNoiseLevel], [1, 50]);
}

// a click shows at once and survives a stale read
{
    const p = make();
    p.set('noiseMode', 2);
    same('click shows at once', p.overlay(status({noise_mode: 1})).noiseMode, 2);
    same('stale read cannot overwrite', p.overlay(status({noise_mode: 1})).noiseMode, 2);
    same('daemon agrees', p.overlay(status({noise_mode: 2})).noiseMode, 2);
    same('after agreement a stem change wins', p.overlay(status({noise_mode: 3})).noiseMode, 3);
}

// expiry: a daemon that ignored the verb gets the control back
{
    const p = make();
    clock = 1000;
    p.set('noiseMode', 0);
    clock = 1000 + 3999;
    same('still pending before the ttl', p.overlay(status({noise_mode: 1})).noiseMode, 0);
    clock = 1000 + 4001;
    same('daemon value wins after the ttl', p.overlay(status({noise_mode: 1})).noiseMode, 1);
}

// fields are independent
{
    const p = make();
    clock = 5000;
    p.set('conversationalAwareness', true);
    const o = p.overlay(status({noise_mode: 2, conversational_awareness: false}));
    same('only the clicked field is held', [o.conversationalAwareness, o.noiseMode], [true, 2]);
    p.set('oneBudAnc', true);
    p.set('earDetection', 2);
    const o2 = p.overlay(status());
    same('three pending fields', [o2.conversationalAwareness, o2.oneBudAnc, o2.earDetection], [true, true, 2]);
}

// adaptive level: clamped and rounded like the verb
{
    const p = make();
    clock = 9000;
    p.set('adaptiveNoiseLevel', 140);
    same('clamped', p.overlay(status({adaptive_noise_level: 50})).adaptiveNoiseLevel, 100);
    const q = make();
    q.set('adaptiveNoiseLevel', 41.6);
    same('rounded', q.overlay(status({adaptive_noise_level: 50})).adaptiveNoiseLevel, 42);
    same('agreement at the rounded value clears it', q.overlay(status({adaptive_noise_level: 42})).adaptiveNoiseLevel, 42);
    same('and then the daemon rules', q.overlay(status({adaptive_noise_level: 10})).adaptiveNoiseLevel, 10);
}

// clear() snaps one field back (the verb failed)
{
    const p = make();
    clock = 12000;
    p.set('noiseMode', 3);
    p.clear('noiseMode');
    same('clear', p.overlay(status({noise_mode: 1})).noiseMode, 1);
}

// unknown fields are refused, input is never mutated, non-ok states pass through
{
    const p = make();
    same('unknown field refused', p.set('state', 'x'), false);
    same('known field accepted', p.set('noiseMode', 1), true);
    const s = status({noise_mode: 0});
    p.set('noiseMode', 2);
    p.overlay(s);
    same('input not mutated', s.noiseMode, 0);
    const down = parseStatus(null);
    same('daemon-down passes through untouched', p.overlay(down).state, 'daemon-down');
    same('and keeps its unknown mode', p.overlay(down).noiseMode, -1);
}

if (failures.length > 0) {
    print(failures.join('\n'));
    imports.system.exit(1);
}
print('ok');
