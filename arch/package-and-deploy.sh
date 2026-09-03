#!/bin/bash
# ==============================================================================
# Pulsar OS - Arch Linux Package Builder and Deployer
# ==============================================================================
# Compiles all PKGBUILD packages into .pkg.tar.zst archives using makepkg
# and deploys them to the Inled Arch Linux repository via GitHub releases.
#
# Usage:
#   ./package-and-deploy.sh <package_name | all> [--deploy] [--branch <stable|forky|rolling>] [--upload]
#
# Examples:
#   ./package-and-deploy.sh pulsaros-theme   # Build a single package
#   ./package-and-deploy.sh all              # Build ALL packages
#   ./package-and-deploy.sh all --deploy     # Build and deploy all
#   ./package-and-deploy.sh pulsaros-gnome --upload      # Rebuild and upload a package
#   ./package-and-deploy.sh pulsaros-gnome --onlyupload  # Upload an already built package
# ==============================================================================

set -e

OWNER_REPO="Inled-Pulsar-OS/PKG"
RELEASE_TAG="packages-repo"
PKG_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="$PKG_DIR/build"
PKGBUILDS_DIR="$PKG_DIR/pkgbuilds"
OUTPUT_DIR="$BUILD_DIR/packages"

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

# Determine non-root user for makepkg if running under sudo/root
BUILD_USER=""
if [ "$EUID" -eq 0 ]; then
    if [ -n "$SUDO_USER" ] && [ "$SUDO_USER" != "root" ]; then
        BUILD_USER="$SUDO_USER"
    elif [ -n "$PKEXEC_UID" ] && [ "$PKEXEC_UID" != "0" ]; then
        BUILD_USER=$(id -un "$PKEXEC_UID" 2>/dev/null || true)
    fi
    if [ -z "$BUILD_USER" ] || [ "$BUILD_USER" = "root" ]; then
        BUILD_USER=$(stat -c '%U' "$PKG_DIR" 2>/dev/null || true)
    fi
    if [ -z "$BUILD_USER" ] || [ "$BUILD_USER" = "root" ]; then
        BUILD_USER="nobody"
    fi
fi

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
                echo "❌ Unknown parameter: $1"
                exit 1
            fi
            shift
            ;;
    esac
done

if [ -z "$PACKAGE_NAME" ]; then
    echo "❌ Error: Specify a package name or 'all'."
    echo "Usage: $0 <package_name | all> [--deploy] [--branch <stable|forky|rolling>] [--upload|--onlyupload] [--incremental]"
    exit 1
fi

COMPILED_PKGS=()

# Stamp directory holding the last git HEAD that each package source was built
# from, so incremental builds can detect changes committed inside nested git
# subrepos (e.g. PKG/sayri) even when git operations preserve file mtimes.
GIT_STAMP_DIR="$BUILD_DIR/.git-stamps"
mkdir -p "$GIT_STAMP_DIR"

# Resolve the source dir used to build package $name (mirrors each PKGBUILD's
# prepare()/package() reference to its source subrepo).
resolve_src_dir() {
    local name="$1"
    local d
    # Prefer the sibling source dir referenced as startdir/../../../sayri (i.e. PKG/<name>)
    d="$PKG_DIR/../../PKG/$name"
    [ -d "$d" ] && { echo "$d"; return; }
    d="$PKG_DIR/../$name"
    [ -d "$d" ] && { echo "$d"; return; }
    return 1
}

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
    local src
    src=$(resolve_src_dir "$name") || return 0
    local commit
    commit=$(git_source_commit "$src") || return 0
    printf '%s\n' "$commit" > "$GIT_STAMP_DIR/$name" 2>/dev/null || true
}

