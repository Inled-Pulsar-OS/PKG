// Run with: gjs -m tests/ctl_check.js
// runControl must spawn the control CLI with exactly one whitelisted verb, report success or
// failure, and spawn nothing for a value controlVerb refuses.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import {runControl} from '../gnome-shell-extension/ctl.js';

const failures = [];
const loop = new GLib.MainLoop(null, false);
const dir = GLib.dir_make_tmp('ctl-check-XXXXXX');
const log = GLib.build_filenamev([dir, 'calls.log']);
const fake = GLib.build_filenamev([dir, 'librepods-ctl']);
const failing = GLib.build_filenamev([dir, 'librepods-ctl-fails']);

GLib.file_set_contents(fake, `#!/bin/sh\necho "$#:$*" >> '${log}'\nexit 0\n`);
GLib.file_set_contents(failing, '#!/bin/sh\nexit 3\n');
for (const f of [fake, failing])
    Gio.File.new_for_path(f).set_attribute_uint32('unix::mode', 0o755, Gio.FileQueryInfoFlags.NONE, null);

const calls = () => {
    try {
        return new TextDecoder().decode(GLib.file_get_contents(log)[1]).trim().split('\n').filter(Boolean);
    } catch (_e) {
        return [];
    }
};
const run = (ctl, kind, value) => new Promise(resolve => runControl(ctl, kind, value, ok => resolve(ok)));
const check = (what, got, want) => {
    if (JSON.stringify(got) !== JSON.stringify(want))
        failures.push(`${what}\n    got  ${JSON.stringify(got)}\n    want ${JSON.stringify(want)}`);
};

(async () => {
    check('mode verb succeeds', await run(fake, 'noise', 3), true);
    check('one argument, the whitelisted verb', calls(), ['1:noise:adaptive']);
    check('adaptive level', await run(fake, 'adaptive', 41.6), true);
    check('level verb is rounded', calls().pop(), '1:adaptive:42');
    check('switch', await run(fake, 'ca', false), true);
    check('switch verb', calls().pop(), '1:ca:off');

    const before = calls().length;
    check('refused value reports failure', await run(fake, 'noise', '1; reboot'), false);
    check('unknown kind reports failure', await run(fake, 'rm', 1), false);
    check('refused values spawn nothing', calls().length, before);

    check('non-zero exit is a failure', await run(failing, 'noise', 1), false);
    check('missing binary is a failure', await run(GLib.build_filenamev([dir, 'nope']), 'noise', 1), false);
})().catch(e => failures.push(`exception: ${e.message}`)).finally(() => loop.quit());

loop.run();
if (failures.length > 0) {
    print(failures.join('\n'));
    imports.system.exit(1);
}
print('ok');
