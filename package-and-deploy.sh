#!/bin/bash
# ==============================================================================
# Pulsar OS - Package and Deploy Utility
# ==============================================================================
# This script packages one or all folders under the repository root into .deb packages
# and notifies the Inled central APT repository for deployment.
#
# Usage:
#   ./package-and-deploy.sh <package_folder_name | all> [--deploy]
#
# Examples:
#   ./package-and-deploy.sh pulsaros-theme            # Compila un paquete en local
#   ./package-and-deploy.sh all                       # Compila TODOS los paquetes en local
#   ./package-and-deploy.sh all --deploy              # Compila y despliega TODOS a Inled APT
# ==============================================================================

set -e

# Configuración básica
OWNER_REPO="Inled-Pulsar-OS/PKG"
RELEASE_TAG="packages-repo"
PKG_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="$PKG_DIR/build"
STAGING_DIR="$BUILD_DIR/pkg-staging"
OUTPUT_DIR="$BUILD_DIR/packages"

# Parámetros
PACKAGE_NAME="$1"
DEPLOY_FLAG="$2"

if [ -z "$PACKAGE_NAME" ]; then
    echo "❌ Error: Debes especificar el nombre de la carpeta del paquete o 'all'."
    echo "Ejemplo: $0 pulsaros-branding"
    exit 1
fi

