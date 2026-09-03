#!/usr/bin/env bash
# ==============================================================================
# Pulsar OS - Package Version Checker (Local vs apt.inled.es / PKG releases)
# ==============================================================================
# Compara las versiones locales de los paquetes Arch y Debian con las versiones
# actualmente publicadas en el repositorio central (apt.inled.es / InledGroup/apt)
# y en la release de staging (Inled-Pulsar-OS/PKG packages-repo).
#
# Uso:
#   ./check-repo-versions.sh            # Comprueba Arch y Debian
#   ./check-repo-versions.sh --arch     # Solo paquetes Arch
#   ./check-repo-versions.sh --deb      # Solo paquetes Debian
# ==============================================================================

set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKG_ROOT="$SCRIPT_DIR"

MODE_ARCH=true
MODE_DEB=true

for arg in "$@"; do
    case "$arg" in
        --arch|--arch-only)
            MODE_ARCH=true
            MODE_DEB=false
            ;;
        --deb|--debian|--deb-only)
            MODE_ARCH=false
            MODE_DEB=true
            ;;
        --help|-h)
            echo "Uso: $0 [--arch | --deb]"
            exit 0
            ;;
    esac
done

# Colores ANSI
BOLD="\033[1m"
RESET="\033[0m"
GREEN="\033[32m"
YELLOW="\033[33m"
RED="\033[31m"
BLUE="\033[34m"
CYAN="\033[36m"
GRAY="\033[90m"

echo -e "${BOLD}${CYAN}🔍 Obteniendo lista de paquetes desplegados en repositorios remotos...${RESET}"

# 1. Obtener lista de assets de InledGroup/apt (producción) y Inled-Pulsar-OS/PKG (staging)
REMOTE_PROD_ASSETS=()
REMOTE_STAGING_ASSETS=()

if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    while IFS= read -r asset; do
        [ -n "$asset" ] && REMOTE_PROD_ASSETS+=("$asset")
    done < <(gh release view packages --repo InledGroup/apt --json assets --jq '.assets[].name' 2>/dev/null || true)

    while IFS= read -r asset; do
        [ -n "$asset" ] && REMOTE_STAGING_ASSETS+=("$asset")
    done < <(gh release view packages-repo --repo Inled-Pulsar-OS/PKG --json assets --jq '.assets[].name' 2>/dev/null || true)
fi

# Combinar todos los assets para tener cobertura completa
ALL_REMOTE_ASSETS=("${REMOTE_PROD_ASSETS[@]}" "${REMOTE_STAGING_ASSETS[@]}")

echo -e "${GRAY}✔ Encontrados ${#REMOTE_PROD_ASSETS[@]} archivos en apt.inled.es y ${#REMOTE_STAGING_ASSETS[@]} en staging (PKG).${RESET}\n"

