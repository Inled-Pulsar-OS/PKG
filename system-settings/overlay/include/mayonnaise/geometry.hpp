#pragma once

#include <algorithm>
#include <cmath>
#include <limits>

namespace mayo {

inline constexpr float kInfinity = std::numeric_limits<float>::infinity();

struct Vec2 {
    float x = 0;
    float y = 0;

    constexpr Vec2() = default;
    constexpr Vec2(float x_, float y_) : x(x_), y(y_) {}

    constexpr Vec2 operator+(Vec2 o) const { return {x + o.x, y + o.y}; }
    constexpr Vec2 operator-(Vec2 o) const { return {x - o.x, y - o.y}; }
    constexpr Vec2 operator*(float s) const { return {x * s, y * s}; }
    constexpr Vec2 operator/(float s) const { return {x / s, y / s}; }
    constexpr Vec2 operator-() const { return {-x, -y}; }
    constexpr Vec2& operator+=(Vec2 o) { x += o.x; y += o.y; return *this; }
    constexpr Vec2& operator-=(Vec2 o) { x -= o.x; y -= o.y; return *this; }
    constexpr Vec2& operator*=(float s) { x *= s; y *= s; return *this; }
    constexpr bool operator==(const Vec2&) const = default;

    [[nodiscard]] float length() const { return std::sqrt(x * x + y * y); }
    [[nodiscard]] constexpr float dot(Vec2 o) const { return x * o.x + y * o.y; }
};

using Point = Vec2;

struct Size {
    float width = 0;
    float height = 0;

    constexpr Size() = default;
    constexpr Size(float w, float h) : width(w), height(h) {}
    constexpr bool operator==(const Size&) const = default;
    [[nodiscard]] constexpr bool isEmpty() const { return width <= 0 || height <= 0; }
};

struct Insets {
    float top = 0;
    float leading = 0;
    float bottom = 0;
    float trailing = 0;

    constexpr Insets() = default;
    constexpr Insets(float all) : top(all), leading(all), bottom(all), trailing(all) {}  // NOLINT(google-explicit-constructor)
    constexpr Insets(float vertical, float horizontal)
        : top(vertical), leading(horizontal), bottom(vertical), trailing(horizontal) {}
    constexpr Insets(float t, float l, float b, float r) : top(t), leading(l), bottom(b), trailing(r) {}

    [[nodiscard]] constexpr float horizontal() const { return leading + trailing; }
    [[nodiscard]] constexpr float vertical() const { return top + bottom; }
    constexpr bool operator==(const Insets&) const = default;
};

struct Rect {
    float x = 0;
    float y = 0;
    float width = 0;
    float height = 0;

    constexpr Rect() = default;
    constexpr Rect(float x_, float y_, float w, float h) : x(x_), y(y_), width(w), height(h) {}
    constexpr Rect(Point origin, Size size) : x(origin.x), y(origin.y), width(size.width), height(size.height) {}

    static constexpr Rect fromLTRB(float l, float t, float r, float b) { return {l, t, r - l, b - t}; }

    [[nodiscard]] constexpr float left() const { return x; }
    [[nodiscard]] constexpr float top() const { return y; }
    [[nodiscard]] constexpr float right() const { return x + width; }
    [[nodiscard]] constexpr float bottom() const { return y + height; }
    [[nodiscard]] constexpr Point origin() const { return {x, y}; }
    [[nodiscard]] constexpr Size size() const { return {width, height}; }
    [[nodiscard]] constexpr Point center() const { return {x + width * 0.5f, y + height * 0.5f}; }
    [[nodiscard]] constexpr bool isEmpty() const { return width <= 0 || height <= 0; }

    [[nodiscard]] constexpr bool contains(Point p) const {
        return p.x >= x && p.y >= y && p.x < x + width && p.y < y + height;
    }
    [[nodiscard]] constexpr Rect inset(const Insets& i) const {
        return {x + i.leading, y + i.top, std::max(0.0f, width - i.horizontal()),
                std::max(0.0f, height - i.vertical())};
    }
    [[nodiscard]] constexpr Rect inset(float dx, float dy) const { return inset(Insets(dy, dx)); }
    [[nodiscard]] constexpr Rect inset(float d) const { return inset(Insets(d)); }
    [[nodiscard]] constexpr Rect outset(float d) const { return {x - d, y - d, width + 2 * d, height + 2 * d}; }
    [[nodiscard]] constexpr Rect translated(Vec2 d) const { return {x + d.x, y + d.y, width, height}; }
    [[nodiscard]] constexpr Rect intersected(const Rect& o) const {
        float l = std::max(left(), o.left());
        float t = std::max(top(), o.top());
        float r = std::min(right(), o.right());
        float b = std::min(bottom(), o.bottom());
        return {l, t, std::max(0.0f, r - l), std::max(0.0f, b - t)};
    }
    [[nodiscard]] constexpr Rect united(const Rect& o) const {
        if (isEmpty()) return o;
        if (o.isEmpty()) return *this;
        return fromLTRB(std::min(left(), o.left()), std::min(top(), o.top()), std::max(right(), o.right()),
                        std::max(bottom(), o.bottom()));
    }
    [[nodiscard]] constexpr bool intersects(const Rect& o) const {
        return left() < o.right() && o.left() < right() && top() < o.bottom() && o.top() < bottom();
    }
    constexpr bool operator==(const Rect&) const = default;
};

/// Per-corner radii in points, clockwise from the top-left corner.
struct CornerRadii {
    float topLeft = 0;
    float topRight = 0;
    float bottomRight = 0;
    float bottomLeft = 0;

    constexpr CornerRadii() = default;
    constexpr CornerRadii(float all) : topLeft(all), topRight(all), bottomRight(all), bottomLeft(all) {}  // NOLINT(google-explicit-constructor)
    constexpr CornerRadii(float tl, float tr, float br, float bl)
        : topLeft(tl), topRight(tr), bottomRight(br), bottomLeft(bl) {}

    [[nodiscard]] constexpr bool isZero() const {
        return topLeft == 0 && topRight == 0 && bottomRight == 0 && bottomLeft == 0;
    }
    [[nodiscard]] constexpr float max() const { return std::max({topLeft, topRight, bottomRight, bottomLeft}); }
    constexpr bool operator==(const CornerRadii&) const = default;
};

[[nodiscard]] constexpr float lerp(float a, float b, float t) { return a + (b - a) * t; }
[[nodiscard]] constexpr Vec2 lerp(Vec2 a, Vec2 b, float t) { return {lerp(a.x, b.x, t), lerp(a.y, b.y, t)}; }
[[nodiscard]] constexpr Rect lerp(const Rect& a, const Rect& b, float t) {
    return {lerp(a.x, b.x, t), lerp(a.y, b.y, t), lerp(a.width, b.width, t), lerp(a.height, b.height, t)};
}

}  // namespace mayo
