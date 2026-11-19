#pragma once

#include <initializer_list>
#include <vector>

#include "widget.hpp"

namespace mayo {

/// Lays children out along one axis with spacing; expanding children share free space by flex.
class StackView : public Widget {
public:
    explicit StackView(Axis axis);

    void setSpacing(float spacing);
    void setAlignment(Alignment alignment);
    [[nodiscard]] Axis axis() const { return m_axis; }
    [[nodiscard]] float spacing() const { return m_spacing; }
    [[nodiscard]] Alignment alignment() const { return m_alignment; }

    Size measureContent(Size available) override;
    void arrangeContent(const Rect& content) override;
    [[nodiscard]] bool expandsContent(Axis axis) const override;
    void onChildAdded(Widget& child) override;

private:
    struct Solution {
        std::vector<Size> sizes;  // per child, hidden children get {0,0}
        float mainUsed = 0;
        float crossMax = 0;
        bool anyMainFlex = false;
        bool anyCrossFlex = false;
    };
    Solution solve(Size available);

    Axis m_axis;
    float m_spacing = 8;
    Alignment m_alignment = Alignment::TopLeading;
};

/// Overlays children; each is aligned within the stack's bounds (expanding children fill it).
class ZStackView : public Widget {
public:
    void setAlignment(Alignment alignment);
    void arrangeContent(const Rect& content) override;
    [[nodiscard]] bool expandsContent(Axis axis) const override;

private:
    Alignment m_alignment = Alignment::Center;
};

class SpacerView : public Widget {
public:
    explicit SpacerView(float minLength) : m_minLength(minLength) {}
    Size measureContent(Size available) override;
    [[nodiscard]] bool expandsContent(Axis axis) const override;

private:
    float m_minLength;
};

class DividerView : public Widget {
public:
    Size measureContent(Size available) override;
    [[nodiscard]] bool expandsContent(Axis axis) const override;
    void draw(Painter& painter) override;
    [[nodiscard]] bool isHorizontalLine() const;
};

namespace detail {
template <class Derived, class Node>
class ContainerHandle : public WidgetHandle<Derived, Node> {
public:
    Derived& add(const Element& child) {
        this->m_node->addChild(child.widget());
        return static_cast<Derived&>(*this);
    }
    void remove(const Element& child) { this->m_node->removeChild(child.widget().get()); }
    void clear() { this->m_node->clearChildren(); }

protected:
    using WidgetHandle<Derived, Node>::WidgetHandle;
    void addAll(const std::vector<Element>& children) {
        for (const auto& c : children) this->m_node->addChild(c.widget());
    }
};

template <class Derived>
class StackHandle : public ContainerHandle<Derived, StackView> {
public:
    Derived& spacing(float s) { return this->apply([&](StackView& n) { n.setSpacing(s); }); }
    Derived& align(Alignment a) { return this->apply([&](StackView& n) { n.setAlignment(a); }); }

protected:
    StackHandle(Axis axis, const std::vector<Element>& children)
        : ContainerHandle<Derived, StackView>(std::make_shared<StackView>(axis)) {
        this->addAll(children);
    }
};
}  // namespace detail

/// Vertical stack. Default alignment is top-leading; `.align(Alignment::Center)` centres both ways.
class VStack : public detail::StackHandle<VStack> {
public:
    VStack(std::initializer_list<Element> children = {}) : StackHandle(Axis::Vertical, children) {}  // NOLINT
    explicit VStack(const std::vector<Element>& children) : StackHandle(Axis::Vertical, children) {}
};

/// Horizontal stack. Children are vertically centred by default.
class HStack : public detail::StackHandle<HStack> {
public:
    HStack(std::initializer_list<Element> children = {}) : StackHandle(Axis::Horizontal, children) {  // NOLINT
        m_node->setAlignment(Alignment::Leading);
    }
    explicit HStack(const std::vector<Element>& children) : StackHandle(Axis::Horizontal, children) {
        m_node->setAlignment(Alignment::Leading);
    }
};

class ZStack : public detail::ContainerHandle<ZStack, ZStackView> {
public:
    ZStack(std::initializer_list<Element> children = {})  // NOLINT
        : ContainerHandle(std::make_shared<ZStackView>()) {
        addAll(children);
    }
    ZStack& align(Alignment a) { return apply([&](ZStackView& n) { n.setAlignment(a); }); }
};

/// Flexible space along the enclosing stack's axis.
class Spacer : public WidgetHandle<Spacer, SpacerView> {
public:
    explicit Spacer(float minLength = 0) : WidgetHandle(std::make_shared<SpacerView>(minLength)) {}
};

/// Hairline separator; horizontal in a VStack, vertical in an HStack.
class Divider : public WidgetHandle<Divider, DividerView> {
public:
    Divider() : WidgetHandle(std::make_shared<DividerView>()) {}
};

}  // namespace mayo
