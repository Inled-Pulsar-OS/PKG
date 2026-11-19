#pragma once

#include <cstdint>
#include <deque>

#include "animation.hpp"
#include "events.hpp"
#include "geometry.hpp"

namespace mayo {

/// The scrolling feel of ScrollView, for custom widgets that scroll their own content (a terminal's
/// history, a sidebar list): wheel notches glide, touchpads track 1:1 with a rubber band past the edges
/// and coast with momentum after the fingers lift.
///
/// Offsets run from 0 (start) to `maxOffset` on each axis. Feed events to `handle()`, call `step()` every
/// frame while `isAnimating()`, and lay the content out at `offset()` (which may be briefly outside the
/// range while it bounces).
class ScrollPhysics {
public:
    /// Scrollable range and the visible size (the rubber band stretches relative to it).
    void setLimits(Vec2 maxOffset, Size viewport);
    void setAxes(bool horizontal, bool vertical);
    /// Lets touchpad gestures stretch an axis whose content fits (macOS bounces a lone scroll view).
    void setBouncesWhenFitting(bool horizontal, bool vertical);

    /// Applies a scroll event at time `now`. False if it can't move in that direction, so a parent can
    /// take it (scroll chaining).
    bool handle(const ScrollEvent& e, double now);
    /// Advances the physics to `now`. True while still moving.
    bool step(double now);
    void scrollTo(Vec2 offset, bool animated, double now);
    /// Moves without animation and stops any motion (content changed underneath, keyboard navigation).
    void setOffset(Vec2 offset);
    /// Shifts the offset and anything in flight (content was inserted above the visible part).
    void shift(Vec2 by);

    [[nodiscard]] Vec2 offset() const { return m_offset; }
    [[nodiscard]] Vec2 maxOffset() const { return m_max; }
    [[nodiscard]] bool isAnimating() const { return m_phase != Phase::Idle; }
    /// How far past the nearest edge the content currently is (0 inside the range).
    [[nodiscard]] Vec2 overscroll() const;
    /// When the widget must step again even if no events arrive (gesture-end detection), or -1.
    [[nodiscard]] double deadline() const;

private:
    enum class Phase : std::uint8_t { Idle, Tracking, Momentum, Gliding };
    [[nodiscard]] bool scrolls(int axis) const { return m_axes[axis]; }
    [[nodiscard]] bool canMove(int axis, float delta, bool precise) const;
    void startMomentum(double endTime);

    Vec2 m_max;
    Size m_viewport;
    bool m_axes[2] = {false, true};
    bool m_bounceFitting[2] = {false, false};
    Vec2 m_offset;
    Vec2 m_velocity;  // points/second
    std::deque<std::pair<double, Vec2>> m_samples;  // recent touchpad movement, for the fling velocity
    Phase m_phase = Phase::Idle;
    bool m_explicitEnd = false;  // the platform reports when the fingers lift
    double m_lastEvent = 0;
    double m_lastStep = 0;
    Animated<Vec2> m_glide;
};

}  // namespace mayo
