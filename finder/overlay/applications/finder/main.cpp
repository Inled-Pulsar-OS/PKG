// Finder — a file manager modelled on the macOS Finder, built on Mayonnaise.
//
//   finder [folder]
//
// Icons / List / Columns views (Ctrl+1/2/3), Quick Look (Space), Get Info (Ctrl+I), inline rename
// (Enter), drag and drop (Ctrl = copy), copy / cut / paste, Move to Trash with Put Back and Undo
// (Ctrl+Z), recursive search, a path bar and a live-updating folder (inotify). Linux only.
#include <mayonnaise/mayonnaise.hpp>

#include <algorithm>
#include <cmath>
#include <sys/stat.h>

#include <cerrno>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <map>
#include <set>
#include <sstream>
#include <string>
#include <vector>

#include "browser.hpp"
#include "file_icons.hpp"
#include "applications.hpp"
#include "archives.hpp"
#include "file_model.hpp"
#include "file_ops.hpp"
#include "file_clipboard.hpp"
#include "file_panels.hpp"
#include "panels.hpp"
#include "preview_cache.hpp"
#include "views.hpp"
#include "volumes.hpp"
#include "tag_manager.hpp"
#include <fstream>
#include <sstream>
#include <iostream>
#include <mayonnaise/i18n.hpp>

namespace fs = std::filesystem;
using namespace mayo;
using namespace finder;
using mayo::i18n::tr;

namespace {

constexpr float kPathBarHeight = 28;
constexpr float kStatusBarHeight = 26;

struct HistoryItem {
    std::string path;
    std::string select;
};

struct FinderTab {
    std::string path;
    std::string title;
    ViewMode mode = ViewMode::Icons;
    std::vector<HistoryItem> back;
    std::vector<HistoryItem> forward;
    std::set<std::string> selection;
    std::string lead;
    std::string anchor;
    std::string searchQuery;
    bool searching = false;
};

class TabBarView : public Widget, public DropTarget {
public:
    std::shared_ptr<Browser> browser;
    App* app = nullptr;
    std::function<const std::vector<FinderTab>&()> getTabs;
    std::function<size_t()> getActiveTab;
    std::function<void(size_t index)> onSelectTab;
    std::function<void(size_t index)> onCloseTab;
    std::function<void()> onNewTab;

    [[nodiscard]] bool expandsContent(Axis a) const override { return a == Axis::Horizontal; }
    [[nodiscard]] bool isInteractive() const override { return true; }
    Size measureContent(Size available) override { return {available.width, 30.0f}; }

    std::string dropFolderAt(Point p) override {
        int hoveredTab = -1;
        for (const auto& lt : m_layout) {
            if (lt.rect.contains(p)) {
                hoveredTab = static_cast<int>(lt.index);
                break;
            }
        }

        if (hoveredTab < 0) {
            m_springHoverTab = -1;
            ++m_springGeneration;
            return {};
        }

        if (!getTabs) return {};
        const auto& tabs = getTabs();
        if (hoveredTab >= static_cast<int>(tabs.size())) return {};

        const auto& tab = tabs[static_cast<size_t>(hoveredTab)];

        size_t curActive = getActiveTab ? getActiveTab() : 0;
        if (static_cast<size_t>(hoveredTab) != curActive) {
            if (hoveredTab != m_springHoverTab) {
                m_springHoverTab = hoveredTab;
                const int gen = ++m_springGeneration;
                if (app) {
                    app->after(0.40, [this, gen, hoveredTab] {
                        if (gen == m_springGeneration && m_springHoverTab == hoveredTab) {
                            if (onSelectTab) onSelectTab(static_cast<size_t>(hoveredTab));
                            m_springHoverTab = -1;
                        }
                    });
                }
            }
        } else {
            m_springHoverTab = -1;
            ++m_springGeneration;
        }

        return tab.path;
    }

    void draw(Painter& p) override {
        const Theme& t = theme();
        const auto& c = t.colors;
        const Rect f = frame();
        p.fillRect(f, c.windowBackground);
        const float px = 1.0f / p.scale();
        p.fillRect({f.x, p.snap(f.bottom() - px), f.width, px}, c.separator);

        if (!getTabs) return;
        const auto& tabs = getTabs();
        if (tabs.empty()) return;
        const size_t active = getActiveTab ? getActiveTab() : 0;

        const float leftMargin = 6.0f;
        const float rightReserve = 34.0f;
        const float availableW = std::max(60.0f, f.width - leftMargin - rightReserve);
        const float count = static_cast<float>(tabs.size());
        const float tabW = std::clamp(availableW / count, 80.0f, 210.0f);
        const float tabH = 24.0f;
        const float tabY = f.y + (f.height - tabH) * 0.5f;

        m_layout.clear();
        for (size_t i = 0; i < tabs.size(); ++i) {
            float x = f.x + leftMargin + static_cast<float>(i) * (tabW + 2.0f);
            Rect tabRect{x, tabY, tabW, tabH};
            LayoutTab lt;
            lt.index = i;
            lt.rect = tabRect;
            lt.closeRect = {tabRect.x + 5, tabRect.center().y - 6, 13, 13};
            lt.iconRect = {tabRect.x + 22, tabRect.center().y - 7, 14, 14};
            lt.textRect = {lt.iconRect.right() + 5, tabRect.y, std::max(0.0f, tabRect.right() - lt.iconRect.right() - 6), tabRect.height};
            m_layout.push_back(lt);
        }

        float plusX = f.x + leftMargin + count * (tabW + 2.0f) + 4.0f;
        if (plusX + 22.0f > f.right() - 4.0f) plusX = f.right() - 26.0f;
        m_plusRect = {plusX, tabY + 1, 22, 22};

        const Font font = t.font(TextStyle::Subheadline);
        for (const auto& lt : m_layout) {
            const bool isActive = (lt.index == active);
            const bool isHovered = (static_cast<int>(lt.index) == m_hoverTab);
            const auto& tab = tabs[lt.index];
            const bool isDropHover = (browser && !browser->dropHover.empty() && browser->dropHover == tab.path);

            if (isDropHover) {
                p.fillRoundedRect(lt.rect, 5.0f, c.accent.withAlpha(0.28f));
                p.strokeRoundedRect(lt.rect, 5.0f, c.accent, 2.0f);
            } else if (isActive) {
                p.fillRoundedRect(lt.rect, 5.0f, c.contentBackground);
                p.strokeRoundedRect(lt.rect, 5.0f, c.separator.multipliedAlpha(0.6f), 1.0f);
            } else {
                if (isHovered) {
                    p.fillRoundedRect(lt.rect, 5.0f, c.hover);
                } else {
                    p.fillRoundedRect(lt.rect, 5.0f, c.control.multipliedAlpha(0.25f));
                }
                p.strokeRoundedRect(lt.rect, 5.0f, c.separator.multipliedAlpha(0.35f), 1.0f);
            }

            const bool hoverClose = (static_cast<int>(lt.index) == m_hoverClose);
            if (hoverClose) {
                p.fillCircle(lt.closeRect.center(), 6.5f, c.hover);
            }
            if ((isActive || isHovered || hoverClose) && !isDropHover) {
                p.drawIcon(Icon::Xmark, {lt.closeRect.x + 2, lt.closeRect.y + 2, 9, 9}, hoverClose ? c.label : c.secondaryLabel, 1.5f);
            }

            Icon tabIcon = tab.searching ? Icon::Magnifier : placeIcon(tab.path);
            p.drawIcon(tabIcon, lt.iconRect, (isDropHover || isActive) ? c.accent : c.secondaryLabel);

            std::string title = tab.title.empty() ? displayName(tab.path) : tab.title;
            std::string truncated = truncateEnd(p, title, font, lt.textRect.width);
            p.drawText(truncated, lt.textRect, font, (isDropHover || isActive) ? c.label : c.secondaryLabel);
        }

        if (m_hoverPlus) p.fillRoundedRect(m_plusRect, 4.0f, c.hover);
        p.drawIcon(Icon::Plus, {m_plusRect.center().x - 6, m_plusRect.center().y - 6, 12, 12}, m_hoverPlus ? c.label : c.secondaryLabel, 1.5f);
    }

    bool onPointerDown(const PointerEvent& e) override {
        if (m_plusRect.contains(e.position)) {
            if (e.button == MouseButton::Left && onNewTab) onNewTab();
            return true;
        }
        for (const auto& lt : m_layout) {
            if (lt.rect.contains(e.position)) {
                if (lt.closeRect.contains(e.position) || e.button == MouseButton::Middle) {
                    if (onCloseTab) onCloseTab(lt.index);
                    return true;
                }
                if (e.button == MouseButton::Left) {
                    if (onSelectTab) onSelectTab(lt.index);
                    return true;
                }
            }
        }
        return false;
    }

    void onPointerMove(const PointerEvent& e) override {
        int oldTab = m_hoverTab, oldClose = m_hoverClose;
        bool oldPlus = m_hoverPlus;
        m_hoverTab = -1;
        m_hoverClose = -1;
        m_hoverPlus = m_plusRect.contains(e.position);
        for (const auto& lt : m_layout) {
            if (lt.rect.contains(e.position)) {
                m_hoverTab = static_cast<int>(lt.index);
                if (lt.closeRect.contains(e.position)) m_hoverClose = static_cast<int>(lt.index);
                break;
            }
        }
        if (oldTab != m_hoverTab || oldClose != m_hoverClose || oldPlus != m_hoverPlus) {
            invalidate();
        }
    }

    void onPointerLeave() override {
        m_hoverTab = -1;
        m_hoverClose = -1;
        m_hoverPlus = false;
        m_springHoverTab = -1;
        ++m_springGeneration;
        invalidate();
    }

private:
    struct LayoutTab {
        size_t index = 0;
        Rect rect;
        Rect closeRect;
        Rect iconRect;
        Rect textRect;
    };
    std::vector<LayoutTab> m_layout;
    Rect m_plusRect;
    int m_hoverTab = -1;
    int m_hoverClose = -1;
    bool m_hoverPlus = false;
    int m_springHoverTab = -1;
    int m_springGeneration = 0;
};

/// Detail area. Icons scroll under the toolbar glass; list and columns start below it. Path bar and
/// status bar sit along the bottom.
class FinderPage : public Widget {
public:
    std::shared_ptr<Widget> iconScroll, listHeader, listScroll, columnScroll, empty, bottom, pathBar, statusBar, pickerBar;
    std::shared_ptr<Widget> tabBar;
    std::shared_ptr<Widget> menuAnchor = std::make_shared<Widget>();
    ViewMode mode = ViewMode::Icons;

    [[nodiscard]] bool extendsIntoTitleBar() const override { return true; }
    [[nodiscard]] bool expandsContent(Axis /*axis*/) const override { return true; }

    void arrangeContent(const Rect& c) override {
        const float bar = host() ? host()->titleBarHeight() : 52.0f;
        const float tabH = (tabBar && tabBar->isVisible()) ? 30.0f : 0.0f;
        const float bottomH = bottomHeight();
        const Rect area{c.x, c.y, c.width, c.height - bottomH};
        const Rect below{c.x, c.y + bar + tabH, c.width, area.height - (bar + tabH)};
        auto place = [](const std::shared_ptr<Widget>& w, const Rect& r) {
            w->measure(r.size());
            w->arrange(r);
        };
        static_cast<ScrollViewView*>(iconScroll.get())->setContentInsetTop(bar + tabH);
        place(iconScroll, area);
        if (tabBar && tabBar->isVisible()) {
            place(tabBar, {c.x, c.y + bar, c.width, tabH});
        }
        place(listHeader, {below.x, below.y, below.width, 28});
        place(listScroll, {below.x, below.y + 28, below.width, below.height - 28});
        place(columnScroll, below);
        place(empty, below);
        place(bottom, {c.x, area.bottom(), c.width, bottomH});
    }

    void draw(Painter& p) override {
        const auto& c = theme().colors;
        p.fillRect(frame(), c.contentBackground);
        const Rect b = bottom->frame();
        if (b.height > 0) {
            p.fillRect(b, c.windowBackground);
            const float px = 1.0f / p.scale();
            p.fillRect({b.x, p.snap(b.y), b.width, px}, c.separator);
            if (pathBar->isVisible() && ((pickerBar && pickerBar->isVisible()) || (statusBar && statusBar->isVisible())))
                p.fillRect({b.x, p.snap(b.y + kPathBarHeight), b.width, px}, c.separator.multipliedAlpha(0.6f));
        }
        paintChildren(p);
    }

    [[nodiscard]] float bottomHeight() const {
        float h = 0.0f;
        if (pathBar && pathBar->isVisible()) h += kPathBarHeight;
        if (pickerBar && pickerBar->isVisible()) h += 56.0f;
        else if (statusBar && statusBar->isVisible()) h += kStatusBarHeight;
        return h;
    }
};

/// "Go to Folder" sheet (Ctrl+Shift+G).
class GoToFolderPanel : public CenteredPanel {
public:
    explicit GoToFolderPanel(std::function<void(const std::string&)> go) : CenteredPanel(Material::Popover) {
        setPadding(Insets(18));
        field = TextField(tr("Enter a path, e.g. ~/Documents"));
        field.onSubmit([this, go](const std::string& text) {
            auto keep = shared_from_this();
            dismiss();
            go(text);
        });
        auto cancel = Button(tr("Cancel")).onClick([this] { dismiss(); });
        auto open = Button(tr("Go")).style(ButtonStyle::Prominent).onClick([this, go] {
            auto keep = shared_from_this();
            std::string text = field.value();
            dismiss();
            go(text);
        });
        auto stack = VStack({Label(tr("Go to Folder")).font(TextStyle::Headline), field,
                             HStack({Spacer(), cancel, open}).spacing(8)})
                         .spacing(12);
        stack.width(380);
        addChild(stack.node());
    }
    TextField field;
};

std::string expandUser(std::string path) {
    while (!path.empty() && (path.back() == ' ' || path.back() == '\n')) path.pop_back();
    if (path.rfind("file://", 0) == 0) path = path.substr(7);
    if (!path.empty() && path[0] == '~') path = homeDirectory() + path.substr(1);
    if (path.size() > 1 && path.back() == '/') path.pop_back();
    return path;
}

}  // namespace


struct NautilusScript {
    std::string name;
    std::string path;
    bool isDir = false;
    std::vector<NautilusScript> children;
};

inline std::vector<NautilusScript> loadNautilusScripts(const fs::path& dir = fs::path(homeDirectory()) / ".local/share/nautilus/scripts") {
    std::vector<NautilusScript> scripts;
    std::error_code ec;
    if (!fs::exists(dir, ec) || !fs::is_directory(dir, ec)) return scripts;
    for (const auto& entry : fs::directory_iterator(dir, ec)) {
        if (ec) break;
        std::string filename = entry.path().filename().string();
        if (filename.empty() || filename.front() == '.') continue;
        if (entry.is_directory(ec)) {
            NautilusScript sub;
            sub.name = filename;
            sub.path = entry.path().string();
            sub.isDir = true;
            sub.children = loadNautilusScripts(entry.path());
            if (!sub.children.empty()) scripts.push_back(std::move(sub));
        } else if (entry.is_regular_file(ec)) {
            auto perms = entry.status(ec).permissions();
            if ((perms & (fs::perms::owner_exec | fs::perms::group_exec | fs::perms::others_exec)) != fs::perms::none) {
                scripts.push_back({filename, entry.path().string(), false, {}});
            }
        }
    }
    std::sort(scripts.begin(), scripts.end(), [](const auto& a, const auto& b) {
        if (a.isDir != b.isDir) return a.isDir > b.isDir;
        return a.name < b.name;
    });
    return scripts;
}

