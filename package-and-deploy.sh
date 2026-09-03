#!/bin/bash
# ==============================================================================
# Pulsar OS - Package and Deploy Utility
# ==============================================================================
# This script packages one or all folders under the repository root into .deb packages
# and notifies the Inled central APT repository for deployment.
#
# Usage:
#   ./package-and-deploy.sh <package_folder_name | all> [--deploy] [--upload]
#
# Examples:
#   ./package-and-deploy.sh pulsaros-theme            # Compila un paquete en local
#   ./package-and-deploy.sh all                       # Compila TODOS los paquetes en local
#   ./package-and-deploy.sh all --deploy              # Compila y despliega TODOS a Inled APT
#   ./package-and-deploy.sh pulsaros-gnome --upload      # Recompila y sube un paquete .deb
#   ./package-and-deploy.sh pulsaros-gnome --onlyupload  # Sube un .deb ya compilado
# ==============================================================================

set -e

# Configuración básica
OWNER_REPO="Inled-Pulsar-OS/PKG"
RELEASE_TAG="packages-repo"
PKG_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="$PKG_DIR/build"
STAGING_DIR="$BUILD_DIR/pkg-staging"
OUTPUT_DIR="$BUILD_DIR/packages"

# Parámetros / Parameters
PACKAGE_NAME=""
DEPLOY_FLAG=""
DEPLOY_ONLY_FLAG=""
UPLOAD_FLAG=""
ONLY_UPLOAD_FLAG=""
BRANCH="stable"

# Packages that must be uploaded manually from a trusted machine (for example
# because their build downloads external assets, like EGO GNOME Shell
# extensions, which rate-limit CI runner IPs). The "all" build skips them and
# warns that they must be uploaded by hand with --upload.
MANUAL_UPLOAD_ONLY=("pulsaros-gnome")

is_manual_upload_only() {
    local name="$1"
    for p in "${MANUAL_UPLOAD_ONLY[@]}"; do
        if [ "$p" = "$name" ]; then
            return 0
        fi
    done
    return 1
}

INCREMENTAL=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --deploy|-d)
            DEPLOY_FLAG="--deploy"
            shift
            ;;
        --deploy-only)
            DEPLOY_ONLY_FLAG="--deploy-only"
            shift
            ;;
        --upload|-u)
            UPLOAD_FLAG="--upload"
            shift
            ;;
        --onlyupload)
            ONLY_UPLOAD_FLAG="--onlyupload"
            shift
            ;;
        --incremental|-i|--smart)
            INCREMENTAL=true
            shift
            ;;
        --branch|-b)
            BRANCH="$2"
            shift 2
            ;;
        *)
            if [ -z "$PACKAGE_NAME" ]; then
                PACKAGE_NAME="$1"
            else
                echo "❌ Parámetro desconocido: $1"
                exit 1
            fi
            shift
            ;;
    esac
done

if [ -z "$PACKAGE_NAME" ]; then
    echo "❌ Error: Debes especificar el nombre de la carpeta del paquete o 'all'."
    echo "Ejemplo: $0 pulsaros-branding [--incremental]"
    exit 1
fi

# Stamp directory holding the last git HEAD that each package source was built
# from, so incremental builds can detect changes committed inside nested git
# subrepos (e.g. PKG/sayri) even when git operations preserve file mtimes.
GIT_STAMP_DIR="$BUILD_DIR/.git-stamps"
mkdir -p "$GIT_STAMP_DIR" 2>/dev/null || true

# If $dir is itself a git work tree root, return the current HEAD commit hash.
# This covers nested git repos and git submodules (e.g. PKG/sayri). It does NOT
# walk up to an enclosing repository, so a plain folder inside the main PKG repo
# (which is not itself a git repo) is deliberately ignored here.
git_source_commit() {
    local dir="$1"
    local toplevel
    toplevel=$(git -C "$dir" rev-parse --show-toplevel 2>/dev/null) || return 1
    [ "$toplevel" = "$dir" ] || return 1
    git -C "$dir" rev-parse HEAD 2>/dev/null || return 1
}

# Record the git HEAD the package was built from.
stamp_git_commit() {
    local name="$1"
    local src="$PKG_DIR/$name"
    local commit
    commit=$(git_source_commit "$src") || return 0
    printf '%s\n' "$commit" > "$GIT_STAMP_DIR/$name" 2>/dev/null || true
}

