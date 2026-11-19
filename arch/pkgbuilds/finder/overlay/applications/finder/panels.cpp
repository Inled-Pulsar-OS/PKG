#include "panels.hpp"

#include <algorithm>
#include <cmath>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <thread>
#include <mayonnaise/app.hpp>
#include <mayonnaise/i18n.hpp>

#include "file_icons.hpp"

namespace fs = std::filesystem;
using namespace mayo;
using namespace mayo::i18n;

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
    setPadding(Insets(24, 24, 22, 24));
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
    icon.frame(40, 40);
    auto titleLabel = Label(std::move(title)).font(TextStyle::Headline).textAlign(TextAlign::Center).maxWidth(304);
    auto messageLabel = Label(std::move(message)).font(TextStyle::Subheadline).textAlign(TextAlign::Center).maxWidth(304);
    std::vector<Element> row;
    for (size_t i = 0; i < m_buttons.size(); ++i) {
        auto b = Button(m_buttons[i].title).style(m_buttons[i].style).onClick([this, i] { choose(i); });
        b.fillWidth();
        row.emplace_back(b);
    }
    auto buttonRow = VStack(row).spacing(8);
    buttonRow.fillWidth();
    auto stack = VStack({icon, titleLabel, messageLabel, Spacer(4), buttonRow}).spacing(8).align(Alignment::Top);
    stack.width(304);
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

// --- OperationProgressPanel ----------------------------------------------------------------------------

OperationProgressPanel::OperationProgressPanel(std::function<void()> onCancel)
    : CenteredPanel(Material::Glass), m_onCancel(std::move(onCancel)) {}

void OperationProgressPanel::setOperation(std::string title, std::string item, double fraction, std::string detail) {
    m_title = std::move(title);
    m_item = std::move(item);
    m_fraction = std::clamp(fraction, 0.0, 1.0);
    m_detail = std::move(detail);
    invalidate();
}

Size OperationProgressPanel::measureContent(Size /*available*/) {
    return {420.0f, 115.0f};
}

void OperationProgressPanel::cancel() {
    if (m_onCancel) m_onCancel();
    dismiss();
}

bool OperationProgressPanel::onPointerDown(const PointerEvent& e) {
    float barX = frame().x + 60.0f;
    float barY = frame().y + 48.0f;
    float barW = frame().width - 100.0f;
    Rect cancelRect{barX + barW + 10.0f, barY - 7.0f, 20.0f, 20.0f};
    if (cancelRect.contains(e.position)) {
        cancel();
        return true;
    }
    return true;
}

void OperationProgressPanel::onPointerMove(const PointerEvent& e) {
    float barX = frame().x + 60.0f;
    float barY = frame().y + 48.0f;
    float barW = frame().width - 100.0f;
    Rect cancelRect{barX + barW + 10.0f, barY - 7.0f, 20.0f, 20.0f};
    bool hover = cancelRect.contains(e.position);
    if (hover != m_hoverCancel) {
        m_hoverCancel = hover;
        invalidate();
    }
}

