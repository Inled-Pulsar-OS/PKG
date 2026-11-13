# VM test plan: pulsaros-airpods on Arch

Repeatable test of the Arch build of `pulsaros-airpods` inside the QEMU VM that `ISO/run-qemu.sh` starts.
Hardware used for the real-AirPods layer: AirPods 4 **without ANC**. Guest: stable Arch minimal ISO
(GNOME Shell 51.1, BlueZ 5.88, SDDM autologin of `live` into GNOME Wayland, passwordless sudo).

The decisions behind this plan are on the wayfinder map "Map: verify pulsaros-airpods in a QEMU VM (Arch)"
and its closed tickets. The build work is the spec "[spec] pulsaros-airpods: VM test harness and plan (Arch)".
Research notes: branches `research/airpods4-non-anc-model`, `research/arch-iso-guest-facts`,
`research/qemu-usb-bt-passthrough` (docs only, never merged).

The baseline layer must fully pass before the real-AirPods layer starts.

## Setup

Sign-off run (a dedicated test ISO that contains the package):

1. Nothing to add to a package list: with `--local`, `build-iso.sh` builds every directory under
   `arch/pkgbuilds` (except the excluded ones) and installs all resulting packages with `pacman -U`,
   so `pulsaros-airpods` is included when the branch you build from contains it.
2. From the repo root: `./build-iso-docker.sh` (default flags: `--arch --local --incremental --grub --minimal`).
3. Boot it: `ISO/run-qemu.sh ISO/build/pulsaros-stable-arch-grub.iso`. Every run is a live session, so a reboot
   from the ISO is a clean reset.

Quick loop (while fixing a failed check, no ISO rebuild):

1. `pulsaros-airpods/build-local.sh arch`, then serve `arch/build/packages/` with `python -m http.server 8000 --bind 127.0.0.1`.
2. In the guest: `curl -fsSO http://10.0.2.2:8000/<pkg file>` and `sudo pacman -U <pkg file>`.
3. Log out and back in once (SDDM logs `live` straight back in). On Wayland the Shell only finds a new
   extension, and the user unit only starts, at login.

Baseline script (both routes), in a terminal inside the guest session:

```sh
# host, in pulsaros-airpods/ (10.0.2.2 in the guest reaches the host's loopback):
python -m http.server 8000 --bind 127.0.0.1
# guest:
curl -fsS http://10.0.2.2:8000/vm-test/vm-baseline.sh | bash
```

## Baseline layer (no Bluetooth). All blocking.

`vm-baseline.sh` prints one PASS or FAIL line per check and exits 1 when any check fails.

| ID | Check | Pass when | How |
|---|---|---|---|
| B1 | Install | package installed, `pacman -Qk` clean | script |
| B2 | Binaries | `/usr/bin/librepods` and `librepods-ctl` executable, no missing shared library | script |
| B3 | Unit | `librepods.service` enabled for the graphical session; with no adapter it may exit but `NRestarts` stays at 2 or less after 16 s | script |
| B4 | Extension | listed, enables and is active, no errors from it in the Shell journal, `metadata.json` lists Shell 51 | script |
| B5 | Hidden with no data | no `status.json` exists and no AirPods icon in the top bar | script checks the file, **you confirm the icon by eye** (the Shell does not expose panel contents to a script) |
| B6 | Fixture UI | see below | manual |

The ISO sets `disable-extension-version-validation=true`, so the Shell does not enforce the `shell-version` list.
That is why B4 checks `metadata.json` for 51 explicitly.

### B6 fixture UI (manual)

```sh
mkdir -p ~/.local/state/librepods
curl -fsS http://10.0.2.2:8000/tests/fixtures/airpods4.json -o ~/.local/state/librepods/status.json
```

The same host server (started in `pulsaros-airpods/`) serves the fixtures. The low-battery fixture is
`tests/fixtures/airpods4-low.json`.

Pass when all of these hold:

- the AirPods icon appears in the top bar;
- the popover shows left, right and case battery rows and an Ear detection submenu;
- there is **no** mode row, noise slider, Conversation Awareness switch or One-Bud ANC switch;
- `rm ~/.local/state/librepods/status.json` hides the icon again;
- copying `airpods4-low.json` over the status file raises a low-battery alert for the left bud (15 percent).

## Real-AirPods layer (USB passthrough). All blocking.

Prerequisite: the host Bluetooth adapter is passed to the guest with `BT_USB` (see the `ISO/run-qemu.sh` header
for the udev rule and recovery). The host has no Bluetooth while the VM runs. Put the AirPods in pairing mode
(hold the case button) before R1.

| ID | Check | Pass when |
|---|---|---|
| R1 | Pair and connect | paired and connected in the guest |
| R2 | Icon | the panel icon appears within 10 s of connecting |
| R3 | Battery | left, right and case levels are present and plausible: each 0 to 100, no case value while the buds are out of the case |
| R4 | Ear detection | removing a bud changes its state in `status.json` and the popover within 3 s |
| R5 | Disconnect | the icon hides within 5 s of disconnecting and comes back after reconnecting |

The time limits are starting values. The first real run may revise them; record what you observed.

No noise commands (`librepods-ctl noise:*`) are sent in this layer.

## Observations (recorded, not asserted)

- `status.json` reports `noise_mode: 2` for a model with no noise control (stale default).
- Mode buttons may show briefly while the model is still unknown.
- `librepods-ctl noise:*` is not gated by model (not exercised here).
- What `librepods.service` does with no Bluetooth adapter (the baseline script prints its state).

## Not testable on a non-ANC AirPods 4

Listening modes, adaptive level, Conversation Awareness, One-Bud ANC. Low-battery alerts are checked by fixture only (B6).

## After a real run

The AirPods stay paired to the guest adapter. Re-pair them with your phone and other devices. Reboot the VM
between runs and re-pair them each time. If QEMU crashed, the host adapter may stay detached: replug it, or as
root rebind `btusb` for its interfaces.

## Results

Copy one row per run.

| Date | ISO / package version | B1 | B2 | B3 | B4 | B5 | B6 | R1 | R2 | R3 | R4 | R5 | Notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| | | | | | | | | | | | | | |
