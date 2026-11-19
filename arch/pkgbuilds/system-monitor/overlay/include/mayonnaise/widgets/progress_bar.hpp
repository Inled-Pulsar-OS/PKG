#pragma once

#include <optional>

#include "../widget.hpp"

namespace mayo {

enum class ProgressStyle : std::uint8_t { Bar, Spinner };

class ProgressBarView : public Widget {
public:
    explicit ProgressBarView(std::optional<float> value);
    /// nullopt = indeterminate.
    void setValue(std::optional<float> value);
    [[nodiscard]] std::optional<float> value() const { return m_value; }
    void setStyle(ProgressStyle style);

    Size measureContent(Size available) override;
    [[nodiscard]] bool expandsContent(Axis axis) const override {
        return m_style == ProgressStyle::Bar && axis == Axis::Horizontal;
    }
    void draw(Painter& painter) override;

private:
    std::optional<float> m_value;
    Animated<float> m_shown{0.0f};
    ProgressStyle m_style = ProgressStyle::Bar;
};

/// Determinate (`ProgressBar(0.4f)`) or indeterminate (`ProgressBar()`) progress.
class ProgressBar : public WidgetHandle<ProgressBar, ProgressBarView> {
public:
    ProgressBar() : WidgetHandle(std::make_shared<ProgressBarView>(std::nullopt)) {}
    explicit ProgressBar(float value) : WidgetHandle(std::make_shared<ProgressBarView>(value)) {}
    ProgressBar& style(ProgressStyle s) { return apply([&](ProgressBarView& n) { n.setStyle(s); }); }
    void setValue(std::optional<float> v) { m_node->setValue(v); }
};

/// Circular indeterminate spinner.
inline ProgressBar ProgressSpinner() { return ProgressBar().style(ProgressStyle::Spinner); }

}  // namespace mayo