void OperationProgressPanel::drawPanel(Painter& p) {
    const Theme& t = theme();
    const auto& c = t.colors;
    const Rect r = frame();

    p.drawMaterial(r, 12.0f, Material::Glass);
    p.drawShadow(r, 12.0f, c.shadow.multipliedAlpha(0.6f), 20.0f, {0, 8});

    Rect iconRect{r.x + 18.0f, r.y + 20.0f, 28.0f, 28.0f};
    p.drawIcon(Icon::DocOnDoc, iconRect, c.accent);

    Font titleFont = t.font(TextStyle::Headline);
    titleFont.weight = FontWeight::Semibold;
    Rect titleRect{r.x + 60.0f, r.y + 18.0f, r.width - 90.0f, 20.0f};
    p.drawText(m_title, titleRect, titleFont, c.label);

    float barX = r.x + 60.0f;
    float barY = r.y + 48.0f;
    float barW = r.width - 100.0f;
    float barH = 6.0f;
    Rect trackRect{barX, barY, barW, barH};
    p.fillRoundedRect(trackRect, 3.0f, c.control);

    if (m_fraction > 0.001) {
        float fillW = std::max(6.0f, static_cast<float>(barW * m_fraction));
        Rect fillRect{barX, barY, fillW, barH};
        p.fillRoundedRect(fillRect, 3.0f, c.accent);
    }

    Rect cancelRect{barX + barW + 10.0f, barY - 7.0f, 20.0f, 20.0f};
    if (m_hoverCancel) p.fillCircle(cancelRect.center(), 9.5f, c.hover);
    p.strokeCircle(cancelRect.center(), 9.0f, c.secondaryLabel.multipliedAlpha(0.4f), 1.0f);
    p.drawIcon(Icon::Xmark, cancelRect.inset(4.5f), c.secondaryLabel, 1.2f);

    Font detailFont = t.font(TextStyle::Subheadline);
    Rect detailRect{barX, barY + 16.0f, barW, 20.0f};
    p.drawText(m_detail.empty() ? m_item : m_detail, detailRect, detailFont, c.secondaryLabel);
}

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
            });
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
                for (int page = 1; page <= 40; ++page) {
                    char buf[64];
                    std::snprintf(buf, sizeof(buf), "%s/p-%d.png", tempDir.c_str(), page);
                    std::string p1 = buf;
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
            });
        }
    } else if (isTextLike(e) || e.size < 256 * 1024) {
        m_isText = true;
        std::ifstream file(e.path);
        std::string line;
        int count = 0;
        constexpr int kMaxLines = 300;
        while (count < kMaxLines && std::getline(file, line)) {
            if (!line.empty() && line.back() == '\r') line.pop_back();
            std::string expanded;
            for (char ch : line) {
                if (ch == '\t') expanded += "    ";
                else expanded.push_back(ch);
            }
            m_lines.push_back(std::move(expanded));
            ++count;
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
    p.drawText(tr("Open"), {open.x + 30, open.y, open.width - 30, open.height}, t.font(TextStyle::Body), c.label);
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

        // Floating Liquid Glass video HUD controls (QuickTime Player design)
        const float hudW = 400.0f;
        const float hudH = 46.0f;
        const float hudR = 23.0f;
        Rect hudRect{b.center().x - hudW * 0.5f, b.bottom() - 56.0f, hudW, hudH};
        p.drawShadow(hudRect, hudR, Color::black(0.45f), 24.0f, {0, 8});
        MaterialStyle ms = t.material(Material::Popover);
        ms.tint = Color::rgb8(28, 28, 32, 0.70f);
        ms.highlight = Color::white(0.20f);
        ms.border = Color::white(0.12f);
        p.drawMaterial(hudRect, hudR, ms);

        double pos = m_player ? m_player->position() : 0;
        double dur = m_player ? m_player->duration() : 0;

        // Skip back (-10s) button
        Rect backBtn{hudRect.x + 14.0f, hudRect.center().y - 12.0f, 24.0f, 24.0f};
        p.drawIcon(Icon::ChevronLeft, backBtn.inset(3.0f), Color::white(0.9f), 1.5f);

        // Play / Pause button
        Icon playIcon = m_videoPaused ? Icon::Play : Icon::Pause;
        Rect playBtn{hudRect.x + 44.0f, hudRect.center().y - 14.0f, 28.0f, 28.0f};
        p.drawIcon(playIcon, playBtn.inset(4.0f), Color::white(), 1.4f);

        // Skip forward (+10s) button
        Rect fwdBtn{hudRect.x + 78.0f, hudRect.center().y - 12.0f, 24.0f, 24.0f};
        p.drawIcon(Icon::ChevronRight, fwdBtn.inset(3.0f), Color::white(0.9f), 1.5f);

        // Elapsed time
        int pMin = static_cast<int>(pos) / 60, pSec = static_cast<int>(pos) % 60;
        char elapsedBuf[16];
        std::snprintf(elapsedBuf, sizeof(elapsedBuf), "%d:%02d", pMin, pSec);
        Rect elapsedRect{hudRect.x + 108.0f, hudRect.center().y - 8.0f, 40.0f, 16.0f};
        p.drawText(elapsedBuf, elapsedRect, t.font(TextStyle::Caption), Color::white(0.85f), TextAlign::Center);

        // TrackBar timeline
        Rect track{hudRect.x + 152.0f, hudRect.center().y - 2.5f, 180.0f, 5.0f};
        p.fillRoundedRect(track, 2.5f, Color::white(0.22f));
        if (dur > 0) {
            float progress = static_cast<float>(pos / dur);
            Rect filled{track.x, track.y, track.width * std::clamp(progress, 0.0f, 1.0f), track.height};
            p.fillRoundedRect(filled, 2.5f, c.accent);
            Point knob{filled.right(), track.center().y};
            p.fillCircle(knob, 5.5f, Color::white());
        }

        // Remaining time
        int remSec = std::max(0, static_cast<int>(dur - pos));
        int rMin = remSec / 60, rSec = remSec % 60;
        char remBuf[16];
        std::snprintf(remBuf, sizeof(remBuf), "-%d:%02d", rMin, rSec);
        Rect remRect{hudRect.right() - 56.0f, hudRect.center().y - 8.0f, 44.0f, 16.0f};
        p.drawText(remBuf, remRect, t.font(TextStyle::Caption), Color::white(0.85f), TextAlign::Leading);

        if (m_player && m_player->isPlaying()) {
            requestAnimationFrame();
        }
    } else if (m_isPdf) {
        if (m_physics.isAnimating()) {
            if (m_physics.step(now())) requestAnimationFrame();
            else if (host() && m_physics.deadline() >= 0) host()->scheduleFrame(m_physics.deadline());
            m_scroll = m_physics.offset().y;
        }
        p.fillRect(b, c.contentBackground);

        if (!m_pdfPages.empty()) {
            float availableW = b.width - 40.0f;
            float y = b.y + 20.0f - m_scroll;
            int pageIdx = 0;
            int visiblePage = 1;
            for (const auto& pageImg : m_pdfPages) {
                ++pageIdx;
                if (!pageImg) continue;
                float pw = static_cast<float>(pageImg->width());
                float ph = static_cast<float>(pageImg->height());
                if (pw <= 0 || ph <= 0) continue;
                float scale = std::min(availableW / pw, 1.0f);
                float renderW = pw * scale;
                float renderH = ph * scale;
                Rect pageRect{b.center().x - renderW * 0.5f, y, renderW, renderH};
                if (pageRect.bottom() >= b.y && pageRect.y <= b.bottom()) {
                    p.drawShadow(pageRect, 4.0f, Color::black(0.12f), 8.0f, {0, 2});
                    p.fillRoundedRect(pageRect, 4.0f, Color::white());
                    p.drawImage(pageRect, *pageImg);
                    visiblePage = pageIdx;
                }
                y += renderH + 20.0f;
            }

            // Floating page indicator pill
            char pagePill[32];
            std::snprintf(pagePill, sizeof(pagePill), "%d / %d", visiblePage, static_cast<int>(m_pdfPages.size()));
            Font pillFont = t.font(TextStyle::Caption);
            pillFont.weight = FontWeight::Medium;
            float pillW = p.measureText(pagePill, pillFont) + 20.0f;
            Rect pillRect{b.center().x - pillW * 0.5f, b.bottom() - 32.0f, pillW, 22.0f};
            p.fillRoundedRect(pillRect, 11.0f, Color::black(0.6f));
            p.drawText(pagePill, pillRect, pillFont, Color::white(), TextAlign::Center);
        } else if (m_image) {
            const float iw = static_cast<float>(m_image->originalWidth());
            const float ih = static_cast<float>(m_image->originalHeight());
            const float k = std::min({(b.width - 40) / iw, (b.height - 40) / ih, 1.0f});
            Rect pic{b.center().x - iw * k * 0.5f, b.center().y - ih * k * 0.5f, iw * k, ih * k};
            p.drawShadow(pic, 4.0f, Color::black(0.12f), 8.0f, {0, 2});
            p.fillRoundedRect(pic, 4.0f, Color::white());
            p.drawImage(pic, *m_image);
        }
    } else if (m_image && m_image->width() > 0) {
        const float iw = static_cast<float>(m_image->originalWidth());
        const float ih = static_cast<float>(m_image->originalHeight());
        const float k = std::min(b.width / iw, b.height / ih);
        Rect pic{b.center().x - iw * k * 0.5f, b.center().y - ih * k * 0.5f, iw * k, ih * k};
        p.drawImage(pic, *m_image);
    } else if (m_isText) {
        if (m_physics.isAnimating()) {
            if (m_physics.step(now())) requestAnimationFrame();
            else if (host() && m_physics.deadline() >= 0) host()->scheduleFrame(m_physics.deadline());
            m_scroll = m_physics.offset().y;
        }
        const Font f = t.font(TextStyle::Callout);
        const float pad = 16;
        float y = b.y + pad - m_scroll;
        for (const auto& line : m_lines) {
            if (y + kTextLine >= b.y && y <= b.bottom()) {
                p.drawText(line, {b.x + pad, y, b.width - pad * 2, kTextLine}, f, c.label);
            }
            y += kTextLine;
        }
    } else {
        float y = b.center().y - 60;
        Rect ic{b.center().x - 32, y, 64, 64};
        if (m_entry.isDir) drawFolderIcon(p, ic);
        else p.drawIcon(Icon::Document, ic, c.secondaryLabel);
        y += 74;
        const Font sub = t.font(TextStyle::Headline);
        p.drawText(m_entry.name, {b.x + 20, y, b.width - 40, 22}, sub, c.label, TextAlign::Center);
        y += 24;
        std::string detail;
        if (m_entry.isDir) {
            detail = std::to_string(m_childCount) + " " + tr("items");
        } else {
            detail = formatSize(m_entry.size);
            detail += " · " + kindDescription(m_entry);
        }
        const Font small = t.font(TextStyle::Subheadline);
        p.drawText(detail, {b.x + 20, y, b.width - 40, 18}, small, c.secondaryLabel, TextAlign::Center);
    }
    p.popClip();
}

