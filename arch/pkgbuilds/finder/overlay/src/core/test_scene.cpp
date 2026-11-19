// Renderer smoke test scene, shown when MAYO_TEST_SCENE=1. Exercises every primitive.
#include <cmath>

#include "../render/renderer2d.hpp"
#include "mayonnaise/painter.hpp"
#include "mayonnaise/theme.hpp"

namespace mayo {

void drawRendererTestScene(render::Renderer2D& r, Size size, const Theme& theme) {
    const auto& c = theme.colors;
    // Colourful backdrop so materials have something to blur.
    r.fillGradient({0, 0, size.width, size.height}, {}, c.indigo, c.pink, {0, 0}, {size.width, size.height});
    for (int i = 0; i < 8; ++i) {
        float x = 40.0f + static_cast<float>(i) * 90.0f;
        r.fillRoundedRect({x, 300, 60, 160}, 30, i % 2 ? c.yellow : c.mint);
    }

    // Cards with shadows and per-corner radii.
    r.shadow({40, 40, 200, 120}, 16, Color::black(0.25f), 24, {0, 8});
    r.fillRoundedRect({40, 40, 200, 120}, 16, c.windowBackground, c.separator, 1);
    r.fillRoundedRect({280, 40, 200, 120}, CornerRadii(4, 30, 4, 30), c.accent);
    r.strokeRoundedRect({520, 40, 200, 120}, 20, Color::white(), 3);

    // Lines and arcs.
    for (int i = 0; i < 6; ++i) {
        float a = static_cast<float>(i) * 0.5f;
        r.line({80, 230}, {80 + 60 * std::cos(a), 230 + 60 * std::sin(a)}, Color::white(), 1.0f + static_cast<float>(i));
    }
    r.arc({300, 230}, 40, -1.57f, 4.5f, Color::white(), 6);
    r.arc({420, 230}, 40, 0, 6.2832f, c.yellow, 2);

    // Rounded clip.
    r.pushClip({520, 180, 200, 100}, 24);
    r.fillGradient({500, 160, 260, 160}, {}, c.green, c.blue, {500, 160}, {760, 160});
    r.popClip();

    // Materials over the busy background.
    r.shadow({60, 330, 300, 140}, 18, Color::black(0.3f), 30, {0, 10});
    r.backdrop({60, 330, 300, 140}, 18, theme.material(Material::Sidebar));
    r.backdrop({400, 330, 300, 140}, 18, theme.material(Material::Glass));
    r.strokeRoundedRect({400, 330, 300, 140}, 18, Color::white(0.5f), 1);
}

void drawTextTestScene(Painter& p, Size size, const Theme& theme) {
    const auto& c = theme.colors;
    p.fillRect({0, 0, size.width, size.height}, c.windowBackground);
    struct Row {
        TextStyle style;
        const char* name;
    };
    const Row rows[] = {
        {TextStyle::LargeTitle, "Large Title"}, {TextStyle::Title1, "Title 1"}, {TextStyle::Title2, "Title 2"},
        {TextStyle::Title3, "Title 3"},         {TextStyle::Headline, "Headline"}, {TextStyle::Body, "Body"},
        {TextStyle::Callout, "Callout"},       {TextStyle::Subheadline, "Subheadline"},
        {TextStyle::Footnote, "Footnote"},     {TextStyle::Caption, "Caption"},
    };
    float y = 40;
    for (const auto& row : rows) {
        Font f = theme.font(row.style);
        p.drawText(row.name, {40, y + f.size}, f, c.label);
        p.drawText("The quick brown fox jumps over the lazy dog — Ünïcödé 0123456789", {200, y + f.size}, f,
                   c.secondaryLabel);
        y += f.lineHeight + 10;
    }
    Rect box{40, y + 10, 240, 40};
    p.fillRoundedRect(box, 8, c.accent);
    p.drawText("Centred in a box", box, theme.font(TextStyle::Headline), c.onAccent, TextAlign::Center);
}

}  // namespace mayo
