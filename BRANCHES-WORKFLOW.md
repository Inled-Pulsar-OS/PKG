# 🌿 Guía de Ramas y Canales en Pulsar OS (PKG)

Esta guía explica cómo funciona el sistema de canales **`stable`** y **`unstable`** para empaquetar, desplegar y promocionar paquetes de Pulsar OS.

---

## 📌 Canales Disponibles

- **`unstable` (Desarrollo / Testing):**
  - Canal por defecto al desarrollar nuevas características o realizar correcciones.
  - Al compilar para Debian, añade el sufijo `-unstable` para permitir actualizaciones continuas sin colisionar con versiones estables.
  - Se notifica a `apt.inled.es` para que se incorpore en la rama `unstable`.

- **`stable` (Producción / Versión Oficial):**
  - Canal para versiones definitivas que se incluirán en las ISOs oficiales estables.
  - Genera versiones limpias y consolidadas.

---

## 🚀 Cómo Desplegar un Paquete

### Opción A: Desde GitHub Actions (Recomendado)
1. Ve a la pestaña **Actions** del repositorio `PKG`.
2. Selecciona **Deploy Package to Inled APT**.
3. Haz clic en **Run workflow**:
   - **Nombre del paquete:** `pulsaros-branding` (o varios separados por comas, o `all`).
   - **Rama de destino:** Elige `unstable` para probar o `stable` para lanzamiento oficial.

### Opción B: Mediante Issue Declarativo
1. Abre un nuevo issue usando la plantilla **🚀 Desplegar Paquete Pulsar OS**.
2. Rellena el nombre del paquete y selecciona en el desplegable la rama (`unstable` o `stable`).

### Opción C: Desde la línea de comandos (Local)
```bash
# Compilar y desplegar a 'unstable' (Debian + Arch)
./package-and-deploy.sh pulsaros-theme --branch unstable --deploy

# Compilar y desplegar todos los paquetes a 'unstable'
./package-and-deploy.sh all --branch unstable --deploy

# Compilar y desplegar directamente a 'stable'
./package-and-deploy.sh pulsaros-theme --branch stable --deploy
```

---

## 🌟 Cómo Promocionar Paquetes de `unstable` a `stable`

Cuando hayas probado un paquete en `unstable` (por ejemplo probándolo en una ISO unstable o en tu máquina local) y quieras pasarlo a `stable`:

### 1. Desde GitHub Actions
- Ejecuta el workflow **🌟 Promote Package to Stable** e introduce el nombre del paquete (o `all`).

### 2. Desde la Línea de Comandos
```bash
# Promociona un paquete específico
./promote-package.sh pulsaros-theme

# Promociona todos los paquetes a stable
./promote-package.sh all
```
El script compilará la versión limpia para Debian y Arch, la subirá a la release de paquetes y notificará a `apt.inled.es` para actualizar la rama `stable`.
