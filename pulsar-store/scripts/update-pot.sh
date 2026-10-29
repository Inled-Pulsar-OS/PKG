#!/usr/bin/env bash
# Regenerate assets/locale/pulsar-store.pot from tr!, ntr!, and ptr! calls,
# then merge the result into every existing .po file.
set -euo pipefail
cd "$(dirname "$0")/.."

pot=assets/locale/pulsar-store.pot
find src -name '*.rs' ! -path src/core/i18n.rs -print0 | sort -z | xargs -0 xgettext \
    --language=Rust --from-code=UTF-8 \
    --keyword=tr! --keyword=ntr!:1,2 --keyword=ptr!:1c,2 \
    --package-name=pulsar-store --copyright-holder="Pulsar Store contributors" \
    --msgid-bugs-address="https://github.com/Victor1890/pulsaros-PKG/issues" --add-comments=TRANSLATORS \
    --output="$pot"

# English is the source language: en.po is generated from the template, never edited by hand.
msgen --output-file=assets/locale/en.po "$pot"
sed -i -e 's/^"Language: .*/"Language: en\\n"/' \
       -e 's/^"Plural-Forms: .*/"Plural-Forms: nplurals=2; plural=(n != 1);\\n"/' assets/locale/en.po

for po in assets/locale/*.po; do
    [ -e "$po" ] || continue
    [ "$po" = assets/locale/en.po ] && continue
    msgmerge --update --backup=none --quiet "$po" "$pot"
done
echo "updated $pot"
