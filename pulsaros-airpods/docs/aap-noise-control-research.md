# AAP noise control on Linux: feasibility research

Ticket: Victor1890/pulsaros-PKG #16. Date: 2026-11-03. Research only; nothing here is implemented.

Legend: **[V]** verified by reading source or docs (link given); **[I]** inferred, not verified;
**[?]** unknown. Every byte value below is quoted from a linked source.

## Summary and recommendation

**Verdict: feasible, no root needed. Recommendation: spike first, then go.**

- AAP is a plain L2CAP SEQPACKET channel on PSM 0x1001 to the *classic* BR/EDR address of an
  already connected AirPods. Three independent codebases open it as a normal user: LibrePods
  (Python snippet, C++ and Rust), and podctl (Rust, raw libc). [V]
- The kernel needs no capability for it: `CAP_NET_RAW` is only checked for `SOCK_RAW`, and
  `CAP_NET_BIND_SERVICE` only for PSMs below 0x1001 (0x1001 itself is the first dynamic PSM). [V, kernel source]
- Noise control is two small frames: select mode `04 00 04 00 09 00 0D <mode> 00 00 00` with
  mode 1 Off, 2 NC, 3 Transparency, 4 Adaptive; the AirPods echo the same frame, also when the
  mode is changed by the stem. [V, LibrePods docs + podctl source]
- Everything is reverse engineered. The only primary evidence of a working session on real
  hardware is AirPods Pro 2 (USB-C) (LibrePods "AAP Definitions", podctl README). Other models
  are extrapolated. [V]
- AAP complements, not replaces, BLE advertisements: BLE works before/without a classic
  connection and for lid-open popups; AAP gives exact, event-driven state once connected.
- Licensing: LibrePods is GPL-3.0. Use it as a protocol reference, write our own code, record the
  provenance. podctl (MIT OR Apache-2.0) did exactly this and documents it in its NOTICE.md.
- Biggest unknowns for a spike: Debian trixie's Python socket support and BlueZ 5.82 behaviour
  (nothing in the sources is version-gated, but nobody lists 5.82 as tested), the exact
  handshake needed on each model, and whether AAP leaves audio untouched on PipeWire.
- Estimate: spike 0.5 day; daemon feature (read + set mode, tests) 3 to 4 days; extension UI
  1 to 2 days; hardening/multi-model 1+ day. About 1.5 weeks total, dominated by hardware time.

## 1. The protocol

### Transport and framing [V]

