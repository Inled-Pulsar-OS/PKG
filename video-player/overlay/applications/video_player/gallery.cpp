#include "gallery.hpp"

#include <algorithm>
#include <cmath>
#include <cstdio>

using namespace mayo;

namespace {

constexpr float kPad = 24;
constexpr float kGap = 18;
constexpr float kRowGap = 22;
constexpr float kMinCard = 210;
constexpr float kTextHeight = 42;
constexpr float kHeaderHeight = 40;
constexpr float kRadius = 10;

/// Cuts `text` with an ellipsis so it fits `width` (on UTF-8 boundaries).
std::string fit(Painter& p, const std::string& text, const Font& font, float width) {
    if (p.measureText(text, font) <= width) return text;
    std::vector<size_t> cuts;
    for (size_t i = 1; i < text.size(); ++i) {
        if ((static_cast<unsigned char>(text[i]) & 0xC0) != 0x80) cuts.push_back(i);
    }
    size_t lo = 0;
    size_t hi = cuts.size();
    while (lo < hi) {
        const size_t mid = (lo + hi + 1) / 2;
        if (p.measureText(text.substr(0, cuts[mid - 1]) + "…", font) <= width) lo = mid;
        else hi = mid - 1;
    }
    return lo == 0 ? "…" : text.substr(0, cuts[lo - 1]) + "…";
}

std::string sizeText(double bytes) {
    const char* units[] = {"B", "KB", "MB", "GB", "TB"};
    int u = 0;
    while (bytes >= 1000 && u < 4) {
        bytes /= 1000;
        ++u;
    }
    char buf[32];
    std::snprintf(buf, sizeof buf, u == 0 || bytes >= 100 ? "%.0f %s" : "%.1f %s", bytes, units[u]);
    return buf;
}

void drawCover(Painter& p, const Rect& r, const ImageData& img, float radius) {
    const float ia = static_cast<float>(img.width()) / static_cast<float>(std::max(1, img.height()));
    const float ra = r.width / r.height;
    Rect src{0, 0, 1, 1};
    if (ia > ra) {
        src.width = ra / ia;
        src.x = (1 - src.width) * 0.5f;
    } else {
        src.height = ia / ra;
        src.y = (1 - src.height) * 0.5f;
    }
    p.drawImage(r, img, radius, 1, src);
}

}  // namespace

std::string formatTime(double seconds) {
    if (!std::isfinite(seconds) || seconds < 0) seconds = 0;
    auto total = static_cast<long>(seconds);
    char buf[32];
    if (total >= 3600)
        std::snprintf(buf, sizeof buf, "%ld:%02ld:%02ld", total / 3600, (total / 60) % 60, total % 60);
    else
        std::snprintf(buf, sizeof buf, "%ld:%02ld", total / 60, total % 60);
    return buf;
}

// Grid -------------------------------------------------------------------------------------------------------

void VideoGridView::setSections(std::vector<GallerySection> sections) {
    VideoPtr selected = m_selected >= 0 && m_selected < static_cast<int>(m_cards.size()) ? m_cards[static_cast<size_t>(m_selected)].item : nullptr;
    m_sections = std::move(sections);
    m_layoutWidth = -1;
    m_cards.clear();
    m_headers.clear();
    for (const auto& s : m_sections) {
        for (const auto& item : s.items) m_cards.push_back({item, {}});
    }
    m_selected = -1;
    for (size_t i = 0; i < m_cards.size(); ++i) {
        if (m_cards[i].item == selected) m_selected = static_cast<int>(i);
    }
    m_hover = -1;
    invalidateLayout();
}

float VideoGridView::layout(float width) {
    if (width == m_layoutWidth) return m_height;
    m_layoutWidth = width;
    const float avail = std::max(kMinCard, width - kPad * 2);
    const int cols = std::max(1, static_cast<int>((avail + kGap) / (kMinCard + kGap)));
    const float cardW = (avail - kGap * static_cast<float>(cols - 1)) / static_cast<float>(cols);
    m_thumbHeight = std::round(cardW * 9.0f / 16.0f);
    const float cardH = m_thumbHeight + kTextHeight;
    m_headers.clear();
    float y = 8;
    size_t index = 0;
    for (const auto& s : m_sections) {
        if (s.items.empty()) continue;
        if (!s.title.empty()) {
            m_headers.push_back({s.title, y});
            y += kHeaderHeight;
        }
        for (size_t i = 0; i < s.items.size(); ++i, ++index) {
            const int col = static_cast<int>(i) % cols;
            const int row = static_cast<int>(i) / cols;
            m_cards[index].rect = {kPad + static_cast<float>(col) * (cardW + kGap), y + static_cast<float>(row) * (cardH + kRowGap),
                                   cardW, cardH};
        }
        const auto rows = static_cast<float>((s.items.size() + static_cast<size_t>(cols) - 1) / static_cast<size_t>(cols));
        y += rows * cardH + (rows - 1) * kRowGap + 30;
    }
    m_height = y + 10;
    return m_height;
}

