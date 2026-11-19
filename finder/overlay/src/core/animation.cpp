#include "mayonnaise/animation.hpp"

namespace mayo {

namespace {

// Closed-form damped spring step response, normalised so the motion settles by t = 1.
float spring(float t, float damping) {
    const float omega = 4.6f / damping;  // settles to ~1% by t = 1
    if (damping >= 1.0f) {
        float e = std::exp(-omega * t);
        return 1.0f - e * (1.0f + omega * t);
    }
    const float wd = omega * std::sqrt(1.0f - damping * damping);
    const float e = std::exp(-damping * omega * t);
    return 1.0f - e * (std::cos(wd * t) + (damping * omega / wd) * std::sin(wd * t));
}

}  // namespace

float applyCurve(Curve curve, float t) {
    t = std::clamp(t, 0.0f, 1.0f);
    switch (curve) {
        case Curve::Linear: return t;
        case Curve::EaseOut: {
            float u = 1.0f - t;
            return 1.0f - u * u * u;
        }
        case Curve::EaseInOut: return t < 0.5f ? 4.0f * t * t * t : 1.0f - std::pow(-2.0f * t + 2.0f, 3.0f) * 0.5f;
        case Curve::Spring: return t >= 1.0f ? 1.0f : spring(t, 0.82f);
        case Curve::Bouncy: return t >= 1.0f ? 1.0f : spring(t, 0.6f);
    }
    return t;
}

}  // namespace mayo
