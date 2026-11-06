// Run with: gjs -m tests/alerts_check.js
// Specification of gnome-shell-extension/alerts.js: macOS-like low-battery thresholds (20 / 10 / 5 percent)
// over parsed status objects. Ported from the v1 Python tests, which pinned the same rules.
import {BatteryAlerts, ConnectNotice} from '../gnome-shell-extension/alerts.js';
import {parseStatus} from '../gnome-shell-extension/model.js';

const failures = [];
const same = (what, got, want) => {
    const g = JSON.stringify(got), w = JSON.stringify(want);
    if (g !== w)
        failures.push(`${what}\n    got  ${g}\n    want ${w}`);
};

const pod = (level, charging = false) => ({available: true, level, charging, in_ear: false});
const snap = (over = {}) => parseStatus(JSON.stringify({schema_version: 1, connected: true, ...over}));
const lr = (left, right, rest = {}) => snap({left: pod(left), right: pod(right), ...rest});

const fresh = () => new BatteryAlerts();

{ // no alert above 20
    const a = fresh();
    same('above 20', a.update(lr(30, 90, {case: {available: true, level: 100}})), []);
}
{ // crossing 20 alerts once, for that bud only
    const a = fresh();
    a.update(lr(30, 70));
    same('crossing 20', a.update(lr(20, 70)), [{which: 'left', level: 20}]);
    same('no repeat', a.update(lr(20, 70)), []);
    same('still none', a.update(lr(20, 70)), []);
}
{ // each lower threshold alerts once, and the real level is reported
    const a = fresh();
    a.update(snap({right: pod(30)}));
    same('20', a.update(snap({right: pod(20)})), [{which: 'right', level: 20}]);
    same('10', a.update(snap({right: pod(10)})), [{which: 'right', level: 10}]);
    same('10 again', a.update(snap({right: pod(10)})), []);
    same('0 reports 0', a.update(snap({right: pod(0)})), [{which: 'right', level: 0}]);
}
{ // a jump across thresholds alerts only the lowest
    const a = fresh();
    a.update(snap({left: pod(50)}));
    same('jump', a.update(snap({left: pod(10)})), [{which: 'left', level: 10}]);
}
{ // the case is independent
    const a = fresh();
    a.update(lr(60, 60, {case: {available: true, level: 30, charging: false}}));
    same('case', a.update(lr(60, 60, {case: {available: true, level: 20, charging: false}})), [{which: 'case', level: 20}]);
}
{ // charging never alerts, and re-arms
    const a = fresh();
    same('charging pod', a.update(snap({left: pod(10, true)})), []);
    same('charging case', a.update(snap({case: {available: true, level: 10, charging: true}})), []);
    const b = fresh();
    b.update(snap({left: pod(10)}));
    b.update(snap({left: pod(60, true)}));
    same('drained again after charging', b.update(snap({left: pod(20)})), [{which: 'left', level: 20}]);
}
{ // rising above a threshold without the charging flag re-arms
    const a = fresh();
    same('first', a.update(snap({left: pod(20)})), [{which: 'left', level: 20}]);
    a.update(snap({left: pod(60)}));
    same('second', a.update(snap({left: pod(20)})), [{which: 'left', level: 20}]);
}
{ // an unavailable level is ignored and does not re-arm
    const a = fresh();
    a.update(snap({left: pod(20)}));
    same('unavailable', a.update(snap({left: {available: false}})), []);
    same('back at 20, no repeat', a.update(snap({left: pod(20)})), []);
}
{ // disconnect re-arms; a daemon that is down or unreadable does too
    const a = fresh();
    a.update(snap({left: pod(20)}));
    a.update(snap({connected: false, left: pod(20)}));
    same('reconnect', a.update(snap({left: pod(20)})), [{which: 'left', level: 20}]);
    a.update(parseStatus(null));
    same('after daemon restart', a.update(snap({left: pod(20)})), [{which: 'left', level: 20}]);
}
{ // headset: one battery, reported as headphones
    const a = fresh();
    const max = lvl => snap({is_headset: true, model_name: 'AirPods Max 2', left: pod(lvl)});
    same('headset', a.update(max(20)), [{which: 'headphones', level: 20}]);
    same('headset no repeat', a.update(max(20)), []);
    same('headset never reports pods or case', a.update(max(10)).map(x => x.which), ['headphones']);
}
{ // first reading below the threshold alerts (connecting with a low battery)
    same('connect low', fresh().update(lr(15, 90)), [{which: 'left', level: 15}]);
}

{ // connect notice: one per connection, with model and battery levels, from the real fixtures
    const here = imports.gi.GLib.path_get_dirname(imports.gi.Gio.File.new_for_uri(import.meta.url).get_path());
    const fx = n => parseStatus(new TextDecoder().decode(imports.gi.GLib.file_get_contents(`${here}/fixtures/${n}.json`)[1]));
    const off = snap({connected: false});
    const pro = fx('pro3');
    const n = new ConnectNotice();
    same('disconnected', n.update(off), null);
    same('connect', n.update(pro), {name: 'AirPods Pro 3', levels: [['left', 80], ['right', 70], ['case', 50]]});
    same('repeat', n.update(pro), null);
    same('repeat 2', n.update(snap({left: pod(79), right: pod(70)})), null);
    n.update(off);
    same('reconnect', n.update(pro)?.name, 'AirPods Pro 3');
    same('daemon down re-arms', [n.update(parseStatus(null)), n.update(pro)?.name], [null, 'AirPods Pro 3']);
    same('headset', new ConnectNotice().update(fx('max2')).levels.map(l => l[0]), ['headphones']);
}

if (failures.length > 0) {
    print(failures.join('\n'));
    imports.system.exit(1);
}
print('ok');
