import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';

import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

export default class PulsarIdleGuardPreferences extends ExtensionPreferences {
    fillPreferencesWindow(window) {
        const settings = this.getSettings();

        const page = new Adw.PreferencesPage({
            title: 'Idle Guard',
            icon_name: 'preferences-system-power-symbolic',
        });

        const group = new Adw.PreferencesGroup({
            title: 'Keep the screen awake while it changes',
            description:
                'Pulsar OS suspends the machine when it is idle. This guard holds ' +
                'an idle + suspend inhibitor only while the display is actively ' +
                'changing (video, animation, remote desktop, games), then releases ' +
                'it so the machine can sleep again when the screen goes static.',
        });
        page.add(group);

        const activeRow = new Adw.SwitchRow({
            title: 'Enable the activity guard',
            subtitle: 'Turn off to restore plain GNOME idle behaviour',
        });
        settings.bind('active', activeRow, 'active', Gio.SettingsBindFlags.DEFAULT);
        group.add(activeRow);

        const fpsRow = new Adw.SpinRow({
            title: 'Activity threshold',
            subtitle: 'Painted frames per second that count as “the screen is changing”',
            adjustment: new Gtk.Adjustment({
                lower: 1,
                upper: 30,
                step_increment: 1,
                page_increment: 5,
            }),
        });
        settings.bind('minimum-fps', fpsRow, 'value', Gio.SettingsBindFlags.DEFAULT);
        group.add(fpsRow);

        const graceRow = new Adw.SpinRow({
            title: 'Grace period',
            subtitle: 'Seconds the guard keeps inhibiting after the last change',
            adjustment: new Gtk.Adjustment({
                lower: 0,
                upper: 120,
                step_increment: 1,
                page_increment: 5,
            }),
        });
        settings.bind('grace-seconds', graceRow, 'value', Gio.SettingsBindFlags.DEFAULT);
        group.add(graceRow);

        const debugRow = new Adw.SwitchRow({
            title: 'Debug logging',
            subtitle: 'Log the frame rate and inhibitor transitions to the journal',
        });
        settings.bind('debug', debugRow, 'active', Gio.SettingsBindFlags.DEFAULT);
        group.add(debugRow);

        window.add(page);
    }
}
