#!/bin/bash
# Pulsar OS - NVIDIA hibernate hook
# Este script se ejecuta DESPUÉS de que systemd congela los procesos de usuario,
# por lo que gnome-shell ya no tiene abierto /dev/nvidia*, y podemos descargar el módulo.

case "$1" in
    pre)
        case "$2" in
            hibernate|suspend-then-hibernate)
                # Descargar módulos NVIDIA para que el kernel pueda volcar sin interferencias
                modprobe -r nvidia_drm nvidia_modeset nvidia_uvm nvidia 2>/dev/null
                # Si la descarga falla (improbable aquí), intentar forzar runtime suspend
                echo auto > /sys/bus/pci/devices/0000:04:00.0/power/control 2>/dev/null || true
                ;;
        esac
        ;;
    post)
        case "$2" in
            hibernate|suspend-then-hibernate)
                # Recargar módulos NVIDIA tras reanudar
                modprobe nvidia 2>/dev/null
                modprobe nvidia_uvm 2>/dev/null
                modprobe nvidia_modeset 2>/dev/null
                modprobe nvidia_drm 2>/dev/null
                ;;
        esac
        ;;
esac
exit 0
