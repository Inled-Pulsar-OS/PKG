#pragma once

// The Finder's content views: icon grid, list (with disclosure triangles), column browser with a preview
// pane, and the path bar.

#include <mayonnaise/mayonnaise.hpp>

#include <array>
#include <functional>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <unordered_map>
#include <vector>

#include "browser.hpp"

namespace finder {

// --- Icons -------------------------------------------------------------------------------------------

class IconGridView : public FileView {
public:
    using FileView::FileView;
    mayo::Size measureContent(mayo::Size available) override;
    void draw(mayo::Painter& p) override;
    void reload() override;
    bool onScroll(const mayo::ScrollEvent& e) override;
    bool onMagnify(const mayo::MagnifyEvent& e) override;

protected:
    [[nodiscard]] std::vector<const FileEntry*> order() const override;
    [[nodiscard]] std::string itemAt(mayo::Point p) const override;
    [[nodiscard]] mayo::Rect itemRect(const std::string& path) const override;
    [[nodiscard]] mayo::Rect nameRect(const std::string& path) const override;
    [[nodiscard]] std::string neighbour(const std::string& from, mayo::Key key) const override;

private:
    struct Label {
        std::string lines[2];
        float widths[2] = {0, 0};
        int count = 0;
    };
    void computeLayout(float width);
    [[nodiscard]] int indexOf(const std::string& path) const;
    [[nodiscard]] mayo::Rect cellRect(int i) const;
    [[nodiscard]] mayo::Rect iconRect(int i) const;
    const Label& label(mayo::Painter& p, const FileEntry& e);

    int m_cols = 1;
    float m_cellW = 100;
    float m_cellH = 110;
    float m_labelWidth = 90;
    std::unordered_map<std::string, int> m_index;
    std::unordered_map<std::string, Label> m_labels;  // wrapped names for m_labelWidth
    std::unordered_map<std::string, float> m_labelExtents;  // first line width, for hit testing
};

// --- List --------------------------------------------------------------------------------------------

struct ListColumns {
    float modified = 168;
    float size = 84;
    float kind = 150;
    struct Span {
        float x;
        float width;
    };
    /// Name, Date Modified, Size, Kind spans for a table `width` wide (name takes the rest).
    [[nodiscard]] std::array<Span, 4> layout(float width) const;
};

class ListHeaderView : public mayo::Widget {
public:
    ListHeaderView(std::shared_ptr<Browser> browser, std::shared_ptr<ListColumns> columns)
        : m_browser(std::move(browser)), m_columns(std::move(columns)) {}
    std::function<void()> onColumnsResized;

    mayo::Size measureContent(mayo::Size available) override { return {available.width, 28}; }
    [[nodiscard]] bool expandsContent(mayo::Axis a) const override { return a == mayo::Axis::Horizontal; }
    [[nodiscard]] bool isInteractive() const override { return true; }
    void draw(mayo::Painter& p) override;
    bool onPointerDown(const mayo::PointerEvent& e) override;
    void onPointerMove(const mayo::PointerEvent& e) override;
    void onPointerUp(const mayo::PointerEvent& e) override;
    void onPointerLeave() override;

private:
    [[nodiscard]] int columnAt(float x) const;
    [[nodiscard]] int edgeAt(float x) const;

    std::shared_ptr<Browser> m_browser;
    std::shared_ptr<ListColumns> m_columns;
    int m_hover = -1;
    int m_pressed = -1;
    int m_resizing = -1;
    float m_resizeX = 0;
    float m_resizeWidth = 0;
};

class ListRowsView : public FileView {
public:
    static constexpr float kRowHeight = 24;
    ListRowsView(std::shared_ptr<Browser> browser, std::shared_ptr<ListColumns> columns)
        : FileView(std::move(browser)), m_columns(std::move(columns)) {}

    mayo::Size measureContent(mayo::Size available) override;
    void draw(mayo::Painter& p) override;
    void reload() override;
    bool onPointerDown(const mayo::PointerEvent& e) override;
    /// Expanded folders are forgotten when the folder changes.
    void collapseAll() { m_expanded.clear(); }

protected:
    [[nodiscard]] std::vector<const FileEntry*> order() const override;
    [[nodiscard]] std::string itemAt(mayo::Point p) const override;
    [[nodiscard]] const FileEntry* entry(const std::string& path) const override;
    [[nodiscard]] mayo::Rect itemRect(const std::string& path) const override;
    [[nodiscard]] mayo::Rect nameRect(const std::string& path) const override;
    bool handleKey(const mayo::KeyEvent& e) override;

private:
    struct Row {
        FileEntry entry;
        int depth = 0;
    };
    void rebuild();
    void toggle(const std::string& path);
    [[nodiscard]] int rowIndex(const std::string& path) const;
    [[nodiscard]] mayo::Rect rowRect(int i) const;
    [[nodiscard]] mayo::Rect disclosureRect(int i) const;

