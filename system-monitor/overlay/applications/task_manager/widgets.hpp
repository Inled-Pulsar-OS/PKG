#pragma once

// Custom widgets for the task manager: history graphs, per-core bars, usage bars and the process table.

#include <mayonnaise/mayonnaise.hpp>

#include <deque>
#include <functional>
#include <memory>
#include <string>
#include <vector>

#include "system_stats.hpp"

/// Rolling history graph. Stacked: series pile up from the bottom (CPU system + user). Mirrored:
/// series 0 grows up from the middle and series 1 down (reads/writes, in/out). Single: one area.
class HistoryGraph : public mayo::Widget {
public:
    enum class Mode { Stacked, Mirrored };
    explicit HistoryGraph(Mode mode, int capacity = 90);

    void setColors(std::vector<mayo::Color> colors) { m_colors = std::move(colors); }
    /// Called per frame so colours can follow the theme (and memory pressure).
    void setColorProvider(std::function<std::vector<mayo::Color>()> fn) { m_colorProvider = std::move(fn); }
    /// 0 = automatic scale (rounded up to a "nice" value).
    void setFixedMax(double max) { m_fixedMax = max; }
    void setMinimumMax(double min) { m_minMax = min; }
    void setFormatter(std::function<std::string(double)> fn) { m_format = std::move(fn); }
    void push(std::vector<double> values);

    mayo::Size measureContent(mayo::Size available) override;
    [[nodiscard]] bool expandsContent(mayo::Axis /*axis*/) const override { return true; }
    void draw(mayo::Painter& p) override;

private:
    [[nodiscard]] double scaleMax() const;

    Mode m_mode;
    int m_capacity;
    std::deque<std::vector<double>> m_samples;
    std::vector<mayo::Color> m_colors;
    std::function<std::vector<mayo::Color>()> m_colorProvider;
    std::function<std::string(double)> m_format;
    double m_fixedMax = 0;
    double m_minMax = 1;
};

/// One thin vertical bar per logical core (user + system).
class CoreBars : public mayo::Widget {
public:
    void setCores(std::vector<CoreUsage> cores) {
        m_cores = std::move(cores);
        invalidate();
    }
    mayo::Size measureContent(mayo::Size available) override;
    [[nodiscard]] bool expandsContent(mayo::Axis /*axis*/) const override { return true; }
    void draw(mayo::Painter& p) override;

private:
    std::vector<CoreUsage> m_cores;
};

/// Horizontal stacked capacity bar (memory breakdown, disk usage).
class UsageBar : public mayo::Widget {
public:
    struct Segment {
        double fraction;
        mayo::Color color;
    };
    void setSegments(std::vector<Segment> s) {
        m_segments = std::move(s);
        invalidate();
    }
    mayo::Size measureContent(mayo::Size available) override;
    [[nodiscard]] bool expandsContent(mayo::Axis a) const override { return a == mayo::Axis::Horizontal; }
    void draw(mayo::Painter& p) override;

private:
    std::vector<Segment> m_segments;
};

// --- process table -------------------------------------------------------------------------------

struct Column {
    std::string title;
    float width = 90;    // the first column (name) takes the remaining space
    bool numeric = true;  // right-aligned
    std::function<std::string(const ProcessInfo&)> text;
    std::function<double(const ProcessInfo&)> key;  // sort key; empty = sort by text
};

/// Shared state between the sticky header and the rows.
struct TableModel {
    std::vector<Column> columns;
    int sortColumn = 1;
    bool descending = true;
    std::shared_ptr<SystemSnapshot> snapshot;  // keeps `rows` valid
    std::vector<const ProcessInfo*> rows;
    pid_t selected = -1;
    std::uint64_t memTotal = 1;

    /// x offset and width of every column for a table `width` wide.
    [[nodiscard]] std::vector<std::pair<float, float>> layout(float width) const;
    void sort();
};

class TableHeader : public mayo::Widget {
public:
    explicit TableHeader(std::shared_ptr<TableModel> model) : m_model(std::move(model)) {}
    std::function<void()> onSortChanged;
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
    [[nodiscard]] int resizeHandleAt(float x) const;

    std::shared_ptr<TableModel> m_model;
    int m_hover = -1;
    int m_pressed = -1;
    int m_resizing = -1;
    float m_resizeStartX = 0;
    float m_resizeStartWidth = 0;
};

class TableRows : public mayo::Widget {
public:
    static constexpr float kRowHeight = 24;
    explicit TableRows(std::shared_ptr<TableModel> model);

    std::function<void(pid_t)> onSelect;
    std::function<void(pid_t)> onOpen;          // double click / Enter
    std::function<void(pid_t, bool)> onQuit;    // pid, force
    void setScroller(mayo::ScrollViewView* s) { m_scroller = s; }
    void rowsChanged();
    void select(pid_t pid, bool scroll);

    mayo::Size measureContent(mayo::Size available) override;
    [[nodiscard]] bool acceptsFocus() const override { return true; }
    [[nodiscard]] bool isInteractive() const override { return true; }
    void draw(mayo::Painter& p) override;
    bool onPointerDown(const mayo::PointerEvent& e) override;
    void onPointerMove(const mayo::PointerEvent& e) override;
    void onPointerLeave() override;
    bool onKeyDown(const mayo::KeyEvent& e) override;
    void onFocusChanged(bool) override { invalidate(); }

private:
    [[nodiscard]] int rowAt(float y) const;
    [[nodiscard]] int selectedIndex() const;
    void ensureVisible(int index);
    void showContextMenu(mayo::Point at, pid_t pid);

    std::shared_ptr<TableModel> m_model;
    mayo::ScrollViewView* m_scroller = nullptr;
    int m_hover = -1;
    std::shared_ptr<mayo::Widget> m_menuAnchor;
    std::shared_ptr<mayo::MenuView> m_menu;
};

/// Small coloured circle with the process' initial (Linux has no reliable per-process icons).
void drawProcessIcon(mayo::Painter& p, const mayo::Rect& r, const ProcessInfo& info);
