# pulsaros-airpods

AirPods icon and battery card in the GNOME top bar, like macOS. A Python daemon reads Apple
Continuity advertisements through BlueZ and publishes state on the session bus; a Shell
extension renders it. Targets GNOME Shell 48 (Debian trixie) and 51 (Arch rolling).

Design and decisions: `Docs/pulsaros-airpods-plan.md` in Victor1890/AirPodsDesktop.
Model ids last synced with upstream `AppleCP.cpp` on 2026-11-03 (through AirPods 5, `tests/test_models.py`).
Protocol reference: that repo's `Source/Core/AppleCP.*`, `AirPodsAdvertisement.cpp`,
`AirPodsStateManager.cpp`.

## Layout

Same shape as `pulsaros-spotlight-launcher`: sources by kind, installed to `/usr` by `prepare-assets.sh`
(and by the PKGBUILD on Arch).

| Path | What | Installed to |
|------|------|--------------|
| `src/pulsaros_airpods/protocol.py` | 27-byte ProximityPairing parser (pure) | `/usr/share/pulsaros-airpods/pulsaros_airpods/` |
| `.../state.py` | merges the two earbuds' advertisements, RSSI floor, lost/reset timers (pure) | same |
| `.../tracker.py` | which BlueZ device is our AirPods, the paired list, forget | same |
| `.../controller.py` | combines device + advertisements into the published snapshot | same |
| `.../alerts.py` | low-battery thresholds (pure) | same |
| `.../bluez.py` | org.bluez scan, device events, connect/disconnect, self-healing | same |
| `.../service.py` | `org.pulsaros.AirPods` on the session bus (GDBus) | same |
| `.../mpris.py` | ear detection: pause/resume MPRIS players | same |
| `data/org.pulsaros.AirPods.xml` | the D-Bus contract (source of truth) | `/usr/share/pulsaros-airpods/dbus/` |
| `data/pulsaros-airpods.service` | systemd user unit | `/usr/lib/systemd/user/` |
| `cli/pulsaros-airpods-daemon` | launcher | `/usr/bin/` |
| `gnome-shell-extension/` | the extension; `interface.js` embeds the contract | `/usr/share/gnome-shell/extensions/airpods@pulsaros.org/` |
| `gnome-shell-extension/i18n/` | translation catalogs, one JSON file per language | with the extension |
| `scripts/` | hardware spike tools | not installed |
| `tests/` | the test suite | not installed |

## Tests

```sh
python3 -m unittest discover -s tests          # pure tests; bus tests are skipped (103)
PULSAROS_TEST_BUS=1 dbus-run-session -- python3 -m unittest discover -s tests   # all 149
```

Tests sit at four seams and avoid private details:

| File | Seam |
|------|------|
| `test_protocol_state.py`, `test_logic.py`, `test_domain_edges.py`, `test_models.py`, `test_alerts.py`, `test_tracker_rebind.py`, `test_paired_devices.py` | domain: protocol, state manager, tracker, controller, config |
| `test_bluez_boundary.py` | `BluezBackend` against `fake_bluez.py` (a fake `org.bluez`) |
| `test_wire_contract.py`, `test_integration.py` | the real daemon, observed only over D-Bus |
| `test_mpris_and_extension.py`, `test_i18n.py`, `i18n_check.js` | MPRIS backend vs `fake_player.py`; extension vs contract drift; every visible string translatable and translated; the JS loader (needs `gjs`) |

Bus tests need a private session bus (`dbus-run-session`) and are skipped otherwise, so they never
touch your real session.

## Try it by hand

```sh
# terminal 1 (private bus so nothing touches the real session)
dbus-run-session -- bash
python3 tests/fake_bluez.py                      # then type JSON commands, see its docstring
# terminal 2, same bus
PYTHONPATH=src python3 -m pulsaros_airpods.daemon --bluez-bus session --debug
busctl --user introspect org.pulsaros.AirPods /org/pulsaros/AirPods
```

Real hardware: `python3 scripts/scan_spike.py 30` prints the Apple payloads BlueZ delivers.

## Extension

Enable per user: `gnome-extensions enable airpods@pulsaros.org`. To iterate on the UI use a
nested shell with a mock service (`fake_bluez.py` plus the daemon above).

## Known limits

- AirPods use random BLE addresses; matching advertisements to the connected pair is heuristic
  (model, battery drift, RSSI drift). Two pairs of the same model in range can mix.
- Noise-control modes need Apple's AAP protocol and are not implemented.
- Translations: Spanish only. See "Translations" below.

## Translations

Catalogs are plain JSON in `gnome-shell-extension/i18n/<lang>.json`: the English text is the key,
the translation the value. `i18n.js` picks the first catalog that matches the session language
(`es_ES.UTF-8` tries `es_ES`, then `es`) and falls back to English for a missing file or string.
No gettext, no build step.

To add a language, copy `es.json` to `<lang>.json` and translate the values. Keep the `{name}` and
`{level}` placeholders. Wrap new UI strings in `_('...')` in `indicator.js`: `tests/test_i18n.py`
fails when a catalog lacks a string, has an extra one, or a placeholder differs.
