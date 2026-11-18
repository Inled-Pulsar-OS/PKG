#pragma once
#include <string>
#include <string_view>
#include "i18n.hpp"

namespace mayo {
// Optional display-only translation. Register before constructing widgets; values,
// IDs, editable contents and application callbacks retain their original text.
using UiTranslator = std::string (*)(std::string_view);
inline UiTranslator uiTranslator = nullptr;
inline void setUiTranslator(UiTranslator translator) { uiTranslator = translator; }
inline std::string uiText(std::string_view text) {
    if (uiTranslator) return uiTranslator(text);
    return i18n::tr(text);
}
}  // namespace mayo