is_deb_up_to_date() {
    local name="$1"
    local existing_deb=$(ls -t "$OUTPUT_DIR/${name}_"*.deb 2>/dev/null | head -n 1)
    [ -z "$existing_deb" ] && return 1
    [ ! -f "$existing_deb" ] && return 1

    local deb_time=$(stat -c %Y "$existing_deb" 2>/dev/null || echo 0)
    local pkg_src_dir="$PKG_DIR/$name"
    [ ! -d "$pkg_src_dir" ] && return 1

    local newest_src=$(find "$pkg_src_dir" -type f -not -path "*/target/*" -not -path "*/.git/*" -printf '%T@\n' 2>/dev/null | sort -nr | head -n 1 | cut -d. -f1)
    [ -n "$newest_src" ] && [ "$newest_src" -gt "$deb_time" ] && return 1

    # Detect changes inside nested git subrepos (e.g. sayri) by comparing the
    # current HEAD commit to the commit this package was built from.
    local current_commit built_commit
    current_commit=$(git_source_commit "$pkg_src_dir")
    if [ -n "$current_commit" ]; then
        built_commit=$(cat "$GIT_STAMP_DIR/$name" 2>/dev/null || true)
        if [ -z "$built_commit" ] || [ "$current_commit" != "$built_commit" ]; then
            return 1
        fi
    fi

    return 0
}

