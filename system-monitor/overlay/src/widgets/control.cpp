#include "mayonnaise/control.hpp"

#include "mayonnaise/app.hpp"
#include "mayonnaise/painter.hpp"

namespace mayo {

void ControlView::setControlSize(ControlSize size) {
    m_controlSize = size;
    invalidateLayout();
}

void ControlView::onHoverChanged(bool hovered) {
    m_hover.animateTo(hovered ? 1.0f : 0.0f, now(), Transition::fast());
}

void ControlView::onPressedChanged(bool pressed) {
    // Press lands instantly-ish; release eases out so quick clicks are still visible.
    m_press.animateTo(pressed ? 1.0f : 0.0f, now(), pressed ? Transition{0.05f, Curve::EaseOut} : Transition::standard());
}

void ControlView::onFocusChanged(bool /*focused*/) { invalidate(); }

float ControlView::focusAmount() {
    float target = isFocusVisible() ? 1.0f : 0.0f;
    if (target != m_focus.target()) m_focus.animateTo(target, now(), Transition{0.2f, Curve::EaseOut});
    return sample(m_focus);
}

float ControlView::controlHeight() const {
    const auto& m = theme().metrics;
    switch (m_controlSize) {
        case ControlSize::Small: return m.controlHeight - 4;
        case ControlSize::Regular: return m.controlHeight;
        case ControlSize::Large: return m.largeControlHeight;
    }
    return m.controlHeight;
}

void ControlView::focusRingShape(Rect& rect, CornerRadii& radii) const {
    rect = frame();
    radii = CornerRadii(rect.height * 0.5f);
}

void ControlView::drawOverlay(Painter& painter) {
    float f = focusAmount();
    if (f <= 0.01f) return;
    Rect r;
    CornerRadii radii;
    focusRingShape(r, radii);
    painter.drawFocusRing(r, radii, f);
}

}  // namespace mayo
