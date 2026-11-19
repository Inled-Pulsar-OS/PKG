#include "mayonnaise/color.hpp"

#include <cmath>

namespace mayo {

namespace {

float srgbToLinear(float c) { return c <= 0.04045f ? c / 12.92f : std::pow((c + 0.055f) / 1.055f, 2.4f); }

int hexDigit(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

}  // namespace

std::optional<Color> Color::parse(std::string_view text) {
    if (!text.empty() && text.front() == '#') text.remove_prefix(1);
    if (text.size() != 3 && text.size() != 6 && text.size() != 8) return std::nullopt;
    for (char c : text) {
        if (hexDigit(c) < 0) return std::nullopt;
    }
    auto byteAt = [&](size_t i) { return hexDigit(text[i]) * 16 + hexDigit(text[i + 1]); };
    if (text.size() == 3) {
        auto nib = [&](size_t i) { return static_cast<float>(hexDigit(text[i]) * 17) / 255.0f; };
        return Color{nib(0), nib(1), nib(2), 1.0f};
    }
    Color c = Color::rgb8(byteAt(0), byteAt(2), byteAt(4));
    if (text.size() == 8) c.a = static_cast<float>(byteAt(6)) / 255.0f;
    return c;
}

float Color::luminance() const {
    return 0.2126f * srgbToLinear(r) + 0.7152f * srgbToLinear(g) + 0.0722f * srgbToLinear(b);
}

std::uint32_t Color::toRGBA8() const {
    auto q = [](float v) { return static_cast<std::uint32_t>(std::lround(std::clamp(v, 0.0f, 1.0f) * 255.0f)); };
    return (q(r) << 24) | (q(g) << 16) | (q(b) << 8) | q(a);
}

}  // namespace mayo