    std::shared_ptr<ListColumns> m_columns;
    std::vector<Row> m_rows;
    std::unordered_map<std::string, int> m_index;
    std::set<std::string> m_expanded;
};

// --- Columns -----------------------------------------------------------------------------------------

/// One folder of the column browser.
class ColumnListView : public FileView {
public:
    static constexpr float kRowHeight = 24;
    ColumnListView(std::shared_ptr<Browser> browser, std::string folder)
        : FileView(std::move(browser)), m_folder(std::move(folder)) {}

    void setEntries(std::vector<FileEntry> entries, std::string chainChild);
    [[nodiscard]] const std::string& folderPath() const { return m_folder; }
    [[nodiscard]] bool isActive() const;

    mayo::Size measureContent(mayo::Size available) override;
    [[nodiscard]] bool expandsContent(mayo::Axis /*axis*/) const override { return true; }
    void draw(mayo::Painter& p) override;

protected:
    [[nodiscard]] std::vector<const FileEntry*> order() const override;
    [[nodiscard]] std::string itemAt(mayo::Point p) const override;
    [[nodiscard]] const FileEntry* entry(const std::string& path) const override;
    [[nodiscard]] mayo::Rect itemRect(const std::string& path) const override;
    [[nodiscard]] mayo::Rect nameRect(const std::string& path) const override;
    [[nodiscard]] std::string folder() const override { return m_folder; }
    [[nodiscard]] bool ownsSelection() const override { return isActive(); }
    void activate(const std::string& path) override;
    bool handleKey(const mayo::KeyEvent& e) override;

private:
    [[nodiscard]] int indexOf(const std::string& path) const;
    std::string m_folder;
    std::vector<FileEntry> m_entries;
    std::string m_chainChild;  // selected in an ancestor column
};

/// Last column: large preview and details of the selected file.
class PreviewPaneView : public mayo::Widget {
public:
    explicit PreviewPaneView(std::shared_ptr<Browser> browser) : m_browser(std::move(browser)) {}
    void setEntry(const FileEntry& e);
    [[nodiscard]] const std::string& path() const { return m_entry.path; }
    mayo::Size measureContent(mayo::Size available) override { return {available.width, available.height}; }
    void draw(mayo::Painter& p) override;

private:
    std::shared_ptr<Browser> m_browser;
    FileEntry m_entry;
    std::shared_ptr<mayo::ImageData> m_large;
    std::string m_largeFor;
};

class ColumnBrowserView : public mayo::Widget {
public:
    static constexpr float kColumnWidth = 236;
    static constexpr float kPreviewWidth = 300;
    explicit ColumnBrowserView(std::shared_ptr<Browser> browser) : m_browser(std::move(browser)) {}

    /// Folders the column chain starts at (the deepest one containing the current folder wins).
    std::vector<std::string> roots;
    void setScroller(mayo::ScrollViewView* s) { m_scroller = s; }
    /// Rebuilds the columns for the browser's folder and selection.
    void rebuild(bool focusActive);
    /// The column showing the current folder (keyboard focus goes there).
    [[nodiscard]] ColumnListView* activeColumn() const;
    void beginRename(const std::string& path);

    mayo::Size measureContent(mayo::Size available) override;
    void arrangeContent(const mayo::Rect& content) override;
    void draw(mayo::Painter& p) override;

private:
    struct Column {
        std::shared_ptr<ColumnListView> list;
        std::shared_ptr<mayo::ScrollViewView> scroll;
    };
    Column columnFor(const std::string& folder);

    std::shared_ptr<Browser> m_browser;
    std::vector<Column> m_columns;
    std::map<std::string, Column> m_cache;  // reuse widgets (and scroll offsets) across rebuilds
    std::shared_ptr<PreviewPaneView> m_preview;
    bool m_showPreview = false;
    mayo::ScrollViewView* m_scroller = nullptr;
};

// --- Path bar ----------------------------------------------------------------------------------------

class PathBarView : public mayo::Widget, public DropTarget {
public:
    explicit PathBarView(std::shared_ptr<Browser> browser) : m_browser(std::move(browser)) {}
    mayo::Size measureContent(mayo::Size available) override { return {available.width, 28}; }
    [[nodiscard]] bool expandsContent(mayo::Axis a) const override { return a == mayo::Axis::Horizontal; }
    [[nodiscard]] bool isInteractive() const override { return true; }
    void draw(mayo::Painter& p) override;
    bool onPointerDown(const mayo::PointerEvent& e) override;
    void onPointerMove(const mayo::PointerEvent& e) override;
    void onPointerLeave() override;
    std::string dropFolderAt(mayo::Point p) override;

private:
    struct Segment {
        std::string path;
        std::string title;
        mayo::Rect rect;
        bool isDir = true;
    };
    std::vector<Segment> segments(mayo::Painter* p) const;
    [[nodiscard]] int segmentAt(mayo::Point p) const;

    std::shared_ptr<Browser> m_browser;
    std::vector<Segment> m_laidOut;
    int m_hover = -1;
};

}  // namespace finder
