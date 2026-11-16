// Runs the librepods control CLI. Only a verb that model.js builds from validated values ever reaches
// the command line, as a single argument (no shell involved).
import Gio from 'gi://Gio';

import {controlVerb} from './model.js';

/**
 * ctlPath: the librepods-ctl binary (a bare name is looked up on PATH).
 * done(ok): true when the command exited 0; false for a refused value, a missing binary or a failure.
 */
export function runControl(ctlPath, kind, value, done) {
    const verb = controlVerb(kind, value);
    if (verb === null) {
        done(false);
        return;
    }
    let proc;
    try {
        proc = Gio.Subprocess.new([ctlPath, verb], Gio.SubprocessFlags.STDOUT_SILENCE | Gio.SubprocessFlags.STDERR_SILENCE);
    } catch (e) {
        console.warn(`[AirPods] cannot run ${ctlPath}: ${e.message}`);
        done(false);
        return;
    }
    proc.wait_check_async(null, (p, result) => {
        try {
            p.wait_check_finish(result);
            done(true);
        } catch (e) {
            console.warn(`[AirPods] ${ctlPath} ${verb} failed: ${e.message}`);
            done(false);
        }
    });
}
