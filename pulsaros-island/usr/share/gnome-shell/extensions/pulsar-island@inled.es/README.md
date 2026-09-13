# Pulsar OS - Clipboard & Drop Island

A floating macOS-style island for GNOME Shell (Pulsar OS).

- Appears when you **copy text or an image** to the clipboard.
- Appears when a new **screenshot** is saved to `~/Pictures/Screenshots`.
- Holds the content: preview for images, truncated text for text, filename for files.
- You can **drag the island's content** into any window to drop it there.
- Drop **files onto the island** to hold them and drag them elsewhere later.

## Notes on drag support

Clipboard and screenshot detection use Mutter's `MetaSelection` `owner-changed`
signal, which works on both X11 and Wayland. Dragging *into* the island uses
gnome-shell's internal DND; dragging *out* to regular windows is bridged by the
shell. Detecting a drag *started in another application* is not currently
possible with public gnome-shell APIs on Wayland — see the extension README
discussion in the Pulsar OS repo.

## Install

```sh
sudo cp -r usr /.
sudo glib-compile-schemas /usr/share/gnome-shell/extensions/pulsar-island@inled.es/schemas/
# then log out and back in, and enable:
gnome-extensions enable pulsar-island@inled.es
```