clean_orphan_packages() {
    [ ! -d "$OUTPUT_DIR" ] && return 0
    for f in "$OUTPUT_DIR"/*.deb; do
        [ -f "$f" ] || continue
        local pkg_name
        pkg_name=$(dpkg-deb -f "$f" Package 2>/dev/null || echo "")
        [ -z "$pkg_name" ] && continue

        if [[ "$pkg_name" == *calamares* ]] || [[ "$pkg_name" == *-debug* ]]; then
            echo "🗑️  Removing obsolete/debug deb from cache: $(basename "$f")"
            rm -f "$f"
            continue
        fi

        if [ ! -d "$PKG_DIR/$pkg_name" ] && [ ! -d "$PKG_DIR/${pkg_name#pulsaros-}" ]; then
            echo "🗑️  Removing orphan deb with no source from cache: $(basename "$f")"
            rm -f "$f"
        fi
    done
}

if [ "$BRANCH" != "stable" ] && [ "$BRANCH" != "forky" ] && [ "$BRANCH" != "rolling" ]; then
    echo "❌ Error: Rama inválida '$BRANCH'. Debe ser stable, forky o rolling."
    exit 1
fi

get_branch_suffix() {
    if [ "$BRANCH" = "forky" ]; then
        echo "-deb14"
    elif [ "$BRANCH" = "rolling" ]; then
        echo "-rolling"
    else
        echo ""
    fi
}

# English: Helper to auto-increment SemVer or Debian format versions (X.Y.Z-R or X.Y.Z)
# Español: Utilidad para auto-incrementar versiones de SemVer o formato Debian (X.Y.Z-R o X.Y.Z)
increment_version() {
    local version="$1"
    if [[ "$version" =~ ^([0-9]+\.[0-9]+\.[0-9]+)-([0-9]+)$ ]]; then
        local base="${BASH_REMATCH[1]}"
        local rev="${BASH_REMATCH[2]}"
        local new_rev=$((rev + 1))
        echo "${base}-${new_rev}"
    elif [[ "$version" =~ ^([0-9]+\.[0-9]+)\.([0-9]+)$ ]]; then
        local base="${BASH_REMATCH[1]}"
        local patch="${BASH_REMATCH[2]}"
        local new_patch=$((patch + 1))
        echo "${base}.${new_patch}"
    elif [[ "$version" =~ ^([0-9]+)\.([0-9]+)$ ]]; then
        local base="${BASH_REMATCH[1]}"
        local minor="${BASH_REMATCH[2]}"
        local new_minor=$((minor + 1))
        echo "${base}.${new_minor}"
    else
        echo "${version}.1"
    fi
}

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
    
    # Auto-increment package version before building
    local control_file="$source_folder/DEBIAN/control"
    local current_version=$(grep "^Version:" "$control_file" | cut -d' ' -f2)
    # Strip any existing branch suffix (like +deb13, +deb14, +rolling, or any suffix starting with + or -)
    local base_version=$(echo "$current_version" | sed -E 's/(\+|-)(deb14|rolling).*$//')
    local new_base=$(increment_version "$base_version")
    local suffix=$(get_branch_suffix)
    local new_version="${new_base}${suffix}"
    echo "🔄 Auto-incrementando versión de $name: $current_version -> $new_version"
    sed -i "s/^Version:.*/Version: $new_version/" "$control_file"
    
    # Limpieza previa de staging y debs antiguos en la carpeta de salida
    rm -rf "$STAGING_DIR/$name" 2>/dev/null || pkexec rm -rf "$STAGING_DIR/$name"
    mkdir -p "$STAGING_DIR/$name"
    rm -f "$OUTPUT_DIR/${name}_"*.deb
    
    # Copiar archivos
    cp -r "$source_folder/." "$STAGING_DIR/$name/"

    # Sobrescribir versión en etc/os-release de pulsaros-branding si PULSAR_VERSION está definido
    if [ "$name" = "pulsaros-branding" ] && [ -n "$PULSAR_VERSION" ]; then
        echo "⚙️ Sobrescribiendo versión del sistema en etc/os-release con: $PULSAR_VERSION"
        if [ -f "$STAGING_DIR/$name/etc/os-release" ]; then
            sed -i "s/^PRETTY_NAME=.*/PRETTY_NAME=\"Pulsar OS Bitten Fruit Debian Based $PULSAR_VERSION\"/" "$STAGING_DIR/$name/etc/os-release"
            sed -i "s/^NAME=.*/NAME=\"Pulsar OS Bitten Fruit Debian Based\"/" "$STAGING_DIR/$name/etc/os-release"
            sed -i "s/^VERSION_ID=.*/VERSION_ID=\"$PULSAR_VERSION\"/" "$STAGING_DIR/$name/etc/os-release"
            sed -i "s/^VERSION=.*/VERSION=\"$PULSAR_VERSION\"/" "$STAGING_DIR/$name/etc/os-release"
        fi
    fi
    
    # Hook de preparación
    local prepare_hook="$source_folder/prepare-assets.sh"
    if [ -f "$prepare_hook" ]; then
        echo "🚀 Ejecutando script de preparación del paquete..."
        # Set DEBIAN_VERSION environment variable for the hook
        local debian_version="trixie"
        case "$BRANCH" in
            forky) debian_version="forky" ;;
            rolling) debian_version="testing" ;;
            *) debian_version="trixie" ;;
        esac
        export DEBIAN_VERSION="$debian_version"
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
    stamp_git_commit "$name"
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
    
    # Prefer gh whenever it is available and authenticated (both locally and in
    # CI), since the upload to the PKG release assets needs a token with write
    # access to this repository. Fall back to curl + INLED_REPO_PAT otherwise.
    if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
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
        local file_name_github="${file_name//:/.}"
        local package_url=""
        
        echo "⬆️ Subiendo asset / Uploading asset: $file_name (GitHub asset: $file_name_github)..."
        
        if [ "$use_gh" = true ]; then
            gh release delete-asset "$RELEASE_TAG" "$file_name_github" --repo "$OWNER_REPO" -y >/dev/null 2>&1 || true
            gh release upload "$RELEASE_TAG" "$deb_file" --clobber --repo "$OWNER_REPO"
            package_url="https://github.com/${OWNER_REPO}/releases/download/${RELEASE_TAG}/${file_name_github}"
        else
            # Delete previous asset if it exists / Eliminar asset anterior si existe
            local asset_id=$(curl -s -H "Authorization: token $INLED_REPO_PAT" \
                "https://api.github.com/repos/${OWNER_REPO}/releases/$release_id" | \
                jq -r ".assets[] | select(.name==\"$file_name_github\") | .id")
                
            if [ -n "$asset_id" ] && [ "$asset_id" != "null" ]; then
                echo "🗑️ Eliminando asset anterior / Deleting previous asset: $file_name..."
                local del_resp=$(mktemp)
                local del_code=$(curl -s -o "$del_resp" -w "%{http_code}" -X DELETE \
                    -H "Authorization: token $INLED_REPO_PAT" \
                    "https://api.github.com/repos/${OWNER_REPO}/releases/assets/$asset_id")
                if [ "$del_code" != "200" ] && [ "$del_code" != "204" ]; then
                    echo "⚠️ Aviso: no se pudo eliminar el asset (HTTP $del_code). Intentando subir igualmente... / Warning: Could not delete existing asset (HTTP $del_code). Trying to upload anyway..."
                    cat "$del_resp"
                fi
                rm -f "$del_resp"
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
            echo "   Respuesta de GitHub / Response from GitHub:"
            echo "$upload_response"
            return 1
        fi
        
        echo "✅ Subido en / Uploaded at: $package_url"
        urls+=("$package_url")
    done
    
    # 3. Construct JSON payload for dispatch
    # 3. Construir el payload JSON para el dispatch
    # Join all URLs into a space-separated string to comply with GitHub's 10-properties limit on client_payload.
    # Une todas las URLs en una cadena separada por espacios para cumplir con el límite de 10 propiedades de GitHub.
    local urls_str="${urls[*]}"
    local json_payload=$(jq -n --arg urls "$urls_str" --arg branch "$BRANCH" '{package_urls: $urls, branch: $branch}')
    
    local request_body=$(jq -c -n --arg ev "package_upload" --argjson pay "$json_payload" '{event_type: $ev, client_payload: $pay}')
    
    # 4. Send a single repository dispatch to central Inled APT repository
    # 4. Enviar un único repository dispatch al repositorio central APT de Inled
    echo "📡 Enviando dispatch en masa a repositorio APT central... / Sending bulk dispatch to central APT repository..."
    
    local response_file=$(mktemp)
    local dispatch_response=$(curl -s -w "%{http_code}" -X POST \
         -H "Accept: application/vnd.github.v3+json" \
         -H "Authorization: token $INLED_REPO_PAT" \
         https://api.github.com/repos/InledGroup/apt/dispatches \
         -d "$request_body" -o "$response_file")
         
    if [ "$dispatch_response" -eq 204 ] || [ "$dispatch_response" -eq 200 ] || [ "$dispatch_response" -eq 201 ]; then
        echo "🚀 ¡Despliegue de todos los paquetes notificado con éxito! / All packages deployment notified successfully! HTTP $dispatch_response"
        rm -f "$response_file"
    else
        echo "❌ Error al notificar al repositorio APT. / Error notifying APT repository. HTTP $dispatch_response"
        echo "Detalles del error de GitHub / GitHub Error Details:"
        cat "$response_file"
        echo ""
        rm -f "$response_file"
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

