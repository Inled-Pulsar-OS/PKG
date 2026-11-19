#include "mayonnaise/scroll_physics.hpp"

#include <algorithm>
#include <cmath>

namespace mayo {

namespace {
constexpr double kGestureEnd = 0.08;     // no end events: this long without touchpad events = fingers lifted
constexpr double kEndFallback = 1.0;     // end events promised but none came (pointer left mid-gesture)
constexpr double kFlingWindow = 0.08;    // momentum velocity is measured over the gesture's last 80 ms
constexpr float kMinFling = 120.0f;      // slower than this (points/second) and the content just stops
constexpr float kMaxFling = 9000.0f;
constexpr float kDecay = 2.0f;           // momentum decay per second (macOS: 0.998 per ms)
constexpr float kSpringK = 170.0f;       // rubber-band spring stiffness (critically damped)
constexpr float kMaxBounce = 2600.0f;    // velocity cap when momentum hits an edge, so bounces stay modest
constexpr float kRubber = 0.55f;         // macOS rubber-band coefficient
constexpr double kStepMax = 1.0 / 240.0;  // physics substep

float& at(Vec2& v, int i) { return i == 0 ? v.x : v.y; }
float at(const Vec2& v, int i) { return i == 0 ? v.x : v.y; }
float extent(Size s, int i) { return i == 0 ? s.width : s.height; }

/// Distance past [0, max] (signed: negative before the start).
float pastEdge(float o, float max) { return o < 0 ? o : (o > max ? o - max : 0.0f); }
}  // namespace

void ScrollPhysics::setLimits(Vec2 maxOffset, Size viewport) {
    m_max = {std::max(0.0f, maxOffset.x), std::max(0.0f, maxOffset.y)};
    m_viewport = viewport;
    if (m_phase == Phase::Idle) m_offset = {std::clamp(m_offset.x, 0.0f, m_max.x), std::clamp(m_offset.y, 0.0f, m_max.y)};
}

void ScrollPhysics::setAxes(bool horizontal, bool vertical) {
    m_axes[0] = horizontal;
    m_axes[1] = vertical;
}

void ScrollPhysics::setBouncesWhenFitting(bool horizontal, bool vertical) {
    m_bounceFitting[0] = horizontal;
    m_bounceFitting[1] = vertical;
}

Vec2 ScrollPhysics::overscroll() const { return {pastEdge(m_offset.x, m_max.x), pastEdge(m_offset.y, m_max.y)}; }

bool ScrollPhysics::canMove(int i, float d, bool precise) const {
    if (!scrolls(i) || d == 0) return false;
    const float o = at(m_offset, i);
    const float m = at(m_max, i);
    if ((d < 0 && o > 0) || (d > 0 && o < m)) return true;
    // At the edge a touchpad stretches the content instead of passing the gesture on.
    return precise && (m > 0 || m_bounceFitting[i]);
}

bool ScrollPhysics::handle(const ScrollEvent& e, double now) {
    if (e.phase == ScrollPhase::Ended) {
        if (m_phase != Phase::Tracking) return false;
        startMomentum(now);
        return true;
    }
    Vec2 delta = e.delta;
    if (!scrolls(0)) delta.x = 0;
    if (!scrolls(1)) delta.y = 0;
    if (!canMove(0, delta.x, e.precise) && !canMove(1, delta.y, e.precise)) return false;

    if (!e.precise) {
        // Wheel notches glide to a target inside the range; quick notches pile up on the running glide.
        Vec2 base = m_phase == Phase::Gliding ? m_glide.target() : m_offset;
        Vec2 target{std::clamp(base.x + delta.x, 0.0f, m_max.x), std::clamp(base.y + delta.y, 0.0f, m_max.y)};
        m_glide.set(m_offset);
        m_glide.animateTo(target, now, Transition{0.2f, Curve::EaseOut});
        m_phase = Phase::Gliding;
        m_velocity = {};
        m_lastStep = now;
        return true;
    }

    if (m_phase != Phase::Tracking) {
        // Fingers touching down stop momentum and bounces where they are.
        m_velocity = {};
        m_samples.clear();
    }
    m_explicitEnd = e.reportsEnd;
    for (int i = 0; i < 2; ++i) {
        float d = at(delta, i);
        if (d == 0) continue;
        float o = at(m_offset, i);
        const float m = at(m_max, i);
        const bool bounces = m > 0 || m_bounceFitting[i];
        // Inside the range the content follows the fingers 1:1 ...
        if (d > 0 && o < m) {
            const float take = std::min(d, m - o);
            o += take;
            d -= take;
        } else if (d < 0 && o > 0) {
            const float take = std::max(d, -o);
            o += take;
            d -= take;
        }
        // ... past it, it stretches like a rubber band: x * c / (1 + x * c / dim) overall, applied
        // incrementally from the current stretch so reversing direction is symmetric.
        if (d != 0 && bounces) {
            const float dim = std::max(1.0f, extent(m_viewport, i));
            const float over = std::abs(pastEdge(o, m));
            const float k = std::max(0.0f, 1.0f - over / dim);
            o += d * kRubber * k * k;
        }
        at(m_offset, i) = o;
    }
    m_samples.emplace_back(now, delta);
    while (!m_samples.empty() && now - m_samples.front().first > kFlingWindow * 2) m_samples.pop_front();
    m_phase = Phase::Tracking;
    m_lastEvent = now;
    m_lastStep = now;
    return true;
}

void ScrollPhysics::startMomentum(double endTime) {
    m_phase = Phase::Momentum;
    m_lastStep = endTime;
    // Fling velocity from the movement just before the fingers lifted. A finger that slowed down and rested
    // (rather than flicked) has moved little in that window, so the content stops instead of drifting.
    Vec2 moved;
    double first = m_lastEvent;
    for (const auto& [ts, d] : m_samples) {
        if (endTime - ts > kFlingWindow) continue;
        moved = moved + d;
        first = std::min(first, ts);
    }
    m_samples.clear();
    const auto span = static_cast<float>(std::max(0.02, m_lastEvent - first + 0.008));
    m_velocity = moved * (1.0f / span);
    if (m_velocity.length() < kMinFling) m_velocity = {};
    if (const float len = m_velocity.length(); len > kMaxFling) m_velocity = m_velocity * (kMaxFling / len);
    // Lifting while stretched springs back; it never flings further out.
    for (int i = 0; i < 2; ++i) {
        const float past = pastEdge(at(m_offset, i), at(m_max, i));
        if (past * at(m_velocity, i) > 0) at(m_velocity, i) = 0;
    }
}

bool ScrollPhysics::step(double now) {
    if (m_phase == Phase::Idle) return false;
    if (m_phase == Phase::Gliding) {
        m_offset = m_glide.value(now);
        if (!m_glide.isAnimating(now)) m_phase = Phase::Idle;
        m_lastStep = now;
        return m_phase != Phase::Idle;
    }
    if (m_phase == Phase::Tracking) {
        if (now - m_lastEvent < (m_explicitEnd ? kEndFallback : kGestureEnd)) return false;
        startMomentum(m_lastEvent);
        m_lastStep = now;
    }

    double remaining = std::clamp(now - m_lastStep, 0.0, 0.05);
    m_lastStep = now;
    bool active = false;
    for (int i = 0; i < 2; ++i) {
        float o = at(m_offset, i);
        float v = at(m_velocity, i);
        const float m = at(m_max, i);
        for (double left = remaining; left > 1e-6; left -= kStepMax) {
            const auto dt = static_cast<float>(std::min(left, kStepMax));
            const float x = pastEdge(o, m);
            if (x != 0) {
                v += (-kSpringK * x - 2.0f * std::sqrt(kSpringK) * v) * dt;
            } else {
                v *= std::exp(-kDecay * dt);
            }
            o += v * dt;
            if (x == 0 && pastEdge(o, m) != 0) v = std::clamp(v, -kMaxBounce, kMaxBounce);
        }
        if (std::abs(v) < 6 && std::abs(pastEdge(o, m)) < 0.5f) {
            o = std::clamp(o, 0.0f, m);
            v = 0;
        } else {
            active = true;
        }
        at(m_offset, i) = o;
        at(m_velocity, i) = v;
    }
    if (!active) m_phase = Phase::Idle;
    return active;
}

double ScrollPhysics::deadline() const {
    if (m_phase != Phase::Tracking) return -1;
    return m_lastEvent + (m_explicitEnd ? kEndFallback : kGestureEnd) + 0.005;
}

void ScrollPhysics::scrollTo(Vec2 offset, bool animated, double now) {
    Vec2 target{std::clamp(offset.x, 0.0f, m_max.x), std::clamp(offset.y, 0.0f, m_max.y)};
    if (!animated) {
        setOffset(target);
        return;
    }
    m_glide.set(m_offset);
    m_glide.animateTo(target, now, Transition{0.3f, Curve::EaseInOut});
    m_phase = Phase::Gliding;
    m_velocity = {};
    m_lastStep = now;
}

void ScrollPhysics::setOffset(Vec2 offset) {
    m_offset = {std::clamp(offset.x, 0.0f, m_max.x), std::clamp(offset.y, 0.0f, m_max.y)};
    m_velocity = {};
    m_samples.clear();
    m_phase = Phase::Idle;
}

void ScrollPhysics::shift(Vec2 by) {
    m_offset = m_offset + by;
    if (m_phase == Phase::Gliding) {
        // Keep it simple: stop the glide at the shifted position rather than retargeting it.
        m_phase = Phase::Idle;
        m_offset = {std::clamp(m_offset.x, 0.0f, m_max.x), std::clamp(m_offset.y, 0.0f, m_max.y)};
    }
}

}  // namespace mayo
