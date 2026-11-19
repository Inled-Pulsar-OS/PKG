#pragma once

// Floating panels: Quick Look (Space), alerts (confirmations, errors), and file operations progress.

#include <mayonnaise/mayonnaise.hpp>

#include <functional>
#include <memory>
#include <string>
#include <vector>

#include "browser.hpp"
#include "../video_player/media_player.hpp"

namespace finder {

/// A popup centred in the window instead of anchored to a widget.
class CenteredPanel : public mayo::PopupView {
public:
    using PopupView::PopupView;
    void present(mayo::WidgetHost& host);
    mayo::Rect placePopup(mayo::Size window, mayo::Size measured) override;
};

struct AlertButton {
    std::string title;
    std::function<void()> action;
    mayo::ButtonStyle style = mayo::ButtonStyle::Default;
};

/// macOS-style alert: icon, bold title, message and a row of buttons. Escape picks the first button,
/// Enter the last one (the default action).
class AlertView : public CenteredPanel {
public:
    AlertView(std::string title, std::string message, std::vector<AlertButton> buttons, bool caution = false);
    bool onKeyDown(const mayo::KeyEvent& e) override;
    void show(mayo::WidgetHost& host);

protected:
    void drawPanel(mayo::Painter& p) override;

private:
    void choose(size_t index);
    std::vector<AlertButton> m_buttons;
};

/// Apple-style floating operation progress sheet (copy, move, compress, extract).
class OperationProgressPanel : public CenteredPanel {
public:
    explicit OperationProgressPanel(std::function<void()> onCancel);
    void setOperation(std::string title, std::string item, double fraction, std::string detail);
    mayo::Size measureContent(mayo::Size available) override;
    [[nodiscard]] bool acceptsFocus() const override { return true; }
    [[nodiscard]] bool isInteractive() const override { return true; }
    bool onPointerDown(const mayo::PointerEvent& e) override;
    void onPointerMove(const mayo::PointerEvent& e) override;
    void cancel();

protected:
    void drawPanel(mayo::Painter& p) override;

private:
    std::string m_title;
    std::string m_item;
    double m_fraction = 0.0;
    std::string m_detail;
    std::function<void()> m_onCancel;
    bool m_hoverCancel = false;
};

class QuickLookView : public CenteredPanel {
public:
    explicit QuickLookView(std::shared_ptr<Browser> browser);
    ~QuickLookView() override;

    std::function<void(mayo::Key)> onArrow;
    std::function<void(const FileEntry&)> onOpen;
    void setEntry(const FileEntry& e);
    [[nodiscard]] const FileEntry& entry() const { return m_entry; }

    mayo::Size measureContent(mayo::Size available) override;
    [[nodiscard]] bool acceptsFocus() const override { return true; }
    [[nodiscard]] bool isInteractive() const override { return true; }
    bool onKeyDown(const mayo::KeyEvent& e) override;
    bool onPointerDown(const mayo::PointerEvent& e) override;
    void onPointerMove(const mayo::PointerEvent& e) override;
    bool onScroll(const mayo::ScrollEvent& e) override;

protected:
    void willDismiss() override;
    void drawPanel(mayo::Painter& p) override;

private:
    [[nodiscard]] mayo::Rect closeRect() const;
    [[nodiscard]] mayo::Rect openRect() const;
    [[nodiscard]] mayo::Rect body() const;

    std::shared_ptr<Browser> m_browser;
    FileEntry m_entry;
    std::shared_ptr<mayo::ImageData> m_image;
    std::vector<std::string> m_lines;  // text preview
    bool m_isText = false;
    size_t m_childCount = 0;
    float m_scroll = 0;  // follows m_physics
    mayo::ScrollPhysics m_physics;
    int m_hover = -1;  // 0 close, 1 open

    bool m_isPdf = false;
    std::vector<std::shared_ptr<mayo::ImageData>> m_pdfPages;
    std::string m_pdfTempDir;

    bool m_isVideo = false;
    bool m_videoPaused = false;
    std::unique_ptr<MediaPlayer> m_player;
    std::shared_ptr<mayo::VideoTexture> m_videoTexture;
    MediaPlayer::Frame m_shownFrame;
};

}  // namespace finder
