# Pulsar OS Settings

The system Settings app of Pulsar OS and the macOS-style design it follows.

## Language

**Settings app**:
The `pulsaros-control-center` package: a patched `gnome-control-center` whose look and behavior follow macOS System Settings.
_Avoid_: Control Center, control panel, preferences

**Quick Settings**:
The macOS-style toggle panel in the top bar, shipped as the `Pulsar-CC-27-Gnome` shell extension. Not part of the Settings app.
_Avoid_: Control Center, quick panel

**Reference set**:
The macOS 26/27 System Settings screenshots in `~/Pictures/control-center/` that define the target look.
_Avoid_: mockups, designs

**Principal branch**:
`feat/control-center-macos`, the long-lived integration branch off `main`. All feature branches target it.
_Avoid_: epic branch, main feature branch

**Test loop**:
Build the `gnome-control-center` package, install it in a QEMU VM booted from the prebuilt Arch ISO, and see the Settings app run. Needed because the dev host (Omarchy) cannot run the patch.
_Avoid_: VM test, smoke test
