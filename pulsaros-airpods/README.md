# pulsaros-airpods

AirPods icon and battery card in the GNOME top bar, like macOS. A Python daemon reads Apple
Continuity advertisements through BlueZ and publishes state on the session bus; a Shell
extension renders it. Targets GNOME Shell 48 (Debian trixie) and 51 (Arch rolling).

Design and decisions: `Docs/pulsaros-airpods-plan.md` in Victor1890/AirPodsDesktop.
Protocol reference: that repo's `Source/Core/AppleCP.*`, `AirPodsAdvertisement.cpp`,
`AirPodsStateManager.cpp`.

## Layout

| Path | What |
|------|------|
| `usr/share/pulsaros-airpods/pulsaros_airpods/protocol.py` | 27-byte ProximityPairing parser (pure) |
| `.../state.py` | merges the two earbuds' advertisements, RSSI floor, lost/reset timers (pure) |
| `.../tracker.py` | which BlueZ device is our AirPods (Modalias, name, bound address) |
| `.../controller.py` | combines device + advertisements into the published snapshot |
| `.../bluez.py` | org.bluez scan + device events, self-healing |
| `.../service.py` | `org.pulsaros.AirPods` on the session bus (GDBus) |
| `.../mpris.py` | ear detection: pause/resume MPRIS players |
| `usr/share/pulsaros-airpods/dbus/org.pulsaros.AirPods.xml` | the contract (source of truth) |
| `usr/share/gnome-shell/extensions/airpods@pulsaros.org/` | the extension; `interface.js` embeds the contract |
| `tests/`, `tools/` | development only; removed from packages by `prepare-assets.sh` |

## Tests

```sh
python3 -m unittest discover -s tests          # pure tests; bus tests are skipped (67)
PULSAROS_TEST_BUS=1 dbus-run-session -- python3 -m unittest discover -s tests   # all 95
```

Tests sit at four seams and avoid private details:

| File | Seam |
|------|------|
| `test_protocol_state.py`, `test_logic.py`, `test_domain_edges.py` | domain: protocol, state manager, tracker, controller, config |
| `test_bluez_boundary.py` | `BluezBackend` against `fake_bluez.py` (a fake `org.bluez`) |
| `test_wire_contract.py`, `test_integration.py` | the real daemon, observed only over D-Bus |
| `test_mpris_and_extension.py` | MPRIS backend vs `fake_player.py`; extension vs contract drift |

Bus tests need a private session bus (`dbus-run-session`) and are skipped otherwise, so they never
touch your real session.

## Try it by hand

```sh
# terminal 1 (private bus so nothing touches the real session)
dbus-run-session -- bash
python3 tests/fake_bluez.py                      # then type JSON commands, see its docstring
# terminal 2, same bus
PYTHONPATH=usr/share/pulsaros-airpods python3 -m pulsaros_airpods.daemon --bluez-bus session --debug
busctl --user introspect org.pulsaros.AirPods /org/pulsaros/AirPods
```

Real hardware: `python3 tools/scan_spike.py 30` prints the Apple payloads BlueZ delivers.

## Extension

Enable per user: `gnome-extensions enable airpods@pulsaros.org`. To iterate on the UI use a
nested shell with a mock service (`fake_bluez.py` plus the daemon above).

## Known limits

- AirPods use random BLE addresses; matching advertisements to the connected pair is heuristic
  (model, battery drift, RSSI drift). Two pairs of the same model in range can mix.
- Noise-control modes need Apple's AAP protocol and are not implemented.
- Strings are English; no gettext yet.