Size VideoGridView::measureContent(Size available) {
    const float w = std::isfinite(available.width) ? available.width : 900.0f;
    return {w, layout(w)};
}

void VideoGridView::arrangeContent(const Rect& content) { layout(content.width); }

Rect VideoGridView::cardRect(int i) const {
    const Rect r = m_cards[static_cast<size_t>(i)].rect;
    const Rect c = contentRect();
    return {c.x + r.x, c.y + r.y, r.width, r.height};
}

int VideoGridView::cardAt(Point p) const {
    for (int i = 0; i < static_cast<int>(m_cards.size()); ++i) {
        Rect r = cardRect(i);
        r.height = m_thumbHeight + kTextHeight;
        if (r.contains(p)) return i;
    }
    return -1;
}

void VideoGridView::draw(Painter& p) {
    const Theme& t = theme();
    const Rect c = contentRect();
    const Rect clip = p.clipBounds();
    for (const auto& h : m_headers) {
        p.drawText(h.title, {c.x + kPad + 2, c.y + h.y, c.width - kPad * 2, kHeaderHeight - 10},
                   t.font(TextStyle::Title2).withWeight(FontWeight::Bold), t.colors.label);
    }
    for (int i = 0; i < static_cast<int>(m_cards.size()); ++i) {
        const Rect r = cardRect(i);
        if (r.bottom() + 30 < clip.y || r.y - 30 > clip.bottom()) continue;  // off screen
        drawCard(p, i);
    }
}

void VideoGridView::drawCard(Painter& p, int i) {
    const Theme& t = theme();
    const auto& col = t.colors;
    const VideoItem& v = *m_cards[static_cast<size_t>(i)].item;
    const Rect card = cardRect(i);
    const Rect thumb{card.x, card.y, card.width, m_thumbHeight};
    const bool hovered = i == m_hover;

    p.drawShadow(thumb, kRadius, Color::black(hovered ? 0.45f : 0.22f), hovered ? 18 : 8, {0, hovered ? 6.0f : 2.0f});
    if (v.thumb) {
        drawCover(p, thumb, *v.thumb, kRadius);
    } else {
        p.fillVerticalGradient(thumb, kRadius, col.secondaryFill.mix(col.indigo, 0.12f), col.fill);
        const float s = std::min(44.0f, m_thumbHeight * 0.32f);
        p.drawIcon(v.audioOnly ? Icon::MusicNote : Icon::Film, {thumb.center().x - s * 0.5f, thumb.center().y - s * 0.5f, s, s},
                   col.tertiaryLabel, 0.8f);
    }
    p.strokeRoundedRect(thumb, kRadius, Color::white(t.isDark() ? 0.08f : 0.25f), 1);

    const double prog = progress ? progress(m_cards[static_cast<size_t>(i)].item) : -1;
    // Duration badge.
    if (v.duration > 0) {
        const std::string d = formatTime(v.duration);
        const Font f = t.font(TextStyle::Caption).withWeight(FontWeight::Semibold);
        const float w = p.measureText(d, f) + 12;
        const Rect badge{thumb.right() - w - 8, thumb.bottom() - (prog > 0 ? 34.0f : 26.0f), w, 18};
        p.fillRoundedRect(badge, 6, Color::black(0.62f));
        p.drawText(d, badge, f, Color::white(), TextAlign::Center);
    }
    // Where it was left.
    if (prog > 0) {
        const Rect bar{thumb.x + 10, thumb.bottom() - 10, thumb.width - 20, 4};
        p.fillRoundedRect(bar, 2, Color::white(0.35f));
        p.fillRoundedRect({bar.x, bar.y, std::max(4.0f, bar.width * static_cast<float>(std::min(1.0, prog))), bar.height}, 2, col.accent);
    }
    if (hovered) {
        p.fillRoundedRect(thumb, kRadius, Color::black(0.18f));
        const Point ctr = thumb.center();
        p.drawShadow({ctr.x - 26, ctr.y - 26, 52, 52}, 26, Color::black(0.35f), 12, {0, 3});
        p.fillCircle(ctr, 26, Color::white(0.92f));
        p.drawIcon(Icon::Play, {ctr.x - 11, ctr.y - 12, 24, 24}, Color::black(0.85f));
    }
    if (i == m_selected && isFocused()) p.strokeRoundedRect(thumb.outset(3.5f), kRadius + 3, col.accent, 3);

    const Font title = t.font(TextStyle::Callout).withWeight(FontWeight::Semibold);
    p.drawText(fit(p, v.title, title, card.width - 4), {card.x + 2, thumb.bottom() + 7, card.width - 4, 17}, title, col.label);
    std::string sub;
    if (prog > 0 && v.duration > 0) sub = formatTime(v.duration * (1 - prog)) + " left";
    else if (v.failed) sub = "Can't be played";
    else sub = v.folder.empty() ? sizeText(static_cast<double>(v.size)) : v.folder + " · " + sizeText(static_cast<double>(v.size));
    const Font small = t.font(TextStyle::Caption);
    p.drawText(fit(p, sub, small, card.width - 4), {card.x + 2, thumb.bottom() + 25, card.width - 4, 14}, small,
               v.failed ? col.red : col.secondaryLabel);
}

