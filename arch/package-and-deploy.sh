#!/bin/bash
# ==============================================================================
# Pulsar OS - Arch Linux Package Builder and Deployer
# ==============================================================================
# Compiles all PKGBUILD packages into .pkg.tar.zst archives using makepkg
# and deploys them to the Inled Arch Linux repository via GitHub releases.
#
# Usage:
#   ./package-and-deploy.sh <package_name | all> [--deploy] [--branch <stable|forky|rolling>]
#
# Examples:
#   ./package-and-deploy.sh pulsaros-theme   # Build a single package
#   ./package-and-deploy.sh all              # Build ALL packages
#   ./package-and-deploy.sh all --deploy     # Build and deploy all
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
BRANCH="stable"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --deploy|-d)
            DEPLOY_FLAG="--deploy"
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
    echo "Usage: $0 <package_name | all> [--deploy] [--branch <stable|forky|rolling>]"
    exit 1
fi

COMPILED_PKGS=()

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
    PKGDEST="$OUTPUT_DIR" makepkg -cfd --noconfirm --nosign

    local pkg_file=$(ls "$OUTPUT_DIR/${name}-"*.pkg.tar.zst 2>/dev/null | head -n 1)
    if [ -n "$pkg_file" ]; then
        echo "✅ Package built: $(basename "$pkg_file")"
        COMPILED_PKGS+=("$pkg_file")
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
    
    if command -v gh >/dev/null 2>&1 && [ -n "$GITHUB_ACTIONS" ]; then
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
        echo "⬆️ Uploading: $file_name..."
        local package_url=""
        
        if [ "$use_gh" = true ]; then
            # Delete asset if already exists in release to overwrite
            gh release delete-asset "$RELEASE_TAG" "$file_name" --repo "$OWNER_REPO" -y >/dev/null 2>&1 || true
            local upload_response=$(gh release upload "$RELEASE_TAG" "$pkg_file" --repo "$OWNER_REPO" --clobber)
            package_url="https://github.com/${OWNER_REPO}/releases/download/${RELEASE_TAG}/${file_name}"
        else
            # Delete asset using curl fallback if exists
            local asset_id=$(curl -s -H "Authorization: token $INLED_REPO_PAT" \
                "https://api.github.com/repos/${OWNER_REPO}/releases/tags/${RELEASE_TAG}" | \
                jq -r --arg name "$file_name" '.assets[] | select(.name==$name) | .id')
                
            if [ -n "$asset_id" ] && [ "$asset_id" != "null" ]; then
                echo "🗑️ Deleting existing asset ID: $asset_id..."
                curl -s -X DELETE -H "Authorization: token $INLED_REPO_PAT" \
                    "https://api.github.com/repos/${OWNER_REPO}/releases/assets/$asset_id"
            fi
            
            local upload_url="https://uploads.github.com/repos/${OWNER_REPO}/releases/${release_id}/assets?name=${file_name}"
            local upload_response=$(curl -s -X POST -H "Authorization: token $INLED_REPO_PAT" \
                -H "Content-Type: application/octet-stream" \
                --data-binary @"$pkg_file" \
                "$upload_url")
            package_url=$(echo "$upload_response" | jq -r '.browser_download_url')
        fi
        
        if [ "$package_url" == "null" ] || [ -z "$package_url" ]; then
            echo "❌ Error: Upload of $file_name to GitHub release failed."
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

if [ "$PACKAGE_NAME" == "all" ]; then
    echo "🏗️  FULL BUILD: Building all packages..."
    for pkg_dir in "$PKGBUILDS_DIR"/*/; do
        pkg_name=$(basename "$pkg_dir")
        if [ -f "$pkg_dir/PKGBUILD" ]; then
            build_single_package "$pkg_name"
        fi
    done
    echo "=============================================================================="
    echo "🎉 All packages built successfully!"
    echo "=============================================================================="
else
    build_single_package "$PACKAGE_NAME"
fi

if [ "$DEPLOY_FLAG" == "--deploy" ]; then
    deploy_packages "${COMPILED_PKGS[@]}"
fi
