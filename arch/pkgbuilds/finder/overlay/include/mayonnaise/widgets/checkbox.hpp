#pragma once

#include "toggle.hpp"

namespace mayo {

class CheckboxView : public BooleanControlView {
public:
    using BooleanControlView::BooleanControlView;
    Size measureContent(Size available) override;
    void draw(Painter& painter) override;

protected:
    void focusRingShape(Rect& rect, CornerRadii& radii) const override;
    Transition transition() const override { return Transition{0.18f, Curve::EaseOut}; }

private:
    [[nodiscard]] Rect boxRect() const;
};

class Checkbox : public WidgetHandle<Checkbox, CheckboxView> {
public:
    explicit Checkbox(std::string label = {}, bool checked = false)
        : WidgetHandle(std::make_shared<CheckboxView>(std::move(label), checked)) {}
    Checkbox& isOn(bool on) { return apply([&](CheckboxView& n) { n.setOn(on, false); }); }
    Checkbox& onChange(std::function<void(bool)> fn) {
        return apply([&](CheckboxView& n) { n.setOnChange(std::move(fn)); });
    }
    void setOn(bool on) { m_node->setOn(on); }
    [[nodiscard]] bool on() const { return m_node->isOn(); }
};

}  // namespace mayo
