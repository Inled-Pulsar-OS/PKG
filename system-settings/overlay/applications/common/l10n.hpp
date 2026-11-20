#pragma once

// Language selection for the Forma applications. English is the source language and the
// fallback: a key that has no translation is shown as written in the code.
//
// The language follows the device locale (LC_ALL, LC_MESSAGES, LANGUAGE, then LANG). Supported
// locales are Spanish, French, Italian and Simplified Chinese; anything else falls back to
// English. FORMA_LANGUAGE=es|fr|it|en|zh overrides the detection (useful for screenshots/tests).

#include <mayonnaise/ui_translation.hpp>

#include <clocale>
#include <cstdlib>
#include <cstring>
#include <string>
#include <string_view>
#include <unordered_map>

#include "l10n_es.hpp"
#include "l10n_fr.hpp"
#include "l10n_it.hpp"
#include "zh_cn.hpp"

namespace forma {

enum class Language { English, Spanish, French, Italian, Chinese };

inline std::string_view languageCode(std::string_view locale) {
    const size_t end = locale.find_first_of("._@:-");
    const std::string_view code = locale.substr(0, end);
    if (code == "es") return "es";
    if (code == "fr") return "fr";
    if (code == "it") return "it";
    if (code == "zh") return "zh";
    if (code == "en") return "en";
    return {};
}

inline Language detectLanguage() {
    if (const char* forced = std::getenv("FORMA_LANGUAGE")) {
        const std::string_view code = languageCode(forced);
        if (code == "es") return Language::Spanish;
        if (code == "fr") return Language::French;
        if (code == "it") return Language::Italian;
        if (code == "zh") return Language::Chinese;
        if (code == "en") return Language::English;
    }
    // LC_ALL wins over LC_MESSAGES; LANGUAGE is a colon-separated preference list.
    for (const char* name : {"LC_ALL", "LC_MESSAGES", "LANGUAGE", "LANG"}) {
        const char* value = std::getenv(name);
        if (!value || !*value) continue;
        const std::string_view list(value);
        size_t start = 0;
        while (start <= list.size()) {
            const size_t stop = list.find(':', start);
            const std::string_view entry = list.substr(start, stop == std::string_view::npos ? std::string_view::npos : stop - start);
            const std::string_view code = languageCode(entry);
            if (!code.empty()) {
                if (code == "es") return Language::Spanish;
                if (code == "fr") return Language::French;
                if (code == "it") return Language::Italian;
                if (code == "zh") return Language::Chinese;
                if (code == "en") return Language::English;
            }
            if (stop == std::string_view::npos) break;
            start = stop + 1;
        }
    }
    if (const char* loc = std::setlocale(LC_ALL, nullptr)) {
        const std::string_view code = languageCode(loc);
        if (code == "es") return Language::Spanish;
        if (code == "fr") return Language::French;
        if (code == "it") return Language::Italian;
        if (code == "zh") return Language::Chinese;
    }
    return Language::English;
}

inline Language& uiLanguage() {
    static Language language = detectLanguage();
    return language;
}

inline const std::string_view* lookup(std::string_view text) {
    const std::unordered_map<std::string_view, std::string_view>* catalogue = nullptr;
    switch (uiLanguage()) {
        case Language::Spanish: catalogue = &spanishMessages(); break;
        case Language::French: catalogue = &frenchMessages(); break;
        case Language::Italian: catalogue = &italianMessages(); break;
        case Language::Chinese:
        case Language::English: return nullptr;
    }
    if (auto found = catalogue->find(text); found != catalogue->end()) return &found->second;
    return nullptr;
}

/// Translates display text. Unknown keys stay in English, whatever the locale.
inline std::string tr(std::string_view text) {
    if (uiLanguage() == Language::Chinese) return zhCN(text);
    if (const auto* translated = lookup(text)) return std::string(*translated);
    return std::string(text);
}

/// Replaces `{name}`-style placeholders in an already translated message.
inline std::string message(std::string_view key,
                           std::initializer_list<std::pair<std::string_view, std::string>> values) {
    std::string result = tr(key);
    for (const auto& [name, value] : values) {
        const std::string placeholder = "{" + std::string(name) + "}";
        size_t at = 0;
        while ((at = result.find(placeholder, at)) != std::string::npos) {
            result.replace(at, placeholder.size(), value);
            at += value.size();
        }
    }
    return result;
}

/// Registers the catalogue as the framework's display-only translator. Call before creating widgets.
inline void localizeUi() {
    mayo::setUiTranslator([](std::string_view text) { return tr(text); });
}

}  // namespace forma
