// Run under a session bus with the daemon + fake_bluez up:  gjs -m tests/proxy_check.js
// Verifies that the extension's proxy wrapper (interface.js) can read the daemon's properties.
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import {AirPodsProxy, BUS_NAME, OBJECT_PATH} from '../usr/share/gnome-shell/extensions/airpods@pulsaros.org/interface.js';

const loop = new GLib.MainLoop(null, false);
const failures = [];
const expect = (what, got, want) => { if (got !== want) failures.push(`${what}: got ${got}, want ${want}`); };

new AirPodsProxy(Gio.DBus.session, BUS_NAME, OBJECT_PATH, (proxy, error) => {
    if (error) { print(`proxy error: ${error.message}`); imports.system.exit(1); }
    expect('Connected', proxy.Connected, true);
    expect('Model', proxy.Model, 'AirPods Pro 2');
    expect('Left', proxy.Left, 80);
    expect('Right', proxy.Right, 70);
    expect('Case', proxy.Case, 50);
    expect('FallbackBattery', proxy.FallbackBattery, 60);
    expect('EarDetection', proxy.EarDetection, true);
    proxy.EarDetection = false;
    GLib.timeout_add(GLib.PRIORITY_DEFAULT, 500, () => {
        expect('EarDetection after set', proxy.EarDetection, false);
        loop.quit();
        return GLib.SOURCE_REMOVE;
    });
});
loop.run();
if (failures.length) { print(failures.join('\n')); imports.system.exit(1); }
print('proxy_check ok');