void VideoGridView::select(int i) {
    if (m_cards.empty()) return;
    m_selected = std::clamp(i, 0, static_cast<int>(m_cards.size()) - 1);
    ensureVisible(m_selected);
    invalidate();
}

void VideoGridView::ensureVisible(int i) {
    if (!m_scroller || i < 0) return;
    const float inset = host() ? host()->titleBarHeight() : 0.0f;
    const Rect view = m_scroller->frame();
    const Rect r = m_cards[static_cast<size_t>(i)].rect;
    const float top = r.y - kGap;
    const float bottom = r.bottom() + kGap;
    float offset = m_scroller->offset().y;
    const float visible = view.height - inset;
    if (top < offset) offset = top;
    if (bottom > offset + visible) offset = bottom - visible;
    if (offset != m_scroller->offset().y) m_scroller->scrollTo({0, offset}, true);
}

bool VideoGridView::onPointerDown(const PointerEvent& e) {
    const int i = cardAt(e.position);
    if (i < 0) return false;
    m_selected = i;
    invalidate();
    const VideoPtr item = m_cards[static_cast<size_t>(i)].item;
    if (e.button == MouseButton::Right) {
        if (onContextMenu) onContextMenu(item, e.position);
    } else if (e.button == MouseButton::Left && onOpen) {
        onOpen(item);
    }
    return true;
}

void VideoGridView::onPointerMove(const PointerEvent& e) {
    const int h = cardAt(e.position);
    if (h != m_hover) {
        m_hover = h;
        setCursor(h >= 0 ? std::optional(CursorShape::PointingHand) : std::nullopt);
        invalidate();
    }
}

void VideoGridView::onPointerLeave() {
    m_hover = -1;
    invalidate();
}

bool VideoGridView::onKeyDown(const KeyEvent& e) {
    if (m_cards.empty()) return false;
    const int cur = std::max(0, m_selected);
    // Columns from the card positions (sections may end with a short row).
    int cols = 1;
    while (cols < static_cast<int>(m_cards.size()) && m_cards[static_cast<size_t>(cols)].rect.y == m_cards[0].rect.y) ++cols;
    switch (e.key) {
        case Key::Left: select(cur - 1); return true;
        case Key::Right: select(m_selected < 0 ? 0 : cur + 1); return true;
        case Key::Up: select(cur - cols); return true;
        case Key::Down: select(m_selected < 0 ? 0 : cur + cols); return true;
        case Key::Home: select(0); return true;
        case Key::End: select(static_cast<int>(m_cards.size()) - 1); return true;
        case Key::Enter:
        case Key::Space:
            if (m_selected >= 0 && onOpen) onOpen(m_cards[static_cast<size_t>(m_selected)].item);
            return true;
        default: return false;
    }
}

// Page ---------------------------------------------------------------------------------------------------------

void LibraryPage::arrangeContent(const Rect& c) {
    const float bar = host() ? host()->titleBarHeight() : 0.0f;
    scroll->setContentInsetTop(bar);
    scroll->measure(c.size());
    scroll->arrange(c);
    const Rect below{c.x, c.y + bar, c.width, c.height - bar};
    empty->measure(below.size());
    empty->arrange(below);
}