is_pkg_up_to_date() {
    local name="$1"
    local pkgbuild_dir="$PKGBUILDS_DIR/$name"
    local existing_pkg=$(ls -t "$OUTPUT_DIR/${name}-"*.pkg.tar.zst 2>/dev/null | head -n 1)
    [ -z "$existing_pkg" ] && return 1
    [ ! -f "$existing_pkg" ] && return 1

    local pkg_time=$(stat -c %Y "$existing_pkg" 2>/dev/null || echo 0)

    # Check PKGBUILD directory
    local newest_src=$(find "$pkgbuild_dir" -type f -printf '%T@\n' 2>/dev/null | sort -nr | head -n 1 | cut -d. -f1)
    [ -n "$newest_src" ] && [ "$newest_src" -gt "$pkg_time" ] && return 1

    # Also check corresponding root PKG directory if present
    local root_src_dir="$PKG_DIR/../../PKG/$name"
    [ ! -d "$root_src_dir" ] && root_src_dir="$PKG_DIR/../$name"
    [ ! -d "$root_src_dir" ] && root_src_dir="$PKG_DIR/../${name#pulsaros-}"
    [ ! -d "$root_src_dir" ] && root_src_dir="$PKG_DIR/../pulsar-${name#pulsaros-}"
    if [ -d "$root_src_dir" ]; then
        local newest_root_src=$(find "$root_src_dir" -type f -not -path "*/target/*" -not -path "*/.git/*" -printf '%T@\n' 2>/dev/null | sort -nr | head -n 1 | cut -d. -f1)
        [ -n "$newest_root_src" ] && [ "$newest_root_src" -gt "$pkg_time" ] && return 1

        # Detect changes inside nested git subrepos (e.g. sayri) by comparing
        # the current HEAD commit to the commit this package was built from.
        local current_commit built_commit
        current_commit=$(git_source_commit "$root_src_dir")
        if [ -n "$current_commit" ]; then
            built_commit=$(cat "$GIT_STAMP_DIR/$name" 2>/dev/null || true)
            if [ -z "$built_commit" ] || [ "$current_commit" != "$built_commit" ]; then
                return 1
            fi
        fi
    fi

    return 0
}