inline void executeNautilusScript(const std::string& scriptPath, const std::string& currentFolder, const std::vector<std::string>& selectedPaths) {
    pid_t pid = fork();
    if (pid == 0) {
        setsid();
        std::string selectedPathsStr;
        std::string selectedUrisStr;
        for (const auto& p : selectedPaths) {
            selectedPathsStr += p + "\n";
            selectedUrisStr += "file://" + p + "\n";
        }
        setenv("NAUTILUS_SCRIPT_SELECTED_FILE_PATHS", selectedPathsStr.c_str(), 1);
        setenv("NAUTILUS_SCRIPT_SELECTED_URIS", selectedUrisStr.c_str(), 1);
        std::string currentUri = "file://" + currentFolder;
        setenv("NAUTILUS_SCRIPT_CURRENT_URI", currentUri.c_str(), 1);
        
        chdir(currentFolder.c_str());
        execl("/bin/sh", "sh", "-c", scriptPath.c_str(), (char*)nullptr);
        _exit(127);
    }
}

inline MenuItem buildScriptMenuItem(const NautilusScript& s, const std::string& currentFolder, const std::vector<std::string>& selectedPaths) {
    if (s.isDir) {
        std::vector<MenuItem> subItems;
        for (const auto& child : s.children) {
            subItems.push_back(buildScriptMenuItem(child, currentFolder, selectedPaths));
        }
        return MenuItem(s.name).withSubmenu(std::move(subItems)).withIcon(Icon::Folder);
    } else {
        return MenuItem(s.name, [path = s.path, currentFolder, selectedPaths] {
            executeNautilusScript(path, currentFolder, selectedPaths);
        }).withIcon(Icon::Gear);
    }
}

