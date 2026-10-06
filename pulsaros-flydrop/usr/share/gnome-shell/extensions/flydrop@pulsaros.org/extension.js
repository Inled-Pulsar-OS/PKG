import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import {DynamicDropIsland} from './dynamicIsland.js';

export default class FlyDropExtension extends Extension {
    enable() {
        console.log('[FlyDrop] Enabling FlyDrop Dynamic Island Drop Shelf');

        // Dynamic Island Drop Shelf (Top Chrome layer so pointer/DnD is always on top)
        this._island = new DynamicDropIsland();
        Main.layoutManager.addTopChrome(this._island, {
            affectsInputRegion: true,
            affectsStruts: false,
            trackFullscreen: true,
        });
    }

    disable() {
        console.log('[FlyDrop] Disabling FlyDrop extension');

        if (this._island) {
            Main.layoutManager.removeChrome(this._island);
            this._island.destroy();
            this._island = null;
        }
    }
}
