#pragma once

// Font families beyond the built-in Inter: files on disk, the desktop's fonts (fontconfig), fallbacks for
// characters a font lacks, and metrics for laying out text on a grid (terminals, code).

#include <optional>
#include <string>
#include <string_view>
#include <vector>

#include "theme.hpp"

namespace mayo {

struct FontFile {
    std::string path;
    int index = 0;  ///< face index in a collection (and named instance << 16 for variable fonts)
};

/// The faces of one family. Only `regular` is required: a missing bold or italic is synthesized
/// (emboldened / slanted outlines). Variable fonts with a weight axis serve every weight from one file.
struct FontFamilyFiles {
    FontFile regular;
    FontFile bold;
    FontFile italic;
    FontFile boldItalic;
};

/// Registers a family; returns its id for Font::family, or nullopt if the regular face can't be opened.
std::optional<FontFamily> registerFontFamily(const FontFamilyFiles& files);

/// Resolves a fontconfig pattern ("monospace", "JetBrains Mono", "sans-serif:lang=ja") to installed files and
/// registers them. Linux: libfontconfig is loaded at run time; nullopt if it or the font is unavailable.
/// Results are cached per pattern.
std::optional<FontFamily> systemFontFamily(std::string_view pattern);

/// Installed family names, sorted (for font pickers).
std::vector<std::string> systemFontFamilies(bool monospaceOnly = false);

/// Families tried in order for characters the requested family has no glyph for (CJK, symbols...).
void addFallbackFontFamily(FontFamily family);

struct LineMetrics {
    float ascender = 0;   ///< points above the baseline
    float descender = 0;  ///< points below the baseline (positive)
    float lineGap = 0;    ///< the font's recommended extra leading
    float capHeight = 0;
};
LineMetrics fontMetrics(const Font& font);

/// Horizontal advance of one character in points (Inter's tracking included, other families untouched).
float glyphAdvance(char32_t cp, const Font& font);

}  // namespace mayo
