#pragma once

#include <functional>

#include "../control.hpp"

namespace mayo {

class SliderView : public ControlView {
public:
    SliderView(float value, float min, float max);

    void setValue(float value, bool notify = false);
    [[nodiscard]] float value() const { return m_value; }
    void setRange(float min, float max);
    void setStep(float step);
    void setOnChange(std::function<void(float)> fn) { m_onChange = std::move(fn); }

    Size measureContent(Size available) override;
    [[nodiscard]] bool expandsContent(Axis axis) const override { return axis == Axis::Horizontal; }
    void draw(Painter& painter) override;
    bool onPointerDown(const PointerEvent& e) override;
    void onPointerMove(const PointerEvent& e) override;
    void onPointerUp(const PointerEvent& e) override;
    bool onKeyDown(const KeyEvent& e) override;

protected:
    void focusRingShape(Rect& rect, CornerRadii& radii) const override;

private:
    [[nodiscard]] Rect trackRect() const;
    [[nodiscard]] Rect knobRect(float fraction) const;
    [[nodiscard]] float fraction() const;
    void setFromPosition(float x);

    float m_value;
    float m_min;
    float m_max;
    float m_step = 0;
    float m_grabOffset = 0;
    bool m_dragging = false;
    Animated<float> m_visual;  // animates jumps when clicking the track
    std::function<void(float)> m_onChange;
};

class Slider : public WidgetHandle<Slider, SliderView> {
public:
    explicit Slider(float value = 0.5f, float min = 0.0f, float max = 1.0f)
        : WidgetHandle(std::make_shared<SliderView>(value, min, max)) {}
    Slider& range(float min, float max) { return apply([&](SliderView& n) { n.setRange(min, max); }); }
    Slider& step(float s) { return apply([&](SliderView& n) { n.setStep(s); }); }
    Slider& onChange(std::function<void(float)> fn) {
        return apply([&](SliderView& n) { n.setOnChange(std::move(fn)); });
    }
    void setValue(float v) { m_node->setValue(v); }
    [[nodiscard]] float value() const { return m_node->value(); }
};

}  // namespace mayo
