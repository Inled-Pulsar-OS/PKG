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
#include "file_model.hpp"
#include "file_ops.hpp"
#include "panels.hpp"
#include "preview_cache.hpp"
#include "tag_manager.hpp"
#include "views.hpp"
#include <iostream>

namespace fs = std::filesystem;
using namespace mayo;
using namespace finder;

namespace {

constexpr float kPathBarHeight = 28;
constexpr float kStatusBarHeight = 26;

/// Detail area. Icons scroll under the toolbar glass; list and columns start below it. Path bar and
/// status bar sit along the bottom.
class FinderPage : public Widget {
public:
    std::shared_ptr<Widget> iconScroll, listHeader, listScroll, columnScroll, empty, bottom, pathBar, statusBar;
    std::shared_ptr<Widget> menuAnchor = std::make_shared<Widget>();
    ViewMode mode = ViewMode::Icons;

    [[nodiscard]] bool extendsIntoTitleBar() const override { return true; }
    [[nodiscard]] bool expandsContent(Axis /*axis*/) const override { return true; }

    void arrangeContent(const Rect& c) override {
        const float bar = host() ? host()->titleBarHeight() : 52.0f;
        const float bottomH = bottomHeight();
        const Rect area{c.x, c.y, c.width, c.height - bottomH};
        const Rect below{c.x, c.y + bar, c.width, area.height - bar};
        auto place = [](const std::shared_ptr<Widget>& w, const Rect& r) {
            w->measure(r.size());
            w->arrange(r);
        };
        static_cast<ScrollViewView*>(iconScroll.get())->setContentInsetTop(bar);
        place(iconScroll, area);
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
            if (pathBar->isVisible() && statusBar->isVisible())
                p.fillRect({b.x, p.snap(b.y + kPathBarHeight), b.width, px}, c.separator.multipliedAlpha(0.6f));
        }
        paintChildren(p);
    }

    [[nodiscard]] float bottomHeight() const {
        if (!bottom) return 0.0f;
        return bottom->measure({1000, 1000}).height;
    }
};

