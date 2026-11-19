#include "text_system.hpp"

#include <cmath>

#include "../core/utf8.hpp"
#include "renderer2d.hpp"

namespace mayo::render {

namespace {
constexpr int kInitialAtlas = 1024;
constexpr int kMaxAtlas = 4096;
constexpr int kPadding = 1;
}  // namespace

TextSystem::TextSystem() { reset(kInitialAtlas); }

void TextSystem::reset(int size) {
    m_size = size;
    m_shelfX = kPadding;
    m_shelfY = kPadding;
    m_shelfHeight = 0;
    m_entries.clear();
    // Glyph quads land on whole pixels, so nearest sampling is exact and avoids bleeding. Starts out cleared,
    // so the padding between glyphs stays empty.
    m_atlas = rhi::device()->createTexture(
        {.width = size, .height = size, .format = rhi::Format::R8, .filter = rhi::Filter::Nearest});
}

bool TextSystem::allocate(int w, int h, int& x, int& y) {
    if (m_shelfX + w + kPadding > m_size) {
        m_shelfX = kPadding;
        m_shelfY += m_shelfHeight + kPadding;
        m_shelfHeight = 0;
    }
    if (m_shelfY + h + kPadding > m_size) return false;
    x = m_shelfX;
    y = m_shelfY;
    m_shelfX += w + kPadding;
    m_shelfHeight = std::max(m_shelfHeight, h);
    return true;
}

const TextSystem::Entry& TextSystem::glyph(char32_t cp, const Font& font, float pixelSize, int subpixel,
                                           Renderer2D& renderer) {
    const Key key{(static_cast<std::uint64_t>(cp) << 32) | static_cast<std::uint64_t>(std::lround(pixelSize * 4.0f)),
                  (static_cast<std::uint64_t>(font.family) << 32) | (static_cast<std::uint64_t>(font.italic) << 24) |
                      (static_cast<std::uint64_t>(std::lround(std::clamp(font.size, 14.0f, 32.0f))) << 16) |
                      (static_cast<std::uint64_t>(font.weight) << 4) | static_cast<std::uint64_t>(subpixel)};
    auto it = m_entries.find(key);
    if (it != m_entries.end()) return it->second;

    Entry e;
    if (FontLibrary::instance().rasterize(cp, font, pixelSize, subpixel, m_scratch) && m_scratch.width > 0 &&
        m_scratch.height > 0) {
        int x = 0;
        int y = 0;
        if (!allocate(m_scratch.width, m_scratch.height, x, y)) {
            // Atlas full: draw what's queued against the old contents, then start over bigger.
            renderer.flushIfUsing(m_atlas.get());
            reset(std::min(m_size * 2, kMaxAtlas));
            allocate(m_scratch.width, m_scratch.height, x, y);
        }
        rhi::device()->upload(*m_atlas, x, y, m_scratch.width, m_scratch.height, m_scratch.pixels.data());
        auto fs = static_cast<float>(m_size);
        e.uv = Rect(static_cast<float>(x) / fs, static_cast<float>(y) / fs, static_cast<float>(m_scratch.width) / fs,
                    static_cast<float>(m_scratch.height) / fs);
        e.width = m_scratch.width;
        e.height = m_scratch.height;
        e.left = m_scratch.left;
        e.top = m_scratch.top;
    }
    return m_entries.emplace(key, e).first->second;
}

void TextSystem::drawLine(Renderer2D& renderer, std::string_view text, Point baseline, const Font& font,
                          const Color& color) {
    if (text.empty() || color.a <= 0) return;
    auto& lib = FontLibrary::instance();
    const float s = renderer.scale();
    const float pixelSize = font.size * s;
    // Light text on dark backgrounds reads thinner; embolden it a little via coverage gamma.
    const float gamma = color.luminance() > 0.5f ? 0.80f : 0.92f;
    const float baselinePx = std::round(baseline.y * s);
    float penPx = baseline.x * s;
    size_t i = 0;
    while (i < text.size()) {
        char32_t cp = utf8::next(text, i);
        float x = std::floor(penPx);
        int subpixel = std::clamp(static_cast<int>((penPx - x) * 4.0f), 0, 3);
        const Entry& e = glyph(cp, font, pixelSize, subpixel, renderer);
        if (e.width > 0) {
            Rect quad(x + static_cast<float>(e.left), baselinePx - static_cast<float>(e.top),
                      static_cast<float>(e.width), static_cast<float>(e.height));
            renderer.glyphPx(quad, m_atlas.get(), e.uv, color, gamma);
        }
        penPx += lib.advance(cp, font) * s;
    }
}

}  // namespace mayo::render
