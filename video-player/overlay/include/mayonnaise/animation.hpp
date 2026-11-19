#pragma once

#include <algorithm>
#include <cmath>

#include "color.hpp"
#include "geometry.hpp"

namespace mayo {

enum class Curve : unsigned char {
    Linear,
    EaseOut,     ///< cubic ease-out: fast start, gentle stop (hover, fades)
    EaseInOut,   ///< cubic ease-in-out (larger movements)
    Spring,      ///< critically-damped-ish spring with a hint of overshoot (toggles, knobs)
    Bouncy,      ///< springier: visible overshoot (popovers appearing)
};

struct Transition {
    float duration = 0.2f;  ///< seconds
    Curve curve = Curve::EaseOut;

    static constexpr Transition none() { return {0.0f, Curve::Linear}; }
    static constexpr Transition fast() { return {0.15f, Curve::EaseOut}; }
    static constexpr Transition standard() { return {0.25f, Curve::EaseOut}; }
    static constexpr Transition spring(float duration = 0.35f) { return {duration, Curve::Spring}; }
    static constexpr Transition bouncy(float duration = 0.45f) { return {duration, Curve::Bouncy}; }
};

/// Maps linear progress t in [0, 1] through a curve (springs may exceed 1 transiently).
float applyCurve(Curve curve, float t);

inline Color interpolate(const Color& a, const Color& b, float t) { return a.mix(b, t); }
inline float interpolate(float a, float b, float t) { return lerp(a, b, t); }
inline Vec2 interpolate(Vec2 a, Vec2 b, float t) { return lerp(a, b, t); }
inline Rect interpolate(const Rect& a, const Rect& b, float t) { return lerp(a, b, t); }

/// A value that tweens towards a target over time. Time is passed explicitly (the window's frame
/// clock) so values are deterministic and testable.
template <class T>
class Animated {
public:
    Animated() = default;
    explicit Animated(T value) : m_from(value), m_to(value) {}

    /// Jumps to `value` immediately.
    void set(T value) {
        m_from = m_to = value;
        m_duration = 0;
    }

    /// Starts animating from the current value towards `target`. No-op if already heading there.
    void animateTo(T target, double now, Transition transition) {
        if (target == m_to) return;
        if (transition.duration <= 0) {
            set(target);
            return;
        }
        m_from = value(now);
        m_to = target;
        m_start = now;
        m_duration = transition.duration;
        m_curve = transition.curve;
    }

    [[nodiscard]] T value(double now) const {
        if (m_duration <= 0) return m_to;
        auto t = static_cast<float>((now - m_start) / m_duration);
        if (t >= 1.0f) return m_to;
        return interpolate(m_from, m_to, applyCurve(m_curve, std::max(t, 0.0f)));
    }

    [[nodiscard]] bool isAnimating(double now) const { return m_duration > 0 && now - m_start < m_duration; }
    [[nodiscard]] const T& target() const { return m_to; }

private:
    T m_from{};
    T m_to{};
    double m_start = 0;
    float m_duration = 0;
    Curve m_curve = Curve::EaseOut;
};

}  // namespace mayo
