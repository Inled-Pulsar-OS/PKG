#pragma once

// CPU side of text: FreeType faces, metrics, shaping-lite (advance + tracking) and rasterization.
// Independent of GL so layout can be unit tested headless.

#include <cstdint>
#include <memory>
#include <optional>
#include <string_view>
#include <vector>

#include "mayonnaise/font.hpp"
#include "mayonnaise/theme.hpp"

namespace mayo::render {

struct FontMetrics {
    float ascender = 0;   // points, positive
    float descender = 0;  // points, positive (below baseline)
    float capHeight = 0;
    float xHeight = 0;
    float lineGap = 0;
};

struct GlyphBitmap {
    int width = 0;
    int height = 0;
    int left = 0;  // pixels from pen to left edge
    int top = 0;   // pixels from baseline to top edge (up is positive)
    std::vector<std::uint8_t> pixels;
};

class FontLibrary {
public:
    static FontLibrary& instance();
    ~FontLibrary();
    FontLibrary(const FontLibrary&) = delete;
    FontLibrary& operator=(const FontLibrary&) = delete;

    std::optional<FontFamily> registerFamily(const FontFamilyFiles& files);
    void addFallback(FontFamily family);
    /// Horizontal advance of `cp` in points, including Inter's size-dependent tracking.
    float advance(char32_t cp, const Font& font);
    FontMetrics metrics(const Font& font);
    /// Renders `cp` at `pixelSize` with a horizontal subpixel offset of `subpixel / 4` px.
    bool rasterize(char32_t cp, const Font& font, float pixelSize, int subpixel, GlyphBitmap& out);

    /// Inter "dynamic metrics" tracking, in em, for a size in points.
    static float tracking(float size);

private:
    FontLibrary();
    struct Impl;
    std::unique_ptr<Impl> m_impl;
};

/// A laid-out line of text: byte range in the source string and its width in points.
struct TextLine {
    size_t begin = 0;
    size_t end = 0;
    float width = 0;
};

float measureText(std::string_view text, const Font& font);
/// Greedy word wrap; hard breaks on '\n'. maxWidth <= 0 or infinite means no wrapping.
std::vector<TextLine> wrapText(std::string_view text, const Font& font, float maxWidth);
/// X offset (points) of every code point boundary: result[k] is the caret position before the
/// k-th boundary in `boundaries` (byte indices, including 0 and text.size()).
void caretPositions(std::string_view text, const Font& font, std::vector<size_t>& boundaries,
                    std::vector<float>& positions);

}  // namespace mayo::render
