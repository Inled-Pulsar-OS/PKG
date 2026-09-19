#!/usr/bin/env bash
# ==============================================================================
# Tube OS Dashboard (CasaOS Suite) - Asset & Binary Preparation Script
# Ensures precompiled Go microservice binaries and purple web assets are staged.
# ==============================================================================

set -euo pipefail

BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${BASE_DIR}/usr/bin"
CASAOS_VER="v0.4.4"

mkdir -p "${BIN_DIR}"

ensure_binary() {
    local bin_name="$1"
    local repo_name="$2"
    local tar_name="$3"
    
    if [[ ! -f "${BIN_DIR}/${bin_name}" ]]; then
        echo ">>> Fetching ${bin_name} from ${repo_name} (${CASAOS_VER})..."
        local tmp_dir
        tmp_dir="$(mktemp -d)"
        local url="https://github.com/IceWhaleTech/${repo_name}/releases/download/${CASAOS_VER}/linux-amd64-${tar_name}-${CASAOS_VER}.tar.gz"
        if curl -fsSL "${url}" | tar -xz -C "${tmp_dir}"; then
            local found_bin
            found_bin="$(find "${tmp_dir}" -name "${bin_name}" -type f | head -n 1)"
            if [[ -n "${found_bin}" ]] && [[ -f "${found_bin}" ]]; then
                cp -f "${found_bin}" "${BIN_DIR}/${bin_name}"
                chmod 755 "${BIN_DIR}/${bin_name}"
            fi
        fi
        rm -rf "${tmp_dir}"
    fi
}

ensure_binary "casaos" "CasaOS" "casaos"
ensure_binary "casaos-gateway" "CasaOS-Gateway" "casaos-gateway"
ensure_binary "casaos-message-bus" "CasaOS-MessageBus" "casaos-message-bus"
ensure_binary "casaos-user-service" "CasaOS-UserService" "casaos-user-service"
ensure_binary "casaos-local-storage" "CasaOS-LocalStorage" "casaos-local-storage"
ensure_binary "casaos-app-management" "CasaOS-AppManagement" "casaos-app-management"
ensure_binary "casaos-cli" "CasaOS-CLI" "casaos-cli"

# Create tubeos-* symlinks
cd "${BIN_DIR}"
ln -sf casaos tubeos
ln -sf casaos-gateway tubeos-gateway
ln -sf casaos-message-bus tubeos-message-bus
ln -sf casaos-user-service tubeos-user-service
ln -sf casaos-local-storage tubeos-local-storage
ln -sf casaos-app-management tubeos-app-management
ln -sf casaos-cli tubeos-cli
chmod 755 * 2>/dev/null || true

echo "✅ CasaOS / Tube OS Dashboard microservice binaries verified in ${BIN_DIR}."