# ==============================================================================
# ARCH LINUX CHECK
# ==============================================================================
if [ "$MODE_ARCH" = true ]; then
    echo -e "${BOLD}${BLUE}📦 === PAQUETES ARCH LINUX (.pkg.tar.zst) ===${RESET}"
    printf "${BOLD}%-32s %-16s %-16s %-12s %-26s${RESET}\n" "PAQUETE" "LOCAL" "REPO REMOTO" "EN PROD?" "ESTADO"
    printf "%-32s %-16s %-16s %-12s %-26s\n" "--------------------------------" "----------------" "----------------" "------------" "--------------------------"

    total_arch=0
    ok_arch=0
    newer_arch=0
    outdated_arch=0
    missing_arch=0

    for pkgbuild in "$PKG_ROOT"/arch/pkgbuilds/*/PKGBUILD; do
        [ -f "$pkgbuild" ] || continue
        pkg_name=$(basename "$(dirname "$pkgbuild")")
        total_arch=$((total_arch + 1))

        # Extraer versión local del PKGBUILD
        local_ver=$(bash -c "unset pkgver pkgrel epoch; source '$pkgbuild' 2>/dev/null; [ -n \"\$epoch\" ] && echo \"\${epoch}:\${pkgver}-\${pkgrel}\" || echo \"\${pkgver}-\${pkgrel}\"")
        local_ver_clean="${local_ver// /}"

        # Buscar versiones remotas para este paquete
        matched_remote_vers=()
        in_prod="NO"
        for asset in "${ALL_REMOTE_ASSETS[@]}"; do
            case "$asset" in
                *.sig) continue ;;
            esac
            if [[ "$asset" == "${pkg_name}-"*".pkg.tar.zst" ]]; then
                raw="${asset#"${pkg_name}-"}"
                raw="${raw%.pkg.tar.zst}"
                ver_part="${raw%-*}"
                [ -n "$ver_part" ] && matched_remote_vers+=("$ver_part")
            fi
        done

        # Comprobar si está en producción (InledGroup/apt)
        for asset in "${REMOTE_PROD_ASSETS[@]}"; do
            if [[ "$asset" == "${pkg_name}-"*".pkg.tar.zst" ]]; then
                in_prod="SÍ"
                break
            fi
        done

        if [ ${#matched_remote_vers[@]} -eq 0 ]; then
            printf "%-32s %-16s %-16s %-12s ${RED}%-26s${RESET}\n" "$pkg_name" "$local_ver_clean" "--" "$in_prod" "🔴 NO SUBIDO"
            missing_arch=$((missing_arch + 1))
            continue
        fi

        # Seleccionar la versión más alta si hay varias
        highest_remote="${matched_remote_vers[0]}"
        for r_ver in "${matched_remote_vers[@]}"; do
            if command -v vercmp >/dev/null 2>&1; then
                if [ "$(vercmp "$r_ver" "$highest_remote")" -gt 0 ]; then
                    highest_remote="$r_ver"
                fi
            fi
        done

        cmp_local="$local_ver_clean"
        cmp_remote="$highest_remote"

        if [[ "$cmp_local" == *:* ]] && [[ "$cmp_remote" != *:* ]]; then
            epoch_prefix="${cmp_local%%:*}"
            if [[ "$cmp_remote" == "${epoch_prefix}."* ]]; then
                cmp_remote="${epoch_prefix}:${cmp_remote#"${epoch_prefix}."}"
            fi
        fi

        cmp_result=0
        if command -v vercmp >/dev/null 2>&1; then
            cmp_result=$(vercmp "$cmp_remote" "$cmp_local")
        else
            [ "$cmp_remote" = "$cmp_local" ] && cmp_result=0 || cmp_result=-1
        fi

        if [ "$cmp_result" -gt 0 ]; then
            printf "%-32s %-16s %-16s %-12s ${GREEN}%-26s${RESET}\n" "$pkg_name" "$local_ver_clean" "$highest_remote" "$in_prod" "🟢 POSTERIOR (Repo > Local)"
            newer_arch=$((newer_arch + 1))
        elif [ "$cmp_result" -eq 0 ]; then
            printf "%-32s %-16s %-16s %-12s ${GREEN}%-26s${RESET}\n" "$pkg_name" "$local_ver_clean" "$highest_remote" "$in_prod" "🟢 AL DÍA (Repo == Local)"
            ok_arch=$((ok_arch + 1))
        else
            printf "%-32s %-16s %-16s %-12s ${YELLOW}%-26s${RESET}\n" "$pkg_name" "$local_ver_clean" "$highest_remote" "$in_prod" "🟡 OBSOLETO (Local > Repo)"
            outdated_arch=$((outdated_arch + 1))
        fi
    done

    echo ""
    echo -e "${GRAY}Total Arch: $total_arch | ${GREEN}Al día/Posterior: $((ok_arch + newer_arch))${RESET}${GRAY} | ${YELLOW}Obsoletos: $outdated_arch${RESET}${GRAY} | ${RED}Faltantes: $missing_arch${RESET}"
    echo ""
fi

# ==============================================================================
# DEBIAN CHECK
# ==============================================================================
if [ "$MODE_DEB" = true ]; then
    echo -e "${BOLD}${BLUE}📦 === PAQUETES DEBIAN (.deb) ===${RESET}"
    printf "${BOLD}%-32s %-16s %-20s %-12s %-26s${RESET}\n" "PAQUETE" "LOCAL" "REPO REMOTO" "EN PROD?" "ESTADO"
    printf "%-32s %-16s %-20s %-12s %-26s\n" "--------------------------------" "----------------" "--------------------" "------------" "--------------------------"

    total_deb=0
    ok_deb=0
    newer_deb=0
    outdated_deb=0
    missing_deb=0

    for control in "$PKG_ROOT"/*/DEBIAN/control; do
        [ -f "$control" ] || continue
        pkg_name=$(basename "$(dirname "$(dirname "$control")")")
        total_deb=$((total_deb + 1))

        local_ver=$(grep -E '^Version:' "$control" | awk '{print $2}' | tr -d '[:space:]')

        matched_remote_vers=()
        in_prod="NO"
        for asset in "${ALL_REMOTE_ASSETS[@]}"; do
            if [[ "$asset" == "${pkg_name}_"*".deb" ]] || [[ "$asset" == "${pkg_name}-"*".deb" ]]; then
                raw="${asset#"${pkg_name}_"}"
                [ "$raw" = "$asset" ] && raw="${asset#"${pkg_name}-"}"
                raw="${raw%.deb}"
                ver_part="${raw%_*}"
                [ -n "$ver_part" ] && matched_remote_vers+=("$ver_part")
            fi
        done

        for asset in "${REMOTE_PROD_ASSETS[@]}"; do
            if [[ "$asset" == "${pkg_name}_"*".deb" ]] || [[ "$asset" == "${pkg_name}-"*".deb" ]]; then
                in_prod="SÍ"
                break
            fi
        done

        if [ ${#matched_remote_vers[@]} -eq 0 ]; then
            printf "%-32s %-16s %-20s %-12s ${RED}%-26s${RESET}\n" "$pkg_name" "$local_ver" "--" "$in_prod" "🔴 NO SUBIDO"
            missing_deb=$((missing_deb + 1))
            continue
        fi

        highest_remote="${matched_remote_vers[0]}"
        for r_ver in "${matched_remote_vers[@]}"; do
            if command -v dpkg >/dev/null 2>&1; then
                if dpkg --compare-versions "$r_ver" gt "$highest_remote" 2>/dev/null; then
                    highest_remote="$r_ver"
                fi
            fi
        done

        cmp_result=""
        if command -v dpkg >/dev/null 2>&1; then
            if dpkg --compare-versions "$highest_remote" gt "$local_ver" 2>/dev/null; then
                cmp_result="newer"
            elif dpkg --compare-versions "$highest_remote" eq "$local_ver" 2>/dev/null; then
                cmp_result="equal"
            else
                cmp_result="older"
            fi
        else
            [ "$highest_remote" = "$local_ver" ] && cmp_result="equal" || cmp_result="older"
        fi

        if [ "$cmp_result" = "newer" ]; then
            printf "%-32s %-16s %-20s %-12s ${GREEN}%-26s${RESET}\n" "$pkg_name" "$local_ver" "$highest_remote" "$in_prod" "🟢 POSTERIOR (Repo > Local)"
            newer_deb=$((newer_deb + 1))
        elif [ "$cmp_result" = "equal" ]; then
            printf "%-32s %-16s %-20s %-12s ${GREEN}%-26s${RESET}\n" "$pkg_name" "$local_ver" "$highest_remote" "$in_prod" "🟢 AL DÍA (Repo == Local)"
            ok_deb=$((ok_deb + 1))
        else
            printf "%-32s %-16s %-20s %-12s ${YELLOW}%-26s${RESET}\n" "$pkg_name" "$local_ver" "$highest_remote" "$in_prod" "🟡 OBSOLETO (Local > Repo)"
            outdated_deb=$((outdated_deb + 1))
        fi
    done

    echo ""
    echo -e "${GRAY}Total Debian: $total_deb | ${GREEN}Al día/Posterior: $((ok_deb + newer_deb))${RESET}${GRAY} | ${YELLOW}Obsoletos: $outdated_deb${RESET}${GRAY} | ${RED}Faltantes: $missing_deb${RESET}"
    echo ""
fi
