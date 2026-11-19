#include "mayonnaise/widgets/progress_bar.hpp"

#include <cmath>
#include <numbers>

#include "mayonnaise/app.hpp"
#include "mayonnaise/painter.hpp"

namespace mayo {

ProgressBarView::ProgressBarView(std::optional<float> value) : m_value(value), m_shown(value.value_or(0.0f)) {}

void ProgressBarView::setValue(std::optional<float> value) {
    if (value) value = std::clamp(*value, 0.0f, 1.0f);
    if (value == m_value) return;
    m_value = value;
    if (value) m_shown.animateTo(*value, now(), Transition{0.3f, Curve::EaseOut});
    invalidate();
}

void ProgressBarView::setStyle(ProgressStyle style) {
    m_style = style;
    invalidateLayout();
}

Size ProgressBarView::measureContent(Size available) {
    if (m_style == ProgressStyle::Spinner) return {18, 18};
    return {std::isfinite(available.width) ? available.width : 160.0f, 6};
}

void ProgressBarView::draw(Painter& p) {
    const Theme& t = theme();
    const auto& c = t.colors;
    const Rect r = contentRect();
    const double time = now();

    if (m_style == ProgressStyle::Spinner) {
        // Classic macOS spinner: 8 spokes, brightness rotating around.
        requestAnimationFrame();
        const Point center = r.center();
        const float radius = std::min(r.width, r.height) * 0.5f;
        const int spokes = 8;
        const auto head = static_cast<int>(std::floor(time * 10.0)) % spokes;
        for (int i = 0; i < spokes; ++i) {
            float a = static_cast<float>(i) * 2.0f * std::numbers::pi_v<float> / spokes - std::numbers::pi_v<float> / 2;
            int age = (head - i + spokes) % spokes;
            float alpha = 1.0f - static_cast<float>(age) / spokes * 0.8f;
            Point p0{center.x + std::cos(a) * radius * 0.45f, center.y + std::sin(a) * radius * 0.45f};
            Point p1{center.x + std::cos(a) * radius * 0.92f, center.y + std::sin(a) * radius * 0.92f};
            p.drawLine(p0, p1, c.secondaryLabel.multipliedAlpha(alpha), radius * 0.2f);
        }
        return;
    }

    const float h = r.height;
    const Rect track{r.x, r.center().y - h * 0.5f, r.width, h};
    p.fillRoundedRect(track, h * 0.5f, c.fill);
    if (m_value) {
        float v = std::clamp(sample(m_shown), 0.0f, 1.0f);
        if (v > 0) p.fillRoundedRect({track.x, track.y, std::max(h, track.width * v), h}, h * 0.5f, c.accent);
    } else {
        // Indeterminate: an accent segment sweeping across, eased at both ends.
        requestAnimationFrame();
        double cycle = std::fmod(time, 1.4) / 1.4;
        float e = applyCurve(Curve::EaseInOut, static_cast<float>(cycle));
        float segW = track.width * 0.32f;
        float x = track.x - segW + (track.width + segW) * e;
        p.pushClip(track, h * 0.5f);
        p.fillRoundedRect({x, track.y, segW, h}, h * 0.5f, c.accent);
        p.popClip();
    }
}

}  // namespace mayo