bool QuickLookView::onKeyDown(const KeyEvent& e) {
    if (e.key == Key::Space || e.key == Key::Escape) {
        dismiss();
        return true;
    }
    if (m_isVideo && e.key == Key::Space) {
        m_videoPaused = !m_videoPaused;
        if (m_player) {
            if (m_videoPaused) m_player->pause();
            else m_player->play();
        }
        invalidate();
        return true;
    }
    if (e.key == Key::Left || e.key == Key::Right || e.key == Key::Up || e.key == Key::Down) {
        if (onArrow) onArrow(e.key);
        return true;
    }
    if (e.key == Key::Enter) {
        if (onOpen) onOpen(m_entry);
        return true;
    }
    return false;
}

bool QuickLookView::onPointerDown(const PointerEvent& e) {
    if (e.button != MouseButton::Left) return false;
    if (closeRect().contains(e.position)) {
        dismiss();
        return true;
    }
    if (openRect().contains(e.position)) {
        if (onOpen) onOpen(m_entry);
        return true;
    }
    if (m_isVideo) {
        const float hudW = 400.0f;
        const float hudH = 46.0f;
        Rect hudRect{body().center().x - hudW * 0.5f, body().bottom() - 56.0f, hudW, hudH};
        if (hudRect.contains(e.position)) {
            Rect backBtn{hudRect.x + 14.0f, hudRect.center().y - 12.0f, 24.0f, 24.0f};
            Rect playBtn{hudRect.x + 44.0f, hudRect.center().y - 14.0f, 28.0f, 28.0f};
            Rect fwdBtn{hudRect.x + 78.0f, hudRect.center().y - 12.0f, 24.0f, 24.0f};
            Rect track{hudRect.x + 152.0f, hudRect.center().y - 10.0f, 180.0f, 20.0f};

            if (playBtn.contains(e.position)) {
                m_videoPaused = !m_videoPaused;
                if (m_player) {
                    if (m_videoPaused) m_player->pause();
                    else m_player->play();
                }
            } else if (backBtn.contains(e.position)) {
                if (m_player) m_player->seek(std::max(0.0, m_player->position() - 10.0));
            } else if (fwdBtn.contains(e.position)) {
                if (m_player) m_player->seek(std::min(m_player->duration(), m_player->position() + 10.0));
            } else if (track.contains(e.position)) {
                if (m_player && m_player->duration() > 0) {
                    float frac = std::clamp((e.position.x - track.x) / track.width, 0.0f, 1.0f);
                    m_player->seek(frac * m_player->duration());
                }
            }
            invalidate();
            return true;
        }
    }
    return true;
}

