#include "mayonnaise/widgets/slider.hpp"

#include <cmath>

#include "mayonnaise/app.hpp"
#include "mayonnaise/painter.hpp"

namespace mayo {

SliderView::SliderView(float value, float min, float max)
    : m_value(std::clamp(value, std::min(min, max), std::max(min, max))), m_min(min), m_max(max), m_visual(m_value) {}

void SliderView::setValue(float value, bool notify) {
    value = std::clamp(value, std::min(m_min, m_max), std::max(m_min, m_max));
    if (m_step > 0) value = m_min + std::round((value - m_min) / m_step) * m_step;
    if (value == m_value) return;
    m_value = value;
    m_visual.set(value);
    invalidate();
    if (notify && m_onChange) m_onChange(m_value);
}

void SliderView::setRange(float min, float max) {
    m_min = min;
    m_max = max;
    setValue(m_value);
}

void SliderView::setStep(float step) {
    m_step = step;
    setValue(m_value);
}

Size SliderView::measureContent(Size available) {
    const auto& m = theme().metrics;
    float w = std::isfinite(available.width) ? std::max(available.width, 80.0f) : 160.0f;
    return {w, std::max(m.sliderKnobHeight + 4, 20.0f)};
}

float SliderView::fraction() const {
    float range = m_max - m_min;
    return range != 0 ? (m_visual.value(now()) - m_min) / range : 0.0f;
}

Rect SliderView::trackRect() const {
    const auto& m = theme().metrics;
    Rect c = contentRect();
    return {c.x, c.center().y - m.sliderTrackHeight * 0.5f, c.width, m.sliderTrackHeight};
}

Rect SliderView::knobRect(float f) const {
    const auto& m = theme().metrics;
    Rect c = contentRect();
    float travel = c.width - m.sliderKnobWidth;
    return {c.x + travel * std::clamp(f, 0.0f, 1.0f), c.center().y - m.sliderKnobHeight * 0.5f, m.sliderKnobWidth,
            m.sliderKnobHeight};
}

void SliderView::focusRingShape(Rect& rect, CornerRadii& radii) const {
    rect = knobRect(fraction());
    radii = CornerRadii(rect.height * 0.5f);
}

void SliderView::draw(Painter& p) {
    const Theme& t = theme();
    const auto& c = t.colors;
    const float f = std::clamp(sample(m_visual) - m_min, 0.0f, m_max - m_min) / std::max(1e-6f, m_max - m_min);
    const float hover = hoverAmount();
    const float press = pressAmount();
    const Rect track = trackRect();
    const Rect knob = knobRect(f);

    p.fillRoundedRect(track, track.height * 0.5f, c.fill.mix(c.tertiaryLabel, hover * 0.25f));
    Rect filled{track.x, track.y, knob.center().x - track.x, track.height};
    p.fillRoundedRect(filled, track.height * 0.5f, c.accent);

    // Knob grows slightly while dragged.
    Rect k = knob.outset(press * 1.5f);
    p.drawShadow(k, k.height * 0.5f, Color::black(t.isDark() ? 0.4f : 0.20f), 3.0f, {0, 1});
    p.fillRoundedRect(k, k.height * 0.5f, c.knob, c.knobBorder, 0.5f);
}

void SliderView::setFromPosition(float x) {
    const auto& m = theme().metrics;
    Rect c = contentRect();
    float travel = std::max(1.0f, c.width - m.sliderKnobWidth);
    float f = (x - c.x - m.sliderKnobWidth * 0.5f) / travel;
    float before = m_value;
    setValue(m_min + std::clamp(f, 0.0f, 1.0f) * (m_max - m_min));
    if (m_value != before && m_onChange) m_onChange(m_value);
}

bool SliderView::onPointerDown(const PointerEvent& e) {
    if (e.button != MouseButton::Left) return false;
    m_dragging = true;
    Rect knob = knobRect(fraction());
    if (knob.outset(3).contains(e.position)) {
        m_grabOffset = knob.center().x - e.position.x;
    } else {
        // Jump to the click with a short glide.
        m_grabOffset = 0;
        float from = m_visual.value(now());
        setFromPosition(e.position.x);
        Animated<float> glide(from);
        glide.animateTo(m_value, now(), Transition{0.12f, Curve::EaseOut});
        m_visual = glide;
    }
    return true;
}

void SliderView::onPointerMove(const PointerEvent& e) {
    if (m_dragging) setFromPosition(e.position.x + m_grabOffset);
}

void SliderView::onPointerUp(const PointerEvent& /*e*/) { m_dragging = false; }

bool SliderView::onKeyDown(const KeyEvent& e) {
    float step = m_step > 0 ? m_step : (m_max - m_min) / 20.0f;
    if (e.key == Key::Left || e.key == Key::Down) {
        setValue(m_value - step, true);
        return true;
    }
    if (e.key == Key::Right || e.key == Key::Up) {
        setValue(m_value + step, true);
        return true;
    }
    return false;
}

}  // namespace mayo
