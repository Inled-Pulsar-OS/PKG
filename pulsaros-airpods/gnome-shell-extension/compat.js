// GNOME Shell 48 / 51 differences live here. Add a shim only for a real break and say why.
import Gio from 'gi://Gio';
import * as BoxPointer from 'resource:///org/gnome/shell/ui/boxpointer.js';

// Do Not Disturb: banners off means the popup must stay quiet too.
export function bannersEnabled() {
    try {
        const settings = new Gio.Settings({schema_id: 'org.gnome.desktop.notifications'});
        return settings.get_boolean('show-banners');
    } catch (_e) {
        return true;
    }
}

export function openMenuAnimated(menu) {
    menu.open(BoxPointer.PopupAnimation.FULL);
}

export function closeMenuAnimated(menu) {
    menu.close(BoxPointer.PopupAnimation.FULL);
}
