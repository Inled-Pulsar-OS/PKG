#pragma once

#include "widget.hpp"

namespace mayo {

enum class ControlSize : std::uint8_t { Small, Regular, Large };

/// Base for interactive controls: keyboard focusable, and animated hover/press/focus amounts
/// (0..1) that subclasses blend their colours with.
class ControlView : public Widget {
public:
    [[nodiscard]] bool acceptsFocus() const override { return true; }
    [[nodiscard]] bool isInteractive() const override { return true; }
    void setControlSize(ControlSize size);
    [[nodiscard]] ControlSize controlSize() const { return m_controlSize; }

    /// Draws the keyboard focus ring around focusRingShape().
    void drawOverlay(Painter& painter) override;

protected:
    /// Shape the focus ring hugs; defaults to a capsule around the frame.
    virtual void focusRingShape(Rect& rect, CornerRadii& radii) const;
    void onHoverChanged(bool hovered) override;
    void onPressedChanged(bool pressed) override;
    void onFocusChanged(bool focused) override;
    [[nodiscard]] float hoverAmount() { return sample(m_hover); }
    [[nodiscard]] float pressAmount() { return sample(m_press); }
    /// Focus ring visibility (only for keyboard focus, like macOS).
    [[nodiscard]] float focusAmount();
    /// Standard control height for the current control size.
    [[nodiscard]] float controlHeight() const;

    Animated<float> m_hover{0.0f};
    Animated<float> m_press{0.0f};
    Animated<float> m_focus{0.0f};
    ControlSize m_controlSize = ControlSize::Regular;
};

}  // namespace mayo
