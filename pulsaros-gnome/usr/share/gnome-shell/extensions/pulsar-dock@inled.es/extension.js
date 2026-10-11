// -*- mode: js; js-indent-level: 4; indent-tabs-mode: nil -*-

import {DockManager} from './docking.js';
import {Extension} from './dependencies/shell/extensions/extension.js';

// We export this so it can be accessed by other extensions
export let dockManager;

export default class DashToDockExtension extends Extension.Extension {
    enable() {
        // Build marker: printed to the journal so it is always unambiguous
        // which build of the dock GNOME Shell actually loaded. If a stale
        // install is ever suspected, `journalctl -b | grep pulsar-dock`
        // shows the src path and build id in use.
        try {
            console.log(`[pulsar-dock] enable build=magnify-guard-1 src=${import.meta.url}`);
        } catch {
            console.log('[pulsar-dock] enable build=magnify-guard-1');
        }

        // TODO: Remove this when upstream will disable extensions on shutdown
        // See: https://gitlab.gnome.org/GNOME/gnome-shell/-/merge_requests/4214
        this._shutdownID = global.connect('shutdown', () => this.disable());
        dockManager = new DockManager(this);
    }

    disable() {
        global.disconnect(this._shutdownID);
        delete this._shutdownID;
        dockManager?.destroy();
        dockManager = null;
    }
}