void QuickLookView::onPointerMove(const PointerEvent& e) {
    int next = closeRect().contains(e.position) ? 0 : openRect().contains(e.position) ? 1 : -1;
    if (next != m_hover) {
        m_hover = next;
        invalidate();
    }
}

bool QuickLookView::onScroll(const ScrollEvent& e) {
    if (m_isPdf) {
        float totalH = 0;
        float availableW = body().width - 40.0f;
        for (const auto& pageImg : m_pdfPages) {
            if (!pageImg) continue;
            float pw = static_cast<float>(pageImg->originalWidth());
            float ph = static_cast<float>(pageImg->originalHeight());
            if (pw > 0 && ph > 0) {
                float scale = std::min(availableW / pw, 1.0f);
                totalH += ph * scale + 20.0f;
            }
        }
        float maxScroll = std::max(0.0f, totalH - body().height + 40.0f);
        m_physics.setLimits({0, maxScroll}, body().size());
        if (!m_physics.handle(e, now())) return true;
        m_scroll = m_physics.offset().y;
        if (m_physics.isAnimating()) requestAnimationFrame();
        if (host() && m_physics.deadline() >= 0) host()->scheduleFrame(m_physics.deadline());
        invalidate();
        return true;
    }
    if (!m_isText) return false;
    const float contentH = static_cast<float>(m_lines.size()) * kTextLine + 32;
    const float maxScroll = std::max(0.0f, contentH - body().height);
    m_physics.setLimits({0, maxScroll}, body().size());
    if (!m_physics.handle(e, now())) return true;
    m_scroll = m_physics.offset().y;
    if (m_physics.isAnimating()) requestAnimationFrame();
    if (host() && m_physics.deadline() >= 0) host()->scheduleFrame(m_physics.deadline());
    invalidate();
    return true;
}

}  // namespace finder
