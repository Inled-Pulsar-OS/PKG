#include "mayonnaise/widgets/button.hpp"

#include <cmath>

#include "../render/font.hpp"
#include "mayonnaise/app.hpp"
#include "mayonnaise/painter.hpp"

namespace mayo {

namespace {

Font buttonFont(ControlSize size, ButtonStyle style) {
    const Theme& t = theme();
    Font f = size == ControlSize::Small ? t.font(TextStyle::Subheadline) : t.font(TextStyle::Body);
    if (size == ControlSize::Large) f = f.withSize(14);
    // White text on colour reads thinner; give filled buttons a touch more weight.
    if (style == ButtonStyle::Prominent || style == ButtonStyle::Destructive) f.weight = FontWeight::Medium;
    return f;
}

float iconSize(ControlSize size) { return size == ControlSize::Small ? 12.0f : size == ControlSize::Large ? 16.0f : 14.0f; }

}  // namespace

ButtonView::ButtonView(std::string title) : m_title(std::move(title)) {}

void ButtonView::setTitle(std::string title) {
    m_title = std::move(title);
    invalidateLayout();
}

void ButtonView::setIcon(Icon icon) {
    m_icon = icon;
    invalidateLayout();
}

void ButtonView::setStyle(ButtonStyle style) {
    m_style = style;
    invalidateLayout();
}

void ButtonView::click() {
    if (!isEnabled()) return;
    if (m_onClick) m_onClick();
}

Size ButtonView::measureContent(Size /*available*/) {
    const float h = controlHeight();
    const Font f = buttonFont(m_controlSize, m_style);
    float w = 0;
    if (m_icon != Icon::None) w += iconSize(m_controlSize);
    if (!m_title.empty()) {
        if (w > 0) w += 5;
        w += std::ceil(render::measureText(m_title, f));
    }
    if (m_title.empty()) return {h + (m_style == ButtonStyle::Borderless ? 4.0f : 0.0f), h};
    const float pad = theme().metrics.controlPaddingH + (m_controlSize == ControlSize::Large ? 4.0f : 0.0f);
    return {std::max(w + pad * 2, h * 2.2f), h};
}

void ButtonView::draw(Painter& p) {
    const Theme& t = theme();
    const auto& c = t.colors;
    const bool dark = t.isDark();
    const Rect r = frame();
    const float radius = r.height * 0.5f;
    const float hover = hoverAmount();
    const float press = pressAmount();
    const bool enabled = isEnabled();

    Color text = c.label;
    switch (m_style) {
        case ButtonStyle::Default: {
            // Pulsar OS Liquid Glass button styling
            Color top = dark ? Color::white(0.20f + hover * 0.08f) : Color::white(0.96f + hover * 0.04f);
            Color bottom = dark ? Color::white(0.10f + hover * 0.06f) : Color::white(0.74f + hover * 0.08f);
            if (press > 0.01f) {
                top = top.mix(Color::black(), 0.18f * press);
                bottom = bottom.mix(Color::black(), 0.18f * press);
            }
            p.drawShadow(r, radius, dark ? Color::black(0.40f) : Color::black(0.12f), 4.0f, {0, 1.0f});
            p.fillVerticalGradient(r, radius, top, bottom);
            Color border = dark ? Color::white(0.18f + hover * 0.08f) : Color::black(0.10f).mix(Color::white(), 0.35f);
            p.strokeRoundedRect(r, radius, border, 0.75f);
            // Specular top rim highlight (Liquid Glass signature)
            const float rimH = std::min(r.height * 0.45f, 13.0f);
            p.pushClip({r.x, r.y, r.width, rimH});
            p.strokeRoundedRect(r.inset(0.5f), radius - 0.5f, Color::white(dark ? 0.30f : 0.85f), 1.0f);
            p.popClip();
            break;
        }
        case ButtonStyle::Prominent:
        case ButtonStyle::Destructive: {
            Color base = m_style == ButtonStyle::Prominent ? c.accent : c.destructive;
            Color top = base.lighter(0.12f).mix(base.lighter(0.20f), hover).mix(base.lighter(-0.10f), press);
            Color bottom = base.mix(base.lighter(0.06f), hover).mix(base.lighter(-0.20f), press);
            p.drawShadow(r, radius, base.withAlpha(dark ? 0.45f : 0.35f), 5.0f, {0, 1.5f});
            p.fillVerticalGradient(r, radius, top, bottom);
            p.strokeRoundedRect(r, radius, Color::white(dark ? 0.25f : 0.40f), 0.75f);
            // Specular top rim highlight
            const float rimH = std::min(r.height * 0.45f, 14.0f);
            p.pushClip({r.x, r.y, r.width, rimH});
            p.strokeRoundedRect(r.inset(0.5f), radius - 0.5f, Color::white(0.55f), 1.0f);
            p.popClip();
            text = c.onAccent;
            break;
        }
        case ButtonStyle::Borderless: {
            if (hover > 0.01f || press > 0.01f) {
                Color fill = dark ? Color::white(0.12f * hover + 0.18f * press)
                                  : Color::black(0.06f * hover + 0.12f * press);
                p.fillRoundedRect(r, radius, fill);
                Color border = dark ? Color::white(0.15f * hover) : Color::black(0.05f * hover);
                p.strokeRoundedRect(r, radius, border, 0.5f);
                const float rimH = std::min(r.height * 0.45f, 10.0f);
                p.pushClip({r.x, r.y, r.width, rimH});
                p.strokeRoundedRect(r.inset(0.5f), radius - 0.5f, Color::white(dark ? 0.20f * hover : 0.60f * hover), 0.75f);
                p.popClip();
            }
            text = foreground().value_or(m_title.empty() ? c.secondaryLabel : c.label);
            if (m_title.empty()) text = text.mix(c.label, hover);
            break;
        }
        case ButtonStyle::Glass: {
            p.drawShadow(r, radius, c.shadow.multipliedAlpha(0.5f), 8.0f, {0, 2});
            MaterialStyle glass = t.material(Material::Glass);
            glass.tint = glass.tint.mix(dark ? Color::white(0.16f) : Color::white(0.6f), hover * 0.5f + press * 0.5f);
            p.drawMaterial(r, radius, glass);
            break;
        }
    }
    if (auto fg = foreground(); fg && m_style != ButtonStyle::Prominent && m_style != ButtonStyle::Destructive) {
        text = *fg;
    }
    if (!enabled) text = text.multipliedAlpha(0.8f);

    const Font f = buttonFont(m_controlSize, m_style);
    const float is = iconSize(m_controlSize);
    float contentW = (m_icon != Icon::None ? is : 0.0f);
    float titleW = m_title.empty() ? 0.0f : p.measureText(m_title, f);
    if (!m_title.empty()) contentW += (contentW > 0 ? 5.0f : 0.0f) + titleW;
    float x = r.center().x - contentW * 0.5f;
    if (m_icon != Icon::None) {
        p.drawIcon(m_icon, {x, r.center().y - is * 0.5f, is, is}, text);
        x += is + 5.0f;
    }
    if (!m_title.empty()) p.drawText(m_title, {p.snap(x), r.y, titleW, r.height}, f, text);
}

bool ButtonView::onPointerDown(const PointerEvent& e) { return e.button == MouseButton::Left; }

void ButtonView::onPointerUp(const PointerEvent& e) {
    if (frame().contains(e.position)) click();
}

bool ButtonView::onKeyDown(const KeyEvent& e) {
    if ((e.key == Key::Space || e.key == Key::Enter) && !e.repeat) {
        // Flash the pressed state so keyboard activation is visible.
        m_press.set(1.0f);
        m_press.animateTo(0.0f, now(), Transition::standard());
        invalidate();
        click();
        return true;
    }
    return false;
}

}  // namespace mayo