LibrePods' "AAP Definitions" doc: "AAP runs on top of L2CAP, with a PSM of 0x1001 or 4097."
Frames on an established session are `04 00 04 00 <opcode, 16-bit little endian> <data>`
([control_commands.md](https://github.com/kavishdevar/librepods/blob/main/docs/control_commands.md),
[opcodes.md](https://github.com/kavishdevar/librepods/blob/main/docs/opcodes.md)). LibrePods calls it
AACP in places; same protocol. Each L2CAP SDU is one frame (SEQPACKET preserves boundaries).

### Opening a session [V, with caveats]

The handshake frame is mandatory: "This packet is necessary to establish a connection with the
AirPods. Or else, the AirPods will not respond to any packets."
([AAP Definitions](https://github.com/kavishdevar/librepods/blob/main/docs/AAP%20Definitions.md)).

| Step | Bytes | Source |
|------|-------|--------|
| 1. Handshake | `00 00 04 00 01 00 02 00 00 00 00 00 00 00 00 00` | AAP Definitions; same in LibrePods [`airpods_packets.h`](https://github.com/kavishdevar/librepods/blob/main/linux/airpods_packets.h), [Rust `aacp.rs`](https://github.com/kavishdevar/librepods/blob/linux/rust/linux-rust/src/bluetooth/aacp.rs), podctl [`daemon/aap.rs`](https://github.com/Rockykln/podctl/blob/main/src/bin/daemon/aap.rs) |
| Handshake ack | frame starting `01 00 04 00` | LibrePods `HANDSHAKE_ACK` |
| 2. Set feature flags | doc: `04 00 04 00 4d 00 ff 00 00 00 00 00 00 00`; C++: `... 4d 00 d7 00 ...`; Rust: `ff` | the three sources disagree on the mask byte |
| Features ack | frame starting `04 00 04 00 2b 00` (comment: "only tested with airpods pro 2") | LibrePods `FEATURES_ACK` |
| 3. Subscribe | doc: `04 00 04 00 0F 00 FF FF FE FF` or `FF FF FF FF`; C++ and podctl: `04 00 04 00 0f 00 ff ff ff ff ff` (5 payload bytes) | podctl notes the doc shows four and the five-byte form "is what actually works on Pro 2" |

The feature-flags frame matters: per the doc, it enables Conversational Awareness while audio plays
and Adaptive Transparency; podctl says the `d7` mask "is what actually enables battery
notifications on the Pro 2". Without it, LibrePods warns the AirPods answer a requested Adaptive
mode with another mode (e.g. 0x02). Our spike must test `d7` versus `ff`.

LibrePods C++ sequences the steps by waiting for each ack; podctl simply sleeps 200 ms between
frames. Either works on Pro 2 USB-C. The AirPods can end a session unprompted: podctl treats a
frame starting `02 00 04 00` as "buds start over" and reopens the socket. [V]

### How the channel is found

LibrePods' Qt app connects by SDP service UUID `74ec2172-0bad-4d01-8f77-997b2be0722a`
(`main.cpp`, `connectToService`), which resolves to the PSM; its Python hearing-aid helper
explains "QBluetooth doesn't support connecting to a socket with its PSM, only a UUID"
([linux/README.md](https://github.com/kavishdevar/librepods/blob/main/linux/README.md)). Everything
else (LibrePods Rust, podctl) hard-codes PSM 0x1001. [V] We should hard-code it too.

## 2. Noise-control packets

### Read and set [V]

Set (host to AirPods) and notification (AirPods to host) share one layout
([AAP Definitions](https://github.com/kavishdevar/librepods/blob/main/docs/AAP%20Definitions.md)):

```
04 00 04 00 09 00 0D [mode] 00 00 00
```

| Mode | Byte |
|------|------|
| Off | 01 |
| Noise Cancellation | 02 |
| Transparency | 03 |
| Adaptive (Transparency/Audio) | 04 |

Same table in podctl `aap.rs` (`listening_mode`) and LibrePods `control_commands.md` (identifier
0x0D, "ListeningMode"). The AirPods "respond with the same packet after the mode has been
changed" and also emit it when the user changes mode with the stem. Opcode 0x09 is the generic
"control command" family: `04 00 04 00 09 00 <identifier> <d1> <d2> <d3> <d4>`; unused bytes are 0.
No initial-state query is documented for the mode; state arrives after subscribing (podctl
shows the "mode echo" in `watch`; LibrePods doc says CA state is sent once after subscribing). [V, partly I]

### Conversation Awareness [V]

Toggle (identifier 0x28): `04 00 04 00 09 00 28 [01 on | 02 off] 00 00 00`. Current state is sent once
after subscribing in the same shape. The live "you started/stopped speaking" events are a
different frame: `04 00 04 00 4B 00 02 00 01 [level]`, level 01/02 speaking, 03 stopped, 08/09 normal.
The AirPods lower media volume themselves in the Apple way only if the host acts on those events
(the doc says the host "reduce[s] volume"); podctl implements ducking itself via PipeWire. [V]

### Related settings [V]

| Setting | Frame | Notes |
|---------|-------|-------|
| Adaptive strength | `... 09 00 2E [0..100] 00 00 00` | AAP Definitions "Adaptive Audio Noise"; firmware 7A305 on Pro 2 |
| Which modes the stem cycles | `... 09 00 1A <bitmask> 00 00 00` | bitmask Off 0x01, ANC 0x02, Transparency 0x04, Adaptive 0x08 (control_commands.md) |
| "Allow Off option" | identifier 0x34 | name only; semantics not documented |
| One-bud ANC | identifier 0x1B, 01/02 | |

Whether Off is selectable depends on a device setting (0x34). The UI should disable modes the
device refuses, which means we must read back and compare the echo. [I]

### Which models support what

No primary Apple-to-AAP mapping exists. Two layers:

1. What Apple says each model can do: Apple's guide lists Adaptive Audio for AirPods 4 (ANC),
   Pro 2, Pro 3, Max 2, and Conversation Awareness for 4 (ANC), Pro 2, Pro 3
   ([Apple listening modes guide](https://support.apple.com/guide/airpods/dev6d977ff21/web),
   [Adaptive Audio](https://support.apple.com/en-ie/HT213884)); one Apple page also lists AirPods 5.
   Pages disagree on AirPods 5; re-check before relying on it. [V, web search digest only, not read in full]
2. What the reverse-engineered tools assume: podctl's capability matrix (self-described as
   source of truth in `src/caps.rs`; hardware-tested only on Pro 2 USB-C):

| Feature | 1/2/3 | 4 | 4 ANC | Pro | Pro 2 | Pro 3 | Max |
|---------|:-:|:-:|:-:|:-:|:-:|:-:|:-:|
| ANC / Transparency | no | no | yes | yes | yes | yes | yes |
| Adaptive | no | no | yes | no | yes | yes | no |
| Conversation Awareness | no | no | yes | no | yes | yes | no |

([podctl README](https://github.com/Rockykln/podctl/blob/main/README.md)). Pro 3, 4 ANC and Max are
marked unconfirmed by podctl's own roadmap ("AAP captures from AirPods 4 ANC and AirPods Max
to confirm"). **Design implication**: the model list in this package (`protocol.py`, synced with
AppleCP.cpp) is the BLE id; derive `SupportedNoiseModes` from our model table, but verify at
runtime by whether the AirPods echo the mode we set. Plain AirPods 1/2/3/4 have no noise control, so
we must not even open the socket for them. [I]

## 3. Other data AAP carries

| Data | AAP frame | Notes |
|------|-----------|-------|
| Battery | opcode 0x04: `04 00 04 00 04 00 [count] ([component] 01 [level] [status] 01)*`; components Case 08, Left 04, Right 02; status 00 unknown, 01 charging, 02 discharging, 04 disconnected | level is a plain percent here, not the 0..10 scale of BLE. [V] |
| Ear detection | opcode 0x06: `... 06 00 [primary] [secondary]`; 00 in ear, 01 out, 02 in case | primary/secondary swap on mic change. [V] |
| Device info | opcode 0x1D: null-separated strings (name, model number, manufacturer, serial, firmware...) | useful for exact model id. [V] |
| Rename | opcode 0x1A | we already do this via BlueZ Alias. |
| Conversation events | opcode 0x4B (above) | |
| Head tracking | opcode 0x17: start/stop frames, sensor stream (offsets in doc) | out of scope for a top-bar widget. [V] |
| Hearing aid, EQ, transparency customisation | 0x53, `12 18 ...` | out of scope. |
| Proximity/BLE keys | opcodes 0x30/0x31 | would let one decrypt/resolve the rotating BLE address, which would fix our random-address matching problem (README "Known limits"). podctl requests and stores them (`keys.rs`). Sensitive: IRK/encryption key. [V that they exist, I on benefit] |
| Case lid | not in the AAP docs read | stays BLE-only (podctl also uses BLE for the lid: "The case says it much earlier"). [V] |

**Replace or complement?** Complement. AAP exists only while a classic connection and the socket
are up; the case state and lid popup need BLE; and our whole fallback logic (HasAdvertisement,
FallbackBattery) assumes BLE. AAP battery is exact and event-driven, so it could become the
preferred source while connected (and the BLE model/binding heuristics could be cross-checked
by AAP device info). That is a second step; the ticket's scope is noise control. [I]

## 4. Linux feasibility

### Unprivileged socket [V]

Kernel `net/bluetooth/l2cap_sock.c` (master):

- `l2cap_sock_create`: `if (sock->type == SOCK_RAW && !kern && !capable(CAP_NET_RAW)) return -EPERM;` so
  SEQPACKET needs no `CAP_NET_RAW`.
- `l2cap_validate_bredr_psm`: requires odd PSM with low bit of the upper byte 0 (0x1001 passes) and
  `psm < L2CAP_PSM_DYN_START && !capable(CAP_NET_BIND_SERVICE)`; `L2CAP_PSM_DYN_START` is `0x1001`
  (`include/net/bluetooth/l2cap.h`). The check is in `bind`, and 0x1001 is not below the limit
  anyway. Source:
  [l2cap_sock.c](https://github.com/torvalds/linux/blob/master/net/bluetooth/l2cap_sock.c).

A web-search digest claimed BlueZ 5.66+ needs a debugfs `allowed_psm` setting for 0x1001. I grepped
current `l2cap_core.c` and found no such knob, and no tool above mentions it. Treat as unverified
and probably wrong [?].

Corroboration: podctl's INSTALL.md says "needs no root"; LibrePods' own Python helper does
`socket.socket(AF_BLUETOOTH, SOCK_SEQPACKET, BTPROTO_L2CAP).connect((mac, PSM))` (for PSM 31 / ATT)
in [`hearing-aid-adjustments.py`](https://github.com/kavishdevar/librepods/blob/main/linux/hearing-aid-adjustments.py).
Both tools only ever ran this as the logged-in user [V that they document no sudo; I that all users
tested unprivileged]. Locally, Python 3.14.8 on Arch exposes `socket.AF_BLUETOOTH`/`BTPROTO_L2CAP`. [V]
Debian trixie python3 (3.13): not checked [?]; verify it is built with Bluetooth support in the spike.

### Options compared

| Approach | Verdict |
|----------|---------|
| Python `socket` AF_BLUETOOTH / BTPROTO_L2CAP / SOCK_SEQPACKET, `connect((addr, 0x1001))` | **Recommended.** No new dependency. Make it non-blocking and register the fd with `GLib.io_add_watch` (connect completion via `POLLOUT`, then `POLLIN`) so it sits in the existing GLib main loop. podctl uses a non-blocking connect with deadline, 5 s timeout, and a cancel flag. |
| BlueZ `ProfileManager1.RegisterProfile` with client UUID `74ec2172-...` and option `PSM` | Plausible: BlueZ docs say `PSM` is "used for client and server UUIDs" and, for client UUIDs, `AutoConnect` "will force connection of the RFCOMM or L2CAP channels when a remote device is connected" ([ProfileManager docs](https://github.com/bluez/bluez/blob/master/doc/org.bluez.ProfileManager.rst)); `Profile1.NewConnection(device, fd, props)` then hands an fd. **Not used by any AAP tool I found, so unproven**; also it would make bluetoothd own the connection lifecycle, which is nice (no polling), but adds a D-Bus-served object and an SDP lookup dependency. Candidate for the spike's second experiment. [I] |
| `bluetoothctl` / GATT | Not applicable: AAP is not GATT. (LibrePods' ATT use, PSM 31 / handles, is for hearing-aid settings only.) [V] |
| Raw HCI / sniffing | Needs root/`CAP_NET_RAW`; not needed. |

### Conflicts with BlueZ and audio

- No source documents a conflict with bluetoothd: the socket sits next to A2DP/AVRCP/HFP channels on
  the same ACL link, and LibrePods Rust and podctl run beside normal desktop audio. LibrePods'
  README only asks for `bluez5.dummy-avrcp-player = true` in WirePlumber for stem media keys and says
  not to run `mpris-proxy` alongside it; unrelated to AAP. [V for those docs, I for "no conflict"]
- podctl states audio features go through PipeWire/PulseAudio independently of the AAP socket. [V]
- AAP is an application channel; opening it should not change the audio profile. Needs a spike
  check: run `pw-top`/`wpctl status` while connecting. [?]
- Only one AAP session at a time per AirPods is plausible (LibrePods has an "owns connection"
  setting 0x06). Do not run LibrePods and our daemon together. [I]
- The AirPods must have the classic link up; connect before that fails with ENOTCONN/timeout. Our
  daemon already knows `Connected` from `Device1`. [V/I]
- DeviceID: LibrePods asks users to set `DeviceID = bluetooth:004C:0000:0000` in
  `/etc/bluetooth/main.conf` for *hearing aid* features; podctl says it "works without it" and
  documents it as "purely for the extras" and host-wide. Noise control does not need it. We should
  not require or document it. [V]

### BlueZ versions

Nothing in the kernel code or tools is BlueZ-version gated; the socket is a kernel feature and
needs only that the device is connected. podctl lists "BlueZ 5.x"; LibrePods Rust uses the `bluer`
crate (libbluetooth-free). Our own daemon already runs unprivileged on 5.87 without `--experimental`
(see `bluez.py`). Debian 5.82 is untested in any source [?]; it should behave the same. Test on both.

## 5. Existing implementations

| Project | License | Maturity | What to learn |
|---------|---------|----------|---------------|
| [kavishdevar/librepods](https://github.com/kavishdevar/librepods) | GPL-3.0 | ~30.6k stars, pushed 2026-10-28; Android main, Linux Qt app (`linux/`, noted as superseded) and a Rust rewrite on branch `linux/rust` (PR #241) | Canonical protocol docs (`docs/AAP Definitions.md`, `opcodes.md`, `control_commands.md`); full handshake state machine in `main.cpp`; Rust `aacp.rs` with `bluer` L2CAP socket |
| [Rockykln/podctl](https://github.com/Rockykln/podctl) | MIT OR Apache-2.0 | created 2026-05-20, 11 stars, active; hardware-verified on Pro 2 USB-C only; ships a GNOME Shell extension in `contrib/` | Closest to our design: unprivileged raw L2CAP via libc, per-model capability table, parser robustness tests, NOTICE on GPL provenance |
| Wireshark dissector / "AAP-Protocol-Definition" (tyalie) | not checked | LibrePods credits it as the first protocol documentation; I could not resolve the repo URL via `gh` [?] | Cross-check of frame layouts |
| lifeart "Inside Apple AirPods auto-switching" write-up | n/a | one blog post found by search; not read [?] | Background only |
| OpenPods, AirStatus, pods-ctl | not examined | BLE-advertisement-only tools (battery); podctl cites OpenPods' BLE table | Same role as our BLE side; no AAP |

Note LibrePods' own docs are inconsistent in places (feature flag mask, subscribe length, doc
file links to `/docs/*.md` pages that may not exist). Our spike should capture real traffic and
treat the docs as hints, not contracts.

## 6. Risks

| Risk | Detail | Mitigation |
|------|--------|-----------|
| Undocumented, reverse engineered | No Apple spec; tested on one model | Gate by model, tolerate unknown frames, never fail the daemon on a parse error, log hex at debug |
| Firmware drift | LibrePods doc header says "As per AirPods Pro 2 (USB-C) Firmware 7A305"; new features appear with firmware | Feature flag in config; contract tests with captured frames |
| Handshake variance | Mask `d7` vs `ff`, 4 vs 5 subscribe bytes, ack only tested on Pro 2 | Spike matrix; send the exact LibrePods/podctl bytes verbatim |
| Session instability | AirPods may end a session (`02 00 04 00`); LibrePods warns AirPods "may disconnect after a short period" when they expect Apple behaviour (hearing-aid context) | Reconnect with back-off; stay read-only if the link flaps |
| Write channel | The socket can change settings, rename, and carry key material (opcodes 0x30/0x31) | Whitelist: only send handshake, features, subscribe, `09 00 0D` and `09 00 28`; never expose a raw-frame method on D-Bus; never log key frames (podctl redacts them) |
| D-Bus surface | Any session-bus client could set mode | Same trust as existing rw properties (EarDetection, etc.); validate input to the four known values |
| Battery/connection side effects | Not documented. Opening AAP is what an iPhone does continuously; subscribing keeps a radio channel active | Measure; open lazily when the popup/panel is used, close after idle [?] |
| Per-model differences | See section 2; Off may be disallowed (0x34); Max has no case; Adaptive absent on Pro 1 | Capability table + echo verification |
| Licensing | LibrePods GPL-3.0; this package is MIT-ish | Do not copy code. Protocol facts (opcodes, layouts) are factual; podctl takes the same stance in NOTICE.md. Write our own parser from the spec, cite LibrePods in docs. If any code is ever adapted, license implications change |
| Maintenance | Single-maintainer upstreams | Pin our own captured test vectors |
| Legal/branding | Apple trademarks in names | Not in scope of code; keep "AirPods" usage descriptive as the package does today |

## 7. Recommendation and plan

**Spike first (go conditional on 3 checks), then implement read + set of noise mode only.** Defer
battery-over-AAP, CA toggle and proximity keys.

### Spike plan (about half a day, real AirPods Pro 2 / 4 ANC, trixie and Arch)

Throwaway script `scripts/aap_spike.py` (like `scan_spike.py`), run as a normal user:

1. Pair and connect the AirPods with `bluetoothctl`; play audio.
2. `socket(AF_BLUETOOTH, SOCK_SEQPACKET, BTPROTO_L2CAP)`, `connect((addr, 0x1001))`. Record the
   errno on failure. Check **no root, no capabilities** (`getpcaps $$`).
3. Send handshake; print every received frame as hex with timestamps for 30 s.
4. Send set-features (`d7` run, then `ff` run) and subscribe (4 vs 5 byte). Record which
   combination yields frames `...04 00` (battery), `...06 00` (ear), `...09 00 0D` (mode).
5. Press and hold the stem: confirm a `09 00 0D <mode>` frame arrives.
6. Send each of the four mode frames in turn; confirm echo and audible change. Try Adaptive both
   with and without the set-features frame.
7. While running: `pw-top`/`wpctl status`/`bluetoothctl info` to confirm no codec/profile change,
   no audio glitch, and note whether `scan_spike.py` still sees advertisements.
8. Kill the process mid-session and re-run: does the AirPods accept a fresh session without
   reconnecting the classic link? Test connect with audio idle too.
9. Bonus: second experiment with `RegisterProfile` (client UUID `74ec2172-0bad-4d01-8f77-997b2be0722a`, `PSM=0x1001`, `AutoConnect`) and see if `NewConnection` receives an fd.

Go/no-go checks: (a) works unprivileged on both distros, (b) mode set and echo works without audio
impact, (c) session survives 10+ minutes. If (a) fails on trixie (e.g. kernel or Python build),
no-go until fixed; if (c) fails, ship read-only status or defer.

### D-Bus additions (additive, per the contract's rules)

```xml
<!-- Active mode: "off", "anc", "transparency", "adaptive", or "" when unknown/unsupported. -->
<property name="NoiseMode" type="s" access="readwrite"/>
<!-- Subset of the four values the connected model supports, in UI order. Empty when the
     model has no noise control or AAP is not established. -->
<property name="SupportedNoiseModes" type="as" access="read"/>
<!-- True while the AAP session is up. NoiseMode is writable only then. -->
<property name="AapConnected" type="b" access="read"/>
<!-- Optional later: -->
<property name="ConversationAwareness" type="b" access="readwrite"/>
```

Following the file's conventions: unavailable => `""`, empty array, `false`. A write while
`AapConnected` is false or with an unsupported value fails with `org.pulsaros.AirPods.Error.Failed`
(as `Connect` does). Strings, not enum ints, mirror `PanelBattery`. [I]

### Daemon design (fits current layout)

- New pure module `aap.py` (frame encode/decode, no I/O) with tests next to `test_protocol_state.py`.
- New `aap_link.py` (socket + GLib watch + handshake state machine + reconnect), constructed like
  `BluezBackend` with an injectable `connect` function, so tests can inject a socketpair.
- `Controller` gets `on_aap_mode(mode)` and publishes the new fields in `Snapshot`; `service.py`
  adds the properties; `daemon.py` starts the link when `Connected` and the model supports noise control, stops on disconnect.
- Settings: optional `config.py` toggle (`NoiseControl` rw property if we want an opt-out).

### Extension UI

macOS-style segmented control inside the existing popup card: Off | Noise Cancellation | Transparency
| Adaptive, built from `St.BoxLayout` buttons, hidden when `SupportedNoiseModes` is empty and
disabled with a spinner while a write is in flight; labels through `_()` and `i18n/es.json`
(`test_i18n.py` already enforces catalogs); sync both ways from the property change signal, so stem
changes show up. `interface.js` embeds the contract and `test_extension_contract.py` already checks
drift, so update both together. [I]

### Test seams (no hardware)

1. Pure codec tests with the byte tables above as vectors (including a golden handshake).
2. `socket.socketpair(AF_UNIX, SOCK_SEQPACKET)` as a fake L2CAP peer: a small `fake_airpods.py` in
   `tests/` that replies to handshake and echoes mode frames, can inject battery, a stem change, a
   `02 00 04 00` session end, and silence. SEQPACKET on AF_UNIX preserves message boundaries like L2CAP.
3. Extend `fake_bluez.py`/`test_wire_contract.py`: daemon with an injected AAP address factory
   (e.g. `--aap-socket-factory` env var or test-only hook) to observe NoiseMode over D-Bus only,
   the same seam as the existing integration tests.
4. A one-time hardware capture file (hex) committed under `tests/data/` for regression.

### Effort

| Work | Estimate |
|------|----------|
| Hardware spike and write-up | 0.5 day |
| `aap.py` codec + tests | 0.5 day |
| `aap_link.py` + reconnect + fake peer + tests | 1.5 to 2 days |
| Controller/service/contract + wire tests | 1 day |
| Extension segmented control, i18n, contract sync | 1 to 2 days |
| Multi-model checks (needs access to a 4 ANC / Pro 3 / Max) | 1+ day, hardware-bound |

## Sources

Verified by reading:

- LibrePods repo (GPL-3.0): https://github.com/kavishdevar/librepods
  - AAP Definitions: https://github.com/kavishdevar/librepods/blob/main/docs/AAP%20Definitions.md
  - Opcodes: https://github.com/kavishdevar/librepods/blob/main/docs/opcodes.md
  - Control commands: https://github.com/kavishdevar/librepods/blob/main/docs/control_commands.md
  - Linux packets: https://github.com/kavishdevar/librepods/blob/main/linux/airpods_packets.h
  - Linux app and handshake flow: https://github.com/kavishdevar/librepods/blob/main/linux/main.cpp
  - Linux README: https://github.com/kavishdevar/librepods/blob/main/linux/README.md
  - Python L2CAP helper: https://github.com/kavishdevar/librepods/blob/main/linux/hearing-aid-adjustments.py
  - Rust rewrite (branch `linux/rust`): https://github.com/kavishdevar/librepods/blob/linux/rust/linux-rust/src/bluetooth/aacp.rs
- podctl (MIT OR Apache-2.0): https://github.com/Rockykln/podctl (README.md, INSTALL.md, NOTICE.md,
  `src/aap.rs`, `src/l2cap.rs`, `src/caps.rs`, `src/bin/daemon/aap.rs`, `contrib/gnome-shell-extension`)
- Linux kernel L2CAP sockets: https://github.com/torvalds/linux/blob/master/net/bluetooth/l2cap_sock.c
  and https://github.com/torvalds/linux/blob/master/include/net/bluetooth/l2cap.h
- BlueZ ProfileManager: https://github.com/bluez/bluez/blob/master/doc/org.bluez.ProfileManager.rst
  and Profile: https://github.com/bluez/bluez/blob/master/doc/org.bluez.Profile.rst
- librepods-org mirror (GPL-3.0, listing modes on Linux): https://github.com/librepods-org/librepods

Seen only as search results or digests (not read in full, treat as unverified):

- Apple, listening modes: https://support.apple.com/guide/airpods/dev6d977ff21/web
- Apple, Adaptive Audio: https://support.apple.com/en-ie/HT213884
- podctl search summary of `allowed_psm` claim and lifeart write-up: https://dev.to/lifeart/inside-apple-airpods-auto-switching-bluetooth-continuity-aap-python-packet-sniffing-jf6
- Not found: tyalie/AAP-Protocol-Definition (cited by LibrePods; `gh` returned 404 for that path)
