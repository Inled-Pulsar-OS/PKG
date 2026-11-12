// Tiny translation layer: i18n/<lang>.json maps an English string to its translation.
// A missing catalog, file or string falls back to the English text, so the UI never breaks.
import GLib from 'gi://GLib';

// ['es_ES.UTF-8', 'es_ES', 'es', 'C'] -> ['es_ES', 'es']: no encoding, no modifier, no C.
function candidates(names) {
    const seen = [];
    for (const name of names) {
        const lang = name.split('.')[0].split('@')[0];
        if (lang === 'C' || lang === 'POSIX')
            continue;
        // es_ES then es: the base language is the fallback for a region without its own file.
        for (const entry of [lang, lang.split('_')[0]]) {
            if (entry && !seen.includes(entry))
                seen.push(entry);
        }
    }
    return seen;
}

function readCatalog(dir, lang) {
    try {
        const path = GLib.build_filenamev([dir, 'i18n', `${lang}.json`]);
        const [ok, bytes] = GLib.file_get_contents(path);
        return ok ? JSON.parse(new TextDecoder().decode(bytes)) : null;
    } catch (_e) {
        return null;
    }
}

/** Returns _(text). `dir` is the extension directory; `languages` defaults to the session's. */
export function loadTranslator(dir, languages = GLib.get_language_names()) {
    for (const lang of candidates(languages)) {
        const catalog = readCatalog(dir, lang);
        if (catalog)
            return text => catalog[text] ?? text;
    }
    return text => text;
}
