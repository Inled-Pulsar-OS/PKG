#!/bin/bash
# Pulsar OS - debug helper for the Spotlight shortcut.
# Run this in the live session (as the desktop user), then press the shortcut
# and inspect the log. Usage:
#   debug-spotlight.sh setup    -> redirect shortcut to the logging wrapper
#   debug-spotlight.sh restore  -> put the original command back
# After pressing the shortcut:
#   cat /tmp/pulsaros-spotlight-debug.log
#   ps aux | grep -i spotlight
set -u

KEYDIR='/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/'
SCHEMA='org.gnome.settings-daemon.plugins.media-keys.custom-keybinding'
LOG=/tmp/pulsaros-spotlight-debug.log

case "${1:-setup}" in
  setup)
    rm -f "$LOG"
    echo "== Atajo actual =="
    gsettings get "$SCHEMA:$KEYDIR" binding
    gsettings get "$SCHEMA:$KEYDIR" command
    echo "== Redirigiendo custom0 a /usr/bin/pulsaros-spotlight-debug =="
    gsettings set "$SCHEMA:$KEYDIR" command '/usr/bin/pulsaros-spotlight-debug'
    gsettings get "$SCHEMA:$KEYDIR" command
    echo
    echo "Pulsa el atajo (Win+Espacio / Ctrl+Espacio)."
    echo "Despues mira el log:"
    echo "  cat $LOG"
    echo "  ps aux | grep -i spotlight"
    echo
    echo "Opcional - ver forwarding de activacion en el bus de sesion:"
    echo "  dbus-monitor \"interface='org.freedesktop.Application',member='Activate'\""
    ;;
  restore)
    echo "== Restaurando comando original =="
    gsettings set "$SCHEMA:$KEYDIR" command 'pulsaros-spotlight'
    gsettings get "$SCHEMA:$KEYDIR" command
    ;;
  *)
    echo "Uso: $0 [setup|restore]" >&2
    exit 1
    ;;
esac
