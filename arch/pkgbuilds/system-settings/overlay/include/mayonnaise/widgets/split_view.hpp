#pragma once

#include "../widget.hpp"

namespace mayo {

/// Sidebar + detail layout. The sidebar floats on a translucent glass panel that runs under the
/// traffic lights; the divider can be dragged and the sidebar collapsed (animated).
class SplitViewView : public Widget {
public:
    SplitViewView(std::shared_ptr<Widget> sidebar, std::shared_ptr<Widget> detail);

    void setSidebarWidth(float w);
    void setSidebarLimits(float minW, float maxW);
    void setSidebarVisible(bool visible, bool animated = true);
    [[nodiscard]] bool isSidebarVisible() const { return m_visible; }
    void setDetail(std::shared_ptr<Widget> detail);

    void arrangeContent(const Rect& content) override;
    Size measureContent(Size available) override;
    [[nodiscard]] bool expandsContent(Axis /*axis*/) const override { return true; }
    [[nodiscard]] bool extendsIntoTitleBar() const override { return true; }
    [[nodiscard]] float titleBarLeadingEdge() const override;
    void draw(Painter& painter) override;

    // Divider dragging (driven by the internal handle widget).
    void beginResize();
    void resizeBy(float dx);

private:
    [[nodiscard]] Rect sidebarPanel() const;

    std::shared_ptr<Widget> m_sidebar;
    std::shared_ptr<Widget> m_detail;
    std::shared_ptr<Widget> m_handle;
    float m_width = 0;  // 0 = theme default
    float m_minWidth = 180;
    float m_maxWidth = 360;
    float m_resizeStart = 0;
    bool m_visible = true;
    Animated<float> m_reveal{1.0f};
};

class SplitView : public WidgetHandle<SplitView, SplitViewView> {
public:
    SplitView(const Element& sidebar, const Element& detail)
        : WidgetHandle(std::make_shared<SplitViewView>(sidebar.widget(), detail.widget())) {}
    SplitView& sidebarWidth(float w) { return apply([&](SplitViewView& n) { n.setSidebarWidth(w); }); }
    SplitView& sidebarLimits(float minW, float maxW) {
        return apply([&](SplitViewView& n) { n.setSidebarLimits(minW, maxW); });
    }
    void setSidebarVisible(bool visible) { m_node->setSidebarVisible(visible); }
    void toggleSidebar() { m_node->setSidebarVisible(!m_node->isSidebarVisible()); }
    void setDetail(const Element& detail) { m_node->setDetail(detail.widget()); }
};

}  // namespace mayo
