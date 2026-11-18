#include "file_icons.hpp"

#include <algorithm>
#include <cctype>
#include <cmath>

using namespace mayo;

namespace finder {

namespace {

struct KindArt {
    Color color;
    Icon glyph;
};

KindArt artFor(const FileEntry& e) {
    const auto& c = theme().colors;
    switch (e.kind) {
        case FileKind::Image: return {c.teal, Icon::Photo};
        case FileKind::Video: return {c.indigo, Icon::Film};
        case FileKind::Audio: return {c.pink, Icon::MusicNote};
        case FileKind::Pdf: return {c.red, Icon::Document};
        case FileKind::Text: return {c.gray, Icon::None};
        case FileKind::Code: return {c.blue, Icon::Terminal};
        case FileKind::Archive: return {c.brown, Icon::None};
        case FileKind::Document: return {c.blue, Icon::Pencil};
        case FileKind::Spreadsheet: return {c.green, Icon::Grid};
        case FileKind::Presentation: return {c.orange, Icon::Display};
        case FileKind::DiskImage: return {c.gray, Icon::HardDrive};
        case FileKind::Font: return {c.purple, Icon::None};
        case FileKind::Web: return {c.blue, Icon::Globe};
        default: return {c.gray, Icon::None};
    }
}

/// Page with a folded top-right corner, built from rects and a triangle (the painter has no paths).
void pageShape(Painter& p, const Rect& r, float fold, float radius, const Color& color) {
    p.fillRoundedRect({r.x, r.y, r.width - fold, r.height}, CornerRadii{radius, 0, 0, radius}, color);
    p.fillRoundedRect({r.right() - fold, r.y + fold, fold, r.height - fold}, CornerRadii{0, 0, radius, 0}, color);
    p.fillTriangle({r.right() - fold, r.y}, {r.right(), r.y + fold}, {r.right() - fold, r.y + fold}, color);
}

void drawAliasBadge(Painter& p, const Rect& r) {
    const float s = std::clamp(r.width * 0.2f, 8.0f, 22.0f);
    Rect badge{r.x + r.width * 0.12f, r.bottom() - s - r.height * 0.06f, s, s};
    p.drawShadow(badge, s * 0.22f, Color::black(0.25f), 2, {0, 0.5f});
    p.fillRoundedRect(badge, s * 0.22f, Color::white(), Color::black(0.15f), 0.5f);
    p.drawIcon(Icon::ArrowUpRight, badge.inset(s * 0.08f), Color::black(0.85f), 1.2f);
}

void drawExecutable(Painter& p, const Rect& r) {
    Rect box = r.inset(r.width * 0.1f);
    box = {box.x, box.y + box.height * 0.08f, box.width, box.height * 0.84f};
    const float radius = box.width * 0.12f;
    p.drawShadow(box, radius, Color::black(0.25f), std::max(1.5f, r.width * 0.04f), {0, r.width * 0.015f});
    p.fillVerticalGradient(box, radius, Color::hex(0x3A3A3C), Color::hex(0x1C1C1E));
    p.strokeRoundedRect(box, radius, Color::white(0.12f), 0.5f);
    if (r.width < 22) return;
    Font f{box.width * 0.22f, FontWeight::Semibold, box.width * 0.3f};
    p.drawText("exec", {box.x + box.width * 0.12f, box.y + box.height * 0.1f, box.width, box.height * 0.3f}, f,
               Color::hex(0x32D74B));
}

void drawImageThumbnail(Painter& p, const Rect& r, const ImageData& img) {
    const float iw = static_cast<float>(std::max(1, img.width()));
    const float ih = static_cast<float>(std::max(1, img.height()));
    const float box = r.width * 0.92f;
    const float k = std::min(box / iw, box / ih);
    Rect pic{r.center().x - iw * k * 0.5f, r.center().y - ih * k * 0.5f, iw * k, ih * k};
    // A white photo border scaled with the icon, like Finder's image previews.
    const float border = r.width >= 48 ? std::max(1.5f, r.width * 0.025f) : 0.0f;
    Rect frame = pic.outset(border);
    p.drawShadow(frame, 1, Color::black(0.28f), std::max(1.5f, r.width * 0.035f), {0, r.width * 0.012f});
    if (border > 0) p.fillRect(frame, Color::white());
    p.drawImage(pic, img);
    if (border <= 0) p.strokeRoundedRect(pic, 0, Color::black(0.12f), 0.5f);
}

}  // namespace

void drawFolderIcon(Painter& p, const Rect& r, Icon glyph, float opacity) {
    const float s = std::min(r.width, r.height);
    const Rect sq{r.center().x - s * 0.5f, r.center().y - s * 0.5f, s, s};
    const bool dark = theme().isDark();
    const Color back = dark ? Color::hex(0x2F8FE0) : Color::hex(0x3E9CEC);
    const Color frontTop = dark ? Color::hex(0x5BB2F7) : Color::hex(0x78C3FB);
    const Color frontBottom = dark ? Color::hex(0x3E9DEE) : Color::hex(0x55AEF6);
    const float radius = std::max(1.5f, s * 0.055f);
    p.pushOpacity(opacity);

    // Back panel with the tab.
    Rect tab{sq.x + s * 0.06f, sq.y + s * 0.16f, s * 0.38f, s * 0.14f};
    Rect body{sq.x + s * 0.06f, sq.y + s * 0.22f, s * 0.88f, s * 0.62f};
    // Square top-right corner on the tab so the slope down to the body starts cleanly from it.
    p.fillRoundedRect(tab, CornerRadii{radius, 0, 0, radius}, back);
    p.fillTriangle({tab.right() - 0.5f, tab.y}, {tab.right() + s * 0.075f, body.y + s * 0.01f},
                   {tab.right() - 0.5f, body.y + s * 0.01f}, back);
    p.fillRoundedRect(body, radius, back);

    // Front panel: lighter, with a soft top highlight and a shadow onto the back.
    Rect front{sq.x + s * 0.06f, sq.y + s * 0.3f, s * 0.88f, s * 0.54f};
    if (s >= 24) p.drawShadow(front, radius, Color::black(0.18f), s * 0.03f, {0, -s * 0.008f});
    p.fillVerticalGradient(front, radius, frontTop, frontBottom);
    if (s >= 20) {
        const float px = std::max(0.5f, s * 0.01f);
        p.fillRoundedRect({front.x + radius * 0.5f, front.y, front.width - radius, px}, px * 0.5f, Color::white(0.45f));
    }
    if (glyph != Icon::None && s >= 30) {
        const float g = front.height * 0.52f;
        Rect gr{front.center().x - g * 0.5f, front.center().y - g * 0.5f + s * 0.01f, g, g};
        p.drawIcon(glyph, gr, Color::hex(dark ? 0x1D6FBF : 0x2C82D6, 0.85f), 1.15f);
    }
    p.popOpacity();
}

void drawFileIcon(Painter& p, const Rect& r, const FileEntry& e, const IconStyle& style) {
    const float s = std::min(r.width, r.height);
    const Rect sq{r.center().x - s * 0.5f, r.center().y - s * 0.5f, s, s};
    p.pushOpacity(style.dimmed ? 0.45f : 1.0f);
    const Preview* preview = style.preview;
    if (e.isDir) {
        drawFolderIcon(p, sq, style.folderGlyph);
    } else if (preview && preview->image && preview->image->width() > 0) {
        drawImageThumbnail(p, sq, *preview->image);
    } else if (e.kind == FileKind::Executable && e.ext.empty()) {
        drawExecutable(p, sq);
    } else {
        const KindArt art = artFor(e);
        const float h = s * 0.92f;
        const float w = h * 0.77f;
        Rect page{sq.center().x - w * 0.5f, sq.center().y - h * 0.5f, w, h};
        const float fold = w * 0.26f;
        const float radius = std::max(1.0f, w * 0.06f);
        const float px = 1.0f / p.scale();
        if (s >= 24) p.drawShadow(page.inset(w * 0.04f, 0), radius, Color::black(0.22f), s * 0.03f, {0, s * 0.012f});
        pageShape(p, page.outset(px), fold + px, radius + px, Color::black(theme().isDark() ? 0.35f : 0.16f));
        pageShape(p, page, fold, radius, Color::white());
        // The folded flap.
        p.fillTriangle({page.right() - fold, page.y}, {page.right() - fold, page.y + fold}, {page.right(), page.y + fold},
                       Color::hex(0xD9D9DE), radius * 0.3f);

        const Rect content = page.inset(Insets(fold + h * 0.04f, w * 0.13f, h * 0.1f, w * 0.13f));
        if (preview && !preview->lines.empty() && s >= 28) {
            // Text thumbnail: one grey bar per line, as long as the line is.
            const float lineH = std::max(1.5f, h * 0.045f);
            const float gap = lineH * 0.8f;
            float y = page.y + h * 0.1f;
            const float full = page.width - w * 0.26f;
            for (size_t i = 0; i < preview->lines.size(); ++i) {
                float avail = (y < page.y + fold + gap) ? full - fold * 0.9f : full;
                if (y + lineH > page.bottom() - h * 0.18f) break;
                float len = std::min(avail, full * static_cast<float>(preview->lines[i]) / 48.0f);
                if (len > 0.5f) p.fillRoundedRect({page.x + w * 0.13f, y, len, lineH}, lineH * 0.5f, Color::black(0.16f));
                y += lineH + gap;
            }
        } else if (e.kind == FileKind::Archive && s >= 20) {
            // Zipper down the middle.
            const float zx = page.center().x;
            const float step = std::max(2.0f, h * 0.055f);
            for (float y = page.y + step * 0.5f; y < page.center().y + h * 0.05f; y += step) {
                bool left = static_cast<int>((y - page.y) / step) % 2 == 0;
                p.fillRect({left ? zx - step * 0.9f : zx, y, step * 0.9f, step * 0.5f}, Color::hex(0x8E8E93));
            }
            Rect pull{zx - step * 0.7f, page.center().y + h * 0.05f, step * 1.4f, step * 2.2f};
            p.fillRoundedRect(pull, step * 0.3f, Color::hex(0x8E8E93));
        } else if (e.kind == FileKind::Font && s >= 20) {
            Font f{h * 0.32f, FontWeight::Medium, h * 0.36f};
            p.drawText("Aa", {content.x, content.y, content.width, content.height * 0.8f}, f, art.color,
                       TextAlign::Center);
        } else if (art.glyph != Icon::None && s >= 16) {
            const float g = std::min(content.width, content.height) * 0.7f;
            Rect gr{content.center().x - g * 0.5f, content.y + content.height * 0.38f - g * 0.5f, g, g};
            p.drawIcon(art.glyph, gr, art.color, 1.1f);
        }

        // Extension label along the bottom, like macOS document icons.
        if (!e.ext.empty() && s >= 40) {
            std::string label = e.ext.substr(0, 5);
            std::transform(label.begin(), label.end(), label.begin(),
                           [](unsigned char c) { return static_cast<char>(std::toupper(c)); });
            Font f{h * 0.12f, FontWeight::Bold, h * 0.14f};
            Rect band{page.x, page.bottom() - h * 0.24f, page.width, h * 0.16f};
            p.drawText(label, band, f, art.color.mix(Color::black(), 0.1f), TextAlign::Center);
        }
    }
    if (e.isLink && s >= 16) drawAliasBadge(p, sq);
    p.popOpacity();
}

Icon folderGlyphFor(const std::string& path) {
    // Resolved once: this runs for every folder icon drawn.
    static const std::vector<std::pair<std::string, Icon>> places = [] {
        std::vector<std::pair<std::string, Icon>> v{
            {homeDirectory(), Icon::House},
            {userDirectory("DOWNLOAD", "Downloads"), Icon::ArrowDownCircle},
            {userDirectory("DOCUMENTS", "Documents"), Icon::Document},
            {userDirectory("DESKTOP", "Desktop"), Icon::Display},
            {userDirectory("PICTURES", "Pictures"), Icon::Photo},
            {userDirectory("MUSIC", "Music"), Icon::MusicNote},
            {userDirectory("VIDEOS", "Videos"), Icon::Film},
            {userDirectory("PUBLICSHARE", "Public"), Icon::Share},
        };
        std::erase_if(v, [](const auto& p) { return p.first.empty(); });
        return v;
    }();
    for (const auto& [p, icon] : places) {
        if (p == path) return icon;
    }
    return Icon::None;
}

Icon placeIcon(const std::string& path) {
    if (path == kRecents) return Icon::Clock;
    if (path == "/") return Icon::HardDrive;
    if (path == trashFilesDirectory()) return Icon::Trash;
    Icon g = folderGlyphFor(path);
    return g != Icon::None ? g : Icon::Folder;
}

}  // namespace finder
