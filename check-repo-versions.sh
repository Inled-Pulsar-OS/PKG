#!/usr/bin/env bash
# ==============================================================================
# Pulsar OS - Package Version Checker (Local vs apt.inled.es / PKG releases)
# ==============================================================================
# Compara las versiones locales de los paquetes Arch y Debian con las versiones
# actualmente publicadas en el repositorio central (apt.inled.es / InledGroup/apt)
# y en la release de staging (Inled-Pulsar-OS/PKG packages-repo).
#
# Soporta comprobación separada o conjunta de ramas (stable, unstable, all).
#
# Uso:
#   ./check-repo-versions.sh                         # Comprueba Arch y Debian (todas las ramas)
#   ./check-repo-versions.sh --branch stable         # Solo rama stable
#   ./check-repo-versions.sh --branch unstable       # Solo rama unstable
#   ./check-repo-versions.sh --arch                  # Solo paquetes Arch
#   ./check-repo-versions.sh --deb                   # Solo paquetes Debian
#   ./check-repo-versions.sh --arch --branch unstable # Solo Arch en rama unstable
# ==============================================================================

set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKG_ROOT="$SCRIPT_DIR"

MODE_ARCH=true
MODE_DEB=true
TARGET_BRANCH="all"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --arch|--arch-only)
            MODE_ARCH=true
            MODE_DEB=false
            shift
            ;;
        --deb|--debian|--deb-only)
            MODE_ARCH=false
            MODE_DEB=true
            shift
            ;;
        --branch|-b)
            TARGET_BRANCH="$2"
            shift 2
            ;;
        --branch=*)
            TARGET_BRANCH="${1#*=}"
            shift
            ;;
        --stable)
            TARGET_BRANCH="stable"
            shift
            ;;
        --unstable)
            TARGET_BRANCH="unstable"
            shift
            ;;
        --all)
            TARGET_BRANCH="all"
            shift
            ;;
        --help|-h)
            echo "Uso: $0 [--arch | --deb] [--branch <stable|unstable|all>]"
            echo ""
            echo "Opciones:"
            echo "  --arch, --arch-only         Comprobar solo paquetes Arch Linux"
            echo "  --deb, --deb-only           Comprobar solo paquetes Debian"
            echo "  -b, --branch <branch>       Filtrar por rama: stable, unstable, all (por defecto: all)"
            echo "  --stable                    Equivalente a --branch stable"
            echo "  --unstable                  Equivalente a --branch unstable"
            echo "  --all                       Equivalente a --branch all"
            echo "  -h, --help                  Mostrar esta ayuda"
            exit 0
            ;;
        *)
            echo "Opción desconocida: $1"
            exit 1
            ;;
    esac
done

if [ "$TARGET_BRANCH" != "stable" ] && [ "$TARGET_BRANCH" != "unstable" ] && [ "$TARGET_BRANCH" != "all" ]; then
    echo "❌ Error: Rama inválida '$TARGET_BRANCH'. Debe ser 'stable', 'unstable' o 'all'."
    exit 1
fi

# Colores ANSI
BOLD="\033[1m"
RESET="\033[0m"
GREEN="\033[32m"
YELLOW="\033[33m"
RED="\033[31m"
BLUE="\033[34m"
CYAN="\033[36m"
GRAY="\033[90m"
MAGENTA="\033[35m"

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

ALL_REMOTE_ASSETS=("${REMOTE_PROD_ASSETS[@]}" "${REMOTE_STAGING_ASSETS[@]}")

echo -e "${GRAY}✔ Encontrados ${#REMOTE_PROD_ASSETS[@]} archivos en apt.inled.es y ${#REMOTE_STAGING_ASSETS[@]} en staging (PKG). [Filtro de rama: ${TARGET_BRANCH}]${RESET}\n"

