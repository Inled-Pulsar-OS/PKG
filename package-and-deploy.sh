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
    
    # Add to global list of compiled packages for final bulk deployment
    # Añadir a la lista global de paquetes compilados para el despliegue final en masa
    COMPILED_DEBS+=("$deb_file")
}

# ==============================================================================
# Bulk Deployment Function / Función de Despliegue en Masa
# ==============================================================================
# Uploads all compiled package files (.deb) to the GitHub release and sends
# a single repository dispatch notifying the APT repository about all packages at once.
# Sube todos los archivos de paquetes compilados (.deb) a la release de GitHub y envía
# un único repository dispatch notificando al repositorio APT todos los paquetes a la vez.
deploy_packages() {
    local debs=("$@")
    
    if [ ${#debs[@]} -eq 0 ]; then
        echo "⚠️ No se han compilado paquetes para desplegar. / No packages were compiled for deployment."
        return 0
    fi
    
    echo "=============================================================================="
    echo "🌐 DESPLEGANDO EN MASA AL REPOSITORIO CENTRAL APT / BULK DEPLOY TO CENTRAL APT"
    echo "   Paquetes a desplegar / Packages to deploy: ${#debs[@]}"
    echo "=============================================================================="
    
    if [ -z "$INLED_REPO_PAT" ]; then
        echo "❌ Error: La variable INLED_REPO_PAT no está definida. / Error: INLED_REPO_PAT variable is not defined."
        return 1
    fi
    
    # 1. Ensure the release exists (using gh cli or curl fallback)
    # 1. Asegurar que la release existe (usando gh cli o fallback con curl)
    local use_gh=false
    local release_id=""
    
    if command -v gh >/dev/null 2>&1 && [ -n "$GITHUB_ACTIONS" ]; then
        use_gh=true
        echo "🔧 Usando GitHub CLI para verificar/crear release / Using GitHub CLI to verify/create release..."
        gh release view "$RELEASE_TAG" --repo "$OWNER_REPO" >/dev/null 2>&1 || {
            gh release create "$RELEASE_TAG" --title "PulsarOS Package Repository Assets" --notes "Repositorio de paquetes deb compilados automáticamente." --repo "$OWNER_REPO"
        }
    else
        echo "🔧 Usando API de GitHub (curl) para verificar/crear release / Using GitHub API (curl) to verify/create release..."
        # Obtain release_id using curl / Obtener release_id usando curl
        release_id=$(curl -s -H "Authorization: token $INLED_REPO_PAT" \
            "https://api.github.com/repos/${OWNER_REPO}/releases/tags/${RELEASE_TAG}" | jq -r '.id')
            
        if [ "$release_id" == "null" ] || [ -z "$release_id" ]; then
            echo "🆕 Creando nueva release / Creating new release..."
            local release_data=$(curl -s -X POST -H "Authorization: token $INLED_REPO_PAT" \
                -H "Content-Type: application/json" \
                -d "{\"tag_name\":\"$RELEASE_TAG\",\"title\":\"PulsarOS Package Repository Assets\",\"body\":\"Repositorio de paquetes deb compilados automáticamente.\"}" \
                "https://api.github.com/repos/${OWNER_REPO}/releases")
            release_id=$(echo "$release_data" | jq -r '.id')
        fi
    fi
    
    local urls=()
    
    # 2. Upload each deb and obtain its URL
    # 2. Subir cada deb y obtener su URL
    for deb_file in "${debs[@]}"; do
        local file_name=$(basename "$deb_file")
        local package_url=""
        
        echo "⬆️ Subiendo asset / Uploading asset: $file_name..."
        
        if [ "$use_gh" = true ]; then
            gh release upload "$RELEASE_TAG" "$deb_file" --clobber --repo "$OWNER_REPO"
            package_url="https://github.com/${OWNER_REPO}/releases/download/${RELEASE_TAG}/${file_name}"
        else
            # Delete previous asset if it exists / Eliminar asset anterior si existe
            local asset_id=$(curl -s -H "Authorization: token $INLED_REPO_PAT" \
                "https://api.github.com/repos/${OWNER_REPO}/releases/$release_id" | \
                jq -r ".assets[] | select(.name==\"$file_name\") | .id")
                
            if [ -n "$asset_id" ] && [ "$asset_id" != "null" ]; then
                echo "🗑️ Eliminando asset anterior / Deleting previous asset: $file_name..."
                curl -s -X DELETE -H "Authorization: token $INLED_REPO_PAT" \
                    "https://api.github.com/repos/${OWNER_REPO}/releases/assets/$asset_id"
            fi
            
            # Upload the new asset / Subir el nuevo asset
            local upload_url="https://uploads.github.com/repos/${OWNER_REPO}/releases/${release_id}/assets?name=${file_name}"
            local upload_response=$(curl -s -X POST -H "Authorization: token $INLED_REPO_PAT" \
                -H "Content-Type: application/vnd.debian.binary-package" \
                --data-binary @"$deb_file" \
                "$upload_url")
            package_url=$(echo "$upload_response" | jq -r '.browser_download_url')
        fi
        
        if [ "$package_url" == "null" ] || [ -z "$package_url" ]; then
            echo "❌ Error: Falló la subida de $file_name a la release de GitHub. / Error: Upload of $file_name to GitHub release failed."
            return 1
        fi
        
        echo "✅ Subido en / Uploaded at: $package_url"
        urls+=("$package_url")
    done
    
    # 3. Construct JSON payload for dispatch
    # 3. Construir el payload JSON para el dispatch
    # It structures keys as package_url, package_2_url, package_3_url, etc.
    # Estructura las claves como package_url, package_2_url, package_3_url, etc.
    local payload="{"
    for i in "${!urls[@]}"; do
        local idx=$((i + 1))
        local key="package_url"
        if [ "$idx" -gt 1 ]; then
            key="package_${idx}_url"
        fi
        payload="${payload}\"${key}\": \"${urls[$i]}\""
        if [ "$idx" -lt "${#urls[@]}" ]; then
            payload="${payload}, "
        fi
    done
    payload="${payload}}"
    
    # 4. Send a single repository dispatch to central Inled APT repository
    # 4. Enviar un único repository dispatch al repositorio central APT de Inled
    echo "📡 Enviando dispatch en masa a repositorio APT central... / Sending bulk dispatch to central APT repository..."
    local dispatch_response=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
         -H "Accept: application/vnd.github.v3+json" \
         -H "Authorization: token $INLED_REPO_PAT" \
         https://api.github.com/repos/InledGroup/apt/dispatches \
         -d "{\"event_type\": \"package_upload\", \"client_payload\": ${payload}}")
         
    if [ "$dispatch_response" -eq 204 ] || [ "$dispatch_response" -eq 200 ] || [ "$dispatch_response" -eq 201 ]; then
        echo "🚀 ¡Despliegue de todos los paquetes notificado con éxito! / All packages deployment notified successfully! HTTP $dispatch_response"
    else
        echo "❌ Error al notificar al repositorio APT. / Error notifying APT repository. HTTP $dispatch_response"
        return 1
    fi
}

# ==============================================================================
# Main Execution / Ejecución Principal
# ==============================================================================

# Global array to collect compiled packages
# Array global para almacenar los paquetes compilados
COMPILED_DEBS=()

# 1. Prepare folders / Preparar carpetas
mkdir -p "$STAGING_DIR"
mkdir -p "$OUTPUT_DIR"

# 2. Build logic / Lógica de construcción
if [ "$PACKAGE_NAME" == "all" ]; then
    echo "🏗️  MODO COMPILACIÓN TOTAL: Detectando y compilando todos los paquetes..."
    echo "🏗️  FULL BUILD MODE: Detecting and compiling all packages..."
    
    # Find all subdirectories that contain a DEBIAN/control file
    # Encontrar todos los subdirectorios que tengan un archivo DEBIAN/control
    # Exclude build staging directories / Excluir directorios del sistema de compilación
    PACKAGES=()
    while read -r control_path; do
        dir_name=$(basename "$(dirname "$(dirname "$control_path")")")
        if [[ "$control_path" != *"/pkg-staging/"* ]]; then
            PACKAGES+=("$dir_name")
        fi
    done < <(find "$PKG_DIR" -name "control" -path "*/DEBIAN/control")
    
    echo "📋 Paquetes detectados / Detected packages: ${PACKAGES[*]}"
    
    for pkg in "${PACKAGES[@]}"; do
        build_single_package "$pkg"
    done
    echo "=============================================================================="
    echo "🎉 ¡Compilación de todos los paquetes completada! / All packages compilation completed!"
    echo "=============================================================================="
else
    # Build only requested package / Compilar solo el paquete solicitado
    build_single_package "$PACKAGE_NAME"
fi

# 3. Perform bulk deployment if requested / Realizar despliegue en masa si está activado
if [ "$DEPLOY_FLAG" == "--deploy" ] || [ "$DEPLOY_FLAG" == "-d" ]; then
    deploy_packages "${COMPILED_DEBS[@]}"
fi
