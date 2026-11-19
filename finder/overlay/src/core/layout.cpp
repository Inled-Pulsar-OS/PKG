#include "mayonnaise/layout.hpp"

#include <cmath>

#include "mayonnaise/app.hpp"
#include "mayonnaise/painter.hpp"

namespace mayo {

namespace {

float mainOf(Size s, Axis a) { return a == Axis::Horizontal ? s.width : s.height; }
float crossOf(Size s, Axis a) { return a == Axis::Horizontal ? s.height : s.width; }
Size make(Axis a, float main, float cross) { return a == Axis::Horizontal ? Size{main, cross} : Size{cross, main}; }
Axis other(Axis a) { return a == Axis::Horizontal ? Axis::Vertical : Axis::Horizontal; }

float alignOffset(float free, float factor) { return std::max(0.0f, free) * factor; }
float factorOf(HAlign h) { return h == HAlign::Leading ? 0.0f : h == HAlign::Center ? 0.5f : 1.0f; }
float factorOf(VAlign v) { return v == VAlign::Top ? 0.0f : v == VAlign::Center ? 0.5f : 1.0f; }

}  // namespace

// --- StackView ------------------------------------------------------------------------------

StackView::StackView(Axis axis) : m_axis(axis) {}

void StackView::setSpacing(float spacing) {
    m_spacing = spacing;
    invalidateLayout();
}

void StackView::setAlignment(Alignment alignment) {
    m_alignment = alignment;
    invalidateLayout();
}

void StackView::onChildAdded(Widget& child) { child.setParentAxis(m_axis); }

bool StackView::expandsContent(Axis axis) const {
    for (const auto& c : children()) {
        if (c->expands(axis)) return true;
    }
    return false;
}

StackView::Solution StackView::solve(Size available) {
    Solution sol;
    const auto& kids = children();
    sol.sizes.assign(kids.size(), Size{});
    const float main = mainOf(available, m_axis);
    const float cross = crossOf(available, m_axis);
    const bool finite = std::isfinite(main);

    int visible = 0;
    float totalFlex = 0;
    for (const auto& c : kids) {
        if (!c->isVisible()) continue;
        ++visible;
        if (c->expands(m_axis)) {
            sol.anyMainFlex = true;
            totalFlex += c->flexFactor();
        }
        if (c->expands(other(m_axis))) sol.anyCrossFlex = true;
    }
    const float spacingTotal = m_spacing * static_cast<float>(std::max(0, visible - 1));
    float remaining = finite ? std::max(0.0f, main - spacingTotal) : kInfinity;

    // Rigid children first: they get what they ask for, expanding children share the rest.
    for (size_t i = 0; i < kids.size(); ++i) {
        auto& c = kids[i];
        if (!c->isVisible() || c->expands(m_axis)) continue;
        Size s = c->measure(make(m_axis, remaining, cross));
        sol.sizes[i] = s;
        if (finite) remaining = std::max(0.0f, remaining - mainOf(s, m_axis));
    }
    for (size_t i = 0; i < kids.size(); ++i) {
        auto& c = kids[i];
        if (!c->isVisible() || !c->expands(m_axis)) continue;
        float share = finite ? (totalFlex > 0 ? remaining * c->flexFactor() / totalFlex : 0.0f) : kInfinity;
        Size s = c->measure(make(m_axis, share, cross));
        float m = finite ? std::max(share, mainOf(s, m_axis)) : mainOf(s, m_axis);
        sol.sizes[i] = make(m_axis, m, crossOf(s, m_axis));
    }

    for (size_t i = 0; i < kids.size(); ++i) {
        if (!kids[i]->isVisible()) continue;
        sol.mainUsed += mainOf(sol.sizes[i], m_axis);
        sol.crossMax = std::max(sol.crossMax, crossOf(sol.sizes[i], m_axis));
    }
    sol.mainUsed += spacingTotal;
    return sol;
}

Size StackView::measureContent(Size available) {
    Solution sol = solve(available);
    const float main = mainOf(available, m_axis);
    const float cross = crossOf(available, m_axis);
    float m = sol.anyMainFlex && std::isfinite(main) ? std::max(main, sol.mainUsed) : sol.mainUsed;
    float c = sol.anyCrossFlex && std::isfinite(cross) ? std::max(cross, sol.crossMax) : sol.crossMax;
    return make(m_axis, m, c);
}

void StackView::arrangeContent(const Rect& content) {
    Solution sol = solve(content.size());
    const auto& kids = children();
    const bool horizontal = m_axis == Axis::Horizontal;
    const float mainSize = mainOf(content.size(), m_axis);
    const float crossSize = crossOf(content.size(), m_axis);
    const float mainFactor = horizontal ? factorOf(m_alignment.horizontal) : factorOf(m_alignment.vertical);
    const float crossFactor = horizontal ? factorOf(m_alignment.vertical) : factorOf(m_alignment.horizontal);

    float pos = sol.anyMainFlex ? 0.0f : alignOffset(mainSize - sol.mainUsed, mainFactor);
    bool first = true;
    for (size_t i = 0; i < kids.size(); ++i) {
        auto& c = kids[i];
        if (!c->isVisible()) {
            c->arrange({content.x, content.y, 0, 0});
            continue;
        }
        if (!first) pos += m_spacing;
        first = false;
        float m = mainOf(sol.sizes[i], m_axis);
        float cs = c->expands(other(m_axis)) ? crossSize : std::min(crossOf(sol.sizes[i], m_axis), crossSize);
        float co = alignOffset(crossSize - cs, crossFactor);
        if (horizontal)
            c->arrange({content.x + pos, content.y + co, m, cs});
        else
            c->arrange({content.x + co, content.y + pos, cs, m});
        pos += m;
    }
}

// --- ZStackView ------------------------------------------------------------------------------

void ZStackView::setAlignment(Alignment alignment) {
    m_alignment = alignment;
    invalidateLayout();
}

bool ZStackView::expandsContent(Axis axis) const {
    for (const auto& c : children()) {
        if (c->expands(axis)) return true;
    }
    return false;
}

void ZStackView::arrangeContent(const Rect& content) {
    for (auto& c : children()) {
        Size s = c->measure(content.size());
        float w = c->expands(Axis::Horizontal) ? content.width : std::min(s.width, content.width);
        float h = c->expands(Axis::Vertical) ? content.height : std::min(s.height, content.height);
        float x = content.x + alignOffset(content.width - w, factorOf(m_alignment.horizontal));
        float y = content.y + alignOffset(content.height - h, factorOf(m_alignment.vertical));
        c->arrange({x, y, w, h});
    }
}

// --- Spacer / Divider ----------------------------------------------------------------------------

Size SpacerView::measureContent(Size /*available*/) {
    if (!parentAxis()) return {m_minLength, m_minLength};
    return make(*parentAxis(), m_minLength, 0);
}

bool SpacerView::expandsContent(Axis axis) const { return !parentAxis() || axis == *parentAxis(); }

bool DividerView::isHorizontalLine() const { return parentAxis().value_or(Axis::Vertical) == Axis::Vertical; }

Size DividerView::measureContent(Size /*available*/) {
    return isHorizontalLine() ? Size{0, 1} : Size{1, 0};
}

bool DividerView::expandsContent(Axis axis) const {
    return isHorizontalLine() ? axis == Axis::Horizontal : axis == Axis::Vertical;
}

void DividerView::draw(Painter& p) {
    // One device pixel, centred in the 1pt slot: a true hairline on HiDPI screens.
    const float px = 1.0f / p.scale();
    const Rect f = frame();
    Color c = foreground().value_or(theme().colors.separator);
    if (isHorizontalLine())
        p.fillRect({f.x, p.snap(f.center().y - px * 0.5f), f.width, px}, c);
    else
        p.fillRect({p.snap(f.center().x - px * 0.5f), f.y, px, f.height}, c);
}

}  // namespace mayo
