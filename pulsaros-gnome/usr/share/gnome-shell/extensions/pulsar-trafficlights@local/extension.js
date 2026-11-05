// Pulsar OS - semaforo macOS 27 dentro gnome-shell.
//
// Perche' un'estensione: la shell e' gia' accesa, quindi questa non aggiunge
// nessun processo e non ha bisogno di un timer. Reagisce agli eventi veri -
// cambio tema, cambio schema colori, riscrittura dei CSS in ~/.config - e chiama
// ~/bin/pulsar-trafficlights solo quando serve. Al primo avvio spegne il timer
// di ripiego, che a quel punto e' lavoro doppio.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const DEBOUNCE_MS = 1500;

// le copie che le app GTK caricano davvero: se il sistema le rigenera dal tema,
// il monitor ce lo dice subito
const WATCHED = [
    '.config/gtk-4.0/gtk.css',
    '.config/gtk-4.0/gtk-dark.css',
    '.config/gtk-3.0/gtk.css',
    '.config/gtk-3.0/gtk-dark.css',
];

export default class PulsarTrafficLightsExtension extends Extension {
    enable() {
        this._pending = 0;
        this._monitors = [];
        this._settingsIds = [];

        this._settings = new Gio.Settings({schema_id: 'org.gnome.desktop.interface'});
        this._settingsIds.push(
            this._settings.connect('changed::gtk-theme', () => this._queue()),
            this._settings.connect('changed::color-scheme', () => this._queue()));

        const home = GLib.get_home_dir();
        for (const rel of WATCHED) {
            try {
                const file = Gio.File.new_for_path(`${home}/${rel}`);
                const monitor = file.monitor_file(Gio.FileMonitorFlags.WATCH_MOVES, null);
                const id = monitor.connect('changed', () => this._queue());
                this._monitors.push({monitor, id});
            } catch (e) {
                logError(e, `pulsar-trafficlights: monitor ${rel}`);
            }
        }

        // una passata all'avvio della sessione
        this._run();
    }

    disable() {
        if (this._pending) {
            GLib.source_remove(this._pending);
            this._pending = 0;
        }
        for (const id of this._settingsIds)
            this._settings.disconnect(id);
        this._settingsIds = [];
        this._settings = null;

        for (const {monitor, id} of this._monitors) {
            try {
                monitor.disconnect(id);
                monitor.cancel();
            } catch (e) {
                // monitor gia' chiuso: niente da fare
            }
        }
        this._monitors = [];
    }

    // piu' eventi ravvicinati (tema + colore + file) = una sola chiamata
    _queue() {
        if (this._pending)
            return;
        this._pending = GLib.timeout_add(GLib.PRIORITY_DEFAULT, DEBOUNCE_MS, () => {
            this._pending = 0;
            this._run();
            return GLib.SOURCE_REMOVE;
        });
    }

    _run() {
        const bin = GLib.find_program_in_path('pulsar-trafficlights') ||
                    `${GLib.get_home_dir()}/bin/pulsar-trafficlights`;
        this._quiet([bin]);
    }

    _quiet(argv) {
        try {
            Gio.Subprocess.new(argv,
                Gio.SubprocessFlags.STDOUT_SILENCE | Gio.SubprocessFlags.STDERR_SILENCE);
        } catch (e) {
            // senza systemd non c'e' nessun timer da spegnere: nessun problema
        }
    }
}