/// "Go to Folder" sheet (Ctrl+Shift+G).
class GoToFolderPanel : public CenteredPanel {
public:
    explicit GoToFolderPanel(std::function<void(const std::string&)> go) : CenteredPanel(Material::Popover) {
        setPadding(Insets(18));
        field = TextField("Enter a path, e.g. ~/Documents");
        field.onSubmit([this, go](const std::string& text) {
            auto keep = shared_from_this();
            dismiss();
            go(text);
        });
        auto cancel = Button("Cancel").onClick([this] { dismiss(); });
        auto open = Button("Go").style(ButtonStyle::Prominent).onClick([this, go] {
            auto keep = shared_from_this();
            std::string text = field.value();
            dismiss();
            go(text);
        });
        auto stack = VStack({Label("Go to Folder").font(TextStyle::Headline), field,
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
        {.title = customTitle.empty() ? "Finder" : customTitle.c_str(),
         .width = 1120, .height = 720, .minWidth = 680, .minHeight = 420, .autosaveName = "main"});
    Settings& prefs = app.settings();
    std::function<void()> syncSettings;  // refreshes the Settings window's controls while it is open
    window.setTitleVisible(false);  // the toolbar shows the folder name after the navigation buttons
    const bool demo = std::getenv("MAYO_DEMO") != nullptr;  // never launch other programs from demo runs

    auto browser = std::make_shared<Browser>();
    ViewMode mode = ViewMode::Icons;

    struct HistoryItem {
        std::string path;
        std::string select;
    };
    std::vector<HistoryItem> back, forward;
    struct Clipboard {
        std::vector<std::string> paths;
        bool cut = false;
    } clipboard;
    std::map<std::string, std::vector<FileEntry>> listingCache;
    std::string searchQuery;
    int searchGeneration = 0;

    // Places ------------------------------------------------------------------------------------
    const std::string home = homeDirectory();
    struct Place {
        std::string path;
        const char* title;
    };
    std::vector<Place> favorites{{kRecents, "Recents"}, {home, "Home"}};
    for (auto [key, fallback, title] : {std::tuple{"DESKTOP", "Desktop", "Desktop"},
                                        std::tuple{"DOCUMENTS", "Documents", "Documents"},
                                        std::tuple{"DOWNLOAD", "Downloads", "Downloads"},
                                        std::tuple{"PICTURES", "Pictures", "Pictures"},
                                        std::tuple{"MUSIC", "Music", "Music"}, std::tuple{"VIDEOS", "Videos", "Videos"}}) {
        std::string p = userDirectory(key, fallback);
        std::error_code ec;
        if (!p.empty() && fs::is_directory(p, ec)) favorites.push_back({p, title});
    }

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
    auto listScroll = ScrollView(Element(listRows));
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
    auto opLabel = Label("").font(TextStyle::Subheadline).secondary();
    auto statusLabel = Label("").font(TextStyle::Subheadline).secondary().textAlign(TextAlign::Center);
    auto sizeSlider = Slider(64, 32, 128).width(110).tooltip("Icon size (Ctrl+scroll)");
    auto statusBar = ZStack({HStack({opSpinner, opLabel, Spacer(), sizeSlider}).spacing(6).padding(0, 12), statusLabel});
    statusBar.height(kStatusBarHeight);
    auto pickerCancel = Button("Cancel").style(ButtonStyle::Default).onClick([&] {
        std::exit(1);
    });
    auto pickerFilenameField = TextField("File name").width(220);
    if (!saveFilename.empty()) pickerFilenameField.setText(saveFilename);
    auto submitPicker = [&] {
        if (!pickerMode) return;
        if (saveMode) {
            std::string name = pickerFilenameField.value();
            if (name.empty()) name = saveFilename.empty() ? "Untitled" : saveFilename;
            std::cout << (fs::path(browser->path) / name).string() << std::endl;
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
            if (!sel.empty()) {
                for (size_t i = 0; i < sel.size(); ++i) {
                    if (i > 0) std::cout << (chooseMultiple ? "\n" : " ");
                    std::cout << sel[i];
                }
                std::cout << std::endl;
                std::exit(0);
            }
        }
    };
    auto pickerSelect = Button(saveMode ? "Save" : "Select").style(ButtonStyle::Prominent).onClick(submitPicker);

    HStack pickerBar;
    if (saveMode) {
        auto saveStack = HStack({Element(Label("Save As:")), Element(pickerFilenameField.node())}).spacing(8);
        pickerBar = HStack({Element(saveStack.node()), Element(Spacer().node()), Element(pickerCancel.node()), Element(pickerSelect.node())}).spacing(10).padding(6, 16);
    } else {
        pickerBar = HStack({Element(Spacer().node()), Element(pickerCancel.node()), Element(pickerSelect.node())}).spacing(10).padding(6, 16);
    }
    pickerBar.height(44);

    VStack bottom;
    if (pickerMode) {
        bottom = VStack({Element(pathBar), Element(pickerBar.node()), Element(statusBar.node())}).spacing(0);
    } else {
        bottom = VStack({Element(pathBar), Element(statusBar.node())}).spacing(0);
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
    for (auto& w : {page->iconScroll, page->listHeader, page->listScroll, page->columnScroll, page->empty, page->bottom,
                    page->menuAnchor}) {
        page->addChild(w);
    }

    auto sidebar = Sidebar(std::vector<SidebarItem>{});
    auto split = SplitView(sidebar, Element(page)).sidebarWidth(200).sidebarLimits(160, 300);

    auto backButton = Button(Icon::ChevronLeft).style(ButtonStyle::Borderless).tooltip("Back (Alt+←)");
    auto forwardButton = Button(Icon::ChevronRight).style(ButtonStyle::Borderless).tooltip("Forward (Alt+→)");
    auto titleLabel = Label("").font(TextStyle::Title3).lineLimit(1).maxWidth(320);
    auto viewControl = SegmentedControl({Icon::Grid, Icon::List, Icon::Columns}, 0);
    viewControl.tooltip("View (Ctrl+1, Ctrl+2, Ctrl+3)");
    auto actionMenu = Menu("", {}).icon(Icon::Ellipsis).borderless().tooltip("Actions");
    auto searchField = SearchField("Search").width(190);
    auto emptyTrashButton = Button("Empty").tooltip("Empty Trash (Ctrl+Shift+Delete)");

    auto quickLook = std::make_shared<QuickLookView>(browser);
    auto menu = std::make_shared<MenuView>(std::vector<MenuItem>{});

    FileOps ops(app, [&](const std::string& message) {
        if (!page->host()) return;
        std::make_shared<AlertView>("The operation can't be completed.", message,
                                    std::vector<AlertButton>{{"OK", {}, ButtonStyle::Prominent}}, true)
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
    auto isRealFolder = [&] { return !browser->searching && browser->path != kRecents && browser->path.rfind("tag:", 0) != 0; };
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
            text = (n == 1 ? "1 result" : std::to_string(n) + " results");
        } else if (!b.selection.empty()) {
            text = std::to_string(b.selection.size()) + " of " + std::to_string(n) + " selected";
        } else {
            text = itemCount(n);
        }
        if (auto free = availableSpace(b.path == kRecents ? home : b.path)) text += ", " + formatSize(*free) + " available";
        statusLabel.setText(text);
        const bool busy = ops.busy();
        opSpinner.setHidden(!busy);
        opLabel.setText(busy ? ops.status() : std::string());
        sizeSlider.setHidden(mode != ViewMode::Icons);
        emptyTrashButton.setHidden(!isInside(b.path, trashFilesDirectory()) || b.searching);
        emptyTrashButton.setDisabled(b.entries.empty());
        pathBar->invalidate();
        // Cheap enough to do on every change; keeps checkmarks and enabled states honest.
        if (refreshActionMenu) refreshActionMenu();
    };

    auto updateChrome = [&] {
        const std::string name = browser->searching ? "Searching “" + searchQuery + "”" : displayName(browser->path);
        titleLabel.setText(name);
        window.setTitle(browser->searching ? name : displayName(browser->path));
        backButton.setDisabled(back.empty());
        forwardButton.setDisabled(forward.empty());
        std::string sel;
        if (!browser->searching) {
            for (const auto& f : favorites) {
                if (f.path == browser->path) sel = f.path;
            }
            if (browser->path == "/" || browser->path == trashFilesDirectory()) sel = browser->path;
            if (browser->path.rfind("tag:", 0) == 0) sel = browser->path;
            for (const auto& v : mountedVolumes()) {
                if (v.mountPoint == browser->path) sel = v.mountPoint;
            }
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
            emptyLabel.setText("Can't Open “" + displayName(browser->path) + "”");
            emptyDetail.setText("You may not have permission to see its contents.");
        } else if (browser->searching) {
            emptyLabel.setText("No Results");
            emptyDetail.setText("Nothing below “" + displayName(browser->path) + "” matches “" + searchQuery + "”.");
        } else if (browser->path.rfind("tag:", 0) == 0) {
            emptyLabel.setText("No Files Tagged “" + browser->path.substr(4) + "”");
            emptyDetail.setText("Right-click files to assign this color tag.");
        } else if (browser->path == kRecents) {
            emptyLabel.setText("No Recent Files");
            emptyDetail.setText("Files you open in other apps appear here.");
        } else {
            emptyLabel.setText("Trash Is Empty");
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
    ops.onChanged = [&] { updateStatus(); };
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
    std::function<void(const std::string&, const std::string&, bool)> navigateTo;
    navigateTo = [&](const std::string& rawPath, const std::string& select, bool recordHistory) {
        std::string path = rawPath;
        if (path != kRecents && path.rfind("tag:", 0) != 0) {
            std::error_code ec;
            if (!fs::is_directory(path, ec)) {
                if (fs::exists(path, ec)) {  // a file: show it selected in its folder
                    navigateTo(parentPath(path), path, recordHistory);
                    return;
                }
                if (page->host()) {
                    std::make_shared<AlertView>("The folder can't be found.", "“" + path + "” doesn't exist.",
                                                std::vector<AlertButton>{{"OK", {}, ButtonStyle::Prominent}}, true)
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
    auto goUp = [&] {
        if (!isRealFolder()) return;
        const std::string parent = parentPath(browser->path);
        if (parent != browser->path) navigateTo(parent, browser->path, true);
    };

    auto setMode = [&](ViewMode m) {
        if (auto* v = currentView(); v && v->isRenaming()) focusView();
        mode = m;
        iconScroll.setHidden(m != ViewMode::Icons);
        listHeader->setHidden(m != ViewMode::List);
        listScroll.setHidden(m != ViewMode::List);
        columnScroll.setHidden(m != ViewMode::Columns);
        viewControl->setSelected(static_cast<int>(m));
        prefs.set("view", m == ViewMode::Icons ? "icons" : m == ViewMode::List ? "list" : "columns");
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
        std::make_shared<AlertView>(title, message, std::vector<AlertButton>{{"OK", {}, ButtonStyle::Prominent}}, true)
            ->show(*page->host());
    };

    auto isArchive = [](const std::string& path) {
        std::string s = path;
        for (auto& c : s) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
        for (const char* ext : {".zip", ".tar.gz", ".tgz", ".tar.xz", ".txz", ".tar.bz2", ".tbz2", ".tar.zst", ".tar", ".7z"}) {
            if (s.size() >= std::strlen(ext) && s.rfind(ext) == s.size() - std::strlen(ext)) return true;
        }
        return false;
    };

    auto archiveBaseName = [](const std::string& path) {
        std::string fn = fs::path(path).filename().string();
        for (const char* ext : {".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst", ".tgz", ".txz", ".tbz2", ".zip", ".tar", ".7z"}) {
            if (fn.size() >= std::strlen(ext)) {
                size_t pos = fn.size() - std::strlen(ext);
                std::string sub = fn.substr(pos);
                for (auto& c : sub) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
                if (sub == ext) return fn.substr(0, pos);
            }
        }
        return fs::path(path).stem().string();
    };

    auto extractArchiveTo = [](const std::string& archivePath, const std::string& destDir) {
        std::error_code ec;
        fs::create_directories(destDir, ec);
        std::string s = archivePath;
        for (auto& c : s) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
        std::vector<std::string> cmd;
        if (s.size() >= 4 && s.rfind(".zip") == s.size() - 4) {
            cmd = {"unzip", "-q", "-o", archivePath, "-d", destDir};
        } else if (s.size() >= 3 && s.rfind(".7z") == s.size() - 3) {
            cmd = {"7z", "x", "-y", "-o" + destDir, archivePath};
        } else {
            cmd = {"tar", "-xf", archivePath, "-C", destDir};
        }
        spawnDetached(cmd, destDir);
    };

    browser->open = [&](const FileEntry& e) {
        if (pickerMode && !e.isDir && !saveMode) {
            std::cout << e.path << std::endl;
            std::exit(0);
        }
        if (e.isDir) {
            navigateTo(e.path, {}, true);
        } else if (isInTrash(e.path)) {
            showAlert("“" + e.name + "” is in the Trash.", "To use this item, first drag it out of the Trash or use Put Back.");
        } else if (isArchive(e.path)) {
            std::string parent = isRealFolder() ? browser->path : parentPath(e.path);
            std::string target = uniqueName(parent, archiveBaseName(e.path));
            extractArchiveTo(e.path, target);
            app.after(0.6, [&, parent, target] {
                if (browser->path == parent) {
                    reload();
                    navigateTo(parent, target, false);
                }
            });
        } else if (!demo) {
            openWithDefaultApp(e.path);
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
        browser->selection.clear();
        for (const auto& p : paths) {
            if (parentPath(p) == browser->path) browser->selection.insert(p);
        }
        browser->lead = browser->anchor = paths.empty() ? std::string() : paths.front();
        browser->notifySelection();
        if (!paths.empty()) revealLater(paths.front());
    };

    auto moveToTrashSelected = [&] {
        auto paths = browser->selectedPaths();
        if (paths.empty()) return;
        if (inTrash()) {
            auto alert = std::make_shared<AlertView>(
                "Are you sure you want to delete " + (paths.size() == 1 ? "“" + fs::path(paths[0]).filename().string() + "”"
                                                                        : "these " + std::to_string(paths.size()) + " items") +
                    " immediately?",
                "You can't undo this action.",
                std::vector<AlertButton>{{"Cancel", {}},
                                         {"Delete", [&, paths] { ops.deleteImmediately(paths, [&](auto) { reload(); }); },
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
            "Are you sure you want to permanently erase the items in the Trash?", "You can't undo this action.",
            std::vector<AlertButton>{{"Cancel", {}},
                                     {"Empty Trash", [&] { ops.emptyTrash([&](auto) { reload(); }); }, ButtonStyle::Destructive}},
            true);
        alert->show(*page->host());
    };

    auto copySelection = [&](bool cut) {
        auto paths = browser->selectedPaths();
        if (paths.empty()) return;
        clipboard = {paths, cut};
        browser->cut = cut ? std::set<std::string>(paths.begin(), paths.end()) : std::set<std::string>{};
        std::string text;
        for (const auto& p : paths) text += (text.empty() ? "" : "\n") + p;
        if (auto* h = page->host()) h->setClipboardText(text);
        if (auto* v = currentView()) v->invalidate();
    };

    auto copyPath = [&] {
        auto paths = browser->selectedPaths();
        if (paths.empty() && isRealFolder()) paths.push_back(browser->path);
        std::string text;
        for (const auto& p : paths) text += (text.empty() ? "" : "\n") + p;
        if (auto* h = page->host()) h->setClipboardText(text);
    };

    auto paste = [&] {
        if (!isRealFolder()) return;
        std::vector<std::string> sources = clipboard.paths;
        bool cut = clipboard.cut;
        if (sources.empty()) {
            // Paths copied by other programs (one per line, or file:// URIs).
            std::stringstream ss(page->host() ? page->host()->clipboardText() : std::string());
            std::string line;
            while (std::getline(ss, line)) {
                line = expandUser(line);
                std::error_code ec;
                if (!line.empty() && line[0] == '/' && fs::exists(line, ec)) sources.push_back(line);
            }
            cut = false;
        }
        if (sources.empty()) return;
        auto done = [&](std::vector<std::string> created) { selectAfterOp(created); };
        if (cut) {
            ops.move(sources, browser->path, done);
            clipboard = {};
            browser->cut.clear();
        } else {
            ops.copy(sources, browser->path, done);
        }
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

    browser->rename = [&](const std::string& path, const std::string& newName) {
        std::string renamed = ops.rename(path, newName);
        if (renamed.empty()) return;
        if (browser->searching) {
            for (auto& e : browser->entries) {
                if (e.path == path) {
                    if (auto st = statEntry(renamed)) e = *st;
                }
            }
            browser->select(renamed);
            refreshViews(false);
            return;
        }
        reload();
        browser->select(renamed);
        revealLater(renamed);
        if (mode == ViewMode::Columns) columnBrowser->rebuild(true);
    };

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
            ops.copy(paths, folder, done);
        } else {
            ops.move(paths, folder, done);
        }
    };

    window.onFilesDropped([&](const std::vector<std::string>& paths) {
        if (isRealFolder() && !inTrash()) ops.copy(paths, browser->path, [&](std::vector<std::string> c) { selectAfterOp(c); });
    });

    // Get Info -------------------------------------------------------------------------------------------
    auto showInfo = [&] {
        auto paths = browser->selectedPaths();
        std::string path = paths.empty() ? (isRealFolder() ? browser->path : std::string()) : paths.front();
        auto st = path.empty() ? std::nullopt : statEntry(path);
        if (!st || !page->host()) return;
        const FileEntry e = *st;
        auto sizeValue = Label(e.isDir ? "Calculating…" : formatSize(e.size) + " (" + std::to_string(e.size) + " bytes)");
        auto row = [](const std::string& key, const Element& value) {
            return HStack({Label(key + ":").secondary().textAlign(TextAlign::Trailing).width(92), value}).spacing(8);
        };
        auto icon = Canvas([&previews, e](Painter& p, const Rect& r) {
            IconStyle s;
            s.preview = previews.get(e, r.width * p.scale());
            s.folderGlyph = e.isDir ? folderGlyphFor(e.path) : Icon::None;
            drawFileIcon(p, r, e, s);
        });
        icon.frame(56, 56);
        auto name = Label(e.name).font(TextStyle::Headline).lineLimit(2).maxWidth(250);
        auto headerText = VStack({name, Label(kindDescription(e)).secondary()}).spacing(2);
        std::vector<Element> rows{
            HStack({icon, headerText}).spacing(12).align(Alignment::Leading),
            Divider(),
            row("Kind", Label(kindDescription(e))),
            row("Size", sizeValue),
            row("Where", Label(parentPath(e.path)).lineLimit(3).maxWidth(220)),
        };
        if (e.created > 0) rows.push_back(row("Created", Label(formatDate(e.created))));
        rows.push_back(row("Modified", Label(formatDate(e.modified))));
        rows.push_back(row("Permissions", Label(formatPermissions(e.mode, e.isDir))));
        if (e.isLink) {
            std::error_code ec;
            rows.push_back(row("Original", Label(fs::read_symlink(e.path, ec).string()).lineLimit(2).maxWidth(220)));
        }
        if (isInTrash(e.path)) {
            if (auto original = trashOriginalPath(e.path)) rows.push_back(row("Put Back To", Label(parentPath(*original)).maxWidth(220)));
        }
        auto panel = std::make_shared<CenteredPanel>(Material::Popover);
        panel->setPadding(Insets(18));
        auto stack = VStack(rows).spacing(8);
        stack.width(340);
        panel->addChild(stack.node());
        if (e.isDir) {
            std::weak_ptr<Widget> weak = panel;
            ops.folderSize(e.path, [weak, sizeValue](std::uintmax_t bytes, size_t items) mutable {
                if (!weak.lock()) return;
                sizeValue.setText(formatSize(bytes) + " for " + itemCount(items));
            });
        }
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
        if (!demo) openTerminal(folder);
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
        auto& w = app.createWindow({.title = "Finder Settings", .width = 560, .height = 620, .minWidth = 480, .minHeight = 360});
        const std::string where = prefs.getString("startIn", "home");
        auto startPicker = Picker({"Home", "Recents", "Last Folder Visited"}, where == "recents" ? 1 : where == "last" ? 2 : 0)
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
            FormSection("General", {FormRow("New windows open", startPicker), FormRow("Show hidden files", hidden, "Ctrl+H"),
                                    FormRow("Keep folders on top", foldersOnTop)}),
            FormSection("Window", {FormRow("Show path bar", showPath), FormRow("Show status bar", showStatus)},
                        "Settings are saved in " + (isInside(prefs.directory(), home) ? "~" + prefs.directory().substr(home.size())
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
        const std::pair<SortKey, const char*> keys[] = {
            {SortKey::Name, "Name"}, {SortKey::Kind, "Kind"}, {SortKey::Modified, "Date Modified"}, {SortKey::Size, "Size"}};
        for (auto [key, title] : keys) {
            items.push_back(MenuItem(std::string("Sort by ") + title, [&, key] {
                                if (browser->sort.key == key) return;
                                browser->sort.key = key;
                                browser->sort.ascending = key == SortKey::Name || key == SortKey::Kind;
                                browser->sortChanged();
                            }).setChecked(browser->sort.key == key));
        }
        items.push_back(MenuItem("Keep Folders on Top", [&] {
                            browser->sort.foldersFirst = !browser->sort.foldersFirst;
                            browser->sortChanged();
                        }).setChecked(browser->sort.foldersFirst));
        return items;
    };

    auto viewItems = [&]() {
        std::vector<MenuItem> items;
        const std::pair<ViewMode, const char*> modes[] = {
            {ViewMode::Icons, "as Icons"}, {ViewMode::List, "as List"}, {ViewMode::Columns, "as Columns"}};
        int k = 1;
        for (auto [m, title] : modes) {
            items.push_back(MenuItem(std::string("View ") + title, [&, m] { setMode(m); })
                                .setChecked(mode == m)
                                .withShortcut("Ctrl+" + std::to_string(k++)));
        }
        return items;
    };

    auto backgroundItems = [&]() {
        std::vector<MenuItem> items;
        if (inTrash()) {
            items.push_back(MenuItem("Empty Trash", emptyTrash).asDestructive().disabled(browser->entries.empty()));
            items.push_back(MenuItem::separator());
        }
        items.push_back(MenuItem("New Folder", newFolder).withIcon(Icon::FolderPlus).withShortcut("Ctrl+Shift+N")
                            .disabled(!isRealFolder() || inTrash()));
        const size_t clip = clipboard.paths.size();
        items.push_back(MenuItem(clip > 1 ? "Paste " + std::to_string(clip) + " Items" : "Paste Item", paste)
                            .withShortcut("Ctrl+V")
                            .disabled(!isRealFolder() || inTrash()));
        items.push_back(MenuItem::separator());
        items.push_back(MenuItem("Get Info", showInfo).withIcon(Icon::Info).withShortcut("Ctrl+I").disabled(!isRealFolder()));
        items.push_back(MenuItem("Open in Terminal", openTerminalHere).withIcon(Icon::Terminal));
        items.push_back(MenuItem("Copy Path", copyPath).withShortcut("Ctrl+Alt+C").disabled(!isRealFolder()));
        items.push_back(MenuItem::separator());
        for (auto& it : viewItems()) items.push_back(std::move(it));
        items.push_back(MenuItem::separator());
        for (auto& it : sortItems()) items.push_back(std::move(it));
        items.push_back(MenuItem::separator());
        items.push_back(MenuItem("Show Hidden Files", toggleHidden).setChecked(browser->showHidden).withShortcut("Ctrl+H"));
        return items;
    };

    auto extractHere = [&] {
        auto paths = browser->selectedPaths();
        if (paths.empty() || !isRealFolder()) return;
        std::string parent = browser->path;
        for (const auto& p : paths) {
            if (isArchive(p)) extractArchiveTo(p, parent);
        }
        app.after(0.5, [&, parent] {
            if (browser->path == parent) reload();
        });
    };

    auto extractToSubfolder = [&] {
        auto paths = browser->selectedPaths();
        if (paths.empty() || !isRealFolder()) return;
        std::string parent = browser->path;
        std::string firstTarget;
        for (const auto& p : paths) {
            if (isArchive(p)) {
                std::string target = uniqueName(parent, archiveBaseName(p));
                extractArchiveTo(p, target);
                if (firstTarget.empty()) firstTarget = target;
            }
        }
        app.after(0.6, [&, parent, firstTarget] {
            if (browser->path == parent) {
                reload();
                if (!firstTarget.empty()) navigateTo(parent, firstTarget, false);
            }
        });
    };

    auto compressSelection = [&] {
        auto paths = browser->selectedPaths();
        if (paths.empty() || !isRealFolder()) return;
        std::string parentDir = isInside(paths.front(), browser->path) ? browser->path : parentPath(paths.front());
        std::string zipName;
        if (paths.size() == 1) {
            std::string stem = fs::path(paths.front()).stem().string();
            zipName = uniqueName(parentDir, stem + ".zip");
        } else {
            zipName = uniqueName(parentDir, "Archive.zip");
        }
        std::vector<std::string> args{"zip", "-r", zipName};
        for (const auto& p : paths) {
            args.push_back(fs::path(p).filename().string());
        }
        spawnDetached(args, parentDir);
        app.after(0.5, [&, parentDir, zipName] {
            if (browser->path == parentDir) {
                navigateTo(parentDir, zipName, false);
            }
        });
    };

    auto moveToDirectory = [&](const std::string& targetDir) {
        auto paths = browser->selectedPaths();
        if (paths.empty() || targetDir.empty()) return;
        std::error_code ec;
        fs::create_directories(targetDir, ec);
        ops.move(paths, targetDir);
    };

    auto shareWithFlydrop = [&] {
        auto paths = browser->selectedPaths();
        if (paths.empty()) return;
        std::vector<std::string> args{"flydrop", "send"};
        for (const auto& p : paths) args.push_back(p);
        spawnDetached(args);
    };

    auto itemItems = [&]() {
        std::vector<MenuItem> items;
        auto paths = browser->selectedPaths();
        const bool single = paths.size() == 1;
        auto first = single ? statEntry(paths.front()) : std::nullopt;
        const bool trashed = !paths.empty() && isInTrash(paths.front());
        items.push_back(MenuItem("Open", openSelection).withShortcut("Ctrl+O").disabled(trashed && !(first && first->isDir)));
        if (trashed) items.push_back(MenuItem("Put Back", putBack));
        items.push_back(MenuItem::separator());
        items.push_back(MenuItem(single ? "Quick Look “" + fs::path(paths.front()).filename().string() + "”" : "Quick Look",
                                 [&] { browser->quickLook(); })
                            .withIcon(Icon::Magnifier).withShortcut("Space"));
        items.push_back(MenuItem("Get Info", showInfo).withIcon(Icon::Info).withShortcut("Ctrl+I"));
        items.push_back(MenuItem("Rename", renameSelected).withIcon(Icon::Pencil).withShortcut("Enter").disabled(!single || trashed));
        items.push_back(MenuItem("Duplicate", duplicate).withIcon(Icon::DocOnDoc).withShortcut("Ctrl+D").disabled(!isRealFolder() || trashed));
        items.push_back(MenuItem::separator());
        items.push_back(MenuItem("Copy", [&] { copySelection(false); }).withShortcut("Ctrl+C"));
        items.push_back(MenuItem("Cut", [&] { copySelection(true); }).withShortcut("Ctrl+X").disabled(trashed));
        items.push_back(MenuItem("Copy Path", copyPath).withShortcut("Ctrl+Alt+C"));
        items.push_back(MenuItem::separator());
        bool hasArchive = false;
        for (const auto& p : paths) {
            if (isArchive(p)) { hasArchive = true; break; }
        }
        if (hasArchive && isRealFolder() && !trashed) {
            items.push_back(MenuItem("Extract Here", extractHere).withIcon(Icon::Import));
            items.push_back(MenuItem(single ? "Extract to “" + archiveBaseName(paths.front()) + "/”" : "Extract to Subfolders",
                                     extractToSubfolder).withIcon(Icon::FolderPlus));
        }
        items.push_back(MenuItem(single ? "Compress “" + fs::path(paths.front()).filename().string() + "”"
                                        : "Compress " + std::to_string(paths.size()) + " Items",
                                 compressSelection)
                            .withIcon(Icon::DocOnDoc).disabled(!isRealFolder() || trashed));
        items.push_back(MenuItem("Move to Desktop", [&] { moveToDirectory(userDirectory("DESKTOP", "Desktop")); })
                            .disabled(!isRealFolder() || trashed));
        items.push_back(MenuItem("Move to Documents", [&] { moveToDirectory(userDirectory("DOCUMENTS", "Documents")); })
                            .disabled(!isRealFolder() || trashed));
        items.push_back(MenuItem("Move to Downloads", [&] { moveToDirectory(userDirectory("DOWNLOAD", "Downloads")); })
                            .disabled(!isRealFolder() || trashed));
        items.push_back(MenuItem("Share with FlyDrop", shareWithFlydrop)
                            .withIcon(Icon::Share).disabled(paths.empty() || trashed));
        items.push_back(MenuItem::separator());
        for (const auto& tag : TagManager::instance().allTags()) {
            items.push_back(MenuItem(tag.name, [&, t = tag.name] {
                for (const auto& p : browser->selectedPaths()) TagManager::instance().toggleTag(p, t);
                refreshViews(false);
            }).withDot(tag.color));
        }
        items.push_back(MenuItem("Clear Tags", [&] {
            for (const auto& p : browser->selectedPaths()) TagManager::instance().clearTags(p);
            refreshViews(false);
        }));
        items.push_back(MenuItem::separator());
        if (!isRealFolder()) items.push_back(MenuItem("Show in Enclosing Folder", showInEnclosingFolder).withShortcut("Ctrl+R").disabled(!single));
        if (first && first->isDir) items.push_back(MenuItem("Open in Terminal", openTerminalHere).withIcon(Icon::Terminal));
        if (!isRealFolder() || (first && first->isDir)) items.push_back(MenuItem::separator());
        items.push_back(MenuItem(trashed ? "Delete Immediately…" : "Move to Trash", moveToTrashSelected)
                            .withIcon(Icon::Trash).withShortcut("Delete").asDestructive());
        return items;
    };

    browser->contextMenu = [&](Widget& /*from*/, Point at, bool onItem) {
        menu->setItems(onItem ? itemItems() : backgroundItems());
        menu->setGap(2);
        page->menuAnchor->arrange({at.x, at.y, 1, 1});
        menu->showFrom(*page->menuAnchor);
        if (auto* h = page->host()) h->setFocus(menu.get(), false);
    };
    menu->setOnDismiss([&] { app.post([&] { if (!quickLook->isShown()) focusView(); }); });

    // The toolbar's action menu is rebuilt whenever it might have changed.
    refreshActionMenu = [&] {
        std::vector<MenuItem> items{
            MenuItem("New Folder", newFolder).withIcon(Icon::FolderPlus).withShortcut("Ctrl+Shift+N").disabled(!isRealFolder() || inTrash()),
            MenuItem("Go to Folder…", goToFolder).withShortcut("Ctrl+Shift+G"),
            MenuItem("Settings…", openSettings).withIcon(Icon::Gear).withShortcut("Ctrl+,"),
            MenuItem("Get Info", showInfo).withIcon(Icon::Info).withShortcut("Ctrl+I"),
            MenuItem("Open in Terminal", openTerminalHere).withIcon(Icon::Terminal),
            MenuItem::separator(),
        };
        if (inTrash()) {
            items.push_back(MenuItem("Empty Trash", emptyTrash).asDestructive().disabled(browser->entries.empty()));
            items.push_back(MenuItem::separator());
        }
        items.push_back(MenuItem(ops.undoTitle(), undo).withShortcut("Ctrl+Z").disabled(!ops.canUndo()));
        items.push_back(MenuItem::separator());
        for (auto& it : sortItems()) items.push_back(std::move(it));
        items.push_back(MenuItem::separator());
        items.push_back(MenuItem("Show Hidden Files", toggleHidden).setChecked(browser->showHidden).withShortcut("Ctrl+H"));
        actionMenu.setItems(std::move(items));
    };
    // Sidebar ----------------------------------------------------------------------------------------------
    auto rebuildSidebar = [&] {
        std::vector<SidebarItem> items{SidebarItem::header("Favorites")};
        items.emplace_back("apps:", Icon::Grid, "Applications");
        for (const auto& f : favorites) items.emplace_back(f.path, placeIcon(f.path), f.title);
        items.push_back(SidebarItem::header("Locations"));
        items.emplace_back("/", Icon::HardDrive, "Computer");
        for (const auto& v : mountedVolumes()) items.emplace_back(v.mountPoint, Icon::HardDrive, v.name);
        items.emplace_back(trashFilesDirectory(), Icon::Trash, "Trash");
        items.push_back(SidebarItem::header("Tags"));
        for (const auto& tag : TagManager::instance().allTags()) {
            items.push_back(SidebarItem("tag:" + tag.name, tag.name).withDot(tag.color));
        }
        sidebar.setItems(std::move(items));
    };
    sidebar.onSelect([&](const std::string& id) {
        if (id.empty()) return;
        if (id == "apps:") {
            spawnDetached({"pulsar-apps-viewer"});
            updateChrome();
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
    window.setToolbar(Toolbar({
        ToolbarGroup({backButton, forwardButton}),
        titleLabel,
        Spacer(),
        emptyTrashButton,
        viewControl,
        ToolbarGroup({actionMenu}),
        searchField,
    }));

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
                default: break;
            }
        }
        if (ctrl && !shift) {
            switch (e.key) {
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
                case Key::W: window.close(); return true;
                case Key::Comma: openSettings(); return true;
                default: break;
            }
        }
        if (!ctrl && !e.alt()) {
            switch (e.key) {
                case Key::Enter:
                    if (pickerMode) {
                        submitPicker();
                        return true;
                    }
                    break;
                case Key::Delete: moveToTrashSelected(); return true;
                case Key::F2: renameSelected(); return true;
                case Key::F5: reload(); return true;
                case Key::Escape:
                    if (pickerMode) {
                        std::exit(1);
                    }
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
    // "New windows open": home (default), Recents, or the last folder visited.
    std::string start = home;
    if (const std::string where = prefs.getString("startIn", "home"); where == "recents") {
        start = kRecents;
    } else if (where == "last") {
        std::error_code ec;
        const std::string last = prefs.getString("lastFolder");
        if (last == kRecents || (!last.empty() && fs::is_directory(last, ec))) start = last;
    }
    if (argc > 1) start = expandUser(argv[1]);
    {
        std::error_code ec;
        if (start != kRecents) start = fs::absolute(start, ec).lexically_normal().string();
        if (start.size() > 1 && start.back() == '/') start.pop_back();
    }
    navigateTo(start, {}, false);

    // MAYO_DEMO=<step>[,<step>...] drives the window for screenshots and smoke tests. Steps: list, columns,
    // select:<name>, quicklook[:name], info[:name], menu[:name], rename[:name], search:<query>, size:<points>,
    // goto, open:<name>, back, up, newfolder, trash:<name>, undo, duplicate:<name>, copy:<name>, cut:<name>,
    // paste, renameto:<old>=<new>, drop:<item>=<folder>, hidden, type:<letters>, settings, closesettings,
    // wait:<seconds>.
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
                else if (what == "columns") setMode(ViewMode::Columns);
                else if (what == "select") selectNamed(arg);
                else if (what == "size") browser->resizeIcons(static_cast<float>(std::atof(arg.c_str())));
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
                }
            });
            at += what == "wait" ? std::atof(arg.c_str()) : 0.15;
        }
    }
    return app.run();
}
