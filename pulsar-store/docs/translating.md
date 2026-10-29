# Translating

Pulsar Store uses gettext. Source strings are English. Translations live in `assets/locale/<lang>.po`; Spanish (`es.po`) is complete.

## Add a language

1. `scripts/update-pot.sh` to refresh `assets/locale/pulsar-store.pot`.
2. `msginit --no-translator -l <lang> -i assets/locale/pulsar-store.pot -o assets/locale/<lang>.po`
3. Translate the `msgstr` lines. Keep placeholders such as `{name}` unchanged. Strings with a `msgctxt` (for example `button`) are short labels; translate them as such.
4. `msgfmt --check --statistics -o /dev/null assets/locale/<lang>.po` must report no errors.

## Try it

```
mkdir -p /tmp/loc/<lang>/LC_MESSAGES
msgfmt -o /tmp/loc/<lang>/LC_MESSAGES/pulsar-store.mo assets/locale/<lang>.po
LC_ALL=en_US.UTF-8 LANGUAGE=<lang> PULSAR_STORE_LOCALE_DIR=/tmp/loc cargo run
```

If your shell sets `LC_ALL=C.UTF-8`, override it as above. gettext ignores `LANGUAGE` when the locale is `C`.

## Other checks

- `PULSAR_STORE_PSEUDO=1` shows every string accented and padded, to spot untranslated text and layouts that break with longer words.
- `PULSAR_STORE_RTL=1` forces right-to-left layout.
- After changing code strings, run `scripts/update-pot.sh`; it also merges new strings into every `.po`.
- Catalog text (names, summaries) is translated by publishers through the language maps described in `catalog-submission.md`.