# ==============================================================================
# ARCH LINUX CHECK
# ==============================================================================
if [ "$MODE_ARCH" = true ]; then
    echo -e "${BOLD}${BLUE}📦 === PAQUETES ARCH LINUX (.pkg.tar.zst) [Rama: ${TARGET_BRANCH}] ===${RESET}"

    if [ "$TARGET_BRANCH" = "all" ]; then
        printf "${BOLD}%-32s %-16s %-18s %-22s %-10s %-26s${RESET}\n" "PAQUETE" "LOCAL" "REPO STABLE" "REPO UNSTABLE" "EN PROD?" "ESTADO"
        printf "%-32s %-16s %-18s %-22s %-10s %-26s\n" "--------------------------------" "----------------" "------------------" "----------------------" "----------" "--------------------------"
    elif [ "$TARGET_BRANCH" = "stable" ]; then
        printf "${BOLD}%-32s %-16s %-18s %-10s %-26s${RESET}\n" "PAQUETE" "LOCAL" "REPO STABLE" "EN PROD?" "ESTADO"
        printf "%-32s %-16s %-18s %-10s %-26s\n" "--------------------------------" "----------------" "------------------" "----------" "--------------------------"
    else
        printf "${BOLD}%-32s %-22s %-22s %-10s %-26s${RESET}\n" "PAQUETE" "LOCAL ESPERADO" "REPO UNSTABLE" "EN PROD?" "ESTADO"
        printf "%-32s %-22s %-22s %-10s %-26s\n" "--------------------------------" "----------------------" "----------------------" "----------" "--------------------------"
    fi

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
        local_raw=$(bash -c 'unset pkgver pkgrel epoch; source "'"$pkgbuild"'" 2>/dev/null; if [ -n "$epoch" ]; then echo "${epoch}:${pkgver}-${pkgrel}"; else echo "${pkgver}-${pkgrel}"; fi')
        local_raw_clean="${local_raw// /}"
        
        # Limpiar versión local para stable
        local_clean_ver=$(echo "$local_raw_clean" | sed -E 's/[._-]unstable.*$//')
        
        # Versión esperada para unstable: añadir .unstable antes del pkgrel
        local_unstable_ver=""
        if [[ "$local_raw_clean" =~ ^(.*)-([0-9]+)$ ]]; then
            base_p="${BASH_REMATCH[1]}"
            rel_p="${BASH_REMATCH[2]}"
            clean_base=$(echo "$base_p" | sed -E 's/[._-]unstable.*$//')
            local_unstable_ver="${clean_base}.unstable-${rel_p}"
        else
            local_unstable_ver="${local_clean_ver}.unstable-1"
        fi

        # Buscar versiones remotas para este paquete
        matched_stable_vers=()
        matched_unstable_vers=()
        in_prod_stable="NO"
        in_prod_unstable="NO"

        for asset in "${ALL_REMOTE_ASSETS[@]}"; do
            case "$asset" in
                *.sig) continue ;;
            esac
            if [[ "$asset" == "${pkg_name}-"*".pkg.tar."* ]]; then
                raw="${asset#"${pkg_name}-"}"
                raw="${raw%.pkg.tar.*}"
                ver_rel="${raw%-*}"
                if [ -n "$ver_rel" ]; then
                    if [[ "$ver_rel" == *"unstable"* ]]; then
                        matched_unstable_vers+=("$ver_rel")
                    else
                        matched_stable_vers+=("$ver_rel")
                    fi
                fi
            fi
        done

        # Comprobar si está en producción (InledGroup/apt)
        for asset in "${REMOTE_PROD_ASSETS[@]}"; do
            if [[ "$asset" == "${pkg_name}-"*".pkg.tar."* ]]; then
                if [[ "$asset" == *"unstable"* ]]; then
                    in_prod_unstable="SÍ"
                else
                    in_prod_stable="SÍ"
                fi
            fi
        done

        # Obtener la versión más alta de cada rama
        highest_stable=""
        if [ ${#matched_stable_vers[@]} -gt 0 ]; then
            highest_stable="${matched_stable_vers[0]}"
            for r_ver in "${matched_stable_vers[@]}"; do
                if command -v vercmp >/dev/null 2>&1; then
                    if [ "$(vercmp "$r_ver" "$highest_stable")" -gt 0 ]; then
                        highest_stable="$r_ver"
                    fi
                fi
            done
        fi

        highest_unstable=""
        if [ ${#matched_unstable_vers[@]} -gt 0 ]; then
            highest_unstable="${matched_unstable_vers[0]}"
            for r_ver in "${matched_unstable_vers[@]}"; do
                if command -v vercmp >/dev/null 2>&1; then
                    if [ "$(vercmp "$r_ver" "$highest_unstable")" -gt 0 ]; then
                        highest_unstable="$r_ver"
                    fi
                fi
            done
        fi

        fix_epoch() {
            local loc="$1"
            local rem="$2"
            if [[ "$loc" == *:* ]] && [[ "$rem" != *:* ]] && [ -n "$rem" ]; then
                local epoch_prefix="${loc%%:*}"
                if [[ "$rem" == "${epoch_prefix}."* ]]; then
                    echo "${epoch_prefix}:${rem#"${epoch_prefix}."}"
                else
                    echo "$rem"
                fi
            else
                echo "$rem"
            fi
        }

        cmp_remote_stable=$(fix_epoch "$local_clean_ver" "$highest_stable")
        cmp_remote_unstable=$(fix_epoch "$local_unstable_ver" "$highest_unstable")

        in_prod_label="NO"
        if [ "$in_prod_stable" = "SÍ" ] && [ "$in_prod_unstable" = "SÍ" ]; then
            in_prod_label="SÍ"
        elif [ "$in_prod_stable" = "SÍ" ]; then
            in_prod_label="SÍ (stb)"
        elif [ "$in_prod_unstable" = "SÍ" ]; then
            in_prod_label="SÍ (uns)"
        fi

        if [ "$TARGET_BRANCH" = "all" ]; then
            display_stable="${highest_stable:---}"
            display_unstable="${highest_unstable:---}"
            
            estado=""
            if [ -z "$highest_stable" ] && [ -z "$highest_unstable" ]; then
                estado="${RED}🔴 NO SUBIDO${RESET}"
                missing_arch=$((missing_arch + 1))
            elif [ -z "$highest_unstable" ]; then
                estado="${CYAN}🔵 SOLO STABLE${RESET}"
                ok_arch=$((ok_arch + 1))
            elif [ -z "$highest_stable" ]; then
                estado="${MAGENTA}🟠 SOLO UNSTABLE${RESET}"
                ok_arch=$((ok_arch + 1))
            else
                cmp_res=0
                if command -v vercmp >/dev/null 2>&1; then
                    cmp_res=$(vercmp "$cmp_remote_stable" "$local_clean_ver")
                fi
                if [ "$cmp_res" -lt 0 ]; then
                    estado="${YELLOW}🟡 OBSOLETO (Local > Stable)${RESET}"
                    outdated_arch=$((outdated_arch + 1))
                else
                    estado="${GREEN}🟢 AL DÍA (Ambas ramas)${RESET}"
                    ok_arch=$((ok_arch + 1))
                fi
            fi
            printf "%-32s %-16s %-18s %-22s %-10s %-26b\n" "$pkg_name" "$local_clean_ver" "$display_stable" "$display_unstable" "$in_prod_label" "$estado"

        elif [ "$TARGET_BRANCH" = "stable" ]; then
            display_stable="${highest_stable:---}"
            estado=""
            if [ -z "$highest_stable" ]; then
                estado="${RED}🔴 NO SUBIDO${RESET}"
                missing_arch=$((missing_arch + 1))
            else
                cmp_res=0
                if command -v vercmp >/dev/null 2>&1; then
                    cmp_res=$(vercmp "$cmp_remote_stable" "$local_clean_ver")
                fi
                if [ "$cmp_res" -gt 0 ]; then
                    estado="${GREEN}🟢 POSTERIOR (Repo > Local)${RESET}"
                    newer_arch=$((newer_arch + 1))
                elif [ "$cmp_res" -eq 0 ]; then
                    estado="${GREEN}🟢 AL DÍA (Repo == Local)${RESET}"
                    ok_arch=$((ok_arch + 1))
                else
                    estado="${YELLOW}🟡 OBSOLETO (Local > Repo)${RESET}"
                    outdated_arch=$((outdated_arch + 1))
                fi
            fi
            printf "%-32s %-16s %-18s %-10s %-26b\n" "$pkg_name" "$local_clean_ver" "$display_stable" "$in_prod_stable" "$estado"

        else # unstable
            display_unstable="${highest_unstable:---}"
            estado=""
            if [ -z "$highest_unstable" ]; then
                estado="${RED}🔴 NO SUBIDO${RESET}"
                missing_arch=$((missing_arch + 1))
            else
                cmp_res=0
                if command -v vercmp >/dev/null 2>&1; then
                    cmp_res=$(vercmp "$cmp_remote_unstable" "$local_unstable_ver")
                fi
                if [ "$cmp_res" -ge 0 ]; then
                    estado="${GREEN}🟢 AL DÍA (En Unstable)${RESET}"
                    ok_arch=$((ok_arch + 1))
                else
                    estado="${YELLOW}🟡 OBSOLETO (Local > Unstable)${RESET}"
                    outdated_arch=$((outdated_arch + 1))
                fi
            fi
            printf "%-32s %-22s %-22s %-10s %-26b\n" "$pkg_name" "$local_unstable_ver" "$display_unstable" "$in_prod_unstable" "$estado"
        fi
    done

    echo ""
    echo -e "${GRAY}Total Arch: $total_arch | ${GREEN}Al día/Publicados: $((ok_arch + newer_arch))${RESET}${GRAY} | ${YELLOW}Obsoletos: $outdated_arch${RESET}${GRAY} | ${RED}Faltantes: $missing_arch${RESET}"
    echo ""
fi

# ==============================================================================
# DEBIAN CHECK
# ==============================================================================
if [ "$MODE_DEB" = true ]; then
    echo -e "${BOLD}${BLUE}📦 === PAQUETES DEBIAN (.deb) [Rama: ${TARGET_BRANCH}] ===${RESET}"

    if [ "$TARGET_BRANCH" = "all" ]; then
        printf "${BOLD}%-32s %-16s %-18s %-22s %-10s %-26s${RESET}\n" "PAQUETE" "LOCAL" "REPO STABLE" "REPO UNSTABLE" "EN PROD?" "ESTADO"
        printf "%-32s %-16s %-18s %-22s %-10s %-26s\n" "--------------------------------" "----------------" "------------------" "----------------------" "----------" "--------------------------"
    elif [ "$TARGET_BRANCH" = "stable" ]; then
        printf "${BOLD}%-32s %-16s %-18s %-10s %-26s${RESET}\n" "PAQUETE" "LOCAL" "REPO STABLE" "EN PROD?" "ESTADO"
        printf "%-32s %-16s %-18s %-10s %-26s\n" "--------------------------------" "----------------" "------------------" "----------" "--------------------------"
    else
        printf "${BOLD}%-32s %-22s %-22s %-10s %-26s${RESET}\n" "PAQUETE" "LOCAL ESPERADO" "REPO UNSTABLE" "EN PROD?" "ESTADO"
        printf "%-32s %-22s %-22s %-10s %-26s\n" "--------------------------------" "----------------------" "----------------------" "----------" "--------------------------"
    fi

    total_deb=0
    ok_deb=0
    newer_deb=0
    outdated_deb=0
    missing_deb=0

    for control in "$PKG_ROOT"/*/DEBIAN/control; do
        [ -f "$control" ] || continue
        pkg_folder=$(basename "$(dirname "$(dirname "$control")")")
        actual_pkg_name=$(grep -E '^Package:' "$control" | awk '{print $2}' | tr -d '[:space:]')
        [ -z "$actual_pkg_name" ] && actual_pkg_name="$pkg_folder"
        pkg_name="$actual_pkg_name"
        total_deb=$((total_deb + 1))

        local_raw=$(grep -E '^Version:' "$control" | awk '{print $2}' | tr -d '[:space:]')
        local_clean_ver=$(echo "$local_raw" | sed -E 's/(\+|-)(unstable|deb14|rolling).*$//')
        local_unstable_ver="${local_clean_ver}-unstable"

        matched_stable_vers=()
        matched_unstable_vers=()
        in_prod_stable="NO"
        in_prod_unstable="NO"

        for asset in "${ALL_REMOTE_ASSETS[@]}"; do
            for name_candidate in "$pkg_name" "$pkg_folder"; do
                if [[ "$asset" == "${name_candidate}_"*".deb" ]] || [[ "$asset" == "${name_candidate}-"*".deb" ]]; then
                    raw="${asset#"${name_candidate}_"}"
                    [ "$raw" = "$asset" ] && raw="${asset#"${name_candidate}-"}"
                    raw="${raw%.deb}"
                    ver_part="${raw%_*}"
                    [ "$ver_part" = "$raw" ] && ver_part="${raw%-*}"
                    if [ -n "$ver_part" ]; then
                        if [[ "$ver_part" == *"unstable"* ]] || [[ "$asset" == *"unstable"* ]]; then
                            matched_unstable_vers+=("$ver_part")
                        else
                            matched_stable_vers+=("$ver_part")
                        fi
                    fi
                fi
            done
        done

        for asset in "${REMOTE_PROD_ASSETS[@]}"; do
            for name_candidate in "$pkg_name" "$pkg_folder"; do
                if [[ "$asset" == "${name_candidate}_"*".deb" ]] || [[ "$asset" == "${name_candidate}-"*".deb" ]]; then
                    if [[ "$asset" == *"unstable"* ]]; then
                        in_prod_unstable="SÍ"
                    else
                        in_prod_stable="SÍ"
                    fi
                fi
            done
        done

        highest_stable=""
        if [ ${#matched_stable_vers[@]} -gt 0 ]; then
            highest_stable="${matched_stable_vers[0]}"
            for r_ver in "${matched_stable_vers[@]}"; do
                if command -v dpkg >/dev/null 2>&1; then
                    if dpkg --compare-versions "$r_ver" gt "$highest_stable" 2>/dev/null; then
                        highest_stable="$r_ver"
                    fi
                fi
            done
        fi

        highest_unstable=""
        if [ ${#matched_unstable_vers[@]} -gt 0 ]; then
            highest_unstable="${matched_unstable_vers[0]}"
            for r_ver in "${matched_unstable_vers[@]}"; do
                if command -v dpkg >/dev/null 2>&1; then
                    if dpkg --compare-versions "$r_ver" gt "$highest_unstable" 2>/dev/null; then
                        highest_unstable="$r_ver"
                    fi
                fi
            done
        fi

        in_prod_label="NO"
        if [ "$in_prod_stable" = "SÍ" ] && [ "$in_prod_unstable" = "SÍ" ]; then
            in_prod_label="SÍ"
        elif [ "$in_prod_stable" = "SÍ" ]; then
            in_prod_label="SÍ (stb)"
        elif [ "$in_prod_unstable" = "SÍ" ]; then
            in_prod_label="SÍ (uns)"
        fi

        if [ "$TARGET_BRANCH" = "all" ]; then
            display_stable="${highest_stable:---}"
            display_unstable="${highest_unstable:---}"
            estado=""

            if [ -z "$highest_stable" ] && [ -z "$highest_unstable" ]; then
                estado="${RED}🔴 NO SUBIDO${RESET}"
                missing_deb=$((missing_deb + 1))
            elif [ -z "$highest_unstable" ]; then
                estado="${CYAN}🔵 SOLO STABLE${RESET}"
                ok_deb=$((ok_deb + 1))
            elif [ -z "$highest_stable" ]; then
                estado="${MAGENTA}🟠 SOLO UNSTABLE${RESET}"
                ok_deb=$((ok_deb + 1))
            else
                cmp_res=0
                if command -v dpkg >/dev/null 2>&1; then
                    if dpkg --compare-versions "$local_clean_ver" gt "$highest_stable" 2>/dev/null; then
                        cmp_res=-1
                    else
                        cmp_res=0
                    fi
                fi
                if [ "$cmp_res" -lt 0 ]; then
                    estado="${YELLOW}🟡 OBSOLETO (Local > Stable)${RESET}"
                    outdated_deb=$((outdated_deb + 1))
                else
                    estado="${GREEN}🟢 AL DÍA (Ambas ramas)${RESET}"
                    ok_deb=$((ok_deb + 1))
                fi
            fi
            printf "%-32s %-16s %-18s %-22s %-10s %-26b\n" "$pkg_name" "$local_clean_ver" "$display_stable" "$display_unstable" "$in_prod_label" "$estado"

        elif [ "$TARGET_BRANCH" = "stable" ]; then
            display_stable="${highest_stable:---}"
            estado=""
            if [ -z "$highest_stable" ]; then
                estado="${RED}🔴 NO SUBIDO${RESET}"
                missing_deb=$((missing_deb + 1))
            else
                if command -v dpkg >/dev/null 2>&1; then
                    if dpkg --compare-versions "$highest_stable" gt "$local_clean_ver" 2>/dev/null; then
                        estado="${GREEN}🟢 POSTERIOR (Repo > Local)${RESET}"
                        newer_deb=$((newer_deb + 1))
                    elif dpkg --compare-versions "$highest_stable" eq "$local_clean_ver" 2>/dev/null; then
                        estado="${GREEN}🟢 AL DÍA (Repo == Local)${RESET}"
                        ok_deb=$((ok_deb + 1))
                    else
                        estado="${YELLOW}🟡 OBSOLETO (Local > Repo)${RESET}"
                        outdated_deb=$((outdated_deb + 1))
                    fi
                fi
            fi
            printf "%-32s %-16s %-18s %-10s %-26b\n" "$pkg_name" "$local_clean_ver" "$display_stable" "$in_prod_stable" "$estado"

        else # unstable
            display_unstable="${highest_unstable:---}"
            estado=""
            if [ -z "$highest_unstable" ]; then
                estado="${RED}🔴 NO SUBIDO${RESET}"
                missing_deb=$((missing_deb + 1))
            else
                clean_unstable=$(echo "$highest_unstable" | sed -E 's/[~._-]unstable.*$//')
                if command -v dpkg >/dev/null 2>&1; then
                    if dpkg --compare-versions "$clean_unstable" ge "$local_clean_ver" 2>/dev/null; then
                        estado="${GREEN}🟢 AL DÍA (En Unstable)${RESET}"
                        ok_deb=$((ok_deb + 1))
                    else
                        estado="${YELLOW}🟡 OBSOLETO (Local > Unstable)${RESET}"
                        outdated_deb=$((outdated_deb + 1))
                    fi
                fi
            fi
            printf "%-32s %-22s %-22s %-10s %-26b\n" "$pkg_name" "$local_unstable_ver" "$display_unstable" "$in_prod_unstable" "$estado"
        fi
    done

    echo ""
    echo -e "${GRAY}Total Debian: $total_deb | ${GREEN}Al día/Publicados: $((ok_deb + newer_deb))${RESET}${GRAY} | ${YELLOW}Obsoletos: $outdated_deb${RESET}${GRAY} | ${RED}Faltantes: $missing_deb${RESET}"
    echo ""
fi
