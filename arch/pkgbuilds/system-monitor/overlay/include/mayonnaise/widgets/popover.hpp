#pragma once

#include "../popup.hpp"

namespace mayo {

class PopoverView : public PopupView {
public:
    explicit PopoverView(std::shared_ptr<Widget> content);
};

/// A floating glass panel with arbitrary content, anchored to another widget.
///     auto pop = mayo::Popover(mayo::VStack({...}));
///     button.onClick([=]() mutable { pop.show(button); });
class Popover : public WidgetHandle<Popover, PopoverView> {
public:
    explicit Popover(const Element& content) : WidgetHandle(std::make_shared<PopoverView>(content.widget())) {}
    void show(const Element& anchor, PopupEdge edge = PopupEdge::Below) {
        if (anchor.widget()) m_node->showFrom(*anchor.widget(), edge);
    }
    void dismiss() { m_node->dismiss(); }
    [[nodiscard]] bool isShown() const { return m_node->isShown(); }
    Popover& onDismiss(std::function<void()> fn) { return apply([&](PopoverView& n) { n.setOnDismiss(std::move(fn)); }); }
};

}  // namespace mayo