int main(int argc, char** argv) {
    bool pickerMode = false;
    bool chooseMultiple = false;
    bool chooseFolder = false;
    bool saveMode = false;
    std::string selectItem;
    std::string customTitle;
    std::string saveFilename;
    std::string startFolder;

    for (int i = 1; i < argc; ++i) {
        std::string arg = argv[i];
        if (arg == "--help" || arg == "-h") {
            std::cout << "Usage: finder [OPTIONS] [PATH]\n\n"
                      << "Options:\n"
                      << "  --choose-file, --file-selection   Open file picker dialog to select a single file\n"
                      << "  --choose-files, --multiple        Open file picker dialog to select multiple files\n"
                      << "  --choose-folder, --directory      Open folder picker dialog\n"
                      << "  --save-file, --save               Open save file picker dialog\n"
                      << "  --title <title>                   Set custom dialog title\n"
                      << "  --filename <name>                 Set default filename for save dialog\n"
                      << "  --select <path>                   Open folder and select specified file\n"
                      << "  -h, --help                        Show this help message\n";
            return 0;
        } else if (arg == "--choose-file" || arg == "--file-selection" || arg == "--open-file") {
            pickerMode = true;
        } else if (arg == "--choose-files" || arg == "--multiple") {
            pickerMode = true;
            chooseMultiple = true;
        } else if (arg == "--choose-folder" || arg == "--directory") {
            pickerMode = true;
            chooseFolder = true;
        } else if (arg == "--save-file" || arg == "--save") {
            pickerMode = true;
            saveMode = true;
        } else if (arg == "--title" && i + 1 < argc) {
            customTitle = argv[++i];
        } else if (arg == "--filename" && i + 1 < argc) {
            saveFilename = argv[++i];
        } else if (arg == "--select" && i + 1 < argc) {
            selectItem = expandUser(argv[++i]);
        } else if (!arg.empty() && arg[0] != '-') {
            std::string p = expandUser(arg);
            std::error_code ec;
            if (fs::is_regular_file(p, ec)) {
                selectItem = p;
            } else {
                startFolder = p;
            }
        }
    }

    App app("finder", MAYO_DESKTOP_ID);
    auto& window = app.createWindow(
        {.title = customTitle.empty() ? "Finder" : customTitle.c_str(), .width = 1120, .height = 720, .minWidth = 680, .minHeight = 420, .autosaveName = "main"});
    Settings& prefs = app.settings();
    std::function<void()> syncSettings;  // refreshes the Settings window's controls while it is open
    window.setTitleVisible(false);  // the toolbar shows the folder name after the navigation buttons
    const bool demo = std::getenv("MAYO_DEMO") != nullptr;  // never launch other programs from demo runs
    const bool demoDevices = demo && std::string(std::getenv("MAYO_DEMO")).find("devices") != std::string::npos;
    if (demo) {
        // The MIME detector starts an external tool; demo runs keep that in-process, and a stream of helper
        // processes at startup would make a screenshot of the very first frame show every file as binary.
        classifyInProcessOnly(true);
    }

    auto browser = std::make_shared<Browser>();
    std::vector<DeviceVolume> devices;
    std::string diskServiceError, diskStatus, activeDevice;
    bool diskBusy = false;
    int diskNoticeGeneration = 0;
    auto deviceForPath = [&](const std::string& path) -> const DeviceVolume* {
        const DeviceVolume* found = nullptr;
        for (const auto& v : devices) {
            if (v.visible() && !v.mountPoint.empty() && isInside(path, v.mountPoint) &&
                (!found || v.mountPoint.size() > found->mountPoint.size())) found = &v;
        }
        return found;
    };
    ViewMode mode = ViewMode::Icons;

    std::vector<HistoryItem> back, forward;
    FileClipboard clipboard;
    std::map<std::string, std::vector<FileEntry>> listingCache;
    std::string searchQuery;
    int searchGeneration = 0;

    std::vector<FinderTab> tabs;
    size_t activeTab = 0;
    auto tabBar = std::make_shared<TabBarView>();
    tabBar->browser = browser;
    tabBar->app = &app;
    tabBar->setHidden(true);
    tabBar->getTabs = [&]() -> const std::vector<FinderTab>& { return tabs; };
    tabBar->getActiveTab = [&]() -> size_t { return activeTab; };
    FinderTab initialTab;
    initialTab.path = homeDirectory();
    initialTab.title = displayName(homeDirectory());
    initialTab.mode = mode;
    tabs.push_back(std::move(initialTab));

    // Places ------------------------------------------------------------------------------------
    const std::string home = homeDirectory();
    struct Place {
        std::string path;
        std::string title;
    };
    std::vector<Place> favorites{{kRecents, tr("Recents")}, {home, tr("Home")}};
    for (auto [key, fallback, title] : {std::tuple{"DESKTOP", "Desktop", tr("Desktop")},
                                        std::tuple{"DOCUMENTS", "Documents", tr("Documents")},
                                        std::tuple{"DOWNLOAD", "Downloads", tr("Downloads")},
                                        std::tuple{"PICTURES", "Pictures", tr("Pictures")},
                                        std::tuple{"MUSIC", "Music", tr("Music")}, std::tuple{"VIDEOS", "Videos", tr("Videos")}}) {
        std::string p = userDirectory(key, fallback);
        std::error_code ec;
        if (!p.empty() && fs::is_directory(p, ec)) favorites.push_back({p, title});
    }

    std::vector<Place> customFavorites;
    std::function<void()> rebuildSidebar;

    auto bookmarksPath = [] {
        const char* h = std::getenv("HOME");
        std::string base = h ? h : "/home/jaime";
        return base + "/.config/gtk-3.0/bookmarks";
    };

    auto loadCustomFavorites = [&] {
        customFavorites.clear();
        std::string bp = bookmarksPath();
        std::ifstream in(bp);
        if (in.is_open()) {
            std::string line;
            while (std::getline(in, line)) {
                if (line.empty() || line[0] == '#') continue;
                std::string path;
                std::string title;
                if (line.starts_with("file://")) {
                    size_t sp = line.find(' ');
                    if (sp != std::string::npos) {
                        path = line.substr(7, sp - 7);
                        title = line.substr(sp + 1);
                    } else {
                        path = line.substr(7);
                    }
                } else {
                    size_t sp = line.find(' ');
                    if (sp != std::string::npos) {
                        path = line.substr(0, sp);
                        title = line.substr(sp + 1);
                    } else {
                        path = line;
                    }
                }
                std::error_code ec;
                if (!path.empty() && fs::is_directory(path, ec)) {
                    if (title.empty()) title = fs::path(path).filename().string();
                    bool dup = false;
                    for (const auto& fav : favorites) { if (fav.path == path) { dup = true; break; } }
                    for (const auto& cf : customFavorites) { if (cf.path == path) { dup = true; break; } }
                    if (!dup) customFavorites.push_back({path, title});
                }
            }
        }
    };

    auto saveCustomFavorites = [&] {
        std::string bp = bookmarksPath();
        std::error_code ec;
        fs::create_directories(fs::path(bp).parent_path(), ec);
        std::ofstream out(bp);
        if (out.is_open()) {
            for (const auto& f : customFavorites) {
                out << "file://" << f.path << " " << f.title << "\n";
            }
        }
    };

    auto addFavorite = [&](const std::string& folderPath, const std::string& name = "") {
        std::error_code ec;
        if (!fs::is_directory(folderPath, ec)) return;
        for (const auto& f : customFavorites) {
            if (f.path == folderPath) return;
        }
        for (const auto& f : favorites) {
            if (f.path == folderPath) return;
        }
        std::string title = name.empty() ? fs::path(folderPath).filename().string() : name;
        customFavorites.push_back({folderPath, title});
        saveCustomFavorites();
        if (rebuildSidebar) rebuildSidebar();
    };

    auto removeFavorite = [&](const std::string& folderPath) {
        auto it = std::remove_if(customFavorites.begin(), customFavorites.end(),
                                 [&](const Place& p) { return p.path == folderPath; });
        if (it != customFavorites.end()) {
            customFavorites.erase(it, customFavorites.end());
            saveCustomFavorites();
            if (rebuildSidebar) rebuildSidebar();
        }
    };

    loadCustomFavorites();

    // Per-folder View Settings ------------------------------------------------------------------
    struct FolderViewConfig {
        std::string mode = "icons";
        bool foldersFirst = true;
        std::string sortBy = "name";
        bool sortAscending = true;
    };
    std::unordered_map<std::string, FolderViewConfig> folderConfigs;

    auto folderViewsConfigPath = [] {
        const char* h = std::getenv("HOME");
        std::string base = h ? h : "/home/jaime";
        return base + "/.config/mayonnaise/finder/folder_views.txt";
    };

    auto loadFolderConfigs = [&] {
        std::ifstream in(folderViewsConfigPath());
        if (!in.is_open()) return;
        std::string line;
        while (std::getline(in, line)) {
            if (line.empty() || line[0] == '#') continue;
            auto pos1 = line.find('\t');
            if (pos1 == std::string::npos) continue;
            std::string p = line.substr(0, pos1);
            std::string rest = line.substr(pos1 + 1);
            std::istringstream iss(rest);
            FolderViewConfig cfg;
            std::string ff, sa;
            if (iss >> cfg.mode >> ff >> cfg.sortBy >> sa) {
                cfg.foldersFirst = (ff == "1" || ff == "true");
                cfg.sortAscending = (sa == "1" || sa == "true");
                folderConfigs[p] = cfg;
            }
        }
    };

    auto saveFolderConfigs = [&] {
        std::string cp = folderViewsConfigPath();
        std::error_code ec;
        fs::create_directories(fs::path(cp).parent_path(), ec);
        std::ofstream out(cp);
        if (!out.is_open()) return;
        for (const auto& [p, cfg] : folderConfigs) {
            out << p << "\t" << cfg.mode << "\t" << (cfg.foldersFirst ? "1" : "0")
                << "\t" << cfg.sortBy << "\t" << (cfg.sortAscending ? "1" : "0") << "\n";
        }
    };

    loadFolderConfigs();

    // Widgets ----------------------------------------------------------------------------------
    std::function<void()> updateAfterPreview;
    PreviewCache previews(app, [&] {
        if (updateAfterPreview) updateAfterPreview();
    });
    browser->previews = &previews;

    auto iconView = std::make_shared<IconGridView>(browser);
    auto iconScroll = ScrollView(Element(iconView));
    iconView->setScroller(iconScroll.node().get(), theme().metrics.titlebarHeight);

    auto columns = std::make_shared<ListColumns>();
    auto listHeader = std::make_shared<ListHeaderView>(browser, columns);
    auto listRows = std::make_shared<ListRowsView>(browser, columns);
    auto listScroll = ScrollView(Element(listRows)).scrollEdge(ScrollEdgeStyle::Hard, theme().metrics.spacingLarge);
    listRows->setScroller(listScroll.node().get());
    listHeader->onColumnsResized = [&] {
        listRows->invalidate();
        prefs.set("list.modifiedWidth", columns->modified);
        prefs.set("list.sizeWidth", columns->size);
        prefs.set("list.kindWidth", columns->kind);
    };

    auto columnBrowser = std::make_shared<ColumnBrowserView>(browser);
    auto columnScroll = ScrollView(Element(columnBrowser)).axes(ScrollAxes::Horizontal);
    columnBrowser->setScroller(columnScroll.node().get());
    columnBrowser->roots = {home, "/", trashFilesDirectory()};
    for (const auto& v : mountedVolumes()) columnBrowser->roots.push_back(v.mountPoint);

    auto emptyLabel = Label("").font(TextStyle::Title3).tertiary().textAlign(TextAlign::Center);
    auto emptyDetail = Label("").secondary().textAlign(TextAlign::Center).maxWidth(360);
    auto empty = VStack({emptyLabel, emptyDetail}).spacing(6).align(Alignment::Center);
    empty.setHidden(true);

    auto pathBar = std::make_shared<PathBarView>(browser);
    auto opSpinner = ProgressSpinner();
    auto opProgressBar = ProgressBar();
    opProgressBar.width(130).height(6);
    opProgressBar.setHidden(true);
    auto opLabel = Label("").font(TextStyle::Subheadline).secondary().lineLimit(1).maxWidth(420);
    auto cancelOperation = Button(tr("Cancel")).style(ButtonStyle::Borderless).tooltip(tr("Cancel queued tasks and safely stop operation"));
    cancelOperation.setHidden(true);
    auto statusLabel = Label("").font(TextStyle::Subheadline).secondary().textAlign(TextAlign::Center);
    auto sizeSlider = Slider(64, 32, 128).width(110).tooltip(tr("Icon size (Ctrl+scroll)"));
    auto statusBar = ZStack(
        {HStack({opSpinner, Element(opProgressBar.node()), opLabel, cancelOperation, Spacer(), sizeSlider}).spacing(8).padding(0, 12).align(Alignment::Center), statusLabel});
    statusBar.height(kStatusBarHeight);
    auto pickerCancel = Button(tr("Cancel")).minWidth(100).height(34).onClick([&] {
        std::exit(1);
    });
    auto pickerFilenameField = TextField(tr("File name:")).width(260).selectBaseNameOnly(true);
    if (!saveFilename.empty()) {
        pickerFilenameField.setText(saveFilename);
        pickerFilenameField.node()->selectBaseName();
    }
    auto submitPicker = [&] {
        if (!pickerMode) return;
        if (saveMode) {
            std::string name = pickerFilenameField.value();
            if (name.empty()) name = saveFilename.empty() ? "Untitled" : saveFilename;
            fs::path targetPath = fs::path(browser->path) / name;
            std::error_code ec;
            if (fs::exists(targetPath, ec)) {
                std::string titleStr = tr("“%1” already exists. Do you want to replace it?");
                auto p = titleStr.find("%1");
                if (p != std::string::npos) titleStr.replace(p, 2, name);
                auto confirmAlert = std::make_shared<AlertView>(
                    titleStr,
                    tr("A file with the same name already exists in this location. Replacing it will overwrite its current contents."),
                    std::vector<AlertButton>{
                        {tr("Cancel"), {}},
                        {tr("Replace"), [targetPath] {
                            std::cout << targetPath.string() << std::endl;
                            std::exit(0);
                        }, ButtonStyle::Destructive}
                    }, true);
                if (auto* h = pickerFilenameField.node()->host()) {
                    confirmAlert->show(*h);
                    return;
                }
            }
            std::cout << targetPath.string() << std::endl;
            std::exit(0);
        } else if (chooseFolder) {
            auto sel = browser->selectedPaths();
            if (!sel.empty() && fs::is_directory(sel.front())) {
                std::cout << sel.front() << std::endl;
            } else {
                std::cout << browser->path << std::endl;
            }
            std::exit(0);
        } else {
            auto sel = browser->selectedPaths();
            if (sel.empty()) return;
            for (const auto& s : sel) std::cout << s << std::endl;
            std::exit(0);
        }
    };
    auto pickerSelect = Button(saveMode ? tr("Save") : tr("Open"))
                            .style(ButtonStyle::Prominent)
                            .minWidth(100)
                            .height(34)
                            .onClick(submitPicker);
    HStack pickerBar;
    if (saveMode) {
        auto saveStack = HStack({Element(Label(tr("File name:"))), Element(pickerFilenameField.node())}).spacing(8);
        pickerBar = HStack({Element(saveStack.node()), Element(Spacer().node()), Element(pickerCancel.node()), Element(pickerSelect.node())})
                        .spacing(14)
                        .padding(Insets(8, 24, 14, 24));
    } else {
        pickerBar = HStack({Element(Spacer().node()), Element(pickerCancel.node()), Element(pickerSelect.node())})
                        .spacing(14)
                        .padding(Insets(8, 24, 14, 24));
    }
    pickerBar.height(56);

    VStack bottom;
    if (pickerMode) {
        bottom = VStack({Element(pathBar), Element(pickerBar.node())}).spacing(0);
    } else {
        bottom = VStack({Element(pathBar), statusBar}).spacing(0);
    }

    auto page = std::make_shared<FinderPage>();
    page->iconScroll = iconScroll.node();
    page->listHeader = listHeader;
    page->listScroll = listScroll.node();
    page->columnScroll = columnScroll.node();
    page->empty = empty.node();
    page->bottom = bottom.node();
    page->pathBar = pathBar;
    page->statusBar = statusBar.node();
    if (pickerMode) page->pickerBar = pickerBar.node();
    page->tabBar = tabBar;
    for (auto& w : {page->iconScroll, page->listHeader, page->listScroll, page->columnScroll, page->empty, page->bottom,
                    page->tabBar, page->menuAnchor}) {
        if (w) page->addChild(w);
    }

    auto sidebar = Sidebar(std::vector<SidebarItem>{});
    auto split = SplitView(sidebar, Element(page)).sidebarWidth(232).sidebarLimits(200, 320);

    auto backButton = Button(Icon::ChevronLeft).style(ButtonStyle::Borderless).tooltip(tr("Back (Alt+Left)"));
    auto forwardButton = Button(Icon::ChevronRight).style(ButtonStyle::Borderless).tooltip(tr("Forward (Alt+Right)"));
    auto titleLabel = Label("").font(TextStyle::Title3).lineLimit(1).maxWidth(320);
    auto viewControl = SegmentedControl({Icon::Grid, Icon::List, Icon::Columns}, 0);
    viewControl.background(Material::Glass).cornerRadius(theme().metrics.controlHeight * 0.5f);
    viewControl.tooltip(tr("View (Ctrl+1, Ctrl+2, Ctrl+3)"));
    auto actionMenu = Menu("", {}).icon(Icon::Ellipsis).borderless().tooltip(tr("Actions"));
    auto searchField = SearchField(tr("Search")).width(170);
    auto emptyTrashButton = Button(tr("Empty")).tooltip(tr("Empty Trash (Ctrl+Shift+Delete)"));
    auto ejectButton = Button(tr("Eject")).icon(Icon::Eject).style(ButtonStyle::Borderless).tooltip(tr("Eject device (Ctrl+E)"));

    auto quickLook = std::make_shared<QuickLookView>(browser);
    auto menu = std::make_shared<MenuView>(std::vector<MenuItem>{});

    bool closeAfterOperations = false;
    bool closePrompt = false;
    FileOps ops(app, [&](const std::string& message) {
        // A safe-finishing failure must stay visible instead of disappearing with the window.
        closeAfterOperations = false;
        if (!page->host()) return;
        std::make_shared<AlertView>(tr("Cannot complete this operation."), message,
                                    std::vector<AlertButton>{{tr("OK"), {}, ButtonStyle::Prominent}}, true)
            ->show(*page->host());
    });

    // Helpers -----------------------------------------------------------------------------------
    auto currentView = [&]() -> FileView* {
        switch (mode) {
            case ViewMode::Icons: return iconView.get();
            case ViewMode::List: return listRows.get();
            case ViewMode::Columns: return columnBrowser->activeColumn();
        }
        return nullptr;
    };
    auto focusView = [&] {
        if (auto* v = currentView(); v && v->host()) v->host()->setFocus(v, false);
    };
    auto isRealFolder = [&] { return !browser->searching && browser->path != kRecents && !browser->path.starts_with("tag:"); };
    auto inTrash = [&] { return isInside(browser->path, trashFilesDirectory()); };

    browser->listing = [&](const std::string& folder) -> const std::vector<FileEntry>& {
        auto it = listingCache.find(folder);
        if (it != listingCache.end()) return it->second;
        auto entries = listDirectory(folder, browser->showHidden);
        sortEntries(entries, browser->sort);
        return listingCache.emplace(folder, std::move(entries)).first->second;
    };

    std::function<void()> refreshActionMenu;
    std::function<void()> updateStatus = [&] {
        const Browser& b = *browser;
        std::string text;
        const size_t n = b.entries.size();
        if (b.searching) {
            text = (n == 1 ? "1 " + tr("result") : std::to_string(n) + " " + tr("results"));
        } else if (!b.selection.empty()) {
            text = std::to_string(b.selection.size()) + " / " + std::to_string(n) + " " + tr("items selected");
        } else {
            text = itemCount(n);
        }
        if (auto free = availableSpace(b.path == kRecents ? home : b.path)) text += ", " + formatSize(*free) + " " + tr("available");
        statusLabel.setText(text);
        const bool busy = ops.busy() || diskBusy;
        opSpinner.setHidden(!busy);
        opProgressBar.setHidden(!busy);
        statusLabel.setHidden(busy);
        if (ops.busy()) {
            const auto& progress = ops.progress();
            if (progress.total > 0) {
                float frac = static_cast<float>(progress.bytes) / static_cast<float>(progress.total);
                opProgressBar.setValue(std::clamp(frac, 0.0f, 1.0f));
            } else {
                opProgressBar.setValue(std::nullopt);
            }
        } else if (diskBusy) {
            opProgressBar.setValue(std::nullopt);
        } else {
            opProgressBar.setValue(0.0f);
        }
        std::string operation = ops.status();
        if (ops.busy() && !ops.progress().item.empty()) {
            const auto& progress = ops.progress();
            std::string stage = progress.phase == transaction::Phase::Scanning         ? tr("Preparing ")
                                : progress.phase == transaction::Phase::Copying        ? tr("Copying ")
                                : progress.phase == transaction::Phase::Committing     ? tr("Committing ")
                                : progress.phase == transaction::Phase::RemovingSource ? tr("Removing source ")
                                : progress.phase == transaction::Phase::Restoring      ? tr("Restoring ")
                                                                                       : tr("Processing ");
            operation =
                (ops.cancelling() ? std::string(tr("Safely finishing ")) : std::string(stage)) + displayName(progress.item);
            if (progress.total) operation += " · " + formatSize(progress.bytes) + " / " + formatSize(progress.total);
        }
        opLabel.setText(diskBusy || !diskStatus.empty() ? diskStatus : ops.busy() ? operation : diskServiceError);
        cancelOperation.setHidden(!ops.busy());
        cancelOperation.setDisabled(ops.cancelling());
        const bool hideStatus = !busy && !prefs.getBool("showStatusBar", true);
        if (statusBar->isVisible() == hideStatus) {
            statusBar.setHidden(hideStatus);
            page->invalidateLayout();
        }
        const auto* device = deviceForPath(b.path);
        ejectButton.setHidden(!device || !device->canRemove());
        ejectButton.setDisabled(diskBusy || ops.busy());
        sizeSlider.setHidden(mode != ViewMode::Icons);
        emptyTrashButton.setHidden(!isInside(b.path, trashFilesDirectory()) || b.searching);
        emptyTrashButton.setDisabled(b.entries.empty());
        pathBar->invalidate();
        // Cheap enough to do on every change; keeps checkmarks and enabled states honest.
        if (refreshActionMenu) refreshActionMenu();
    };

    auto updateChrome = [&] {
        const std::string name = browser->searching ? tr("Searching for ") + "“" + searchQuery + "”" : displayName(browser->path);
        titleLabel.setText(name);
        window.setTitle(browser->searching ? name : displayName(browser->path));
        if (activeTab < tabs.size()) {
            tabs[activeTab].title = name;
            if (tabBar) tabBar->invalidate();
        }
        backButton.setDisabled(back.empty());
        forwardButton.setDisabled(forward.empty());
        std::string sel;
        if (!browser->searching) {
            for (const auto& f : favorites) {
                if (f.path == browser->path) sel = f.path;
            }
            if (browser->path == "/" || browser->path == trashFilesDirectory()) sel = browser->path;
            for (const auto& v : mountedVolumes()) {
                if (v.mountPoint == browser->path) sel = v.mountPoint;
            }
            if (const auto* v = deviceForPath(browser->path)) sel = v->id;
        }
        sidebar.selection(sel);
    };


    auto refreshViews = [&](bool focusColumns) {
        iconView->reload();
        listRows->reload();
        if (mode == ViewMode::Columns) columnBrowser->rebuild(focusColumns);
        updateStatus();
    };

    Searcher searcher(app);
    std::string listError;

    auto updateEmptyState = [&] {
        const bool none = browser->entries.empty();
        bool show = none && (browser->searching || !listError.empty() || browser->path == kRecents || inTrash());
        if (browser->searching && searcher.running()) show = false;
        empty.setHidden(!show || mode == ViewMode::Columns);
        if (!listError.empty()) {
            emptyLabel.setText(tr("Cannot open") + " “" + displayName(browser->path) + "”");
            emptyDetail.setText(tr("You may not have permission to view its contents."));
        } else if (browser->searching) {
            emptyLabel.setText(tr("No results found"));
            emptyDetail.setText(tr("No matches found for ") + "“" + searchQuery + "” " + tr("in") + " “" + displayName(browser->path) + "”.");
        } else if (browser->path == kRecents) {
            emptyLabel.setText(tr("No recent files"));
            emptyDetail.setText(tr("Files opened in other applications will appear here."));
        } else {
            emptyLabel.setText(tr("Trash is empty"));
            emptyDetail.setText("");
        }
    };

    // Re-reads the folder, keeping the selection where it still exists.
    auto reload = [&] {
        listingCache.clear();
        if (browser->searching) {
            updateStatus();
            return;
        }
        listError.clear();
        auto entries = listDirectory(browser->path, browser->showHidden, &listError);
        sortEntries(entries, browser->sort);
        browser->entries = std::move(entries);
        std::erase_if(browser->selection, [](const std::string& p) {
            std::error_code ec;
            return !fs::exists(fs::symlink_status(p, ec));
        });
        std::erase_if(browser->cut, [](const std::string& p) {
            std::error_code ec;
            return !fs::exists(fs::symlink_status(p, ec));
        });
        refreshViews(false);
        updateEmptyState();
    };

    FolderWatcher watcher(app, [&] { reload(); });
    cancelOperation.onClick([&] { ops.cancel(); });
    auto allowClose = [&] {
        if (!ops.busy()) return true;
        if (closeAfterOperations || closePrompt || !page->host()) return false;
        closePrompt = true;
        std::make_shared<AlertView>(tr("File operations in progress"), tr("Waiting for safe completion before closing."),
                                    std::vector<AlertButton>{{tr("Keep Window Open"), [&] { closePrompt = false; }},
                                                             {tr("Cancel and Close"),
                                                              [&] {
                                                                  closePrompt = false;
                                                                  closeAfterOperations = true;
                                                                  ops.cancel();
                                                                  if (!ops.busy()) window.close();
                                                              },
                                                              ButtonStyle::Prominent}},
                                    true)
            ->show(*page->host());
        return false;
    };
    window.onCloseRequested(allowClose);
    std::shared_ptr<OperationProgressPanel> progressDialog;
    ops.onChanged = [&] {
        updateStatus();
        if (ops.busy() && page->host()) {
            if (!progressDialog) {
                progressDialog = std::make_shared<OperationProgressPanel>([&] { ops.cancel(); });
                progressDialog->present(*page->host());
            }
            double fraction = ops.progress().total > 0 ? static_cast<double>(ops.progress().bytes) / ops.progress().total : 0.0;
            std::string detail = ops.progress().total > 0 ? (formatSize(ops.progress().bytes) + " / " + formatSize(ops.progress().total)) : "";
            progressDialog->setOperation(ops.status(), displayName(ops.progress().item), fraction, detail);
        } else if (!ops.busy() && progressDialog) {
            progressDialog->dismiss();
            progressDialog = nullptr;
        }
        if (closeAfterOperations && !ops.busy())
            app.post([&] {
                if (closeAfterOperations && !ops.busy()) window.close();
            });
    };
    updateAfterPreview = [&] {
        if (auto* v = currentView()) v->invalidate();
        quickLook->invalidate();
    };

    auto revealLater = [&](const std::string& path) {
        app.post([&, path] {
            if (auto* v = currentView()) v->reveal(path, false);
        });
    };

    // Navigation ----------------------------------------------------------------------------------
    std::function<void(ViewMode, bool)> setModeFunc;
    auto setMode = [&](ViewMode m, bool recordForFolder = true) {
        if (setModeFunc) setModeFunc(m, recordForFolder);
    };
    std::function<void(const std::string&, const std::string&, bool)> navigateTo;
    navigateTo = [&](const std::string& rawPath, const std::string& select, bool recordHistory) {
        std::string path = rawPath;
        if (path.starts_with("tag:")) {
            std::string tagName = path.substr(4);
            browser->path = path;
            browser->entries.clear();
            for (const auto& fp : TagManager::instance().filesWithTag(tagName)) {
                if (auto e = statEntry(fp)) browser->entries.push_back(*e);
            }
            browser->selection.clear();
            browser->lead.clear();
            browser->searching = false;
            refreshViews(false);
            updateChrome();
            return;
        }
        if (path != kRecents) {
            std::error_code ec;
            if (!fs::is_directory(path, ec)) {
                if (fs::exists(path, ec)) {  // a file: show it selected in its folder
                    navigateTo(parentPath(path), path, recordHistory);
                    return;
                }
                if (page->host()) {
                    std::make_shared<AlertView>(tr("Folder not found."), "“" + path + "” " + tr("does not exist."),
                                                std::vector<AlertButton>{{tr("OK"), {}, ButtonStyle::Prominent}}, true)
                        ->show(*page->host());
                }
                return;
            }
        }
        if (browser->searching) {
            searcher.cancel();
            browser->searching = false;
            searchQuery.clear();
            searchField.setText("");
        }
        const bool changed = path != browser->path;
        if (changed && recordHistory && !browser->path.empty()) {
            back.push_back({browser->path, browser->lead});
            forward.clear();
        }
        if (changed) {
            browser->path = path;
            prefs.set("lastFolder", path);
            previews.cancelPending();
            watcher.watch(path);
            listRows->collapseAll();
            iconScroll.scrollTo({0, 0}, false);
            listScroll.scrollTo({0, 0}, false);
            browser->cut.clear();
            clipboard.cut = clipboard.cut && !clipboard.paths.empty();

            // Apply per-folder view settings if recorded
            auto it = folderConfigs.find(path);
            if (it != folderConfigs.end()) {
                const auto& cfg = it->second;
                ViewMode targetMode = (cfg.mode == "list" ? ViewMode::List : cfg.mode == "columns" ? ViewMode::Columns : ViewMode::Icons);
                if (targetMode != mode) setMode(targetMode, false);
                browser->sort.foldersFirst = cfg.foldersFirst;
                if (cfg.sortBy == "modified") browser->sort.key = SortKey::Modified;
                else if (cfg.sortBy == "size") browser->sort.key = SortKey::Size;
                else if (cfg.sortBy == "kind") browser->sort.key = SortKey::Kind;
                else browser->sort.key = SortKey::Name;
                browser->sort.ascending = cfg.sortAscending;
            }
        }
        browser->selection.clear();
        browser->lead = browser->anchor = {};
        reload();
        if (!select.empty()) {
            browser->select(select, false);
            revealLater(select);
        }
        if (mode == ViewMode::Columns) columnBrowser->rebuild(true);
        else focusView();
        browser->notifySelection();
        updateChrome();
        if (activeTab < tabs.size()) {
            tabs[activeTab].path = browser->path;
            tabs[activeTab].title = browser->searching ? (tr("Searching for ") + "“" + searchQuery + "”") : displayName(browser->path);
            tabs[activeTab].mode = mode;
            tabs[activeTab].searching = browser->searching;
            tabs[activeTab].searchQuery = searchQuery;
            if (tabBar) tabBar->invalidate();
        }
    };

    browser->navigate = [&](const std::string& folder, const std::string& select) { navigateTo(folder, select, true); };

    auto goBack = [&] {
        if (back.empty()) return;
        forward.push_back({browser->path, browser->lead});
        HistoryItem h = back.back();
        back.pop_back();
        navigateTo(h.path, h.select, false);
    };
    auto goForward = [&] {
        if (forward.empty()) return;
        back.push_back({browser->path, browser->lead});
        HistoryItem h = forward.back();
        forward.pop_back();
        navigateTo(h.path, h.select, false);
    };
    browser->goBack = goBack;
    browser->goForward = goForward;
    auto goUp = [&] {
        if (!isRealFolder()) return;
        const std::string parent = parentPath(browser->path);
        if (parent != browser->path) navigateTo(parent, browser->path, true);
    };

    setModeFunc = [&](ViewMode m, bool recordForFolder) {
        if (auto* v = currentView(); v && v->isRenaming()) focusView();
        mode = m;
        if (activeTab < tabs.size()) {
            tabs[activeTab].mode = m;
        }
        iconScroll.setHidden(m != ViewMode::Icons);
        listHeader->setHidden(m != ViewMode::List);
        listScroll.setHidden(m != ViewMode::List);
        columnScroll.setHidden(m != ViewMode::Columns);
        viewControl->setSelected(static_cast<int>(m));
        prefs.set("view", m == ViewMode::Icons ? "icons" : m == ViewMode::List ? "list" : "columns");
        if (recordForFolder && !browser->path.empty() && browser->path != kRecents && !browser->path.starts_with("tag:")) {
            auto& cfg = folderConfigs[browser->path];
            cfg.mode = (m == ViewMode::Icons ? "icons" : m == ViewMode::List ? "list" : "columns");
            saveFolderConfigs();
        }
        if (m == ViewMode::Columns) columnBrowser->rebuild(true);
        else focusView();
        updateEmptyState();
        updateStatus();
        if (!browser->lead.empty()) revealLater(browser->lead);
    };

    // Opening & actions --------------------------------------------------------------------------------
    std::function<void(const std::string&, const std::string&)> showAlert = [&](const std::string& title,
                                                                                 const std::string& message) {
        if (!page->host()) return;
        std::make_shared<AlertView>(title, message, std::vector<AlertButton>{{tr("OK"), {}, ButtonStyle::Prominent}}, true)
            ->show(*page->host());
    };

    std::function<void(const FileEntry&)> showOpenWith;  // defined with the panels below

    // Opening ---------------------------------------------------------------------------------------
    // Reacts to what actually happened, because the failure modes need a fix, not just a message:
    //   * a program whose execute bit is missing can't be started by anyone (and no launcher may add it), so
    //     the alert offers to mark it executable, like the Finder's "trust" dialog;
    //   * no application is registered for the type: offer tr("Open With");
    //   * the application is gone or can't be started: say which one and why.
    std::function<void(const std::string&)> openPath;
    auto showExecuteBitAlert = [&](const std::string& path) {
        if (!page->host()) return;
        const std::string name = displayName(path);
        std::make_shared<AlertView>(
            "“" + name + "” " + tr("is executable, but lacks execute permissions."),
            tr("For security reasons, Finder will not run executable files without execute permissions."),
            std::vector<AlertButton>{{tr("Cancel"), {}},
                                     {tr("Make Executable and Open"),
                                      [&, path] {
                                          if (auto error = setExecutable(path); !error.empty()) {
                                              showAlert(tr("Cannot open") + " “" + displayName(path) + "”。", error);
                                              return;
                                          }
                                          reload();
                                          openPath(path);
                                      },
                                      ButtonStyle::Prominent}},
            true)
            ->show(*page->host());
    };
    auto showNoDefaultAppAlert = [&](const std::string& path) {
        if (!page->host()) return;
        const std::string name = displayName(path);
        const std::string type = mimeTypeFor(path);
        const std::string label = mimeTypeLabel(type);
        std::make_shared<AlertView>(
            tr("No application available for ") + "“" + name + "”.",
            tr("There is no default application for this file type. You can choose one using Open With."),
            std::vector<AlertButton>{{tr("Cancel"), {}},
                                     {tr("Open With…"),
                                      [&, path] {
                                          if (auto entry = statEntry(path)) showOpenWith(*entry);
                                      },
                                      ButtonStyle::Prominent}},
            true)
            ->show(*page->host());
    };
    openPath = [&](const std::string& path) {
        // Demo runs must not start programs, but the two failure modes before a launch (no execute bit, no
        // default application) are exactly what the screenshots should show, so they are checked here too.
        if (demo) {
            struct stat st {};
            if (::stat(path.c_str(), &st) == 0 && S_ISREG(st.st_mode) && (st.st_mode & 0111) == 0) {
                showExecuteBitAlert(path);
                return;
            }
            if (defaultApplicationId(path).empty()) {
                showNoDefaultAppAlert(path);
                return;
            }
            return;
        }
        OpenResult result = launchWithDefaultApp(path);
        if (result.launched || !page->host()) return;
        const std::string name = displayName(path);
        if (result.needsExecutableBit) {
            std::make_shared<AlertView>(
                "“" + name + "” " + tr("is executable, but lacks execute permissions."),
                tr("For security reasons, Finder will not run executable files without execute permissions."),
                std::vector<AlertButton>{{tr("Cancel"), {}},
                                         {tr("Make Executable and Open"),
                                          [&, path] {
                                              if (auto error = setExecutable(path); !error.empty()) {
                                                  showAlert(tr("Cannot open") + " “" + displayName(path) + "”。", error);
                                                  return;
                                              }
                                              reload();
                                              openPath(path);
                                          },
                                          ButtonStyle::Prominent}},
                true)
                ->show(*page->host());
        } else if (result.noDefaultApp) {
            std::make_shared<AlertView>(
                tr("No application available for ") + "“" + name + "”.",
                tr("There is no default application for this file type. You can choose one using Open With."),
                std::vector<AlertButton>{{tr("Cancel"), {}},
                                         {tr("Open With…"),
                                          [&, path] {
                                              if (auto entry = statEntry(path)) showOpenWith(*entry);
                                          },
                                          ButtonStyle::Prominent}},
                true)
                ->show(*page->host());
        } else {
            showAlert(tr("Cannot open") + " “" + name + "”。", result.error);
        }
    };

    browser->open = [&](const FileEntry& e) {
        if (e.isDir) {
            navigateTo(e.path, {}, true);
        } else if (isInTrash(e.path)) {
            showAlert("“" + e.name + "” " + tr("is in the Trash."), tr("To use this item, please drag it out of the Trash or select Put Back."));
        } else {
            if (pickerMode && !saveMode) {
                std::cout << e.path << std::endl;
                std::exit(0);
            }
            openPath(e.path);  // itself a no-op for real launches in demo mode
        }
    };

    auto openSelection = [&] {
        auto paths = browser->selectedPaths();
        for (const auto& p : paths) {
            if (auto e = statEntry(p)) {
                browser->open(*e);
                if (e->isDir) break;  // navigating replaces the selection
            }
        }
    };

    auto showInEnclosingFolder = [&] {
        auto paths = browser->selectedPaths();
        if (paths.size() == 1) navigateTo(parentPath(paths.front()), paths.front(), true);
    };

    auto selectAfterOp = [&](const std::vector<std::string>& paths) {
        reload();
        std::vector<std::string> visible;
        for (const auto& path : paths) {
            if (parentPath(path) == browser->path) visible.push_back(path);
        }
        // A cancelled task or navigation during work must not erase the current folder's selection.
        if (!visible.empty()) {
            browser->selection = std::set<std::string>(visible.begin(), visible.end());
            browser->lead = browser->anchor = visible.front();
            revealLater(visible.front());
        }
        browser->notifySelection();
    };

    auto moveToTrashSelected = [&] {
        auto paths = browser->selectedPaths();
        if (paths.empty()) return;
        if (inTrash()) {
            auto alert = std::make_shared<AlertView>(
                tr("Permanently delete ") + (paths.size() == 1 ? "“" + fs::path(paths[0]).filename().string() + "”"
                                                                        : std::to_string(paths.size()) + " " + tr("items")) +
                    "?",
                tr("This action cannot be undone."),
                std::vector<AlertButton>{{tr("Cancel"), {}},
                                         {tr("Delete"), [&, paths] { ops.deleteImmediately(paths, [&](auto) { reload(); }); },
                                          ButtonStyle::Destructive}},
                true);
            alert->show(*page->host());
            return;
        }
        if (!isRealFolder() && browser->path != kRecents && !browser->searching) return;
        // Select the item after the last trashed one, as the Finder does.
        std::string next;
        if (auto* v = currentView()) (void)v;
        const auto& es = browser->entries;
        for (size_t i = 0; i < es.size(); ++i) {
            if (es[i].path == paths.back()) {
                for (size_t j = i + 1; j < es.size(); ++j) {
                    if (!browser->isSelected(es[j].path)) {
                        next = es[j].path;
                        break;
                    }
                }
            }
        }
        ops.trash(paths, [&, next](auto) {
            if (browser->searching) {
                std::erase_if(browser->entries, [&](const FileEntry& e) {
                    std::error_code ec;
                    return !fs::exists(fs::symlink_status(e.path, ec));
                });
                browser->selection.clear();
                refreshViews(false);
                return;
            }
            reload();
            if (!next.empty()) {
                browser->select(next);
                revealLater(next);
            }
        });
    };

    auto putBack = [&] {
        auto paths = browser->selectedPaths();
        if (!paths.empty()) ops.putBack(paths, [&](auto) { reload(); });
    };

    auto emptyTrash = [&] {
        auto alert = std::make_shared<AlertView>(
            tr("Are you sure you want to permanently delete items in the Trash?"), tr("This action cannot be undone."),
            std::vector<AlertButton>{{tr("Cancel"), {}},
                                     {tr("Empty Trash"), [&] { ops.emptyTrash([&](auto) { reload(); }); }, ButtonStyle::Destructive}},
            true);
        alert->show(*page->host());
    };

    auto copyPaths = [&](const std::vector<std::string>& paths, bool cut) {
        if (paths.empty()) return;
        if (!writeFileClipboard({paths, cut})) {
            showAlert(tr("Cannot copy files"), tr("Failed to write to clipboard, please try again."));
            return;
        }
        clipboard = {paths, cut};
        browser->cut = cut ? std::set<std::string>(paths.begin(), paths.end()) : std::set<std::string>{};
        if (auto* v = currentView()) v->invalidate();
    };

    auto copySelection = [&](bool cut) { copyPaths(browser->selectedPaths(), cut); };

    auto copyPath = [&] {
        auto paths = browser->selectedPaths();
        if (paths.empty() && isRealFolder()) paths.push_back(browser->path);
        std::string text;
        for (const auto& p : paths) text += (text.empty() ? "" : "\n") + p;
        if (auto* h = page->host()) h->setClipboardText(text);
    };

    // Resolve collisions before dispatching background work, shared by paste and drops.
    auto transfer = [&](std::vector<std::string> sources, std::string folder, bool move, FileOps::Done done) {
        auto perform = [&, sources, folder, move, done] {
            if (move) ops.move(sources, folder, done);
            else ops.copy(sources, folder, done);
        };
        std::vector<std::string> collisions;
        std::vector<FileOps::Replacement> replacements;
        for (const auto& src : sources) {
            if (parentPath(src) == folder || isInside(folder, src)) continue;
            auto target = fs::path(folder) / fs::path(src).filename();
            std::error_code ec;
            if (fs::exists(fs::symlink_status(target, ec))) {
                collisions.push_back(target.string());
                struct stat existing{};
                if (::lstat(target.c_str(), &existing) == 0)
                    replacements.push_back({target.string(), transaction::identity(existing)});
            }
        }
        if (collisions.empty()) { perform(); return; }
        auto replace = [&, sources, folder, move, done, replacements] {
            ops.transfer(sources, folder, move, true, done, replacements);
        };
        auto alert = std::make_shared<AlertView>(
            tr("Already exists:") + " “" + fs::path(collisions.front()).filename().string() + "”",
            tr("An item with the same name already exists in this location.") +
                std::string(collisions.size() > 1 ? "\n" + tr("Apply to all duplicate items.") : ""),
            std::vector<AlertButton>{{tr("Cancel"), {}}, {tr("Keep Both"), perform},
                                     {tr("Replace"), replace, ButtonStyle::Destructive}}, false);
        alert->show(*page->host());
    };

    auto paste = [&] {
        if (!isRealFolder()) return;
        auto files = readFileClipboard();
        if (files.paths.empty()) {
            showAlert(tr("Cannot paste"), tr("Clipboard contains no files or folders to paste."));
            return;
        }
        std::vector<std::string> missing;
        for (const auto& path : files.paths) {
            std::error_code ec;
            if (!fs::exists(fs::symlink_status(path, ec))) missing.push_back(path);
        }
        if (!missing.empty()) {
            showAlert(tr("Cannot paste"), tr("Source item no longer exists or cannot be accessed: ") + missing.front());
            return;
        }
        auto done = [&, files](std::vector<std::string> created) {
            selectAfterOp(created);
            if (files.cut && !created.empty()) {
                const auto current = readFileClipboard();
                if (current.cut && current.paths == files.paths) {
                    auto remaining = files.paths;
                    const auto& completed = ops.completedSources();
                    std::erase_if(remaining, [&](const std::string& path) {
                        return std::find(completed.begin(), completed.end(), path) != completed.end();
                    });
                    // A partial paste keeps only unfinished sources cut; never reference moved originals.
                    const FileClipboard next =
                        remaining.empty() ? FileClipboard{created, false} : FileClipboard{remaining, true};
                    if (writeFileClipboard(next)) {
                        clipboard = remaining.empty() ? FileClipboard{} : next;
                        browser->cut = std::set<std::string>(remaining.begin(), remaining.end());
                    } else
                        showAlert(tr("Cannot update clipboard"), tr("File operation finished, but clipboard could not be updated."));
                }
            }
        };
        transfer(files.paths, browser->path, files.cut, done);
    };

    auto duplicate = [&] {
        auto paths = browser->selectedPaths();
        if (paths.empty() || !isRealFolder()) return;
        ops.copy(paths, browser->path, [&](std::vector<std::string> created) { selectAfterOp(created); });
    };

    auto renameSelected = [&] {
        if (browser->selection.size() != 1 || inTrash()) return;
        const std::string path = *browser->selection.begin();
        if (mode == ViewMode::Columns) columnBrowser->beginRename(path);
        else if (auto* v = currentView()) v->beginRename(path);
    };
    browser->renameSelection = renameSelected;

    auto renameItem = [&](const std::string& path, const std::string& newName) -> std::string {
        std::string renamed = ops.rename(path, newName);
        if (renamed.empty()) return {};
        if (path == browser->path) {
            navigateTo(renamed, {}, false);
            return renamed;
        }
        if (browser->searching) {
            for (auto& e : browser->entries) {
                if (e.path == path) {
                    if (auto st = statEntry(renamed)) e = *st;
                }
            }
            browser->select(renamed);
            refreshViews(false);
            return renamed;
        }
        reload();
        browser->select(renamed);
        revealLater(renamed);
        if (mode == ViewMode::Columns) columnBrowser->rebuild(true);
        return renamed;
    };
    browser->rename = renameItem;

    auto newFolder = [&] {
        if (!isRealFolder() || inTrash()) return;
        std::string created = ops.newFolder(browser->path);
        if (created.empty()) return;
        reload();
        browser->select(created);
        if (mode == ViewMode::Columns) columnBrowser->rebuild(true);
        // Rename once the new item has been laid out.
        app.post([&, created] {
            if (browser->isSelected(created)) renameSelected();
        });
    };

    browser->drop = [&](const std::vector<std::string>& paths, const std::string& folder, bool copy) {
        auto done = [&](std::vector<std::string> results) {
            if (browser->searching) return;
            selectAfterOp(results);
        };
        if (isInside(folder, trashFilesDirectory())) {
            ops.trash(paths, [&](auto) { reload(); });
        } else if (copy) {
            transfer(paths, folder, false, done);
        } else {
            transfer(paths, folder, true, done);
        }
    };

    window.onFilesDroppedAt([&](const std::vector<std::string>& paths, Point position) {
        Widget* root = page.get();
        while (root->parent()) root = root->parent();
        for (Widget* w = root->hitTest(position); w; w = w->parent()) {
            if (auto* target = dynamic_cast<DropTarget*>(w)) {
                const auto folder = target->dropFolderAt(position);
                if (!folder.empty()) browser->drop(paths, folder, true);
                return;
            }
        }
        // Empty scroll areas and empty folders have no FileView under the pointer.
        if (isRealFolder() && !inTrash() && page->frame().contains(position))
            browser->drop(paths, browser->path, true);
    });

    // Open With / Properties -----------------------------------------------------------------------------
    // macOS's Get Info. The application row's button opens tr("Open With"), which can also make the choice stick
    // for everything of this kind.
    auto showInfo = [&] {
        auto paths = browser->selectedPaths();
        const std::string path = paths.empty() ? (isRealFolder() ? browser->path : std::string()) : paths.front();
        auto entry = path.empty() ? std::nullopt : statEntry(path);
        if (!entry || !page->host()) return;
        auto appButton = Button(tr("Change…"));
        appButton.onClick([&, weakButton = std::weak_ptr<Widget>(appButton.node()), e = *entry] {
            auto keep = weakButton.lock();
            closeProperties(*page->host());
            showOpenWith(e);
        });
        presentProperties(*page->host(), *entry, appButton, [&] {
            // New permissions on screen: redraw the item's icon and the status line (a program!)
            if (auto* v = currentView()) v->invalidate();
        }, [&, path](const std::string& name) {
            return !renameItem(path, name).empty();
        });
        if (entry->isDir) {
            ops.folderSize(entry->path, [](std::uintmax_t bytes, size_t items) { setLatestFolderSize(bytes, items); });
        }
    };

    showOpenWith = [&](const FileEntry& entry) {
        if (!page->host()) return;
        auto panel = std::make_shared<OpenWithPanel>(entry, [&] { app.post([&] { focusView(); }); });
        panel->setOnOpen([&, entry](const std::string& id, bool remember) {
            if (remember) {
                if (const std::string problem = rememberProblem(entry, id); !problem.empty()) return problem;
                if (const std::string error = rememberApplication(entry, id); !error.empty()) return error;
            }
            // "Always" also opens the item with the chosen application.
            return openEntryWith(entry, id);
        });
        panel->present(*page->host());
    };

    // Quick Look ---------------------------------------------------------------------------------------
    auto quickLookEntry = [&]() -> std::optional<FileEntry> {
        auto paths = browser->selectedPaths();
        std::string p = !browser->lead.empty() && browser->isSelected(browser->lead) ? browser->lead
                        : paths.empty()                                              ? std::string()
                                                                                     : paths.front();
        if (p.empty()) return std::nullopt;
        return statEntry(p);
    };
    browser->quickLook = [&] {
        if (quickLook->isShown()) {
            quickLook->dismiss();
            return;
        }
        auto e = quickLookEntry();
        if (!e || !page->host()) return;
        quickLook->setEntry(*e);
        quickLook->present(*page->host());
    };
    quickLook->onArrow = [&](Key key) {
        // Arrow keys keep browsing the view underneath while Quick Look stays open.
        if (auto* v = currentView()) v->onKeyDown(KeyEvent{key});
    };
    quickLook->onOpen = [&](const FileEntry& e) {
        quickLook->dismiss();
        browser->open(e);
    };
    quickLook->setOnDismiss([&] { app.post([&] { focusView(); }); });

    browser->selectionChanged = [&] {
        if (activeTab < tabs.size()) {
            tabs[activeTab].selection = browser->selection;
            tabs[activeTab].lead = browser->lead;
            tabs[activeTab].anchor = browser->anchor;
        }
        if (pickerMode && saveMode) {
            auto sel = browser->selectedPaths();
            if (!sel.empty()) {
                std::error_code ec;
                if (!fs::is_directory(sel.front(), ec)) {
                    std::string fname = fs::path(sel.front()).filename().string();
                    pickerFilenameField.setText(fname);
                    pickerFilenameField.node()->selectBaseName();
                }
            }
        }
        updateStatus();
        if (mode == ViewMode::Columns) columnBrowser->rebuild(false);
        if (quickLook->isShown()) {
            if (auto e = quickLookEntry()) quickLook->setEntry(*e);
        }
        if (auto* v = currentView()) v->invalidate();
    };

    browser->sortChanged = [&] {
        constexpr const char* kSortNames[] = {"name", "modified", "size", "kind"};
        prefs.set("sortBy", kSortNames[static_cast<int>(browser->sort.key)]);
        prefs.set("sortAscending", browser->sort.ascending);
        prefs.set("foldersOnTop", browser->sort.foldersFirst);
        if (!browser->path.empty() && browser->path != kRecents && !browser->path.starts_with("tag:")) {
            auto& cfg = folderConfigs[browser->path];
            cfg.foldersFirst = browser->sort.foldersFirst;
            cfg.sortBy = kSortNames[static_cast<int>(browser->sort.key)];
            cfg.sortAscending = browser->sort.ascending;
            saveFolderConfigs();
        }
        if (syncSettings) syncSettings();
        reload();
        if (browser->searching) {
            sortEntries(browser->entries, browser->sort);
            refreshViews(false);
        }
    };

    browser->resizeIcons = [&](float size) {
        browser->iconSize = std::clamp(std::round(size), 32.0f, 128.0f);
        prefs.set("iconSize", browser->iconSize);
        sizeSlider.setValue(browser->iconSize);
        iconView->reload();
    };
    sizeSlider.onChange([&](float v) { browser->resizeIcons(v); });

    // Search ---------------------------------------------------------------------------------------------
    std::string searchRoot;
    auto startSearch = [&](const std::string& q) {
        searchQuery = q;
        if (q.empty()) {
            if (!browser->searching) return;
            searcher.cancel();
            browser->searching = false;
            reload();
            updateEmptyState();
            updateChrome();
            return;
        }
        if (!browser->searching) searchRoot = browser->path == kRecents ? home : browser->path;
        browser->searching = true;
        browser->entries.clear();
        browser->selection.clear();
        browser->lead.clear();
        refreshViews(false);
        updateChrome();
        searcher.start(searchRoot, q, browser->showHidden,
                       [&](std::vector<FileEntry> batch) {
                           for (auto& e : batch) browser->entries.push_back(std::move(e));
                           sortEntries(browser->entries, browser->sort);
                           refreshViews(false);
                           updateEmptyState();
                       },
                       [&] {
                           updateEmptyState();
                           updateStatus();
                       });
        updateEmptyState();
    };
    searchField.onChange([&](const std::string& q) {
        // Wait for a pause in typing before walking the disk.
        const int gen = ++searchGeneration;
        app.after(q.empty() ? 0.0 : 0.25, [&, gen, q] {
            if (gen == searchGeneration) startSearch(q);
        });
    });
    searchField.onSubmit([&](const std::string&) { focusView(); });

    auto saveCurrentTab = [&] {
        if (activeTab < tabs.size()) {
            auto& cur = tabs[activeTab];
            cur.path = browser->path;
            cur.title = browser->searching ? (tr("Searching for ") + "“" + searchQuery + "”") : displayName(browser->path);
            cur.mode = mode;
            cur.back = back;
            cur.forward = forward;
            cur.selection = browser->selection;
            cur.lead = browser->lead;
            cur.anchor = browser->anchor;
            cur.searching = browser->searching;
            cur.searchQuery = searchQuery;
        }
    };

    std::function<void(size_t index)> switchToTab;
    std::function<void(std::string folder)> newTab;
    std::function<void(size_t index)> closeTab;
    std::function<void()> nextTab;
    std::function<void()> prevTab;

    switchToTab = [&](size_t index) {
        if (pickerMode) return;
        if (index >= tabs.size()) return;
        saveCurrentTab();

        activeTab = index;
        const auto& target = tabs[activeTab];
        mode = target.mode;
        back = target.back;
        forward = target.forward;
        searchQuery = target.searchQuery;

        if (target.searching && !searchQuery.empty()) {
            searchField.setText(searchQuery);
            startSearch(searchQuery);
        } else {
            if (browser->searching) {
                searcher.cancel();
                browser->searching = false;
                searchQuery.clear();
                searchField.setText("");
            }
            navigateTo(target.path.empty() ? home : target.path, target.lead, false);
            browser->selection = target.selection;
            browser->lead = target.lead;
            browser->anchor = target.anchor;
            refreshViews(false);
        }
        setMode(target.mode);
        updateChrome();
        tabBar->setHidden(pickerMode || tabs.size() <= 1);
        page->invalidateLayout();
        tabBar->invalidate();
    };

    newTab = [&](std::string folder) {
        if (pickerMode) return;
        if (folder.empty()) folder = (isRealFolder() && !browser->path.empty()) ? browser->path : home;
        saveCurrentTab();

        FinderTab t;
        t.path = folder;
        t.title = displayName(folder);
        t.mode = mode;
        tabs.push_back(std::move(t));

        switchToTab(tabs.size() - 1);
    };

    closeTab = [&](size_t index) {
        if (pickerMode) return;
        if (index >= tabs.size()) return;
        if (tabs.size() <= 1) {
            if (allowClose()) window.close();
            return;
        }
        const bool wasActive = (index == activeTab);
        tabs.erase(tabs.begin() + index);
        if (wasActive) {
            if (activeTab >= tabs.size()) activeTab = tabs.size() - 1;
            const auto target = tabs[activeTab];
            mode = target.mode;
            back = target.back;
            forward = target.forward;
            searchQuery = target.searchQuery;
            if (target.searching && !searchQuery.empty()) {
                searchField.setText(searchQuery);
                startSearch(searchQuery);
            } else {
                if (browser->searching) {
                    searcher.cancel();
                    browser->searching = false;
                    searchQuery.clear();
                    searchField.setText("");
                }
                navigateTo(target.path.empty() ? home : target.path, target.lead, false);
                browser->selection = target.selection;
                browser->lead = target.lead;
                browser->anchor = target.anchor;
                refreshViews(false);
            }
            setMode(target.mode);
            updateChrome();
        } else if (index < activeTab) {
            activeTab--;
        }
        tabBar->setHidden(pickerMode || tabs.size() <= 1);
        page->invalidateLayout();
        tabBar->invalidate();
    };

    nextTab = [&] {
        if (tabs.size() > 1) switchToTab((activeTab + 1) % tabs.size());
    };

    prevTab = [&] {
        if (tabs.size() > 1) switchToTab((activeTab + tabs.size() - 1) % tabs.size());
    };

    tabBar->onSelectTab = [&](size_t i) { switchToTab(i); };
    tabBar->onCloseTab = [&](size_t i) { closeTab(i); };
    tabBar->onNewTab = [&]() { newTab(""); };

    browser->springLoad = [&](const std::string& folder) -> bool {
        for (size_t i = 0; i < tabs.size(); ++i) {
            if (tabs[i].path == folder) {
                if (i != activeTab) {
                    switchToTab(i);
                }
                return true;
            }
        }
        return false;
    };

    auto goToFolder = [&] {
        if (!page->host()) return;
        auto panel = std::make_shared<GoToFolderPanel>([&](const std::string& text) {
            std::string p = expandUser(text);
            if (!p.empty() && p[0] != '/' && isRealFolder()) p = (fs::path(browser->path) / p).lexically_normal().string();
            if (!p.empty()) navigateTo(p, {}, true);
        });
        panel->present(*page->host());
        page->host()->setFocus(panel->field.node().get(), true);
    };

    auto toggleHidden = [&] {
        browser->showHidden = !browser->showHidden;
        prefs.set("showHidden", browser->showHidden);
        if (syncSettings) syncSettings();
        reload();
    };

    auto openTerminalHere = [&] {
        auto paths = browser->selectedPaths();
        std::string folder = isRealFolder() ? browser->path : home;
        if (paths.size() == 1) {
            if (auto e = statEntry(paths.front()); e && e->isDir) folder = e->path;
        }
        if (demo) return;
        // Say why nothing opened instead of looking like the menu item did nothing at all.
        if (const std::string error = openTerminal(folder); !error.empty()) showAlert(tr("Cannot open terminal."), error);
    };

    auto selectAll = [&] {
        for (const auto& e : browser->entries) browser->selection.insert(e.path);
        browser->notifySelection();
    };

    auto undo = [&] {
        if (!ops.canUndo()) return;
        ops.undo([&](std::vector<std::string> restored) { selectAfterOp(restored); });
    };

    // Settings window (Ctrl+,) ------------------------------------------------------------------------
    Window* settingsWindow = nullptr;
    auto setBarVisible = [&](const std::shared_ptr<Widget>& bar, const char* key, bool on) {
        bar->setHidden(!on);
        prefs.set(key, on);
        page->invalidateLayout();
    };
    auto openSettings = [&] {
        if (settingsWindow) {
            settingsWindow->raise();
            return;
        }
        auto& w = app.createWindow({.title = tr("Finder Settings").c_str(), .width = 560, .height = 620, .minWidth = 480, .minHeight = 360});
        const std::string where = prefs.getString("startIn", "home");
        auto startPicker = Picker({tr("Home"), tr("Recents"), tr("Last Visited Folder")}, where == "recents" ? 1 : where == "last" ? 2 : 0)
                               .onChange([&](int i) { prefs.set("startIn", i == 1 ? "recents" : i == 2 ? "last" : "home"); });
        auto hidden = Toggle("", browser->showHidden).onChange([&](bool on) {
            if (on != browser->showHidden) toggleHidden();
        });
        auto foldersOnTop = Toggle("", browser->sort.foldersFirst).onChange([&](bool on) {
            if (on == browser->sort.foldersFirst) return;
            browser->sort.foldersFirst = on;
            browser->sortChanged();
        });
        auto showPath = Toggle("", page->pathBar->isVisible()).onChange([&](bool on) {
            setBarVisible(page->pathBar, "showPathBar", on);
        });
        auto showStatus = Toggle("", page->statusBar->isVisible()).onChange([&](bool on) {
            setBarVisible(page->statusBar, "showStatusBar", on);
        });
        auto form = Form({
            AppearanceSettings(app),
            FormSection(tr("General"), {FormRow(tr("New window opens:"), startPicker), FormRow(tr("Show Hidden Files"), hidden, "Ctrl+H"),
                                    FormRow(tr("Keep folders on top"), foldersOnTop)}),
            FormSection(tr("Window"), {FormRow(tr("Show Path Bar"), showPath), FormRow(tr("Show Status Bar"), showStatus)},
                        tr("Settings saved in ") + (isInside(prefs.directory(), home) ? "~" + prefs.directory().substr(home.size())
                                                                                          : prefs.directory())),
        });
        w.setContent(ScrollView(VStack({form}).padding(Insets(12, 24, 24, 24))));
        w.onKey([&w](const KeyEvent& e) {
            if (e.key == Key::Escape || (e.primary() && e.key == Key::W)) {
                w.close();
                return true;
            }
            return false;
        });
        // Changes made in the main window (Ctrl+H, the sort menu) show up here too.
        syncSettings = [hidden, foldersOnTop, browser]() mutable {
            hidden.isOn(browser->showHidden);
            foldersOnTop.isOn(browser->sort.foldersFirst);
        };
        w.onClosed([&] {
            settingsWindow = nullptr;
            syncSettings = nullptr;
        });
        settingsWindow = &w;
    };

    // Menus --------------------------------------------------------------------------------------------
    auto sortItems = [&]() {
        std::vector<MenuItem> items;
        const std::pair<SortKey, std::string> keys[] = {
            {SortKey::Name, tr("Name")}, {SortKey::Kind, tr("Kind")}, {SortKey::Modified, tr("Date Modified")}, {SortKey::Size, tr("Size")}};
        for (auto [key, title] : keys) {
            items.push_back(MenuItem(std::string(tr("Sort By: ")) + title, [&, key] {
                                if (browser->sort.key == key) return;
                                browser->sort.key = key;
                                browser->sort.ascending = key == SortKey::Name || key == SortKey::Kind;
                                browser->sortChanged();
                            }).setChecked(browser->sort.key == key));
        }
        items.push_back(MenuItem(tr("Keep Folders on Top"), [&] {
                            browser->sort.foldersFirst = !browser->sort.foldersFirst;
                            browser->sortChanged();
                        }).setChecked(browser->sort.foldersFirst));
        return items;
    };

    auto viewItems = [&]() {
        std::vector<MenuItem> items;
        const std::pair<ViewMode, std::string> modes[] = {
            {ViewMode::Icons, tr("Icons")}, {ViewMode::List, tr("List")}, {ViewMode::Columns, tr("Columns")}};
        int k = 1;
        for (auto [m, title] : modes) {
            items.push_back(MenuItem(std::string(tr("View: ")) + title, [&, m] { setMode(m); })
                                .setChecked(mode == m)
                                .withShortcut("Ctrl+" + std::to_string(k++)));
        }
        return items;
    };

    // "Open With ▸": every application that opens the item; the current one is ticked. With no other
    // application installed the row stays visible but disabled, so the menu never looks like
    // the feature is missing.
    auto openWithItems = [&]() {
        std::vector<MenuItem> items;
        auto paths = browser->selectedPaths();
        auto first = paths.size() == 1 ? statEntry(paths.front()) : std::nullopt;
        if (!first) return items;
        auto choices = applicationChoices(*first);
        const std::string current = defaultApplicationId(first->path);
        for (const auto& choice : choices) {
            items.push_back(MenuItem(choice.name, [&, e = *first, id = choice.id] {
                                if (const std::string error = openEntryWith(e, id); !error.empty()) showAlert(tr("Cannot open") + " “" + e.name + "”。", error);
                            })
                                .withImage(applicationIconImage(choice.icon))
                                .setChecked(choice.id == current)
                                .withDetail(choice.id == current ? tr("Default") : ""));
        }
        items.push_back(MenuItem::separator());
        items.push_back(MenuItem(tr("Other Application…"), [&, e = *first] { showOpenWith(e); }).withIcon(Icon::Ellipsis));
        if (!first->isDir && !isInTrash(first->path)) {
            items.push_back(MenuItem(tr("Always Open With"), [&, e = *first] {
                                auto panel = std::make_shared<OpenWithPanel>(e, [&] { app.post([&] { focusView(); }); });
                                panel->setRemember(true);
                                panel->setOnOpen([&, e](const std::string& id, bool remember) {
                                    if (remember) {
                                        if (const std::string problem = rememberProblem(e, id); !problem.empty()) return problem;
                                        if (const std::string error = rememberApplication(e, id); !error.empty()) return error;
                                    }
                                    return openEntryWith(e, id);
                                });
                                panel->present(*page->host());
                            }).withIcon(Icon::Checkmark));
        }
        return items;
    };

    auto backgroundItems = [&]() {
        std::vector<MenuItem> items;
        if (inTrash()) {
            items.push_back(MenuItem(tr("Empty Trash"), emptyTrash).asDestructive().disabled(browser->entries.empty()));
            items.push_back(MenuItem::separator());
        }
        items.push_back(MenuItem(tr("New Tab"), [&] { newTab(""); }).withIcon(Icon::Plus).withShortcut("Ctrl+T"));
        items.push_back(MenuItem(tr("New Folder"), newFolder).withIcon(Icon::FolderPlus).withShortcut("Ctrl+Shift+N")
                            .disabled(!isRealFolder() || inTrash()));
        const size_t clip = clipboard.paths.size();
        items.push_back(MenuItem(clip > 1 ? tr("Paste") + " " + std::to_string(clip) + " " + tr("items") : tr("Paste Item"), paste)
                            .withShortcut("Ctrl+V")
                            .disabled(!isRealFolder() || inTrash()));
        items.push_back(MenuItem::separator());
        items.push_back(MenuItem(tr("Open in Terminal"), openTerminalHere).withIcon(Icon::Terminal));
        items.push_back(MenuItem(tr("Copy Path"), copyPath).withShortcut("Ctrl+Alt+C").disabled(!isRealFolder()));
        if (isRealFolder()) {
            items.push_back(MenuItem(tr("Add to Sidebar"), [&] { addFavorite(browser->path); }).withIcon(Icon::Plus));
        }
        items.push_back(MenuItem::separator());
        items.push_back(MenuItem(tr("Get Info"), showInfo).withIcon(Icon::Info).withShortcut("Ctrl+I").disabled(!isRealFolder()));
        items.push_back(MenuItem(tr("Properties"), showInfo).withIcon(Icon::Gear).withShortcut("Ctrl+I").disabled(!isRealFolder()));
        items.push_back(MenuItem::separator());
        for (auto& it : viewItems()) items.push_back(std::move(it));
        items.push_back(MenuItem::separator());
        for (auto& it : sortItems()) items.push_back(std::move(it));
        items.push_back(MenuItem::separator());
        items.push_back(MenuItem(tr("Show Hidden Files"), toggleHidden).setChecked(browser->showHidden).withShortcut("Ctrl+H"));
        auto scripts = loadNautilusScripts();
        if (!scripts.empty()) {
            std::vector<MenuItem> scriptItems;
            for (const auto& s : scripts) {
                scriptItems.push_back(buildScriptMenuItem(s, browser->path, {}));
            }
            items.push_back(MenuItem(tr("Scripts")).withSubmenu(std::move(scriptItems)).withIcon(Icon::Gear));
        }
        return items;
    };

    auto itemItems = [&]() {
        std::vector<MenuItem> items;
        auto paths = browser->selectedPaths();
        const bool single = paths.size() == 1;
        auto first = single ? statEntry(paths.front()) : std::nullopt;
        const bool trashed = !paths.empty() && isInTrash(paths.front());
        items.push_back(MenuItem(tr("Open"), openSelection).withShortcut("Ctrl+O").disabled(trashed && !(first && first->isDir)));
        auto openWith = openWithItems();
        if (!openWith.empty()) items.push_back(MenuItem(tr("Open With")).withSubmenu(std::move(openWith)).disabled(trashed));
        const bool canCompress = canCompressFiles(paths);
        const bool canExtract = canExtractArchives(paths);
        if (canCompress || canExtract) items.push_back(MenuItem::separator());
        if (canCompress) {
            items.push_back(MenuItem(tr("Compress…"), [&, paths] {
                if (demo) return;
                ops.compress(paths, {}, selectAfterOp);
            }));
        }
        if (canExtract) {
            items.push_back(MenuItem(tr("Extract Here"), [&, paths] {
                if (demo) return;
                ops.extract(paths, {}, selectAfterOp);
            }));
            items.push_back(MenuItem(tr("Extract to…"), [&, paths] {
                if (demo) return;
                std::thread([&, paths] {
                    FILE* fp = popen("/usr/bin/finder --choose-folder --title \"Choose Extraction Destination\"", "r");
                    if (fp) {
                        char buf[4096];
                        std::string chosen;
                        if (fgets(buf, sizeof(buf), fp)) {
                            chosen = buf;
                            while (!chosen.empty() && (chosen.back() == '\n' || chosen.back() == '\r')) chosen.pop_back();
                        }
                        pclose(fp);
                        if (!chosen.empty()) {
                            app.post([&, paths, chosen] {
                                ops.extract(paths, chosen, selectAfterOp);
                            });
                        }
                    }
                }).detach();
            }));
        }
        if (trashed) items.push_back(MenuItem(tr("Put Back"), putBack));
        items.push_back(MenuItem::separator());
        items.push_back(MenuItem(single ? tr("Quick Look") + " “" + fs::path(paths.front()).filename().string() + "”" : tr("Quick Look"),
                                 [&] { browser->quickLook(); })
                            .withIcon(Icon::Magnifier).withShortcut("Space"));
        items.push_back(MenuItem(tr("Get Info"), showInfo).withIcon(Icon::Info).withShortcut("Ctrl+I"));
        items.push_back(MenuItem(tr("Properties"), showInfo).withIcon(Icon::Gear).withShortcut("Ctrl+I"));
        items.push_back(MenuItem(tr("Rename"), renameSelected).withIcon(Icon::Pencil).withShortcut("Enter").disabled(!single || trashed));
        items.push_back(MenuItem(tr("Duplicate"), duplicate).withIcon(Icon::DocOnDoc).withShortcut("Ctrl+D").disabled(!isRealFolder() || trashed));
        items.push_back(MenuItem::separator());
        items.push_back(MenuItem(tr("Copy"), [&, paths] { copyPaths(paths, false); }).withShortcut("Ctrl+C"));
        items.push_back(MenuItem(tr("Cut"), [&, paths] { copyPaths(paths, true); }).withShortcut("Ctrl+X").disabled(trashed));
        items.push_back(MenuItem(tr("Paste Item"), paste).withShortcut("Ctrl+V").disabled(!isRealFolder() || inTrash()));
        items.push_back(MenuItem(tr("Copy Path"), copyPath).withShortcut("Ctrl+Alt+C"));
        items.push_back(MenuItem::separator());
        std::vector<MenuItem> tagSubmenu;
        for (const auto& tag : TagManager::instance().allTags()) {
            tagSubmenu.push_back(MenuItem(tag.name, [&, tag, paths] {
                for (const auto& p : paths) {
                    TagManager::instance().toggleTag(p, tag.name);
                }
                if (auto* v = currentView()) v->invalidate();
            }).withDot(tag.color));
        }
        items.push_back(MenuItem(tr("Tags")).withSubmenu(std::move(tagSubmenu)).withIcon(Icon::Tag).disabled(trashed));
        items.push_back(MenuItem(tr("Share with Flydrop"), [&, paths] {
            std::vector<std::string> args{"flydrop", "send"};
            for (const auto& p : paths) args.push_back(p);
            spawnDetached(args);
        }).withIcon(Icon::Share).disabled(trashed));
        auto moveSelectedToFolder = [&, paths] {
            std::thread([&, paths] {
                FILE* fp = popen("/usr/bin/finder --choose-folder --title \"Choose Destination\"", "r");
                if (fp) {
                    char buf[4096];
                    std::string chosen;
                    if (fgets(buf, sizeof(buf), fp)) {
                        chosen = buf;
                        while (!chosen.empty() && (chosen.back() == '\n' || chosen.back() == '\r')) chosen.pop_back();
                    }
                    pclose(fp);
                    if (!chosen.empty()) {
                        std::error_code ec;
                        if (fs::is_directory(chosen, ec)) {
                            app.post([&, paths, chosen] {
                                ops.move(paths, chosen);
                            });
                        }
                    }
                }
            }).detach();
        };
        std::vector<MenuItem> moveToSubmenu;
        moveToSubmenu.push_back(MenuItem(tr("Choose Location…"), moveSelectedToFolder).withIcon(Icon::Folder));
        moveToSubmenu.push_back(MenuItem::separator());
        moveToSubmenu.push_back(MenuItem(tr("Desktop"), [&, paths] { ops.move(paths, userDirectory("DESKTOP", "Desktop")); }));
        moveToSubmenu.push_back(MenuItem(tr("Documents"), [&, paths] { ops.move(paths, userDirectory("DOCUMENTS", "Documents")); }));
        moveToSubmenu.push_back(MenuItem(tr("Downloads"), [&, paths] { ops.move(paths, userDirectory("DOWNLOAD", "Downloads")); }));
        moveToSubmenu.push_back(MenuItem(tr("Home"), [&, paths] { ops.move(paths, home); }));
        items.push_back(MenuItem(tr("Move To")).withSubmenu(std::move(moveToSubmenu)).disabled(!isRealFolder() || trashed));
        auto scripts = loadNautilusScripts();
        if (!scripts.empty()) {
            std::vector<MenuItem> scriptItems;
            for (const auto& s : scripts) {
                scriptItems.push_back(buildScriptMenuItem(s, browser->path, paths));
            }
            items.push_back(MenuItem(tr("Scripts")).withSubmenu(std::move(scriptItems)).withIcon(Icon::Gear));
        }
        items.push_back(MenuItem::separator());
        if (!isRealFolder()) items.push_back(MenuItem(tr("Show in Enclosing Folder"), showInEnclosingFolder).withShortcut("Ctrl+R").disabled(!single));
        if (first && first->isDir) {
            items.push_back(MenuItem(tr("Add to Sidebar"), [&, p = first->path] { addFavorite(p); }).withIcon(Icon::Plus));
            items.push_back(MenuItem(tr("Open in New Tab"), [&, p = first->path] { newTab(p); }).withIcon(Icon::FolderPlus));
            items.push_back(MenuItem(tr("Open in Terminal"), openTerminalHere).withIcon(Icon::Terminal));
        }
        if (!isRealFolder() || (first && first->isDir)) items.push_back(MenuItem::separator());
        items.push_back(MenuItem(trashed ? tr("Delete Immediately…") : tr("Move to Trash"), moveToTrashSelected)
                            .withIcon(Icon::Trash).withShortcut("Del").asDestructive());
        return items;
    };

    browser->contextMenu = [&](Widget& /*from*/, Point at, bool onItem) {
        menu->setItems(onItem ? itemItems() : backgroundItems());
        menu->setGap(2);
        page->menuAnchor->arrange({at.x, at.y, 1, 1});
        menu->showFrom(*page->menuAnchor);
        if (auto* h = page->host()) h->setFocus(menu.get(), false);
    };
    menu->setOnDismiss([&] {
        app.post([&] {
            // A menu action may already have focused an editor or opened a panel.
            if (auto* h = page->host(); h && !h->focusedWidget()) focusView();
        });
    });

    // The toolbar's action menu is rebuilt whenever it might have changed.
    std::function<void(const std::string&)> removeVolume;
    refreshActionMenu = [&] {
        std::vector<MenuItem> items{
            MenuItem(tr("New Tab"), [&] { newTab(""); }).withIcon(Icon::Plus).withShortcut("Ctrl+T"),
            MenuItem(tr("Close Tab"), [&] { closeTab(activeTab); }).withIcon(Icon::Xmark).withShortcut("Ctrl+W").disabled(tabs.size() <= 1),
            MenuItem(tabBar->isVisible() ? tr("Hide Tab Bar") : tr("Show Tab Bar"), [&] {
                tabBar->setHidden(tabBar->isVisible());
                page->invalidateLayout();
            }).withShortcut("Ctrl+Shift+T"),
            MenuItem::separator(),
            MenuItem(tr("New Folder"), newFolder).withIcon(Icon::FolderPlus).withShortcut("Ctrl+Shift+N").disabled(!isRealFolder() || inTrash()),
            MenuItem(tr("Go to Folder…"), goToFolder).withShortcut("Ctrl+Shift+G"),
            MenuItem(tr("Settings…"), openSettings).withIcon(Icon::Gear).withShortcut("Ctrl+,"),
            MenuItem(tr("Get Info"), showInfo).withIcon(Icon::Info).withShortcut("Ctrl+I"),
            MenuItem(tr("Properties"), showInfo).withIcon(Icon::Gear).withShortcut("Ctrl+I"),
            MenuItem(tr("Open in Terminal"), openTerminalHere).withIcon(Icon::Terminal),
            MenuItem::separator(),
        };
        if (const auto* v = deviceForPath(browser->path); v && v->canRemove()) {
            items.push_back(MenuItem(tr("Eject ") + "“" + v->name + "”", [&, id = v->id] { removeVolume(id); })
                                .withIcon(Icon::Eject).withShortcut("Ctrl+E").disabled(diskBusy || ops.busy()));
            items.push_back(MenuItem::separator());
        }
        if (inTrash()) {
            items.push_back(MenuItem(tr("Empty Trash"), emptyTrash).asDestructive().disabled(browser->entries.empty()));
            items.push_back(MenuItem::separator());
        }
        items.push_back(MenuItem(ops.undoTitle(), undo).withShortcut("Ctrl+Z").disabled(!ops.canUndo()));
        items.push_back(MenuItem::separator());
        for (auto& it : sortItems()) items.push_back(std::move(it));
        items.push_back(MenuItem::separator());
        items.push_back(MenuItem(tr("Show Hidden Files"), toggleHidden).setChecked(browser->showHidden).withShortcut("Ctrl+H"));
        actionMenu.setItems(std::move(items));
    };
    // Sidebar ----------------------------------------------------------------------------------------------
    rebuildSidebar = [&] {
        std::vector<SidebarItem> items{SidebarItem::header(tr("Favorites"))};
        items.emplace_back("applications://pulsar", Icon::Folder, tr("Applications"));
        for (const auto& f : favorites) items.emplace_back(f.path, placeIcon(f.path), f.title);
        for (const auto& f : customFavorites) items.emplace_back(f.path, placeIcon(f.path), f.title);
        items.push_back(SidebarItem::header(tr("Locations")));
        items.emplace_back("/", Icon::HardDrive, tr("Computer"));
        for (const auto& v : mountedVolumes()) {
            if (std::none_of(devices.begin(), devices.end(), [&](const auto& d) { return d.mountPoint == v.mountPoint; }))
                items.emplace_back(v.mountPoint, Icon::HardDrive, v.name);
        }
        items.emplace_back(trashFilesDirectory(), Icon::Trash, tr("Trash"));
        const DeviceVolume* active = nullptr;
        for (const auto& v : devices) if (v.id == activeDevice) active = &v;
        size_t count = 0;
        for (const auto& v : devices) {
            if (!v.visible()) continue;
            if (count++ == 0) items.push_back(SidebarItem::header(tr("External Devices")));
            SidebarItem item(v.id, v.external ? Icon::UsbDrive : Icon::HardDrive, v.name);
            item.busy = diskBusy && active && sameDevice(*active, v);
            const std::string capacity = v.size ? formatSize(v.size) + " · " : "";
            item.subtitle = item.busy ? (diskStatus.find("Mount") != std::string::npos ? tr("Mounting…") : tr("Ejecting…")) : capacity + (v.mountPoint.empty() ? tr("Not mounted") :
                                                                 v.readOnly ? tr("Mounted (Read-Only)") : tr("Mounted"));
            item.enabled = !diskBusy;
            if (v.canRemove()) item.actionIcon = Icon::Eject;
            items.push_back(std::move(item));
        }
        auto clouds = cloudVolumes();
        auto cloudHeader = SidebarItem::header(tr("Cloud"));
        cloudHeader.id = "cloud_header_add";
        cloudHeader.actionIcon = Icon::Plus;
        items.push_back(std::move(cloudHeader));
        for (const auto& c : clouds) {
            items.emplace_back(c.mountPoint, Icon::Globe, c.name);
        }
        items.push_back(SidebarItem("pulsar-cloud://choose", Icon::Plus, tr("Add Cloud Account…")));
        items.push_back(SidebarItem::header(tr("Tags")));
        for (const auto& tag : TagManager::instance().allTags()) {
            SidebarItem item("tag:" + tag.name, tag.name);
            item.withDot(tag.color);
            items.push_back(std::move(item));
        }
        sidebar.setItems(std::move(items));
        updateChrome();
    };

    auto diskNotice = [&](std::string message) {
        diskStatus = std::move(message);
        const int generation = ++diskNoticeGeneration;
        updateStatus();
        app.after(6, [&, generation] {
            if (generation == diskNoticeGeneration && !diskBusy) { diskStatus.clear(); updateStatus(); }
        });
    };
    // Start after the navigation/controller callbacks exist; destruction joins the worker before they disappear.
    VolumeManager volumeManager(app, [&](std::vector<DeviceVolume> fresh, std::string error) {
        if (demoDevices) return;
        if (!error.empty()) {
            diskServiceError = std::move(error);
            rebuildSidebar();
            updateStatus();
            return;
        }
        std::vector<std::string> vanished;
        for (const auto& old : devices) {
            if (old.mountPoint.empty()) continue;
            const bool present = std::any_of(fresh.begin(), fresh.end(), [&](const auto& v) {
                return v.id == old.id && v.mountPoint == old.mountPoint;
            });
            if (!present) vanished.push_back(old.mountPoint);
        }
        devices = std::move(fresh);
        diskServiceError = std::move(error);
        bool leftDevice = false;
        for (const auto& mount : vanished) {
            if (isInside(browser->path, mount)) {
                quickLook->dismiss();
                navigateTo(home, {}, false);
                leftDevice = true;
            }
            std::erase_if(back, [&](const auto& h) { return isInside(h.path, mount); });
            std::erase_if(forward, [&](const auto& h) { return isInside(h.path, mount); });
            std::erase_if(clipboard.paths, [&](const auto& p) { return isInside(p, mount); });
        }
        columnBrowser->roots = {home, "/", trashFilesDirectory()};
        for (const auto& v : mountedVolumes()) columnBrowser->roots.push_back(v.mountPoint);
        for (const auto& v : devices) if (v.visible() && !v.mountPoint.empty()) {
            if (std::find(columnBrowser->roots.begin(), columnBrowser->roots.end(), v.mountPoint) == columnBrowser->roots.end())
                columnBrowser->roots.push_back(v.mountPoint);
        }
        listingCache.clear();
        if (mode == ViewMode::Columns) columnBrowser->rebuild(false);
        rebuildSidebar();
        updateStatus();
        if (leftDevice && !diskBusy) diskNotice(tr("Device disconnected, returned to home"));
    });
    removeVolume = [&](const std::string& id) {
        if (demo || diskBusy) return;
        if (ops.busy()) { showAlert(tr("Device is in use"), tr("Please wait for file operations to complete before ejecting.")); return; }
        auto selected = std::find_if(devices.begin(), devices.end(), [&](const auto& v) { return v.id == id; });
        if (selected == devices.end()) return;
        const auto device = *selected;
        if (!device.canRemove()) return;
        // Release our own previews/searches and folder watches before asking UDisks to unmount.
        previews.cancelPending();
        for (const auto& v : devices) if (sameDevice(device, v) && !v.mountPoint.empty() && isInside(browser->path, v.mountPoint)) {
            quickLook->dismiss();
            navigateTo(home, {}, false);
            break;
        }
        diskBusy = true;
        activeDevice = id;
        ++diskNoticeGeneration;
        diskStatus = tr("Ejecting ") + "“" + device.name + "”…";
        rebuildSidebar();
        updateStatus();
        volumeManager.perform(id, VolumeManager::Action::Remove, [&, name = device.name](VolumeManager::Result result) {
            diskBusy = false;
            activeDevice.clear();
            rebuildSidebar();
            diskNotice(result.error.empty() ? "“" + name + (result.poweredOff ? "” " + tr("was safely ejected and can be removed.") : "” " + tr("was unmounted and can be removed.")) : tr("Eject failed"));
            if (!result.error.empty()) showAlert(tr("Cannot eject") + " “" + name + "”", result.error);
        });
    };
    sidebar.node()->setOnAction([&](const std::string& id) {
        if (id == "pulsar-cloud://choose" || id == "cloud_header_add") {
            spawnDetached({"pulsar-cloud", "choose"});
            return;
        }
        removeVolume(id);
    });
    ejectButton.onClick([&] { if (const auto* v = deviceForPath(browser->path)) removeVolume(v->id); });
    sidebar.onContextMenu([&](const std::string& id, Point at) {
        bool isCustom = false;
        for (const auto& cf : customFavorites) {
            if (cf.path == id) { isCustom = true; break; }
        }
        if (isCustom) {
            menu->setItems({
                MenuItem(tr("Remove from Sidebar"), [&, id] { removeFavorite(id); }).withIcon(Icon::Trash)
            });
            menu->setGap(2);
            page->menuAnchor->arrange({at.x, at.y, 1, 1});
            menu->showFrom(*page->menuAnchor);
            if (auto* h = page->host()) h->setFocus(menu.get(), false);
        }
    });
    sidebar.onSelect([&](std::string id) {
        if (id.empty()) return;
        if (id == "applications://pulsar") {
            spawnDetached({"pulsar-apps-viewer"});
            return;
        }
        if (id == "pulsar-cloud://choose" || id == "cloud_header_add") {
            spawnDetached({"pulsar-cloud", "choose"});
            return;
        }
        if (id.starts_with("tag:")) {
            navigateTo(id, {}, true);
            return;
        }
        auto selected = std::find_if(devices.begin(), devices.end(), [&](const auto& v) { return v.id == id; });
        if (selected != devices.end()) {
            if (diskBusy) return;
            const auto device = *selected;
            if (!device.mountPoint.empty()) { navigateTo(device.mountPoint, {}, true); return; }
            if (demo) return;
            diskBusy = true;
            activeDevice = id;
            ++diskNoticeGeneration;
            diskStatus = tr("Mounting ") + "“" + device.name + "”…”";
            rebuildSidebar();
            updateStatus();
            volumeManager.perform(id, VolumeManager::Action::Mount, [&, id, name = device.name](VolumeManager::Result result) {
                diskBusy = false;
                activeDevice.clear();
                rebuildSidebar();
                if (!result.error.empty()) {
                    diskNotice(tr("Mount failed"));
                    showAlert(tr("Cannot mount") + " “" + name + "”", result.error);
                } else {
                    // Update the cached mount immediately so selection and the toolbar don't wait for polling.
                    for (auto& v : devices) if (v.id == id) v.mountPoint = result.mountPoint;
                    rebuildSidebar();
                    navigateTo(result.mountPoint, {}, true);
                    diskNotice("“" + name + "” " + tr("mounted"));
                }
            });
            return;
        }
        std::error_code ec;
        if (id == trashFilesDirectory()) fs::create_directories(id, ec);
        navigateTo(id, {}, true);
    });

    // Toolbar -----------------------------------------------------------------------------------------------
    emptyTrashButton.onClick([&] { emptyTrash(); });
    backButton.onClick(goBack);
    forwardButton.onClick(goForward);
    viewControl.onChange([&](int i) { setMode(static_cast<ViewMode>(i)); });
    auto toolbar = Toolbar({
        ToolbarGroup({backButton, forwardButton}),
        titleLabel,
        Spacer(),
        emptyTrashButton,
        ejectButton,
        viewControl,
        ToolbarGroup({actionMenu}),
        searchField,
    });
    window.setToolbar(toolbar);

    // Keyboard ------------------------------------------------------------------------------------------------
    window.onKey([&](const KeyEvent& e) {
        const bool ctrl = e.primary();
        const bool shift = e.shift();
        if (e.alt() && !ctrl) {
            if (e.key == Key::Left) return goBack(), true;
            if (e.key == Key::Right) return goForward(), true;
            if (e.key == Key::Up) return goUp(), true;
        }
        if (ctrl && e.alt() && e.key == Key::C) return copyPath(), true;
        if (ctrl && shift) {
            switch (e.key) {
                case Key::N: newFolder(); return true;
                case Key::G: goToFolder(); return true;
                case Key::H: navigateTo(home, {}, true); return true;
                case Key::D: navigateTo(userDirectory("DESKTOP", "Desktop"), {}, true); return true;
                case Key::L: navigateTo(userDirectory("DOWNLOAD", "Downloads"), {}, true); return true;
                case Key::O: navigateTo(userDirectory("DOCUMENTS", "Documents"), {}, true); return true;
                case Key::Backspace:
                case Key::Delete: emptyTrash(); return true;
                case Key::Tab: prevTab(); return true;
                case Key::T: tabBar->setHidden(tabBar->isVisible()); page->invalidateLayout(); return true;
                default: break;
            }
        }
        if (ctrl && !shift) {
            switch (e.key) {
                case Key::T: newTab(""); return true;
                case Key::Tab: nextTab(); return true;
                case Key::PageDown: nextTab(); return true;
                case Key::PageUp: prevTab(); return true;
                case Key::E:
                    if (const auto* v = deviceForPath(browser->path); v && v->canRemove()) removeVolume(v->id);
                    return true;
                case Key::Num1: setMode(ViewMode::Icons); return true;
                case Key::Num2: setMode(ViewMode::List); return true;
                case Key::Num3: setMode(ViewMode::Columns); return true;
                case Key::Up: goUp(); return true;
                case Key::Down:
                case Key::O: openSelection(); return true;
                case Key::Backspace: moveToTrashSelected(); return true;
                case Key::C: copySelection(false); return true;
                case Key::X: copySelection(true); return true;
                case Key::V: paste(); return true;
                case Key::D: duplicate(); return true;
                case Key::A: selectAll(); return true;
                case Key::Z: undo(); return true;
                case Key::I: showInfo(); return true;
                case Key::H: toggleHidden(); return true;
                case Key::R: showInEnclosingFolder(); return true;
                case Key::L: goToFolder(); return true;
                case Key::F:
                    if (auto* h = page->host()) h->setFocus(searchField.node().get(), true);
                    searchField->selectAll();
                    return true;
                case Key::W:
                    if (tabs.size() > 1) {
                        closeTab(activeTab);
                    } else {
                        if (allowClose()) window.close();
                    }
                    return true;
                case Key::Comma: openSettings(); return true;
                default: break;
            }
        }
        if (!ctrl && !e.alt()) {
            switch (e.key) {
                case Key::Delete: moveToTrashSelected(); return true;
                case Key::F2: renameSelected(); return true;
                case Key::F5: reload(); return true;
                case Key::Escape:
                    if (browser->searching) {
                        searchField.setText("");
                        startSearch("");
                        return true;
                    }
                    if (!browser->selection.empty()) browser->select({});
                    return true;
                default: break;
            }
        }
        return false;
    });

    // Start ------------------------------------------------------------------------------------------------------
    window.setContent(split);
    if (demoDevices) {
        DeviceVolume mounted;
        mounted.id = "demo:studio";
        mounted.drive = "demo:drive1";
        mounted.name = "Studio USB";
        mounted.mountPoint = argc > 1 ? expandUser(argv[1]) : home;
        mounted.size = 128ULL * 1000 * 1000 * 1000;
        mounted.external = mounted.mountable = mounted.canPowerOff = true;
        DeviceVolume unmounted = mounted;
        unmounted.id = "demo:travel";
        unmounted.drive = "demo:drive2";
        unmounted.name = "Backup Drive";
        unmounted.size = 32ULL * 1000 * 1000 * 1000;
        unmounted.mountPoint.clear();
        devices = {mounted, unmounted};
    }
    rebuildSidebar();

    // Restore the last session's choices.
    browser->showHidden = prefs.getBool("showHidden");
    {
        const std::string by = prefs.getString("sortBy", "name");
        browser->sort.key = by == "modified" ? SortKey::Modified : by == "size" ? SortKey::Size
                            : by == "kind"   ? SortKey::Kind
                                             : SortKey::Name;
        browser->sort.ascending = prefs.getBool("sortAscending", true);
        browser->sort.foldersFirst = prefs.getBool("foldersOnTop", true);
    }
    browser->iconSize = std::clamp(std::round(prefs.getFloat("iconSize", 64)), 32.0f, 128.0f);
    sizeSlider.setValue(browser->iconSize);
    columns->modified = std::clamp(prefs.getFloat("list.modifiedWidth", columns->modified), 50.0f, 400.0f);
    columns->size = std::clamp(prefs.getFloat("list.sizeWidth", columns->size), 50.0f, 400.0f);
    columns->kind = std::clamp(prefs.getFloat("list.kindWidth", columns->kind), 50.0f, 400.0f);
    pathBar->setHidden(!prefs.getBool("showPathBar", true));
    statusBar.setHidden(!prefs.getBool("showStatusBar", true));
    {
        const std::string view = prefs.getString("view", "icons");
        setMode(view == "list" ? ViewMode::List : view == "columns" ? ViewMode::Columns : ViewMode::Icons);
    }
    // "New window opens:": home (default), Recents, or the last folder visited.
    std::string start = home;
    if (const std::string where = prefs.getString("startIn", "home"); where == "recents") {
        start = kRecents;
    } else if (where == "last") {
        std::error_code ec;
        const std::string last = prefs.getString("lastFolder");
        if (last == kRecents || (!last.empty() && fs::is_directory(last, ec))) start = last;
    }
    if (!startFolder.empty()) {
        std::error_code ec;
        if (fs::is_directory(startFolder, ec)) start = startFolder;
    }
    std::string initialSelect;
    if (!selectItem.empty()) {
        std::error_code ec;
        if (fs::exists(selectItem, ec)) {
            start = parentPath(selectItem);
            initialSelect = selectItem;
        }
    }
    {
        std::error_code ec;
        if (start != kRecents) {
            if (!fs::is_directory(start, ec)) start = home;
            start = fs::absolute(start, ec).lexically_normal().string();
        }
        if (start.size() > 1 && start.back() == '/') start.pop_back();
    }
    if (!tabs.empty()) {
        tabs[0].path = start;
        tabs[0].title = displayName(start);
        tabs[0].mode = mode;
    }
    navigateTo(start, initialSelect, false);

    // MAYO_DEMO=<step>[,<step>...] drives the window for screenshots and smoke tests. Steps: list, columns,
    // select:<name>, shiftselect:<name> (list mode, no expanded folders; Shift double-click regression),
    // quicklook[:name], info[:name], openwith[:name], submenu:<row>, choose:<app id>,
    // perms:<owner|group|others>:<rw|r|none>, menu[:name], rename[:name], search:<query>, size:<points>, narrow,
    // scroll:<points>, overflow, goto, open:<name>, back, up, newfolder, trash:<name>, undo, duplicate:<name>,
    // copy:<name>, cut:<name>, paste, renameto:<old>=<new>, drop:<item>=<folder>, hidden, type:<letters>, settings,
    // closesettings, input:<text>, key:<enter|escape>, wait:<seconds>.
    if (const char* env = std::getenv("MAYO_DEMO")) {
        auto selectNamed = [&](const std::string& name) {
            for (const auto& e : browser->entries) {
                if (e.name == name) {
                    browser->select(e.path);
                    revealLater(e.path);
                    return;
                }
            }
            if (!browser->entries.empty() && name.empty()) browser->select(browser->entries.back().path);
        };
        std::stringstream steps(env);
        std::string step;
        double at = 0.2;
        while (std::getline(steps, step, ',')) {
            const auto colon = step.find(':');
            const std::string what = step.substr(0, colon);
            const std::string arg = colon == std::string::npos ? std::string() : step.substr(colon + 1);
            app.after(at, [&, what, arg, selectNamed] {
                if (what == "list") setMode(ViewMode::List);
                else if (what == "narrow") window.setSize({680, 600});
                else if (what == "scroll") {
                    const float offset = static_cast<float>(std::atof(arg.c_str()));
                    if (mode == ViewMode::Icons) iconScroll.scrollTo({0, offset}, false);
                    else if (mode == ViewMode::List) listScroll.scrollTo({0, offset}, false);
                    else columnScroll.scrollTo({offset, 0}, false);
                } else if (what == "overflow") {
                    for (const auto& child : toolbar.node()->children()) {
                        if (auto button = std::dynamic_pointer_cast<ButtonView>(child);
                            button && button->icon() == Icon::Ellipsis && button->isVisible()) button->click();
                    }
                } else if (what == "columns") setMode(ViewMode::Columns);
                else if (what == "select") selectNamed(arg);
                else if (what == "shiftselect" && mode == ViewMode::List) {
                    const auto found = std::find_if(browser->entries.begin(), browser->entries.end(),
                                                    [&](const FileEntry& entry) { return entry.name == arg; });
                    if (found != browser->entries.end()) {
                        const auto index = static_cast<float>(std::distance(browser->entries.begin(), found));
                        const auto r = listRows->frame();
                        const PointerEvent event{
                            {r.x + theme().metrics.spacingLarge, r.y + (index + 0.5f) * ListRowsView::kRowHeight},
                            MouseButton::Left,
                            2,
                            Modifiers::Shift};
                        listRows->onPointerDown(event);
                        listRows->onPointerUp(event);
                    }
                } else if (what == "size")
                    browser->resizeIcons(static_cast<float>(std::atof(arg.c_str())));
                else if (what == "quicklook") {
                    if (!arg.empty()) selectNamed(arg);
                    browser->quickLook();
                } else if (what == "info") {
                    if (!arg.empty()) selectNamed(arg);
                    showInfo();
                } else if (what == "rename") {
                    if (!arg.empty()) selectNamed(arg);
                    renameSelected();
                } else if (what == "menu") {
                    if (!arg.empty()) selectNamed(arg);
                    if (auto* v = currentView()) {
                        const Rect r = v->frame();
                        browser->contextMenu(*v, {r.x + 140, r.y + 150}, !arg.empty());
                    }
                } else if (what == "openwith") {
                    // The tr("Open With") panel for an item (no application is started in demo mode).
                    if (!arg.empty()) {
                        if (const auto* found = browser->find(arg)) {
                            showOpenWith(*found);
                        } else {
                            for (const auto& item : browser->entries) {
                                if (item.name == arg) {
                                    showOpenWith(item);
                                    break;
                                }
                            }
                        }
                    }
                } else if (what == "clickrow") {
                    // Clicks the context menu's row `arg` (e.g. tr("Open With")), as a pointer would.
                    app.after(0.15, [&, arg] {
                        const auto& items = menu->items();
                        for (size_t i = 0; i < items.size(); ++i) {
                            if (items[i].title == arg) {
                                menu->setHighlighted(static_cast<int>(i));
                                break;
                            }
                        }
                    });
                } else if (what == "submenu") {
                    // Highlights the context menu's row `arg` (0-based) so its submenu opens beside it.
                    app.after(0.1, [&, arg] { menu->setHighlighted(std::atoi(arg.c_str())); });
                } else if (what == "perms") {
                    // perms:owner=rw — a demo of the permission editor's effect (chmod on the scratch folder).
                    const auto eq = arg.find('=');
                    const std::string who = arg.substr(0, eq);
                    const std::string access = arg.substr(eq + 1);
                    if (who.empty() || browser->selection.empty()) return;
                    if (const auto target = statEntry(*browser->selection.begin())) {
                        const int index = who == "owner" ? 0 : who == "group" ? 1 : 2;
                        const unsigned bits = 1u << (8 - index * 3);
                        const unsigned write = 1u << (7 - index * 3);
                        unsigned flags = target->mode & ~(bits | write);
                        if (access == "rw") flags |= bits | write;
                        else if (access == "r") flags |= bits;
                        if (::chmod(target->path.c_str(), flags) != 0) {
                            showAlert(tr("Cannot change permissions."), std::strerror(errno));
                        } else {
                            reload();
                        }
                    }
                } else if (what == "search") {
                    searchField.setText(arg);
                    startSearch(arg);
                } else if (what == "goto") {
                    goToFolder();
                } else if (what == "settings") {
                    openSettings();
                } else if (what == "closesettings") {
                    if (settingsWindow) settingsWindow->close();
                } else if (what == "open") {
                    selectNamed(arg);
                    openSelection();
                } else if (what == "back") {
                    goBack();
                } else if (what == "up") {
                    goUp();
                } else if (what == "newfolder") {
                    newFolder();
                } else if (what == "newtab") {
                    newTab(arg);
                } else if (what == "closetab") {
                    closeTab(activeTab);
                } else if (what == "trash") {
                    selectNamed(arg);
                    moveToTrashSelected();
                } else if (what == "undo") {
                    undo();
                } else if (what == "duplicate") {
                    selectNamed(arg);
                    duplicate();
                } else if (what == "copy" || what == "cut") {
                    selectNamed(arg);
                    copySelection(what == "cut");
                } else if (what == "paste") {
                    paste();
                } else if (what == "renameto") {  // renameto:old=new
                    const auto eq = arg.find('=');
                    selectNamed(arg.substr(0, eq));
                    if (!browser->selection.empty()) browser->rename(*browser->selection.begin(), arg.substr(eq + 1));
                } else if (what == "drop") {  // drop:item=folder
                    const auto eq = arg.find('=');
                    selectNamed(arg.substr(0, eq));
                    auto paths = browser->selectedPaths();
                    browser->drop(paths, (fs::path(browser->path) / arg.substr(eq + 1)).string(), false);
                } else if (what == "hidden") {
                    toggleHidden();
                } else if (what == "type") {
                    for (char ch : arg) {
                        if (auto* v = currentView(); v && ch >= 'a' && ch <= 'z')
                            v->onKeyDown(KeyEvent{static_cast<Key>(static_cast<int>(Key::A) + (ch - 'a'))});
                    }
                } else if (what == "input") {
                    if (auto* h = page->host(); h && h->focusedWidget())
                        h->focusedWidget()->onTextInput({arg});
                } else if (what == "key") {
                    if (auto* h = page->host(); h && h->focusedWidget()) {
                        if (arg == "enter") h->focusedWidget()->onKeyDown({Key::Enter});
                        else if (arg == "escape") h->focusedWidget()->onKeyDown({Key::Escape});
                    }
                }
            });
            at += what == "wait" ? std::atof(arg.c_str()) : 0.15;
        }
    }
    return app.run();
}
