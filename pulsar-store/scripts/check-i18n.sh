#!/usr/bin/env bash
# Fail when a widget setter receives a string literal instead of tr!/ntr!/ptr!.
# Heuristic: it catches the common user-facing setters, not every possible case.
set -euo pipefail
cd "$(dirname "$0")/.."

setters='set_label|set_title|set_subtitle|set_text|set_tooltip_text|set_placeholder_text|set_description|title|subtitle|label|tooltip_text|placeholder_text|heading|body'
hits=$(grep -rnE --include='*.rs' "\.($setters)\((Some\()?&?\"" src \
    | grep -v '^src/core/i18n.rs' || true)

if [ -n "$hits" ]; then
    echo "untranslated user-facing string literals (wrap with tr!/ntr!/ptr!):"
    echo "$hits"
    exit 1
fi
echo "i18n ok"
