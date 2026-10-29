#!/usr/bin/env bash
# Regenerates docs/screenshots from the real app (see scripts/screenshot.sh).
# Windows appear on screen for a few seconds each. Review the diff by eye.
set -euo pipefail
cd "$(dirname "$0")/.."
out=docs/screenshots
mkdir -p "$out"
shot() { scripts/screenshot.sh "$1" "$out/$2.png" "$3"; }

shot discover                         discover-dark   dark
shot discover                         discover-light  light
shot apps                             apps-dark       dark
shot sayri                            sayri-light     light
shot extensions                       extensions-dark dark
shot item:sayri-gateway-telegram      item-dark       dark
shot item:sayri-gateway-telegram      item-light      light
shot updates                          updates-dark    dark
shot search:telegram                  search-dark     dark

# Spanish, to check that translated strings fit.
locale_dir=$(mktemp -d)
mkdir -p "$locale_dir/es/LC_MESSAGES"
msgfmt -o "$locale_dir/es/LC_MESSAGES/pulsar-store.mo" assets/locale/es.po
LANGUAGE=es PULSAR_STORE_LOCALE_DIR="$locale_dir" scripts/screenshot.sh discover "$out/discover-es-dark.png" dark
rm -r -- "$locale_dir"
