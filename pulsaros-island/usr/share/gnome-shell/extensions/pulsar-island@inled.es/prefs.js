import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import GObject from 'gi://GObject';
import Gtk from 'gi://Gtk';

import { ExtensionPreferences, gettext as _ } from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

export default class PulsarIslandPreferences extends ExtensionPreferences {
    fillPreferencesWindow(window) {
        const page = new Adw.PreferencesPage();
        window.add(page);

        const group = new Adw.PreferencesGroup({ title: _('Island') });
        page.add(group);

        const settings = this.getSettings();

        const toggles = [
            ['enabled', _('Enable island')],
            ['clipboard-enabled', _('Show on clipboard copies')],
            ['screenshot-enabled', _('Show on new screenshots')],
        ];
        for (const [key, title] of toggles) {
            const row = new Adw.SwitchRow({ title });
            settings.bind(key, row, 'active', Gio.SettingsBindFlags.DEFAULT);
            group.add(row);
        }

        const minSize = new Adw.SpinRow.new_with_range(0, 200, 1);
        minSize.title = _('Minimum clipboard text length');
        settings.bind('clipboard-min-size', minSize, 'value', Gio.SettingsBindFlags.DEFAULT);
        group.add(minSize);

        const timeout = new Adw.SpinRow.new_with_range(0, 120, 1);
        timeout.title = _('Auto-hide timeout (seconds, 0 disables)');
        settings.bind('auto-hide-timeout', timeout, 'value', Gio.SettingsBindFlags.DEFAULT);
        group.add(timeout);

        const opacity = new Adw.SpinRow.new_with_range(0.1, 1.0, 0.05);
        opacity.title = _('Island opacity');
        settings.bind('island-opacity', opacity, 'value', Gio.SettingsBindFlags.DEFAULT);
        group.add(opacity);

        const edgeRow = new Adw.ComboRow({
            title: _('Screen edge'),
            model: Gtk.StringList.new([_('Left'), _('Right')]),
        });
        const edge = settings.get_string('screen-edge');
        edgeRow.selected = edge === 'left' ? 0 : 1;
        edgeRow.connect('notify::selected', () => {
            settings.set_string('screen-edge', edgeRow.selected === 0 ? 'left' : 'right');
        });
        group.add(edgeRow);
    }
}
