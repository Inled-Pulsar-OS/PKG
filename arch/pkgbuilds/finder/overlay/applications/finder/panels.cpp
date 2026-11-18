#include "panels.hpp"

#include <algorithm>
#include <cmath>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <mayonnaise/app.hpp>

#include "file_icons.hpp"

namespace fs = std::filesystem;
using namespace mayo;

namespace finder {

// --- CenteredPanel ---------------------------------------------------------------------------------

void CenteredPanel::present(WidgetHost& host) {
    if (m_shown) dismiss();
    m_shown = true;
    m_shownHost = &host;
    m_appear.set(0.0f);
    m_appear.animateTo(1.0f, host.time(), Transition{0.18f, Curve::EaseOut});
    host.showOverlay(shared_from_this());
    host.setFocus(this, false);
}

Rect CenteredPanel::placePopup(Size window, Size measured) {
    const float w = std::min(measured.width, window.width - 32);
    const float h = std::min(measured.height, window.height - 32);
    // Optically centred: a little above the middle.
    return {std::round((window.width - w) * 0.5f), std::round(std::max(16.0f, (window.height - h) * 0.42f)), w, h};
}

// --- AlertView -----------------------------------------------------------------------------------------

AlertView::AlertView(std::string title, std::string message, std::vector<AlertButton> buttons, bool caution)
    : CenteredPanel(Material::Popover), m_buttons(std::move(buttons)) {
    setPadding(Insets(20, 20, 18, 20));
    auto icon = Canvas([caution](Painter& p, const Rect& r) {
        const auto& c = theme().colors;
        if (caution) {
            p.fillTriangle({r.center().x, r.y + 2}, {r.right() - 1, r.bottom() - 3}, {r.x + 1, r.bottom() - 3}, c.yellow, 5);
            p.fillRoundedRect({r.center().x - 2.5f, r.y + r.height * 0.32f, 5, r.height * 0.32f}, 2.5f, Color::black(0.75f));
            p.fillCircle({r.center().x, r.y + r.height * 0.76f}, 2.8f, Color::black(0.75f));
        } else {
            drawFolderIcon(p, r);
        }
    });
    icon.frame(52, 52);
    auto titleLabel = Label(std::move(title)).font(TextStyle::Headline).textAlign(TextAlign::Center).maxWidth(250);
    auto messageLabel = Label(std::move(message)).font(TextStyle::Subheadline).textAlign(TextAlign::Center).maxWidth(250);
    std::vector<Element> row;
    for (size_t i = 0; i < m_buttons.size(); ++i) {
        auto b = Button(m_buttons[i].title).style(m_buttons[i].style).onClick([this, i] { choose(i); });
        b.fillWidth();
        row.emplace_back(b);
    }
    auto buttonRow = HStack(row).spacing(8);
    buttonRow.fillWidth();
    auto stack = VStack({icon, titleLabel, messageLabel, Spacer(4), buttonRow}).spacing(8).align(Alignment::Top);
    stack.width(250);
    addChild(stack.node());
}

void AlertView::show(WidgetHost& host) { present(host); }

void AlertView::choose(size_t index) {
    auto keep = shared_from_this();
    dismiss();
    if (index < m_buttons.size() && m_buttons[index].action) m_buttons[index].action();
}

bool AlertView::onKeyDown(const KeyEvent& e) {
    if (e.key == Key::Escape) {
        choose(0);
        return true;
    }
    if (e.key == Key::Enter) {
        choose(m_buttons.empty() ? 0 : m_buttons.size() - 1);
        return true;
    }
    return false;
}

void AlertView::drawPanel(Painter& p) { paintChildren(p); }

// --- QuickLookView -------------------------------------------------------------------------------------

namespace {
constexpr float kHeader = 44;
constexpr float kTextLine = 17;
}  // namespace

QuickLookView::QuickLookView(std::shared_ptr<Browser> browser)
    : CenteredPanel(Material::Popover), m_browser(std::move(browser)) {}

QuickLookView::~QuickLookView() {
    if (m_player) {
        m_player->close();
        m_player.reset();
    }
    if (!m_pdfTempDir.empty()) {
        std::error_code ec;
        fs::remove_all(m_pdfTempDir, ec);
        m_pdfTempDir.clear();
    }
}

void QuickLookView::setEntry(const FileEntry& e) {
    const bool same = e.path == m_entry.path && e.modified == m_entry.modified;
    m_entry = e;
    if (same) return;

    if (m_player) {
        m_player->close();
        m_player.reset();
    }
    m_videoTexture.reset();
    m_shownFrame = {};
    m_isVideo = false;
    m_videoPaused = false;

    if (!m_pdfTempDir.empty()) {
        std::error_code ec;
        fs::remove_all(m_pdfTempDir, ec);
        m_pdfTempDir.clear();
    }
    m_pdfPages.clear();
    m_isPdf = false;

    m_image.reset();
    m_lines.clear();
    m_scroll = 0;
    m_physics.setOffset({});
    m_isText = false;
    m_childCount = 0;

    if (e.isDir) {
        if (m_browser->listing) m_childCount = m_browser->listing(e.path).size();
    } else if (e.kind == FileKind::Video) {
        m_isVideo = true;
        m_videoPaused = false;
        m_player = std::make_unique<MediaPlayer>();
        std::string err;
        if (m_player->open(e.path, err)) {
            m_player->play();
        }
        if (m_browser->previews) {
            std::weak_ptr<Widget> weak = weak_from_this();
            m_browser->previews->loadLarge(e.path, 1920, [this, weak, path = e.path](std::shared_ptr<ImageData> img) {
                if (!weak.lock() || m_entry.path != path) return;
                m_image = std::move(img);
                invalidateLayout();
            }, FileKind::Video);
        }
    } else if (e.kind == FileKind::Pdf) {
        m_isPdf = true;
        if (m_browser->previews) {
            std::weak_ptr<Widget> weak = weak_from_this();
            m_browser->previews->loadLarge(e.path, 1600, [this, weak, path = e.path](std::shared_ptr<ImageData> img) {
                if (!weak.lock() || m_entry.path != path) return;
                if (m_pdfPages.empty() && img) {
                    m_pdfPages.push_back(img);
                    invalidateLayout();
                }
            }, FileKind::Pdf);
        }
        std::string pdfPath = e.path;
        char tempPattern[] = "/tmp/finder_pdf_XXXXXX";
        if (char* d = mkdtemp(tempPattern)) {
            m_pdfTempDir = d;
            std::string tempDir = m_pdfTempDir;
            std::weak_ptr<Widget> weak = weak_from_this();
            std::thread([this, weak, pdfPath, tempDir] {
                std::string cmd = "pdftoppm -png -r 140 -f 1 -l 40 \"" + pdfPath + "\" \"" + tempDir + "/p\" 2>/dev/null";
                (void)std::system(cmd.c_str());

                std::vector<std::string> pagePaths;
                for (int page = 1; page <= 60; ++page) {
                    std::string p1 = tempDir + "/p-" + std::to_string(page) + ".png";
                    char buf[64];
                    std::snprintf(buf, sizeof(buf), "%s/p-%02d.png", tempDir.c_str(), page);
                    std::string p2 = buf;
                    std::snprintf(buf, sizeof(buf), "%s/p-%03d.png", tempDir.c_str(), page);
                    std::string p3 = buf;
                    std::error_code ec;
                    if (fs::exists(p1, ec)) pagePaths.push_back(p1);
                    else if (fs::exists(p2, ec)) pagePaths.push_back(p2);
                    else if (fs::exists(p3, ec)) pagePaths.push_back(p3);
                    else break;
                }

                std::vector<std::shared_ptr<ImageData>> loaded;
                for (const auto& pp : pagePaths) {
                    if (auto img = ImageData::load(pp)) loaded.push_back(img);
                }

                if (!loaded.empty()) {
                    App::current().post([this, weak, loaded = std::move(loaded), pdfPath] {
                        if (!weak.lock() || m_entry.path != pdfPath) return;
                        m_pdfPages = std::move(loaded);
                        invalidateLayout();
                    });
                }
            }).detach();
        }
    } else if (isImageExtension(e.ext)) {
        std::weak_ptr<Widget> weak = weak_from_this();
        if (m_browser->previews) {
            m_browser->previews->loadLarge(e.path, 2560, [this, weak, path = e.path](std::shared_ptr<ImageData> img) {
                if (!weak.lock() || m_entry.path != path) return;
                m_image = std::move(img);
                invalidateLayout();
            }, FileKind::Image);
        }
    } else if (isTextLike(e) || e.size < 256 * 1024) {
        std::ifstream in(e.path, std::ios::binary);
        std::string data(64 * 1024, '\0');
        in.read(data.data(), static_cast<std::streamsize>(data.size()));
        data.resize(static_cast<size_t>(in.gcount()));
        if (!data.empty() && std::memchr(data.data(), 0, data.size()) == nullptr) {
            m_isText = true;
            size_t start = 0;
            while (start <= data.size() && m_lines.size() < 3000) {
                size_t end = data.find('\n', start);
                if (end == std::string::npos) end = data.size();
                std::string line = data.substr(start, end - start);
                if (!line.empty() && line.back() == '\r') line.pop_back();
                std::string expanded;
                for (char ch : line) {
                    if (ch == '\t') expanded += "    ";
                    else expanded.push_back(ch);
                }
                m_lines.push_back(std::move(expanded));
                start = end + 1;
            }
        }
    }
    invalidateLayout();
}

Size QuickLookView::measureContent(Size /*available*/) {
    Size window = host() ? host()->windowSize() : Size{1000, 700};
    const float maxW = std::max(320.0f, window.width * 0.82f);
    const float maxH = std::max(260.0f, window.height * 0.84f);
    if (m_isVideo) {
        float aspect = (m_player && m_player->isOpen() && m_player->aspect() > 0.1) ? static_cast<float>(m_player->aspect()) : (16.0f / 9.0f);
        float w = std::min(maxW, 800.0f);
        float h = w / aspect + kHeader;
        if (h > maxH) {
            h = maxH;
            w = (h - kHeader) * aspect;
        }
        return {std::max(360.0f, w), std::max(260.0f, h)};
    }
    if (m_isPdf) {
        return {std::min(maxW, 760.0f), std::min(maxH, 680.0f)};
    }
    if (m_image && m_image->width() > 0) {
        const float iw = static_cast<float>(m_image->originalWidth());
        const float ih = static_cast<float>(m_image->originalHeight());
        const float k = std::min({maxW / iw, (maxH - kHeader) / ih, 1.0f});
        return {std::max(360.0f, iw * k), std::max(240.0f, ih * k + kHeader)};
    }
    if (m_isText) return {std::min(maxW, 760.0f), std::min(maxH, 640.0f)};
    return {std::min(maxW, 520.0f), std::min(maxH, 400.0f)};
}

Rect QuickLookView::closeRect() const { return {frame().x + 14, frame().y + kHeader * 0.5f - 7, 14, 14}; }

Rect QuickLookView::openRect() const {
    return {frame().right() - 120, frame().y + (kHeader - 26) * 0.5f, 108, 26};
}

Rect QuickLookView::body() const { return Rect::fromLTRB(frame().x, frame().y + kHeader, frame().right(), frame().bottom()); }

void QuickLookView::drawPanel(Painter& p) {
    const Theme& t = theme();
    const auto& c = t.colors;
    const Rect r = frame();
    const float px = 1.0f / p.scale();

    // Header: close button, title, "Open" button.
    const Rect close = closeRect();
    p.fillCircle(close.center(), 7, m_hover == 0 ? c.red : c.fill);
    p.drawIcon(Icon::Xmark, close.inset(3.5f), m_hover == 0 ? Color::black(0.6f) : c.secondaryLabel, 1.4f);
    const Font titleFont = t.font(TextStyle::Headline);
    const Rect titleArea{r.x + 130, r.y, r.width - 260, kHeader};
    p.drawText(truncateMiddle(p, m_entry.name, titleFont, titleArea.width), titleArea, titleFont, c.label, TextAlign::Center);
    const Rect open = openRect();
    if (m_hover == 1) p.fillRoundedRect(open, open.height * 0.5f, c.hover);
    p.drawIcon(Icon::ArrowUpRight, {open.x + 10, open.center().y - 7, 14, 14}, c.label);
    p.drawText("Open", {open.x + 30, open.y, open.width - 30, open.height}, t.font(TextStyle::Body), c.label);
    p.fillRect({r.x, p.snap(r.y + kHeader) - px, r.width, px}, c.separator);

    const Rect b = body();
    p.pushClip(b);

    if (m_isVideo) {
        if (m_player && m_player->isOpen()) {
            MediaPlayer::Frame f;
            if (m_player->takeFrame(f)) {
                if (!m_videoTexture) m_videoTexture = VideoTexture::create();
                if (f.uploadTo(*m_videoTexture)) {
                    m_shownFrame = std::move(f);
                } else {
                    m_player->disableZeroCopy();
                }
            }
        }
        p.fillRect(b, Color::black());
        if (m_videoTexture && !m_videoTexture->isEmpty()) {
            float aspect = (m_player && m_player->aspect() > 0.1) ? static_cast<float>(m_player->aspect()) : (16.0f / 9.0f);
            float vw = b.width;
            float vh = vw / aspect;
            if (vh > b.height) {
                vh = b.height;
                vw = vh * aspect;
            }
            Rect videoRect{b.center().x - vw * 0.5f, b.center().y - vh * 0.5f, vw, vh};
            p.drawVideo(videoRect, *m_videoTexture, 0.0f, 1.0f);
        } else if (m_image && m_image->width() > 0) {
            const float iw = static_cast<float>(m_image->width());
            const float ih = static_cast<float>(m_image->height());
            const float k = std::min(b.width / iw, b.height / ih);
            Rect pic{b.center().x - iw * k * 0.5f, b.center().y - ih * k * 0.5f, iw * k, ih * k};
            p.drawImage(pic, *m_image);
        }

        // Floating Liquid Glass video HUD controls
        Rect hudRect{b.center().x - 170.0f, b.bottom() - 48.0f, 340.0f, 36.0f};
        p.drawShadow(hudRect, 18.0f, Color::black(0.4f), 12.0f, {0, 4});
        p.fillVerticalGradient(hudRect, 18.0f, Color::white(0.24f), Color::white(0.08f));
        p.strokeRoundedRect(hudRect, 18.0f, Color::white(0.30f), 0.75f);
        p.strokeRoundedRect({hudRect.x + 0.5f, hudRect.y + 0.5f, hudRect.width - 1.0f, 1.0f}, 0.5f, Color::white(0.5f), 1.0f);

        // Play/Pause button
        Icon playIcon = m_videoPaused ? Icon::Play : Icon::Pause;
        Rect playBtn{hudRect.x + 12.0f, hudRect.center().y - 8.0f, 16.0f, 16.0f};
        p.drawIcon(playIcon, playBtn, Color::white(), 1.2f);

        // Scrub track
        double pos = m_player ? m_player->position() : 0;
        double dur = m_player ? m_player->duration() : 0;
        Rect track{hudRect.x + 38.0f, hudRect.center().y - 2.5f, 205.0f, 5.0f};
        p.fillRoundedRect(track, 2.5f, Color::white(0.22f));
        if (dur > 0) {
            float progress = static_cast<float>(pos / dur);
            Rect filled{track.x, track.y, track.width * std::clamp(progress, 0.0f, 1.0f), track.height};
            p.fillRoundedRect(filled, 2.5f, c.accent);
            Point knob{filled.right(), track.center().y};
            p.fillCircle(knob, 5.5f, Color::white());
        }

        // Time text
        int pMin = static_cast<int>(pos) / 60, pSec = static_cast<int>(pos) % 60;
        int dMin = static_cast<int>(dur) / 60, dSec = static_cast<int>(dur) % 60;
        char timeBuf[32];
        std::snprintf(timeBuf, sizeof(timeBuf), "%02d:%02d / %02d:%02d", pMin, pSec, dMin, dSec);
        p.drawText(timeBuf, {hudRect.right() - 86.0f, hudRect.y, 80.0f, hudRect.height}, t.font(TextStyle::Caption), Color::white(), TextAlign::Trailing);

        requestAnimationFrame();
    } else if (m_isPdf) {
        if (m_physics.isAnimating()) {
            if (m_physics.step(now())) requestAnimationFrame();
            else if (host() && m_physics.deadline() >= 0) host()->scheduleFrame(m_physics.deadline());
            m_scroll = m_physics.offset().y;
        }
        p.fillRect(b, c.contentBackground);

        float targetW = std::min(b.width - 48.0f, 620.0f);
        float targetH = targetW * 1.35f;
        if (!m_pdfPages.empty() && m_pdfPages[0] && m_pdfPages[0]->width() > 0) {
            float aspect = static_cast<float>(m_pdfPages[0]->height()) / static_cast<float>(m_pdfPages[0]->width());
            targetH = targetW * aspect;
        }

        const float pageSpacing = 16.0f;
        const float totalH = std::max(1.0f, static_cast<float>(m_pdfPages.size())) * (targetH + pageSpacing) + 32.0f;
        const float maxScroll = std::max(0.0f, totalH - b.height);
        m_physics.setLimits({0, maxScroll}, b.size());

        float top = b.y + 16.0f - m_scroll;
        int curPage = 1;
        for (size_t i = 0; i < m_pdfPages.size(); ++i) {
            float y = top + static_cast<float>(i) * (targetH + pageSpacing);
            if (y + targetH >= b.y && y <= b.bottom()) {
                Rect pageRect{b.center().x - targetW * 0.5f, y, targetW, targetH};
                p.drawShadow(pageRect, 4.0f, Color::black(0.25f), 10.0f, {0, 3});
                p.fillRoundedRect(pageRect, 2.0f, Color::white());
                if (m_pdfPages[i]) {
                    p.drawImage(pageRect, *m_pdfPages[i]);
                }
            }
            if (y <= b.center().y && y + targetH >= b.center().y) {
                curPage = static_cast<int>(i + 1);
            }
        }

        if (m_pdfPages.size() > 1) {
            std::string pageStr = std::to_string(curPage) + " / " + std::to_string(m_pdfPages.size());
            Font pf = t.font(TextStyle::Caption);
            float pw = p.measureText(pageStr, pf) + 20.0f;
            Rect pill{b.center().x - pw * 0.5f, b.bottom() - 32.0f, pw, 24.0f};
            p.drawShadow(pill, 12.0f, Color::black(0.35f), 8.0f, {0, 2});
            p.fillRoundedRect(pill, 12.0f, Color::black(0.70f));
            p.drawText(pageStr, pill, pf, Color::white(), TextAlign::Center);
        }
    } else if (m_image && m_image->width() > 0) {
        const float iw = static_cast<float>(m_image->width());
        const float ih = static_cast<float>(m_image->height());
        const float k = std::min(b.width / iw, b.height / ih);
        Rect pic{b.center().x - iw * k * 0.5f, b.center().y - ih * k * 0.5f, iw * k, ih * k};
        p.drawImage(pic, *m_image);
    } else if (m_isText) {
        if (m_physics.isAnimating()) {
            if (m_physics.step(now())) requestAnimationFrame();
            else if (host() && m_physics.deadline() >= 0) host()->scheduleFrame(m_physics.deadline());
            m_scroll = m_physics.offset().y;
        }
        p.fillRect(b, c.contentBackground);
        const Font f = t.font(TextStyle::Callout);
        const float top = b.y + 12 - m_scroll;
        const int first = std::max(0, static_cast<int>((b.y - top) / kTextLine) - 1);
        const int last = std::min(static_cast<int>(m_lines.size()), static_cast<int>((b.bottom() - top) / kTextLine) + 1);
        const Font gutter = t.font(TextStyle::Caption);
        for (int i = first; i < last; ++i) {
            const float y = top + static_cast<float>(i) * kTextLine;
            p.drawText(std::to_string(i + 1), {b.x, y, 38, kTextLine}, gutter, c.tertiaryLabel, TextAlign::Trailing);
            p.drawText(m_lines[static_cast<size_t>(i)], {b.x + 50, y, b.width - 60, kTextLine}, f, c.label);
        }
    } else {
        // Large icon with the essentials, like Quick Look for files it can't render.
        const float s = std::min(160.0f, b.height * 0.45f);
        const Rect icon{b.center().x - s * 0.5f, b.y + b.height * 0.12f, s, s};
        IconStyle style;
        style.preview = m_browser->previews ? m_browser->previews->get(m_entry, s * p.scale()) : nullptr;
        style.folderGlyph = m_entry.isDir ? folderGlyphFor(m_entry.path) : Icon::None;
        drawFileIcon(p, icon, m_entry, style);
        float y = icon.bottom() + 18;
        const Font big = t.font(TextStyle::Title3);
        p.drawText(truncateMiddle(p, m_entry.name, big, b.width - 40), {b.x + 20, y, b.width - 40, 24}, big, c.label,
                   TextAlign::Center);
        y += 28;
        const Font small = t.font(TextStyle::Body);
        std::string line = kindDescription(m_entry) + " – " + (m_entry.isDir ? itemCount(m_childCount) : formatSize(m_entry.size));
        p.drawText(line, {b.x, y, b.width, 18}, small, c.secondaryLabel, TextAlign::Center);
        y += 20;
        p.drawText("Modified " + formatDate(m_entry.modified), {b.x, y, b.width, 18}, small, c.secondaryLabel,
                   TextAlign::Center);
    }
    p.popClip();
}

bool QuickLookView::onKeyDown(const KeyEvent& e) {
    switch (e.key) {
        case Key::Space:
            if (m_isVideo && m_player && m_player->isOpen()) {
                if (m_videoPaused) {
                    m_player->play();
                    m_videoPaused = false;
                } else {
                    m_player->pause();
                    m_videoPaused = true;
                }
                invalidate();
                return true;
            }
            dismiss();
            return true;
        case Key::Escape:
            dismiss();
            return true;
        case Key::Up:
        case Key::Down:
        case Key::Left:
        case Key::Right:
            if (onArrow) onArrow(e.key);
            return true;
        case Key::Enter:
            if (onOpen) onOpen(m_entry);
            return true;
        case Key::PageDown:
        case Key::PageUp: {
            ScrollEvent s;
            s.delta = {0, (e.key == Key::PageDown ? 1.0f : -1.0f) * body().height * 0.85f};
            onScroll(s);
            return true;
        }
        default: return false;
    }
}

bool QuickLookView::onPointerDown(const PointerEvent& e) {
    if (closeRect().outset(4).contains(e.position)) {
        dismiss();
        return true;
    } else if (openRect().contains(e.position)) {
        if (onOpen) onOpen(m_entry);
        return true;
    }
    if (m_isVideo && body().contains(e.position)) {
        if (m_player && m_player->isOpen()) {
            if (m_videoPaused) {
                m_player->play();
                m_videoPaused = false;
            } else {
                m_player->pause();
                m_videoPaused = true;
            }
            invalidate();
        }
        return true;
    }
    if (e.clickCount == 2 && body().contains(e.position)) {
        if (onOpen) onOpen(m_entry);
        return true;
    }
    return true;
}

void QuickLookView::onPointerMove(const PointerEvent& e) {
    int h = closeRect().outset(4).contains(e.position) ? 0 : openRect().contains(e.position) ? 1 : -1;
    if (h != m_hover) {
        m_hover = h;
        invalidate();
    }
}

bool QuickLookView::onScroll(const ScrollEvent& e) {
    if (m_isText) {
        const float content = static_cast<float>(m_lines.size()) * kTextLine + 24;
        const float maxScroll = std::max(0.0f, content - body().height);
        m_physics.setLimits({0, maxScroll}, body().size());
        if (!m_physics.handle(e, now())) return true;
        m_scroll = m_physics.offset().y;
        if (m_physics.isAnimating()) requestAnimationFrame();
        if (host() && m_physics.deadline() >= 0) host()->scheduleFrame(m_physics.deadline());
        invalidate();
        return true;
    }
    if (m_isPdf) {
        float targetW = std::min(body().width - 48.0f, 620.0f);
        float targetH = targetW * 1.35f;
        if (!m_pdfPages.empty() && m_pdfPages[0] && m_pdfPages[0]->width() > 0) {
            float aspect = static_cast<float>(m_pdfPages[0]->height()) / static_cast<float>(m_pdfPages[0]->width());
            targetH = targetW * aspect;
        }
        const float totalH = std::max(1.0f, static_cast<float>(m_pdfPages.size())) * (targetH + 16.0f) + 32.0f;
        const float maxScroll = std::max(0.0f, totalH - body().height);
        m_physics.setLimits({0, maxScroll}, body().size());
        if (!m_physics.handle(e, now())) return true;
        m_scroll = m_physics.offset().y;
        if (m_physics.isAnimating()) requestAnimationFrame();
        if (host() && m_physics.deadline() >= 0) host()->scheduleFrame(m_physics.deadline());
        invalidate();
        return true;
    }
    return true;
}

}  // namespace finder
