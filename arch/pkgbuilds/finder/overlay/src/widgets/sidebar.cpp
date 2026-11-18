#include "mayonnaise/widgets/sidebar.hpp"

#include <cmath>

#include "mayonnaise/app.hpp"
#include "mayonnaise/painter.hpp"

namespace mayo {

namespace {
constexpr float kRowInset = 10;
constexpr float kHeaderGap = 10;
}  // namespace

SidebarView::SidebarView(std::vector<SidebarItem> items) : m_items(std::move(items)) {
    m_physics.setBouncesWhenFitting(false, true);
    for (const auto& it : m_items) {
        if (!it.isHeader) {
            m_selection = it.id;
            break;
        }
    }
}

void SidebarView::setItems(std::vector<SidebarItem> items) {
    m_items = std::move(items);
    m_hovered = -1;
    invalidateLayout();
}

void SidebarView::setSelection(const std::string& id, bool notify) {
    if (id == m_selection) return;
    int oldIndex = -1;
    int newIndex = -1;
    for (int i = 0; i < static_cast<int>(m_items.size()); ++i) {
        if (m_items[static_cast<size_t>(i)].isHeader) continue;
        if (m_items[static_cast<size_t>(i)].id == m_selection) oldIndex = i;
        if (m_items[static_cast<size_t>(i)].id == id) newIndex = i;
    }
    if (newIndex < 0) return;
    m_selection = id;
    // Glide the selection pill from the old row to the new one (positions relative to the list).
    if (oldIndex >= 0 && host()) {
        Rect from = rowRect(oldIndex).translated({-frame().x, -listTop() + m_scroll});
        Rect to = rowRect(newIndex).translated({-frame().x, -listTop() + m_scroll});
        m_highlight.set(m_highlight.isAnimating(now()) ? m_highlight.value(now()) : from);
        m_highlight.animateTo(to, now(), Transition::spring(0.32f));
    } else {
        m_highlight.set({});
    }
    invalidate();
    if (notify && m_onSelect) m_onSelect(m_selection);
}

void SidebarView::setSelectionStyle(SidebarSelectionStyle s) {
    m_style = s;
    invalidate();
}

void SidebarView::setHeader(std::shared_ptr<Widget> header) {
    if (m_header) removeChild(m_header.get());
    m_header = std::move(header);
    if (m_header) addChild(m_header);
}

float SidebarView::rowHeight(const SidebarItem& item) const {
    // An untitled header is just a gap between groups (System Settings style).
    if (item.isHeader) return item.title.empty() ? 12.0f : 24.0f;
    return theme().metrics.rowHeight;
}

float SidebarView::listTop() const {
    // Leave room for the traffic lights, which sit on top of the sidebar.
    float top = frame().y;
    if (auto* h = host()) top = std::max(top, h->titleBarHeight());
    if (m_header) top = m_header->frame().bottom() + 8;
    return top;
}

float SidebarView::contentHeight() const {
    float h = 0;
    for (size_t i = 0; i < m_items.size(); ++i) {
        if (m_items[i].isHeader && i > 0) h += kHeaderGap;
        h += rowHeight(m_items[i]);
    }
    return h + 8;
}

Rect SidebarView::rowRect(int index) const {
    float y = listTop() - m_scroll;
    for (int i = 0; i < static_cast<int>(m_items.size()); ++i) {
        const auto& item = m_items[static_cast<size_t>(i)];
        if (item.isHeader && i > 0) y += kHeaderGap;
        float h = rowHeight(item);
        if (i == index) return {frame().x + kRowInset, y, frame().width - kRowInset * 2, h};
        y += h;
    }
    return {};
}

int SidebarView::rowAt(Point p) const {
    if (p.y < listTop()) return -1;
    for (int i = 0; i < static_cast<int>(m_items.size()); ++i) {
        if (m_items[static_cast<size_t>(i)].isHeader) continue;
        if (rowRect(i).contains(p)) return i;
    }
    return -1;
}

Size SidebarView::measureContent(Size available) {
    if (m_header) m_header->measure({std::max(0.0f, available.width - kRowInset * 2), available.height});
    return {available.width, std::isfinite(available.height) ? available.height : contentHeight()};
}

void SidebarView::arrangeContent(const Rect& content) {
    if (m_header) {
        float top = content.y;
        if (auto* h = host()) top = std::max(top, h->titleBarHeight());
        Size s = m_header->measure({content.width - kRowInset * 2, content.height});
        m_header->arrange({content.x + kRowInset, top, content.width - kRowInset * 2, s.height});
    }
    clampScroll();
}

void SidebarView::clampScroll() {
    const float visible = frame().bottom() - listTop();
    m_physics.setLimits({0, std::max(0.0f, contentHeight() - visible)}, {frame().width, visible});
    m_scroll = m_physics.offset().y;
}

void SidebarView::stepScroll() {
    if (!m_physics.isAnimating()) return;
    if (m_physics.step(now())) requestAnimationFrame();
    else if (auto* h = host(); h && m_physics.deadline() >= 0) h->scheduleFrame(m_physics.deadline());
    m_scroll = m_physics.offset().y;
}

void SidebarView::draw(Painter& p) {
    stepScroll();
    const Theme& t = theme();
    const auto& c = t.colors;
    // Like macOS source lists: accent selection while focused (or when asked for), gray otherwise.
    const bool accent = m_style == SidebarSelectionStyle::Accent || isFocused();
    const bool key = host() && host()->isKeyWindow();
    const Font font = t.font(TextStyle::Body);
    const Font headerFont = t.font(TextStyle::Subheadline).withWeight(FontWeight::Semibold);
    const float radius = t.metrics.rowRadius;

    paintChildren(p);
    p.pushClip({frame().x, listTop(), frame().width, frame().bottom() - listTop()});

    // Selection pill (animated between rows).
    int selected = -1;
    for (int i = 0; i < static_cast<int>(m_items.size()); ++i) {
        if (!m_items[static_cast<size_t>(i)].isHeader && m_items[static_cast<size_t>(i)].id == m_selection) selected = i;
    }
    Color selFill = accent && key ? c.accent : c.fill.mix(c.unemphasizedSelection, 0.3f);
    if (selected >= 0) {
        Rect target = rowRect(selected);
        Rect pill = target;
        if (m_highlight.isAnimating(now()))
            pill = sample(m_highlight).translated({frame().x, listTop() - m_scroll});
        p.fillRoundedRect(pill, radius, selFill);
    }

    for (int i = 0; i < static_cast<int>(m_items.size()); ++i) {
        const auto& item = m_items[static_cast<size_t>(i)];
        Rect row = rowRect(i);
        if (row.bottom() < listTop() || row.y > frame().bottom()) continue;
        if (item.isHeader) {
            p.drawText(item.title, {row.x + 8, row.y + 4, row.width - 8, row.height - 4}, headerFont, c.tertiaryLabel);
            continue;
        }
        const bool isSel = i == selected;
        if (!isSel && i == m_hovered) p.fillRoundedRect(row, radius, c.hover);
        const bool onAccent = isSel && accent && key;
        Color text = onAccent ? c.onAccent : c.label;
        float x = row.x + 8;
        if (item.dot) {
            Point center{x + 7.0f, row.center().y};
            p.fillCircle(center, 5.0f, *item.dot);
            p.strokeCircle(center, 5.0f, Color::black(0.15f), 0.75f);
            x += 24;
        } else if (item.icon != Icon::None) {
            if (item.tile) {
                Rect tile{x, row.center().y - 10, 20, 20};
                p.fillVerticalGradient(tile, 5.5f, item.tile->lighter(0.12f), *item.tile);
                p.strokeRoundedRect(tile, 5.5f, Color::black(0.06f), 0.5f);
                p.drawIcon(item.icon, tile.inset(3.5f), Color::white(), 1.05f);
            } else {
                Color ic = onAccent ? c.onAccent : c.accent;
                p.drawIcon(item.icon, {x + 1, row.center().y - 8, 16, 16}, ic);
            }
            x += 28;
        }
        float right = row.right() - 8;
        if (!item.badge.empty()) {
            Font bf = t.font(TextStyle::Subheadline);
            float bw = p.measureText(item.badge, bf);
            p.drawText(item.badge, {right - bw, row.y, bw, row.height}, bf,
                       onAccent ? c.onAccent.withAlpha(0.8f) : c.secondaryLabel);
            right -= bw + 6;
        }
        p.pushClip({x, row.y, std::max(0.0f, right - x), row.height});
        p.drawText(item.title, {x, row.y, right - x, row.height}, font, text);
        p.popClip();
    }
    p.popClip();
}

bool SidebarView::onPointerDown(const PointerEvent& e) {
    m_pressed = rowAt(e.position);
    if (m_pressed < 0) return false;
    // Selection happens on mouse down in macOS source lists.
    setSelection(m_items[static_cast<size_t>(m_pressed)].id, true);
    if (auto* h = host()) h->setFocus(this, false);
    return true;
}

void SidebarView::onPointerMove(const PointerEvent& e) {
    int r = rowAt(e.position);
    if (r != m_hovered) {
        m_hovered = r;
        invalidate();
    }
}

void SidebarView::onPointerUp(const PointerEvent& /*e*/) { m_pressed = -1; }

void SidebarView::onPointerLeave() {
    m_hovered = -1;
    invalidate();
}

bool SidebarView::onScroll(const ScrollEvent& e) {
    clampScroll();
    if (!m_physics.handle(e, now())) return false;
    m_scroll = m_physics.offset().y;
    if (m_physics.isAnimating()) requestAnimationFrame();
    if (auto* h = host(); h && m_physics.deadline() >= 0) h->scheduleFrame(m_physics.deadline());
    invalidate();
    return true;
}

void SidebarView::moveSelection(int delta) {
    int cur = -1;
    for (int i = 0; i < static_cast<int>(m_items.size()); ++i) {
        if (!m_items[static_cast<size_t>(i)].isHeader && m_items[static_cast<size_t>(i)].id == m_selection) cur = i;
    }
    for (int i = cur + delta; i >= 0 && i < static_cast<int>(m_items.size()); i += delta) {
        if (!m_items[static_cast<size_t>(i)].isHeader) {
            setSelection(m_items[static_cast<size_t>(i)].id, true);
            // Keep the selection visible.
            Rect r = rowRect(i);
            if (r.y < listTop()) m_scroll -= listTop() - r.y;
            if (r.bottom() > frame().bottom()) m_scroll += r.bottom() - frame().bottom();
            m_physics.setOffset({0, m_scroll});
            clampScroll();
            return;
        }
    }
}

bool SidebarView::onKeyDown(const KeyEvent& e) {
    if (e.key == Key::Up) {
        moveSelection(-1);
        return true;
    }
    if (e.key == Key::Down) {
        moveSelection(1);
        return true;
    }
    return false;
}

}  // namespace mayo
