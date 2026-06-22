#!/bin/bash
# ==============================================================================
# Pulsar OS - Package and Deploy Utility
# ==============================================================================
# This script packages a folder under the repository root into a Debian package (.deb)
# and notifies the Inled central APT repository for deployment.
#
# Usage:
#   ./package-and-deploy.sh <package_folder_name> [--deploy]
#
# Requirements for deploy:
#   INLED_REPO_PAT environment variable must be set.
# ==============================================================================

set -e

# Configuración básica
OWNER_REPO="Inled-Pulsar-OS/PKG" # Repositorio de paquetes
RELEASE_TAG="packages-repo"
PKG_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="$PKG_DIR/build"
STAGING_DIR="$BUILD_DIR/pkg-staging"
OUTPUT_DIR="$BUILD_DIR/packages"

# Parámetros
PACKAGE_NAME="$1"
DEPLOY_FLAG="$2"

if [ -z "$PACKAGE_NAME" ]; then
    echo "❌ Error: Debes especificar el nombre de la carpeta del paquete a compilar."
    echo "Ejemplo: $0 pulsaros-branding"
    exit 1
fi

SOURCE_FOLDER="$PKG_DIR/$PACKAGE_NAME"

if [ ! -d "$SOURCE_FOLDER" ]; then
    echo "❌ Error: La carpeta del paquete '$SOURCE_FOLDER' no existe."
    exit 1
fi

if [ ! -f "$SOURCE_FOLDER/DEBIAN/control" ]; then
    echo "❌ Error: No se encontró el archivo de control en '$SOURCE_FOLDER/DEBIAN/control'."
    exit 1
fi

# 1. Preparar directorios de salida
mkdir -p "$STAGING_DIR"
mkdir -p "$OUTPUT_DIR"

# Limpieza previa de staging para este paquete
rm -rf "$STAGING_DIR/$PACKAGE_NAME"
mkdir -p "$STAGING_DIR/$PACKAGE_NAME"

echo "📦 Preparando archivos para el paquete: $PACKAGE_NAME..."
# Copiar estructura
cp -r "$SOURCE_FOLDER/." "$STAGING_DIR/$PACKAGE_NAME/"

# 2. Hooks dinámicos de preparación por paquete
# Permite clonar repositorios externos o descargar cosas pesadas en caliente para no subirlas a git.
PREPARE_HOOK="$SOURCE_FOLDER/prepare-assets.sh"
if [ -f "$PREPARE_HOOK" ]; then
    echo "🚀 Ejecutando script de preparación del paquete..."
    bash "$PREPARE_HOOK" "$STAGING_DIR/$PACKAGE_NAME"
fi

# 3. Ajustar permisos críticos
echo "⚙️ Ajustando permisos y propietarios..."
# El script postinst y otros hooks de debian deben ser ejecutables
if [ -d "$STAGING_DIR/$PACKAGE_NAME/DEBIAN" ]; then
    find "$STAGING_DIR/$PACKAGE_NAME/DEBIAN" -type f -exec chmod 755 {} \;
fi

# Configuración de sudoers y polkit exige permisos estrictos
if [ -d "$STAGING_DIR/$PACKAGE_NAME/etc/sudoers.d" ]; then
    find "$STAGING_DIR/$PACKAGE_NAME/etc/sudoers.d" -type f -exec chmod 0440 {} \;
fi
if [ -d "$STAGING_DIR/$PACKAGE_NAME/etc/polkit-1" ]; then
    find "$STAGING_DIR/$PACKAGE_NAME/etc/polkit-1" -type f -exec chmod 0644 {} \;
fi

# 4. Construir el paquete .deb usando dpkg-deb
# Usamos fakeroot para que mantenga permisos correctos de root dentro del paquete sin requerir root en el host
DEB_FILE_NAME=""
echo "🔨 Compilando paquete debian con dpkg-deb..."
if command -v fakeroot >/dev/null 2>&1; then
    fakeroot dpkg-deb --build "$STAGING_DIR/$PACKAGE_NAME" "$OUTPUT_DIR/"
else
    echo "⚠️ Advertencia: 'fakeroot' no está instalado. Si se ejecuta como usuario no root, los permisos del paquete debian podrían ser incorrectos."
    dpkg-deb --build "$STAGING_DIR/$PACKAGE_NAME" "$OUTPUT_DIR/"
fi

# Leer el nombre exacto del archivo generado basándonos en el control
PKG_VERSION=$(grep "^Version:" "$SOURCE_FOLDER/DEBIAN/control" | cut -d' ' -f2)
PKG_ARCH=$(grep "^Architecture:" "$SOURCE_FOLDER/DEBIAN/control" | cut -d' ' -f2)
DEB_FILE="${OUTPUT_DIR}/${PACKAGE_NAME}_${PKG_VERSION}_${PKG_ARCH}.deb"

if [ ! -f "$DEB_FILE" ]; then
    # Fallback si dpkg-deb lo nombró diferente
    DEB_FILE=$(ls "$OUTPUT_DIR/${PACKAGE_NAME}"*.deb | head -n 1)
fi

echo "✅ Paquete compilado con éxito: $(basename "$DEB_FILE")"

