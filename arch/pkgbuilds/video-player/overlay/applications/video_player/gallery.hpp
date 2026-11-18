// The library gallery: titled sections of video cards (thumbnail, duration, resume progress) that fill the
// width, with hover, keyboard selection and a context menu hook.
#pragma once

#include <mayonnaise/mayonnaise.hpp>

#include <functional>
#include <string>
#include <vector>

#include "library.hpp"

std::string formatTime(double seconds);

struct GallerySection {
    std::string title;
    std::vector<VideoPtr> items;
};

class VideoGridView : public mayo::Widget {
public:
    std::function<void(const VideoPtr&)> onOpen;
    std::function<void(const VideoPtr&, mayo::Point)> onContextMenu;
    /// Saved position as a fraction of the duration, or a negative value when there is none.
    std::function<double(const VideoPtr&)> progress;

    void setScroller(mayo::ScrollViewView* s) { m_scroller = s; }
    void setSections(std::vector<GallerySection> sections);
    [[nodiscard]] bool isEmpty() const { return m_cards.empty(); }

    [[nodiscard]] bool acceptsFocus() const override { return true; }
    [[nodiscard]] bool isInteractive() const override { return true; }
    [[nodiscard]] bool expandsContent(mayo::Axis axis) const override { return axis == mayo::Axis::Horizontal; }
    mayo::Size measureContent(mayo::Size available) override;
    void arrangeContent(const mayo::Rect& content) override;
    void draw(mayo::Painter& p) override;
    bool onPointerDown(const mayo::PointerEvent& e) override;
    void onPointerMove(const mayo::PointerEvent& e) override;
    void onPointerLeave() override;
    bool onKeyDown(const mayo::KeyEvent& e) override;

private:
    struct Card {
        VideoPtr item;
        mayo::Rect rect;  // relative to the content origin
    };
    struct Header {
        std::string title;
        float y;
    };
    float layout(float width);
    [[nodiscard]] int cardAt(mayo::Point p) const;
    [[nodiscard]] mayo::Rect cardRect(int i) const;
    void drawCard(mayo::Painter& p, int i);
    void select(int i);
    void ensureVisible(int i);

    std::vector<GallerySection> m_sections;
    std::vector<Card> m_cards;
    std::vector<Header> m_headers;
    float m_layoutWidth = -1;
    float m_height = 0;
    float m_thumbHeight = 0;
    int m_hover = -1;
    int m_selected = -1;
    mayo::ScrollViewView* m_scroller = nullptr;
};

/// Library content for the split view's detail: the scrolling grid, or a message when it's empty.
class LibraryPage : public mayo::Widget {
public:
    std::shared_ptr<mayo::ScrollViewView> scroll;
    std::shared_ptr<mayo::Widget> empty;

    [[nodiscard]] bool extendsIntoTitleBar() const override { return true; }
    [[nodiscard]] bool expandsContent(mayo::Axis /*axis*/) const override { return true; }
    void arrangeContent(const mayo::Rect& c) override;
};
