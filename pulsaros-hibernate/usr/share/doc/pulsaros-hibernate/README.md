# pulsaros-hibernate

Mejoras de hibernación para Pulsar OS.

## Componentes

### 1. Feedback visual (`/usr/lib/pulsaros/sleep-progress`)

Drop-ins en `systemd-hibernate.service` y `systemd-suspend-then-hibernate.service`:

- `ExecStartPre`: arranca `plymouthd` si no está activo, muestra el splash del
  tema Pulsar con el mensaje «Guardando la sesión en disco…». Cubre tanto el
  volcado de VRAM de NVIDIA (`NVreg_PreserveVideoMemoryAllocations=1`, puede
  tardar minutos) como la escritura de la imagen por el kernel.
- `ExecStartPost`: oculta el splash y cierra `plymouthd` al reanudar.

### 2. Guardián del offset de reanudación (`/usr/lib/pulsaros/verify-resume-offset`)

Servicio `pulsaros-verify-resume-offset.service` (oneshot en el arranque):

- Calcula el offset real del primer extent de `/swapfile` con `filefrag`.
- Lo compara con `resume_offset` de la cmdline y `/sys/power/resume_offset`.
- Si difiere (swapfile recreado/movido), actualiza `/boot/refind_linux.conf`
  automáticamente (copia de seguridad en `.bak`) y avisa si GRUB está
  desincronizado.
- Modo prueba: `verify-resume-offset --dry-run`.

## Prueba de ciclo completo

```bash
sudo systemctl hibernate   # o "Apagar con recuperación" desde la sesión
```
