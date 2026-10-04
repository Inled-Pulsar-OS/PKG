import GLib from 'gi://GLib';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import { NotchNux } from './src/notchnux.js';

export default class NotchNuxExtension extends Extension {
    enable() {
        console.log('NotchNux: Enabling multi-monitor PulsarOS build...');

        if (Main.panel?.statusArea?.dateMenu)
            Main.panel.statusArea.dateMenu.hide();

        this._notches = [];
        this._monitorRebuildId = 0;

        this._buildNotches();

        this._monitorsChangedId =
            Main.layoutManager.connect('monitors-changed', () => {
                if (this._monitorRebuildId)
                    return;

                this._monitorRebuildId =
                    GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, () => {
                        this._monitorRebuildId = 0;
                        this._buildNotches();
                        return GLib.SOURCE_REMOVE;
                    });
            });
    }

    _destroyNotches() {
        for (let notch of this._notches ?? []) {
            try {
                Main.layoutManager.removeChrome(notch);
            } catch (e) {}

            try {
                notch.destroy();
            } catch (e) {
                console.error('NotchNux: failed to destroy monitor instance', e);
            }
        }

        this._notches = [];
    }

    _buildNotches() {
        this._destroyNotches();

        let monitors = Main.layoutManager.monitors ?? [];
        let primary = Main.layoutManager.primaryIndex;

        for (let i = 0; i < monitors.length; i++) {
            let notch = new NotchNux(this, i, i === primary);

            this._notches.push(notch);

            Main.layoutManager.addTopChrome(notch, {
                trackFullscreen: true
            });
        }

        console.log(`NotchNux: ${this._notches.length} monitor notch(es) active`);
    }

    disable() {
        if (this._monitorRebuildId) {
            GLib.Source.remove(this._monitorRebuildId);
            this._monitorRebuildId = 0;
        }

        if (this._monitorsChangedId) {
            Main.layoutManager.disconnect(this._monitorsChangedId);
            this._monitorsChangedId = 0;
        }

        this._destroyNotches();

        if (Main.panel?.statusArea?.dateMenu)
            Main.panel.statusArea.dateMenu.show();
    }
}
