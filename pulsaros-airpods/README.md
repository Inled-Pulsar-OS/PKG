# pulsaros-airpods

AirPods in the GNOME top bar, like macOS: an icon while they are connected, and a popover with the
battery of each earbud and the case, the listening mode, Conversation Awareness, One-Bud ANC and ear
detection. Targets GNOME Shell 48 (Debian trixie) and 51 (Arch).

One package, two parts:

- **`gnome-shell-extension/`**: the GNOME Shell extension (`airpods@pulsaros.org`), our code (MIT).
- **`daemon/`**: the backend, a headless daemon that talks to the AirPods over Bluetooth (Apple Accessory
  Protocol and BLE). It is a modified copy of the omarchy-pods daemon, itself a fork of librepods by Kavish
  Devar, so it stays GPL-3.0. See `daemon/UPSTREAM.md` for the chain, what we changed and how to re-sync.
  It is built from source (C++/Qt 6) and installed as `/usr/bin/librepods` and `/usr/bin/librepods-ctl`.

The two only share a JSON file and a command line, so the backend can be replaced later (for example by
a Python daemon) without touching the extension.

## How it works

```
AirPods --Bluetooth (AAP + BLE)--> librepods (systemd user service)
                                       |  writes $XDG_STATE_HOME/librepods/status.json on every change
                                       v
              airpods@pulsaros.org  --watches the file (Gio.FileMonitor, no polling)
                                       |  on a click: librepods-ctl <one whitelisted verb>
                                       v
                                    daemon
```

The extension never talks to Bluetooth. A missing file means the daemon is not running and the icon is
hidden. The popover only shows what the model has: AirPods Pro 3 has no Off mode, AirPods Max has one
battery and no case, AirPods 1 to 3 have no listening modes at all.

## Layout

| Path | What |
|------|------|
| `gnome-shell-extension/model.js` | parses status.json, lists the modes a model has, builds control verbs (pure) |
| `.../status-source.js` | watches the status file, including a directory that does not exist yet |
| `.../optimistic.js` | holds a click against stale reads until the daemon agrees (pure) |
| `.../alerts.js` | low-battery thresholds 20 / 10 / 5 percent (pure) |
| `.../ctl.js` | runs `librepods-ctl` with one validated verb, no shell |
| `.../indicator.js` | the panel button and popover |
| `.../compat.js` | GNOME Shell 48 / 51 differences |
| `.../i18n.js`, `.../i18n/*.json` | translations (see below) |
| `daemon/` | the librepods daemon (C++/Qt 6, GPL-3.0), its own tests in `daemon/tests` |
| `daemon/packaging/` | the packaged systemd unit and the trixie container build script |
| `docs/` | research notes (AAP protocol feasibility) |
| `tests/` | the extension test suite (not installed) |

## Tests

```sh
python3 -m unittest discover -s tests                      # the extension
cmake -S daemon -B build -G Ninja && cmake --build build   # the daemon (needs Qt 6 dev packages)
LC_ALL=C.UTF-8 QT_QPA_PLATFORM=offscreen ctest --test-dir build
```

The JavaScript is specified by `gjs` scripts in `tests/*_check.js`, run by `tests/test_*.py`. They skip
when `gjs` is not installed. `tests/fixtures/` holds hand-written status files for AirPods 4 (ANC),
Pro 3, Max 2 and a pair in the case; replace them with captures from real hardware when you have them.

## Try it without AirPods

Write a fixture where the daemon would, then enable the extension:

```sh
mkdir -p ~/.local/state/librepods
cp tests/fixtures/pro3.json ~/.local/state/librepods/status.json
gnome-extensions enable airpods@pulsaros.org
```

Controls call `librepods-ctl`, which fails when the daemon is not running; the popover then undoes the
click and shows an error.

## Translations

Catalogs are plain JSON in `gnome-shell-extension/i18n/<lang>.json`: the English text is the key and the
translation the value. `i18n.js` picks the first catalog that matches the session language (`es_ES.UTF-8`
tries `es_ES`, then `es`) and falls back to English. To add a language, copy `es.json`, translate the
values and keep `{name}` and `{level}` placeholders. Wrap new UI strings in `_('...')` in `indicator.js`;
`tests/test_i18n.py` fails when a catalog and the source disagree.

## Known limits

- The extension has never run in a real GNOME Shell yet. The slider and segmented control are the parts
  most likely to need a fix on 48 and 51.
- Low-battery alerts cannot be switched off (no settings store yet).
- Spatial Audio, microphone mode and audio routing are not AirPods controls on Linux and are left to the
  stock Sound panel.
- AAP is reverse engineered; see `docs/aap-noise-control-research.md`.
