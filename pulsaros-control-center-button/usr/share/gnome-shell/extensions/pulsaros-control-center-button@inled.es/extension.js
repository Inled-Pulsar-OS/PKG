import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as QuickSettings from 'resource:///org/gnome/shell/ui/quickSettings.js';

import St from 'gi://St';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';

const ControlCenterButton = GObject.registerClass(
class ControlCenterButton extends QuickSettings.QuickSettingsItem {
    _init(extension) {
        super({
            style_class: 'icon-button system-status-icon',
            can_focus: true,
            icon_name: 'control-center-symbolic',
            accessible_name: 'Control Center',
        });

        this._extension = extension;
        this._iconFile = Gio.File.new_for_path(
            GLib.build_filenamev([this._extension.path, 'icons', 'control-center-symbolic.svg'])
        );

        if (this._iconFile.query_exists(null)) {
            this.set_child(
                new St.Icon({
                    gicon: new Gio.FileIcon({ file: this._iconFile }),
                    style_class: 'system-status-icon',
                })
            );
        }

        this.connect('clicked', () => {
            Main.notify('Control Center', 'Opening Control Center…');
            GLib.spawn_command_line_async('gnome-control-center');
        });
    }
});

export default class PulsarOSControlCenterButton extends Extension {
    enable() {
        this._indicator = new QuickSettings.SystemIndicator(this);
        this._button = new ControlCenterButton(this);
        this._indicator.quickSettingsItems.push(this._button);
        Main.panel.statusArea.quickSettings.addExternalIndicator(this._indicator, 1);
    }

    disable() {
        if (this._indicator) {
            this._indicator.destroy();
            this._indicator = null;
            this._button = null;
        }
    }
}
