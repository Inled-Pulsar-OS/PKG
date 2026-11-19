#pragma once

#include <functional>

#include "painter.hpp"
#include "widget.hpp"

namespace mayo {

class CanvasView : public Widget {
public:
    using DrawFn = std::function<void(Painter&, const Rect&)>;
    /// Receives the pointer position relative to the canvas' content rect (0,0 = top-left).
    using PointFn = std::function<void(Point)>;

    explicit CanvasView(DrawFn draw) : m_draw(std::move(draw)) {}
    void setDraw(DrawFn fn) {
        m_draw = std::move(fn);
        invalidate();
    }
    void setOnClick(PointFn fn) { m_onClick = std::move(fn); }
    void setOnDrag(PointFn fn) { m_onDrag = std::move(fn); }
    void setOnHover(PointFn fn) { m_onHover = std::move(fn); }

    Size measureContent(Size /*available*/) override { return {0, 0}; }
    [[nodiscard]] bool expandsContent(Axis /*axis*/) const override { return true; }
    void draw(Painter& p) override {
        if (m_draw) m_draw(p, contentRect());
    }
    [[nodiscard]] bool isInteractive() const override { return m_onClick || m_onDrag; }
    bool onPointerDown(const PointerEvent& e) override {
        if (!m_onClick && !m_onDrag) return false;
        if (m_onDrag) m_onDrag(local(e.position));
        return true;
    }
    void onPointerMove(const PointerEvent& e) override {
        if (isPressed() && m_onDrag) m_onDrag(local(e.position));
        if (m_onHover) m_onHover(local(e.position));
    }
    void onPointerUp(const PointerEvent& e) override {
        if (m_onClick && frame().contains(e.position)) m_onClick(local(e.position));
    }

private:
    [[nodiscard]] Point local(Point window) const { return window - contentRect().origin(); }
    DrawFn m_draw;
    PointFn m_onClick;
    PointFn m_onDrag;
    PointFn m_onHover;
};

/// Custom drawing without subclassing. Coordinates are window points; `bounds` is the content rect.
///     mayo::Canvas([](mayo::Painter& p, mayo::Rect bounds) { p.fillCircle(bounds.center(), 20, ...); })
///         .frame(120, 80);
/// Pointer callbacks get positions relative to the canvas. Call redraw() after changing captured state.
class Canvas : public WidgetHandle<Canvas, CanvasView> {
public:
    explicit Canvas(CanvasView::DrawFn draw) : WidgetHandle(std::make_shared<CanvasView>(std::move(draw))) {}
    Canvas& onClick(CanvasView::PointFn fn) { return apply([&](CanvasView& n) { n.setOnClick(std::move(fn)); }); }
    Canvas& onDrag(CanvasView::PointFn fn) { return apply([&](CanvasView& n) { n.setOnDrag(std::move(fn)); }); }
    Canvas& onHover(CanvasView::PointFn fn) { return apply([&](CanvasView& n) { n.setOnHover(std::move(fn)); }); }
    void redraw() { m_node->invalidate(); }
};

}  // namespace mayo