clean_orphan_packages() {
    [ ! -d "$OUTPUT_DIR" ] && return 0
    local active_names=()
    for d in "$PKGBUILDS_DIR"/*/; do
        [ -f "$d/PKGBUILD" ] && active_names+=("$(basename "$d")")
    done
    local extra_allowed=("pamtester" "xremap-gnome-bin" "autokey-gtk" "autokey" "winboat-bin" "cloudflare-warp-bin" "localsend-bin")

    for f in "$OUTPUT_DIR"/*.pkg.tar.zst; do
        [ -f "$f" ] || continue
        local pkg_name
        pkg_name=$(LC_ALL=C pacman -Qip "$f" 2>/dev/null | awk -F': ' '/^Name/{print $2; exit}')
        [ -z "$pkg_name" ] && continue

        if [[ "$pkg_name" == *calamares* ]] || [[ "$pkg_name" == *-debug* ]]; then
            echo "🗑️  Removing obsolete/debug package from cache: $(basename "$f")"
            rm -f "$f"
            continue
        fi

        local found=false
        for a in "${active_names[@]}" "${extra_allowed[@]}"; do
            if [ "$pkg_name" = "$a" ]; then
                found=true
                break
            fi
        done

        if [ "$found" = false ]; then
            echo "🗑️  Removing orphan package with no PKGBUILD from cache: $(basename "$f")"
            rm -f "$f"
        fi
    done
}

build_single_package() {
    local name="$1"
    local pkgbuild_dir="$PKGBUILDS_DIR/$name"

    echo "=============================================================================="
    echo "📦 BUILDING: $name"
    echo "=============================================================================="

    if [ ! -d "$pkgbuild_dir" ]; then
        echo "❌ Error: PKGBUILD directory not found: $pkgbuild_dir"
        return 1
    fi

    if [ ! -f "$pkgbuild_dir/PKGBUILD" ]; then
        echo "❌ Error: PKGBUILD not found in: $pkgbuild_dir"
        return 1
    fi

    mkdir -p "$OUTPUT_DIR"

    # Remove stale versions of this package to avoid pacman -U "duplicate
    # target" errors when the ISO build copies every *.pkg.tar.zst.
    rm -f "$OUTPUT_DIR/${name}-"*.pkg.tar.zst

    cd "$pkgbuild_dir"
    # Export PULSAR_VERSION for the makepkg environment
    export PULSAR_VERSION

    # In CI/Docker environments, install official package dependencies
    if command -v pacman >/dev/null 2>&1; then
        local raw_deps=($(bash -c '
            unset depends makedepends
            source ./PKGBUILD 2>/dev/null || true
            echo "${depends[@]} ${makedepends[@]}"
        '))
        local to_install=()
        for dep in "${raw_deps[@]}"; do
            dep="${dep%%[<>=]*}"
            if [[ "$dep" != pulsaros-* ]] && [[ "$dep" != gnome-macos-remap* ]] && \
               [[ "$dep" != droidtux* ]] && [[ "$dep" != macboat* ]] && \
               [[ "$dep" != seafari* ]] && [[ "$dep" != spotlight* ]] && \
               [[ "$dep" != winboat* ]] && [ -n "$dep" ]; then
                to_install+=("$dep")
            fi
        done
        if [ ${#to_install[@]} -gt 0 ]; then
            echo "📦 Installing official build dependencies for $name..."
            local auth_cmd=""
            if [ "$EUID" -ne 0 ]; then
                command -v sudo >/dev/null 2>&1 && auth_cmd="sudo" || auth_cmd="pkexec"
            fi
            $auth_cmd pacman -S --needed --noconfirm "${to_install[@]}" 2>/dev/null || {
                for d in "${to_install[@]}"; do
                    $auth_cmd pacman -S --needed --noconfirm "$d" 2>/dev/null || true
                done
            }
        fi
    fi

    if [ "$EUID" -eq 0 ] && [ -n "$BUILD_USER" ] && [ "$BUILD_USER" != "root" ]; then
        chown -R "$BUILD_USER":"$BUILD_USER" "$BUILD_DIR" "$pkgbuild_dir" 2>/dev/null || true
        if command -v runuser >/dev/null 2>&1; then
            runuser -u "$BUILD_USER" -- env PULSAR_VERSION="$PULSAR_VERSION" PKGDEST="$OUTPUT_DIR" makepkg -cfd --noconfirm --nosign
        else
            sudo -u "$BUILD_USER" env PULSAR_VERSION="$PULSAR_VERSION" PKGDEST="$OUTPUT_DIR" makepkg -cfd --noconfirm --nosign
        fi
    else
        PKGDEST="$OUTPUT_DIR" makepkg -cfd --noconfirm --nosign
    fi

    local pkg_file=$(ls "$OUTPUT_DIR/${name}-"*.pkg.tar.zst 2>/dev/null | head -n 1)
    if [ -n "$pkg_file" ]; then
        echo "✅ Package built: $(basename "$pkg_file")"
        COMPILED_PKGS+=("$pkg_file")
        stamp_git_commit "$name"
    else
        echo "❌ Error: Package built successfully but binary file could not be found."
        return 1
    fi

    cd "$PKG_DIR"
}

deploy_packages() {
    local pkgs=("$@")
    
    if [ ${#pkgs[@]} -eq 0 ]; then
        echo "⚠️ No packages compiled for deployment."
        return 0
    fi
    
    echo "=============================================================================="
    echo "🌐 BULK DEPLOY TO CENTRAL ARCH REPOSITORY"
    echo "   Packages to deploy: ${#pkgs[@]}"
    echo "=============================================================================="
    
    if [ -z "$INLED_REPO_PAT" ]; then
        echo "❌ Error: INLED_REPO_PAT variable is not defined."
        return 1
    fi
    
    local use_gh=false
    local release_id=""
    
    # Prefer gh whenever it is available and authenticated (both locally and in
    # CI), since the upload to the PKG release assets needs a token with write
    # access to this repository. Fall back to curl + INLED_REPO_PAT otherwise.
    if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
        use_gh=true
        echo "🔧 Using GitHub CLI to verify/create release..."
        gh release view "$RELEASE_TAG" --repo "$OWNER_REPO" >/dev/null 2>&1 || {
            gh release create "$RELEASE_TAG" --title "PulsarOS Package Repository Assets" --notes "Repositorio de paquetes deb y arch compilados automáticamente." --repo "$OWNER_REPO"
        }
    else
        echo "🔧 Using GitHub API (curl) to verify/create release..."
        release_id=$(curl -s -H "Authorization: token $INLED_REPO_PAT" \
            "https://api.github.com/repos/${OWNER_REPO}/releases/tags/${RELEASE_TAG}" | jq -r '.id')
            
        if [ "$release_id" == "null" ] || [ -z "$release_id" ]; then
            echo "🆕 Creating new release..."
            local release_data=$(curl -s -X POST -H "Authorization: token $INLED_REPO_PAT" \
                -H "Content-Type: application/json" \
                -d "{\"tag_name\":\"$RELEASE_TAG\",\"title\":\"PulsarOS Package Repository Assets\",\"body\":\"Repositorio de paquetes deb y arch compilados automáticamente.\"}" \
                "https://api.github.com/repos/${OWNER_REPO}/releases")
            release_id=$(echo "$release_data" | jq -r '.id')
        fi
    fi
    
    local urls=()
    
    for pkg_file in "${pkgs[@]}"; do
        local file_name=$(basename "$pkg_file")
        local file_name_github="${file_name//:/.}"
        echo "⬆️ Uploading: $file_name (GitHub asset: $file_name_github)..."
        local package_url=""
        
        if [ "$use_gh" = true ]; then
            # Delete asset if already exists in release to overwrite
            gh release delete-asset "$RELEASE_TAG" "$file_name_github" --repo "$OWNER_REPO" -y >/dev/null 2>&1 || true
            gh release delete-asset "$RELEASE_TAG" "$file_name" --repo "$OWNER_REPO" -y >/dev/null 2>&1 || true
            local upload_response=$(gh release upload "$RELEASE_TAG" "$pkg_file" --repo "$OWNER_REPO" --clobber)
            package_url="https://github.com/${OWNER_REPO}/releases/download/${RELEASE_TAG}/${file_name_github}"
        else
            # Delete asset using curl fallback if exists
            local asset_id=$(curl -s -H "Authorization: token $INLED_REPO_PAT" \
                "https://api.github.com/repos/${OWNER_REPO}/releases/tags/${RELEASE_TAG}" | \
                jq -r --arg name "$file_name_github" '.assets[] | select(.name==$name) | .id')
                
            if [ -n "$asset_id" ] && [ "$asset_id" != "null" ]; then
                echo "🗑️ Deleting existing asset ID: $asset_id..."
                local del_resp=$(mktemp)
                local del_code=$(curl -s -o "$del_resp" -w "%{http_code}" -X DELETE \
                    -H "Authorization: token $INLED_REPO_PAT" \
                    "https://api.github.com/repos/${OWNER_REPO}/releases/assets/$asset_id")
                if [ "$del_code" != "200" ] && [ "$del_code" != "204" ]; then
                    echo "⚠️ Warning: Could not delete existing asset (HTTP $del_code). Trying to upload anyway..."
                    cat "$del_resp"
                fi
                rm -f "$del_resp"
            fi
            
            local upload_url="https://uploads.github.com/repos/${OWNER_REPO}/releases/${release_id}/assets?name=${file_name_github}"
            local upload_response=$(curl -s -X POST -H "Authorization: token $INLED_REPO_PAT" \
                -H "Content-Type: application/octet-stream" \
                --data-binary @"$pkg_file" \
                "$upload_url")
            package_url=$(echo "$upload_response" | jq -r '.browser_download_url')
        fi
        
        if [ "$package_url" == "null" ] || [ -z "$package_url" ]; then
            echo "❌ Error: Upload of $file_name to GitHub release failed."
            echo "   Response from GitHub:"
            echo "$upload_response"
            return 1
        fi
        
        echo "✅ Uploaded at: $package_url"
        urls+=("$package_url")
    done
    
    # Construct JSON payload for dispatch
    local urls_str="${urls[*]}"
    local json_payload=$(jq -n --arg urls "$urls_str" --arg branch "$BRANCH" '{package_urls: $urls, branch: $branch}')
    local request_body=$(jq -c -n --arg ev "package_upload" --argjson pay "$json_payload" '{event_type: $ev, client_payload: $pay}')
    
    # Send a single repository dispatch to central Inled APT/Arch repository
    echo "📡 Sending bulk dispatch to central repository..."
    local response_file=$(mktemp)
    local dispatch_response=$(curl -s -w "%{http_code}" -X POST \
         -H "Accept: application/vnd.github.v3+json" \
         -H "Authorization: token $INLED_REPO_PAT" \
         https://api.github.com/repos/InledGroup/apt/dispatches \
         -d "$request_body" -o "$response_file")
          
    if [ "$dispatch_response" -eq 204 ] || [ "$dispatch_response" -eq 200 ] || [ "$dispatch_response" -eq 201 ]; then
        echo "🚀 All packages deployment notified successfully! HTTP $dispatch_response"
        rm -f "$response_file"
    else
        echo "❌ Error notifying repository. HTTP $dispatch_response"
        cat "$response_file"
        rm -f "$response_file"
        return 1
    fi
}

# Upload mode: deploy a single package. By default it rebuilds the package first
# (useful when the build must run on a trusted machine, e.g. for EGO GNOME Shell
# extension downloads); use --onlyupload to deploy an already built package.
# It accepts either a direct path to a .pkg.tar.zst file or a package name,
# in which case the most recent matching package in the output dir is used.
if [ -n "$UPLOAD_FLAG" ] || [ -n "$ONLY_UPLOAD_FLAG" ]; then
    if [ -z "$PACKAGE_NAME" ]; then
        echo "❌ Error: --upload/--onlyupload requires a package name or a path to a .pkg.tar.zst file."
        exit 1
    fi
    if [ -f "$PACKAGE_NAME" ]; then
        # Direct path to an already built package: just upload it.
        UPLOAD_PKG="$PACKAGE_NAME"
    else
        if [ -z "$ONLY_UPLOAD_FLAG" ]; then
            echo "🏗️  UPLOAD MODE: Rebuilding $PACKAGE_NAME before upload..."
            build_single_package "$PACKAGE_NAME"
        else
            echo "⏫  ONLY-UPLOAD MODE: Uploading an already built package..."
        fi
        UPLOAD_PKG=$(ls -t "$OUTPUT_DIR/${PACKAGE_NAME}-"*.pkg.tar.zst 2>/dev/null | head -n 1)
    fi
    if [ -z "$UPLOAD_PKG" ] || [ ! -f "$UPLOAD_PKG" ]; then
        echo "❌ Error: No package found for '$PACKAGE_NAME' in $OUTPUT_DIR."
        exit 1
    fi
    echo "⏫  Uploading: $(basename "$UPLOAD_PKG")"
    deploy_packages "$UPLOAD_PKG"
    exit $?
fi

# Deploy-only mode: upload previously built packages without rebuilding
if [ -n "$DEPLOY_ONLY_FLAG" ]; then
    echo "🚀  DEPLOY-ONLY MODE: Uploading already built .pkg.tar.zst packages..."
    mapfile -t COMPILED_PKGS < <(find "$OUTPUT_DIR" -maxdepth 1 -name '*.pkg.tar.zst' 2>/dev/null | sort)
    # Never auto-deploy packages that must be uploaded manually.
    if [ ${#COMPILED_PKGS[@]} -gt 0 ]; then
        mapfile -t COMPILED_PKGS < <(for f in "${COMPILED_PKGS[@]}"; do
            base=$(basename "$f")
            skip=false
            for p in "${MANUAL_UPLOAD_ONLY[@]}"; do
                if [[ "$base" == "$p-"* ]]; then
                    skip=true
                    break
                fi
            done
            if $skip; then
                echo "⏭️  Skipping (manual upload only): $base" >&2
                continue
            fi
            echo "$f"
        done)
    fi
    deploy_packages "${COMPILED_PKGS[@]}"
    exit $?
fi

if [ "$PACKAGE_NAME" == "all" ]; then
    clean_orphan_packages
    if $INCREMENTAL; then
        echo "⚡  INCREMENTAL BUILD: Rebuilding only modified packages..."
    else
        echo "🏗️  FULL BUILD: Building all packages..."
    fi
    SKIPPED_MANUAL=()
    for pkg_dir in "$PKGBUILDS_DIR"/*/; do
        pkg_name=$(basename "$pkg_dir")
        # Skip Tube OS flavour packages in the general Pulsar OS build
        if [[ "$pkg_name" == tubeos-* ]] || [[ "$pkg_name" == tube-os-* ]] || [[ "$pkg_name" == "dockermigrate" ]]; then
            continue
        fi
        if [ -f "$pkg_dir/PKGBUILD" ]; then
            if [ -n "$DEPLOY_FLAG" ] || [ -n "$CI" ] || [ -n "$GITHUB_ACTIONS" ]; then
                if is_manual_upload_only "$pkg_name"; then
                    echo "⏭️  Skipping $pkg_name (manual upload only)..."
                    SKIPPED_MANUAL+=("$pkg_name")
                    continue
                fi
            fi

            if $INCREMENTAL && is_pkg_up_to_date "$pkg_name"; then
                existing_pkg=$(ls -t "$OUTPUT_DIR/${pkg_name}-"*.pkg.tar.zst 2>/dev/null | head -n 1)
                echo "⚡ [CACHED] Reusing $pkg_name: $(basename "$existing_pkg") (unmodified)"
                COMPILED_PKGS+=("$existing_pkg")
                stamp_git_commit "$pkg_name"
            else
                build_single_package "$pkg_name"
            fi
        fi
    done
    echo "=============================================================================="
    echo "🎉 Packages processed successfully!"
    echo "=============================================================================="
    if [ ${#SKIPPED_MANUAL[@]} -gt 0 ]; then
        echo "⚠️  ATTENTION: The following packages were NOT built/deployed automatically"
        echo "   because they must be uploaded manually from a trusted machine:"
        for p in "${SKIPPED_MANUAL[@]}"; do
            echo "     - $p"
        done
        echo "   Build them locally (or reuse the local build) and upload with:"
        echo "     ./package-and-deploy.sh <pkg> --upload --branch <stable|forky|rolling>"
        echo "   or, if the package is already built locally:"
        echo "     ./package-and-deploy.sh <pkg> --onlyupload --branch <stable|forky|rolling>"
    fi
else
    build_single_package "$PACKAGE_NAME"
fi

if [ "$DEPLOY_FLAG" == "--deploy" ]; then
    deploy_packages "${COMPILED_PKGS[@]}"
fi
