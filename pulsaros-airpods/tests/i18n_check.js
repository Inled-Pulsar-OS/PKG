// Run with: gjs -m tests/i18n_check.js
// Checks gnome-shell-extension/i18n.js against the shipped catalogs. Exit 1 on failure.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import {loadTranslator} from '../gnome-shell-extension/i18n.js';

const dir = GLib.path_get_dirname(Gio.File.new_for_uri(import.meta.url).get_path());
const extension = GLib.build_filenamev([dir, '..', 'gnome-shell-extension']);
const failures = [];
const expect = (what, got, want) => { if (got !== want) failures.push(`${what}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`); };

expect('es_ES.UTF-8 uses es.json', loadTranslator(extension, ['es_ES.UTF-8', 'es_ES', 'es', 'C'])('Left'), 'Izquierdo');
expect('plain es', loadTranslator(extension, ['es'])('Charging'), 'Cargando');
expect('modifier is ignored', loadTranslator(extension, ['es_ES@euro'])('Left'), 'Izquierdo');
expect('unknown language is English', loadTranslator(extension, ['fr_FR.UTF-8', 'fr', 'C'])('Left'), 'Left');
expect('C is English', loadTranslator(extension, ['C'])('Left'), 'Left');
expect('no languages is English', loadTranslator(extension, [])('Left'), 'Left');
expect('unknown string is returned as is', loadTranslator(extension, ['es'])('Not in the catalog'), 'Not in the catalog');
expect('missing directory is English', loadTranslator('/nonexistent', ['es'])('Left'), 'Left');
expect('later language is used when the first has no catalog', loadTranslator(extension, ['fr', 'es'])('Left'), 'Izquierdo');

if (failures.length > 0) {
    print(failures.join('\n'));
    imports.system.exit(1);
}
print('ok');