# 5. Desplegar al Repositorio APT de Inled si se solicita
if [ "$DEPLOY_FLAG" == "--deploy" ] || [ "$DEPLOY_FLAG" == "-d" ]; then
    echo "🌐 Iniciando proceso de despliegue al repositorio APT..."
    
    if [ -z "$INLED_REPO_PAT" ]; then
        echo "❌ Error: La variable de entorno INLED_REPO_PAT no está definida."
        echo "Es requerida para interactuar con la API de GitHub y notificar al repositorio."
        exit 1
    fi
    
    # Si estamos en GitHub Actions, podemos usar 'gh' CLI para subir el asset a una release
    if command -v gh >/dev/null 2>&1 && [ -n "$GITHUB_ACTIONS" ]; then
        echo "🖥️ Usando GitHub CLI para subir el paquete..."
        # Asegurarse de que el tag de la release existe, si no lo crea
        gh release view "$RELEASE_TAG" --repo "$OWNER_REPO" >/dev/null 2>&1 || {
            echo "Creando release temporal '$RELEASE_TAG'..."
            gh release create "$RELEASE_TAG" --title "PulsarOS Package Repository Assets" --notes "Repositorio de paquetes deb compilados automáticamente." --repo "$OWNER_REPO"
        }
        
        # Subir el deb a la release
        echo "Subiendo $(basename "$DEB_FILE") a la release '$RELEASE_TAG'..."
        gh release upload "$RELEASE_TAG" "$DEB_FILE" --clobber --repo "$OWNER_REPO"
        
        PACKAGE_URL="https://github.com/${OWNER_REPO}/releases/download/${RELEASE_TAG}/$(basename "$DEB_FILE")"
    else
        # Si es local y no está 'gh', intentaremos con curl usando la API de GitHub para subir
        echo "🖥️ Usando la API de GitHub con curl..."
        
        # Obtener ID de la release o crearla
        RELEASE_ID=$(curl -s -H "Authorization: token $INLED_REPO_PAT" \
            "https://api.github.com/repos/${OWNER_REPO}/releases/tags/${RELEASE_TAG}" | jq -r '.id')
            
        if [ "$RELEASE_ID" == "null" ] || [ -z "$RELEASE_ID" ]; then
            echo "Creando la release '$RELEASE_TAG'..."
            RELEASE_DATA=$(curl -s -X POST -H "Authorization: token $INLED_REPO_PAT" \
                -H "Content-Type: application/json" \
                -d "{\"tag_name\":\"$RELEASE_TAG\",\"title\":\"PulsarOS Package Repository Assets\",\"body\":\"Repositorio de paquetes deb compilados.\"}" \
                "https://api.github.com/repos/${OWNER_REPO}/releases")
            RELEASE_ID=$(echo "$RELEASE_DATA" | jq -r '.id')
        fi
        
        # Subir el archivo
        FILE_NAME=$(basename "$DEB_FILE")
        echo "Subiendo archivo $FILE_NAME a la release ID: $RELEASE_ID..."
        
        # Eliminar asset previo con el mismo nombre si existe
        ASSET_ID=$(curl -s -H "Authorization: token $INLED_REPO_PAT" \
            "https://api.github.com/repos/${OWNER_REPO}/releases/$RELEASE_ID" | \
            jq -r ".assets[] | select(.name==\"$FILE_NAME\") | .id")
            
        if [ -n "$ASSET_ID" ] && [ "$ASSET_ID" != "null" ]; then
            echo "Eliminando asset duplicado anterior (ID: $ASSET_ID)..."
            curl -s -X DELETE -H "Authorization: token $INLED_REPO_PAT" \
                "https://api.github.com/repos/${OWNER_REPO}/releases/assets/$ASSET_ID"
        fi
        
        UPLOAD_URL="https://uploads.github.com/repos/${OWNER_REPO}/releases/${RELEASE_ID}/assets?name=${FILE_NAME}"
        UPLOAD_RESPONSE=$(curl -s -X POST -H "Authorization: token $INLED_REPO_PAT" \
            -H "Content-Type: application/vnd.debian.binary-package" \
            --data-binary @"$DEB_FILE" \
            "$UPLOAD_URL")
            
        PACKAGE_URL=$(echo "$UPLOAD_RESPONSE" | jq -r '.browser_download_url')
    fi
    
    if [ "$PACKAGE_URL" == "null" ] || [ -z "$PACKAGE_URL" ]; then
        echo "❌ Error: Falló la subida del paquete a la release de GitHub."
        exit 1
    fi
    
    echo "🎉 Paquete disponible en: $PACKAGE_URL"
    echo "📡 Enviando evento repository_dispatch al repositorio central de Inled APT..."
    
    DISPATCH_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
         -H "Accept: application/vnd.github.v3+json" \
         -H "Authorization: token $INLED_REPO_PAT" \
         https://api.github.com/repos/InledGroup/apt/dispatches \
         -d "{\"event_type\": \"package_upload\", \"client_payload\": {\"package_url\": \"$PACKAGE_URL\"}}")
         
    if [ "$DISPATCH_RESPONSE" -eq 204 ] || [ "$DISPATCH_RESPONSE" -eq 200 ] || [ "$DISPATCH_RESPONSE" -eq 201 ]; then
        echo "🚀 ¡Despliegue notificado con éxito! HTTP $DISPATCH_RESPONSE"
    else
        echo "❌ Error al notificar al repositorio APT de Inled. Código HTTP recibido: $DISPATCH_RESPONSE"
        exit 1
    fi
fi