# 1b. Upload mode: deploy a single package. By default it rebuilds the package first
# (useful when the build must run on a trusted machine, e.g. for EGO GNOME Shell
# extension downloads); use --onlyupload to deploy an already built package.
# Acepta una ruta directa a un .deb o un nombre de paquete (usa el .deb más reciente).
if [ -n "$UPLOAD_FLAG" ] || [ -n "$ONLY_UPLOAD_FLAG" ]; then
    if [ -z "$PACKAGE_NAME" ]; then
        echo "❌ Error: --upload/--onlyupload requiere un nombre de paquete o una ruta a un archivo .deb."
        exit 1
    fi
    if [ -f "$PACKAGE_NAME" ]; then
        # Direct path to an already built package: just upload it.
        UPLOAD_DEB="$PACKAGE_NAME"
    else
        if [ -z "$ONLY_UPLOAD_FLAG" ]; then
            echo "🏗️  MODO SUBIDA / UPLOAD MODE: Recompilando $PACKAGE_NAME antes de subir..."
            build_single_package "$PACKAGE_NAME"
        else
            echo "⏫  MODO SOLO SUBIDA / ONLY-UPLOAD MODE: Subiendo un paquete .deb ya compilado..."
        fi
        UPLOAD_DEB=$(ls -t "$OUTPUT_DIR/${PACKAGE_NAME}_"*.deb 2>/dev/null | head -n 1)
    fi
    if [ -z "$UPLOAD_DEB" ] || [ ! -f "$UPLOAD_DEB" ]; then
        echo "❌ Error: No se encontró ningún .deb para '$PACKAGE_NAME' en $OUTPUT_DIR."
        exit 1
    fi
    echo "⏫  Subiendo / Uploading: $(basename "$UPLOAD_DEB")"
    deploy_packages "$UPLOAD_DEB"
    exit $?
fi

# 1c. Deploy-only mode: upload previously built packages without rebuilding
# 1c. Modo solo despliegue: sube los .deb ya compilados sin recompilar
if [ -n "$DEPLOY_ONLY_FLAG" ]; then
    echo "🚀  MODO SOLO DESPLIEGUE / DEPLOY-ONLY MODE: Subiendo paquetes .deb ya compilados..."
    mapfile -t COMPILED_DEBS < <(find "$OUTPUT_DIR" -maxdepth 1 -name '*.deb' 2>/dev/null | sort)
    # Never auto-deploy packages that must be uploaded manually.
    # Nunca auto-desplegar paquetes que deben subirse manualmente.
    if [ ${#COMPILED_DEBS[@]} -gt 0 ]; then
        mapfile -t COMPILED_DEBS < <(for f in "${COMPILED_DEBS[@]}"; do
            base=$(basename "$f")
            skip=false
            for p in "${MANUAL_UPLOAD_ONLY[@]}"; do
                if [[ "$base" == "$p"_* ]]; then
                    skip=true
                    break
                fi
            done
            if $skip; then
                echo "⏭️  Omitido (subida manual) / Skipping (manual upload only): $base" >&2
                continue
            fi
            echo "$f"
        done)
    fi
    # Keep only the .deb files that belong to the target branch (the version
    # carries the branch suffix: -deb14 for forky, -rolling for rolling).
    # Mantener solo los .deb que pertenecen a la rama destino (la versión lleva
    # el sufijo de rama: -deb14 para forky, -rolling para rolling).
    case "$BRANCH" in
        forky)
            COMPILED_DEBS=($(printf '%s\n' "${COMPILED_DEBS[@]}" | grep -- '-deb14' || true))
            ;;
        rolling)
            COMPILED_DEBS=($(printf '%s\n' "${COMPILED_DEBS[@]}" | grep -- '-rolling' || true))
            ;;
        *)
            COMPILED_DEBS=($(printf '%s\n' "${COMPILED_DEBS[@]}" | grep -v -- '-deb14' | grep -v -- '-rolling' || true))
            ;;
    esac
    deploy_packages "${COMPILED_DEBS[@]}"
    exit $?
