#pragma once

#include <algorithm>
#include <cstdint>
#include <optional>
#include <string_view>

namespace mayo {

/// sRGB color with straight (non-premultiplied) alpha, components in [0, 1].
struct Color {
    float r = 0;
    float g = 0;
    float b = 0;
    float a = 1;

    constexpr Color() = default;
    constexpr Color(float r_, float g_, float b_, float a_ = 1.0f) : r(r_), g(g_), b(b_), a(a_) {}

    /// 0xRRGGBB
    static constexpr Color hex(std::uint32_t rgb, float alpha = 1.0f) {
        return {static_cast<float>((rgb >> 16) & 0xff) / 255.0f, static_cast<float>((rgb >> 8) & 0xff) / 255.0f,
                static_cast<float>(rgb & 0xff) / 255.0f, alpha};
    }
    static constexpr Color rgb8(int r, int g, int b, float alpha = 1.0f) {
        return {static_cast<float>(r) / 255.0f, static_cast<float>(g) / 255.0f, static_cast<float>(b) / 255.0f, alpha};
    }
    static constexpr Color white(float alpha = 1.0f) { return {1, 1, 1, alpha}; }
    static constexpr Color black(float alpha = 1.0f) { return {0, 0, 0, alpha}; }
    static constexpr Color transparent() { return {0, 0, 0, 0}; }

    /// Parses "#RGB", "#RRGGBB" or "#RRGGBBAA" (leading '#' optional).
    static std::optional<Color> parse(std::string_view text);

    [[nodiscard]] constexpr Color withAlpha(float alpha) const { return {r, g, b, alpha}; }
    [[nodiscard]] constexpr Color multipliedAlpha(float factor) const { return {r, g, b, a * factor}; }
    [[nodiscard]] constexpr bool isTransparent() const { return a <= 0.0f; }

    /// Linear interpolation of all components (sRGB space, which matches how designers pick tints).
    [[nodiscard]] constexpr Color mix(const Color& o, float t) const {
        return {r + (o.r - r) * t, g + (o.g - g) * t, b + (o.b - b) * t, a + (o.a - a) * t};
    }
    /// Mixes towards white (amount > 0) or black (amount < 0), keeping alpha.
    [[nodiscard]] constexpr Color lighter(float amount) const {
        Color target = amount >= 0 ? white(a) : black(a);
        return mix(target, amount >= 0 ? amount : -amount);
    }
    /// Relative luminance per WCAG (uses linearized sRGB).
    [[nodiscard]] float luminance() const;

    [[nodiscard]] std::uint32_t toRGBA8() const;

    constexpr bool operator==(const Color&) const = default;
};

}  // namespace mayo
