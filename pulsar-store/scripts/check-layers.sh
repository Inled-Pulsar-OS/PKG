#!/usr/bin/env bash
# Enforce the layer rules from docs/adr/0002-single-crate-layout.md.
#   core    -> imports no ui, modules, or GTK
#   ui      -> imports no modules
#   modules -> import no other module
#   bin     -> imports no ui, modules, or GTK
set -euo pipefail
shopt -s nullglob
cd "$(dirname "$0")/.."

fail=0

# scan <dir> <label> <extended-regex>
scan() {
    local dir=$1 label=$2 pattern=$3
    [ -e "$dir" ] || return 0
    while IFS= read -r hit; do
        [ -n "$hit" ] || continue
        echo "layer violation: $label: $hit"
        fail=1
    done < <(grep -rnE --include='*.rs' "$pattern" "$dir" || true)
}

gui='(gtk4|libadwaita|adw)::|use (gtk4|adw|libadwaita)\b'
scan src/core "core must not use ui/modules/GTK" "crate::(ui|modules)\b|pulsar_store::(ui|modules)\b|$gui"
scan src/ui "ui must not use modules" "crate::modules\b"
scan src/bin "bin must not use ui/modules/GTK" "pulsar_store::(ui|modules)\b|crate::(ui|modules)\b|$gui"

if [ -d src/modules ]; then
    for m in src/modules/*/; do
        name=$(basename "$m")
        while IFS= read -r hit; do
            [ -n "$hit" ] || continue
            echo "layer violation: module '$name' must not use another module: $hit"
            fail=1
        done < <(grep -rnE --include='*.rs' "crate::modules::[a-z_]+" "$m" \
                 | grep -vE "crate::modules::${name}\b" || true)
    done
fi

[ "$fail" -eq 0 ] && echo "layers ok"
exit "$fail"