# Función para compilar y desplegar un único paquete
build_single_package() {
    local name="$1"
    local source_folder="$PKG_DIR/$name"
    
    echo "=============================================================================="
    echo "📦 INICIANDO COMPILACIÓN DE: $name"
    echo "=============================================================================="
    
    if [ ! -d "$source_folder" ]; then
        echo "❌ Error: La carpeta '$source_folder' no existe."
        return 1
    fi
    
    if [ ! -f "$source_folder/DEBIAN/control" ]; then
        echo "❌ Error: No se encontró '$source_folder/DEBIAN/control'."
        return 1
    fi
    
    # Limpieza previa de staging
    rm -rf "$STAGING_DIR/$name"
    mkdir -p "$STAGING_DIR/$name"
    
    # Copiar archivos
    cp -r "$source_folder/." "$STAGING_DIR/$name/"
    
    # Hook de preparación
    local prepare_hook="$source_folder/prepare-assets.sh"
    if [ -f "$prepare_hook" ]; then
        echo "🚀 Ejecutando script de preparación del paquete..."
        bash "$prepare_hook" "$STAGING_DIR/$name"
        # Eliminar el script del directorio staging para evitar colisiones en la raíz del deb
        rm -f "$STAGING_DIR/$name/prepare-assets.sh"
    fi
    
    # Ajustar permisos
    echo "⚙️ Ajustando permisos..."
    if [ -d "$STAGING_DIR/$name/DEBIAN" ]; then
        find "$STAGING_DIR/$name/DEBIAN" -type f -exec chmod 755 {} \;
    fi
    if [ -d "$STAGING_DIR/$name/etc/sudoers.d" ]; then
        find "$STAGING_DIR/$name/etc/sudoers.d" -type f -exec chmod 0440 {} \;
    fi
    if [ -d "$STAGING_DIR/$name/etc/polkit-1" ]; then
        find "$STAGING_DIR/$name/etc/polkit-1" -type f -exec chmod 0644 {} \;
    fi
    
    # Ejecutar dpkg-deb
    local deb_file=""
    echo "🔨 Ejecutando dpkg-deb..."
    if command -v fakeroot >/dev/null 2>&1; then
        fakeroot dpkg-deb --build "$STAGING_DIR/$name" "$OUTPUT_DIR/"
    else
        echo "⚠️ Advertencia: 'fakeroot' no instalado. Se compilará con los permisos del host."
        dpkg-deb --build "$STAGING_DIR/$name" "$OUTPUT_DIR/"
    fi
    
    # Obtener el nombre del deb
    local version=$(grep "^Version:" "$source_folder/DEBIAN/control" | cut -d' ' -f2)
    local arch=$(grep "^Architecture:" "$source_folder/DEBIAN/control" | cut -d' ' -f2)
    deb_file="${OUTPUT_DIR}/${name}_${version}_${arch}.deb"
    
    if [ ! -f "$deb_file" ]; then
        deb_file=$(ls "$OUTPUT_DIR/${name}"*.deb | head -n 1)
    fi
    
    echo "✅ Paquete compilado con éxito: $(basename "$deb_file")"
    
    # Realizar deploy si está activado
    if [ "$DEPLOY_FLAG" == "--deploy" ] || [ "$DEPLOY_FLAG" == "-d" ]; then
        echo "🌐 Desplegando al repositorio central APT..."
        
        if [ -z "$INLED_REPO_PAT" ]; then
            echo "❌ Error: La variable INLED_REPO_PAT no está definida."
            return 1
        fi
        
        local package_url=""
        
        if command -v gh >/dev/null 2>&1 && [ -n "$GITHUB_ACTIONS" ]; then
            gh release view "$RELEASE_TAG" --repo "$OWNER_REPO" >/dev/null 2>&1 || {
                gh release create "$RELEASE_TAG" --title "PulsarOS Package Repository Assets" --notes "Repositorio de paquetes deb compilados automáticamente." --repo "$OWNER_REPO"
            }
            gh release upload "$RELEASE_TAG" "$deb_file" --clobber --repo "$OWNER_REPO"
            package_url="https://github.com/${OWNER_REPO}/releases/download/${RELEASE_TAG}/$(basename "$deb_file")"
        else
            # Fallback API con curl
            local release_id=$(curl -s -H "Authorization: token $INLED_REPO_PAT" \
                "https://api.github.com/repos/${OWNER_REPO}/releases/tags/${RELEASE_TAG}" | jq -r '.id')
                
            if [ "$release_id" == "null" ] || [ -z "$release_id" ]; then
                local release_data=$(curl -s -X POST -H "Authorization: token $INLED_REPO_PAT" \
                    -H "Content-Type: application/json" \
                    -d "{\"tag_name\":\"$RELEASE_TAG\",\"title\":\"PulsarOS Package Repository Assets\",\"body\":\"Repositorio de paquetes.\"}" \
                    "https://api.github.com/repos/${OWNER_REPO}/releases")
                release_id=$(echo "$release_data" | jq -r '.id')
            fi
            
            local file_name=$(basename "$deb_file")
            local asset_id=$(curl -s -H "Authorization: token $INLED_REPO_PAT" \
                "https://api.github.com/repos/${OWNER_REPO}/releases/$release_id" | \
                jq -r ".assets[] | select(.name==\"$file_name\") | .id")
                
            if [ -n "$asset_id" ] && [ "$asset_id" != "null" ]; then
                curl -s -X DELETE -H "Authorization: token $INLED_REPO_PAT" \
                    "https://api.github.com/repos/${OWNER_REPO}/releases/assets/$asset_id"
            fi
            
            local upload_url="https://uploads.github.com/repos/${OWNER_REPO}/releases/${release_id}/assets?name=${file_name}"
            local upload_response=$(curl -s -X POST -H "Authorization: token $INLED_REPO_PAT" \
                -H "Content-Type: application/vnd.debian.binary-package" \
                --data-binary @"$deb_file" \
                "$upload_url")
            package_url=$(echo "$upload_response" | jq -r '.browser_download_url')
        fi
        
        if [ "$package_url" == "null" ] || [ -z "$package_url" ]; then
            echo "❌ Error: Falló la subida del asset a la release de GitHub."
            return 1
        fi
        
        echo "🎉 Subido en: $package_url"
        echo "📡 Enviando dispatch a repositorio APT central de Inled..."
        
        local dispatch_response=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
             -H "Accept: application/vnd.github.v3+json" \
             -H "Authorization: token $INLED_REPO_PAT" \
             https://api.github.com/repos/InledGroup/apt/dispatches \
             -d "{\"event_type\": \"package_upload\", \"client_payload\": {\"package_url\": \"$package_url\"}}")
             
        if [ "$dispatch_response" -eq 204 ] || [ "$dispatch_response" -eq 200 ] || [ "$dispatch_response" -eq 201 ]; then
            echo "🚀 ¡Despliegue notificado con éxito! HTTP $dispatch_response"
        else
            echo "❌ Error al notificar al repositorio APT. HTTP $dispatch_response"
            return 1
        fi
    fi
}

# 1. Preparar carpetas
mkdir -p "$STAGING_DIR"
mkdir -p "$OUTPUT_DIR"

# 2. Lógica de construcción
if [ "$PACKAGE_NAME" == "all" ]; then
    echo "🏗️  MODO COMPILACIÓN TOTAL: Detectando y compilando todos los paquetes..."
    
    # Encontrar todos los subdirectorios que tengan un archivo DEBIAN/control
    # Excluimos directorios del sistema de compilación
    PACKAGES=()
    while read -r control_path; do
        dir_name=$(basename "$(dirname "$(dirname "$control_path")")")
        # Asegurar que no coja el propio build staging si hay bucles
        if [[ "$control_path" != *"/pkg-staging/"* ]]; then
            PACKAGES+=("$dir_name")
        fi
    done < <(find "$PKG_DIR" -name "control" -path "*/DEBIAN/control")
    
    echo "📋 Paquetes detectados: ${PACKAGES[*]}"
    
    for pkg in "${PACKAGES[@]}"; do
        build_single_package "$pkg"
    done
    echo "=============================================================================="
    echo "🎉 ¡Compilación de todos los paquetes completada!"
    echo "=============================================================================="
else
    # Compilar solo el paquete solicitado
    build_single_package "$PACKAGE_NAME"
fi
