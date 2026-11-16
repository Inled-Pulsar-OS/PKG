// Run with: gjs -m tests/status_source_check.js
// StatusSource must report: file missing (null), file content, every rewrite, deletion (null),
// and a file created later inside a directory that did not exist when we started watching.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import {StatusSource, defaultStatusPath} from '../gnome-shell-extension/status-source.js';

const failures = [];
const loop = new GLib.MainLoop(null, false);
const base = GLib.dir_make_tmp('status-source-XXXXXX');
const dir = GLib.build_filenamev([base, 'state', 'librepods']);   // does not exist yet
const path = GLib.build_filenamev([dir, 'status.json']);
const seen = [];

const wait = (ms, check) => new Promise(resolve => GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => { resolve(check()); return GLib.SOURCE_REMOVE; }));
const until = async (what, cond, timeoutMs = 4000) => {
    const end = GLib.get_monotonic_time() + timeoutMs * 1000;
    while (GLib.get_monotonic_time() < end) {
        if (cond())
            return;
        await wait(50, () => 0);
    }
    failures.push(`timeout: ${what}; seen=${JSON.stringify(seen)}`);
};
const write = text => {
    GLib.mkdir_with_parents(dir, 0o700);
    GLib.file_set_contents(path, text);
};

const source = new StatusSource(path, text => seen.push(text));

(async () => {
    source.start();
    await until('initial missing file is reported as null', () => seen.length >= 1 && seen[0] === null);

    write('{"a":1}');
    await until('file created in a directory that did not exist', () => seen.includes('{"a":1}'));

    write('{"a":2}');
    await until('rewrite', () => seen[seen.length - 1] === '{"a":2}');

    Gio.File.new_for_path(path).delete(null);
    await until('deletion is reported as null', () => seen[seen.length - 1] === null);

    write('{"a":3}');
    await until('recreated after deletion', () => seen[seen.length - 1] === '{"a":3}');

    const before = seen.length;
    source.stop();
    write('{"a":4}');
    await wait(500, () => 0);
    if (seen.length !== before)
        failures.push(`reported after stop(): ${JSON.stringify(seen.slice(before))}`);

    const def = defaultStatusPath();
    if (!def.endsWith('/librepods/status.json'))
        failures.push(`defaultStatusPath ${def}`);
})().catch(e => failures.push(`exception: ${e.message}`)).finally(() => loop.quit());

loop.run();
if (failures.length > 0) {
    print(failures.join('\n'));
    imports.system.exit(1);
}
print('ok');