fi

# 2. Build logic / Lógica de construcción
if [ "$PACKAGE_NAME" == "all" ]; then
    clean_orphan_packages
    echo "🏗️  MODO COMPILACIÓN TOTAL: Detectando y compilando todos los paquetes..."
    echo "🏗️  FULL BUILD MODE: Detecting and compiling all packages..."
    
    # Find all subdirectories that contain a DEBIAN/control file
    # Encontrar todos los subdirectorios que tengan un archivo DEBIAN/control
    # Exclude build staging directories and Tube OS flavor packages
    PACKAGES=()
    while read -r control_path; do
        dir_name=$(basename "$(dirname "$(dirname "$control_path")")")
        if [[ "$control_path" != *"/pkg-staging/"* ]] && [[ "$dir_name" != tubeos-* ]] && [[ "$dir_name" != tube-os-* ]] && [[ "$dir_name" != "dockermigrate" ]]; then
            PACKAGES+=("$dir_name")
        fi
    done < <(find "$PKG_DIR" -name "control" -path "*/DEBIAN/control")
    
    echo "📋 Paquetes detectados / Detected packages: ${PACKAGES[*]}"
    
    SKIPPED_MANUAL=()
    for pkg in "${PACKAGES[@]}"; do
        if [ "$DEPLOY_FLAG" == "--deploy" ] || [ "$DEPLOY_FLAG" == "-d" ] || [ -n "$CI" ] || [ -n "$GITHUB_ACTIONS" ]; then
            if is_manual_upload_only "$pkg"; then
                echo "⏭️  Omitido / Skipping $pkg (manual upload only)..."
                SKIPPED_MANUAL+=("$pkg")
                continue
            fi
        fi

        if $INCREMENTAL && is_deb_up_to_date "$pkg"; then
            existing_deb=$(ls -t "$OUTPUT_DIR/${pkg}_"*.deb 2>/dev/null | head -n 1)
            echo "⚡ [CACHED] Reutilizando $pkg: $(basename "$existing_deb") (sin cambios)"
            COMPILED_DEBS+=("$existing_deb")
            stamp_git_commit "$pkg"
        else
            build_single_package "$pkg"
        fi
    done
    echo "=============================================================================="
    echo "🎉 ¡Paquetes procesados con éxito! / Packages processed successfully!"
    echo "=============================================================================="
    if [ ${#SKIPPED_MANUAL[@]} -gt 0 ]; then
        echo "⚠️  ATENCIÓN / ATTENTION: Los siguientes paquetes NO se compilaron/desplegaron"
        echo "   automáticamente porque deben subirse manualmente desde una máquina de confianza:"
        for p in "${SKIPPED_MANUAL[@]}"; do
            echo "     - $p"
        done
        echo "   Compílalo localmente (o reutiliza el build local) y súbelo con:"
        echo "     ./package-and-deploy.sh <paquete> --upload --branch <stable|forky|rolling>"
        echo "   o, si el paquete ya está compilado en local:"
        echo "     ./package-and-deploy.sh <paquete> --onlyupload --branch <stable|forky|rolling>"
    fi
else
    # Build only requested package / Compilar solo el paquete solicitado
    build_single_package "$PACKAGE_NAME"
fi

# 3. Perform bulk deployment if requested / Realizar despliegue en masa si está activado
if [ "$DEPLOY_FLAG" == "--deploy" ] || [ "$DEPLOY_FLAG" == "-d" ]; then
    deploy_packages "${COMPILED_DEBS[@]}"
fi
