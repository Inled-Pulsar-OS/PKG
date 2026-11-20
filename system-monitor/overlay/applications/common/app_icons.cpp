#include "app_icons.hpp"

#include <algorithm>
#include <cctype>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <mutex>
#include <sstream>
#include <unordered_map>
#include <unordered_set>
#include <vector>

#ifdef FORMA_HAVE_APP_ICONS
#include <gdk-pixbuf/gdk-pixbuf.h>
#include <gio/gdesktopappinfo.h>
#include <gio/gio.h>
#endif

#include "mayonnaise/widgets/image.hpp"

namespace fs = std::filesystem;

namespace forma {
namespace {

std::string lowerAscii(std::string s) {
    std::transform(s.begin(), s.end(), s.begin(), [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return s;
}

bool endsWith(const std::string& s, const std::string& suffix) {
    return s.size() >= suffix.size() && s.compare(s.size() - suffix.size(), suffix.size(), suffix) == 0;
}

std::string baseName(std::string path) {
    while (!path.empty() && (path.back() == '"' || path.back() == '\'')) path.pop_back();
    while (!path.empty() && (path.front() == '"' || path.front() == '\'')) path.erase(path.begin());
    return fs::path(path).filename().string();
}

/// First real token of an Exec line (skips env assignments and field codes).
std::string firstExecToken(const std::string& command) {
    std::istringstream in(command);
    for (std::string token; in >> token;) {
        if (token.empty() || token[0] == '%') continue;
        if (token.find('=') != std::string::npos && token.front() != '/' && token.front() != '.') continue;
        return baseName(token);
    }
    return {};
}

int numericSize(const fs::path& path) {
    int best = 0;
    for (const auto& part : path) {
        std::string s = part.string();
        if (s == "scalable") return 512;
        if (s == "symbolic") continue;
        if (endsWith(s, "@2")) s.resize(s.size() - 2);
        const size_t x = s.find('x');
        if (x != std::string::npos) s.resize(x);
        if (s.empty() || !std::all_of(s.begin(), s.end(), [](unsigned char c) { return std::isdigit(c); })) continue;
        const int n = std::atoi(s.c_str());
        if (n > 0 && n <= 1024) best = std::max(best, n);
    }
    return best;
}

int scoreIcon(const fs::path& path) {
    const std::string p = path.string();
    const std::string ext = lowerAscii(path.extension().string());
    int score = 0;
    if (ext == ".svg") score += 60;
    else if (ext == ".xpm") score -= 80;
    if (p.find("-symbolic") != std::string::npos) score -= 10000;
    if (p.find("/apps/") != std::string::npos) score += 300;
    else if (p.find("/categories/") != std::string::npos) score += 200;
    else if (p.find("/places/") != std::string::npos) score += 150;
    else if (p.find("/devices/") != std::string::npos) score += 100;
    else if (p.find("/mimetypes/") != std::string::npos) score += 50;
    else if (p.find("/actions/") != std::string::npos) score += 20;
    else if (p.find("/status/") != std::string::npos) score -= 30;
    score += std::min(512, numericSize(path));
    return score;
}

std::vector<std::string> splitList(const std::string& text) {
    std::vector<std::string> out;
    std::string current;
    for (char c : text) {
        if (c == ',' || c == ';' || c == ' ' || c == '\t') {
            if (!current.empty()) out.push_back(current);
            current.clear();
        } else {
            current.push_back(c);
        }
    }
    if (!current.empty()) out.push_back(current);
    return out;
}

std::string inheritedThemes(const fs::path& themeDir) {
#ifdef FORMA_HAVE_APP_ICONS
    GKeyFile* keyFile = g_key_file_new();
    std::string inherits;
    if (g_key_file_load_from_file(keyFile, (themeDir / "index.theme").c_str(), G_KEY_FILE_NONE, nullptr)) {
        char* value = g_key_file_get_string(keyFile, "Icon Theme", "Inherits", nullptr);
        if (value) {
            inherits = value;
            g_free(value);
        }
    }
    g_key_file_free(keyFile);
    return inherits;
#else
    (void)themeDir;
    return {};
#endif
}

std::string activeIconTheme() {
#ifdef FORMA_HAVE_APP_ICONS
    if (const char* forced = g_getenv("FORMA_ICON_THEME"); forced && *forced) return forced;
    GSettingsSchemaSource* source = g_settings_schema_source_get_default();
    if (source) {
        GSettingsSchema* schema =
            g_settings_schema_source_lookup(source, "org.gnome.desktop.interface", TRUE);
        if (schema) {
            GSettings* settings = g_settings_new_full(schema, nullptr, nullptr);
            char* value = g_settings_get_string(settings, "icon-theme");
            std::string theme = value ? value : "";
            g_free(value);
            g_object_unref(settings);
            g_settings_schema_unref(schema);
            if (!theme.empty()) return theme;
        }
    }
    // Fall back to the GTK settings file used by non-GNOME sessions.
    const char* home = g_get_home_dir();
    if (home) {
        for (const std::string name : {"gtk-4.0", "gtk-3.0"}) {
            std::ifstream in(fs::path(home) / ".config" / name / "settings.ini");
            for (std::string line; std::getline(in, line);) {
                const size_t eq = line.find('=');
                if (eq == std::string::npos) continue;
                if (line.substr(0, eq) == "gtk-icon-theme-name") {
                    std::string value = line.substr(eq + 1);
                    value.erase(0, value.find_first_not_of(" \t\"'"));
                    value.erase(value.find_last_not_of(" \t\"'") + 1);
                    if (!value.empty()) return value;
                }
            }
        }
    }
#endif
    return "hicolor";
}

std::vector<fs::path> iconRoots() {
    std::vector<fs::path> roots;
#ifdef FORMA_HAVE_APP_ICONS
    roots.emplace_back(fs::path(g_get_user_data_dir()) / "icons");
    if (const char* home = g_get_home_dir()) roots.emplace_back(fs::path(home) / ".icons");
    for (const char* const* dir = g_get_system_data_dirs(); dir && *dir; ++dir) roots.emplace_back(fs::path(*dir) / "icons");
#endif
    roots.emplace_back("/usr/share/pixmaps");
    return roots;
}

struct ThemeIndex {
    std::unordered_map<std::string, std::string> best;
};

void scanThemeDir(const fs::path& dir, ThemeIndex& index, std::unordered_set<std::string>& visited) {
    std::error_code ec;
    const fs::path canonical = fs::weakly_canonical(dir, ec);
    if (ec || !visited.insert(canonical.string()).second) return;
    if (!fs::is_directory(canonical, ec)) return;
    for (fs::recursive_directory_iterator it(canonical, fs::directory_options::skip_permission_denied, ec), end;
         it != end; it.increment(ec)) {
        if (ec) {
            ec.clear();
            continue;
        }
        if (!it->is_regular_file(ec)) continue;
        const std::string ext = lowerAscii(it->path().extension().string());
        if (ext != ".png" && ext != ".svg" && ext != ".xpm") continue;
        const std::string name = it->path().stem().string();
        if (name.empty() || name.front() == '.') continue;
        const int score = scoreIcon(it->path());
        auto found = index.best.find(name);
        if (found == index.best.end() || score > scoreIcon(found->second)) index.best[name] = it->path().string();
    }
}

std::shared_ptr<const ThemeIndex> themeIndex(const std::string& theme) {
    static std::string cachedTheme;
    static std::shared_ptr<const ThemeIndex> cached;
    if (cached && cachedTheme == theme) return cached;

    auto index = std::make_shared<ThemeIndex>();
    std::unordered_set<std::string> visited;
    std::vector<std::string> chain;
    std::unordered_set<std::string> seenThemes;
    std::vector<std::string> pending{theme};
    // Resolve Inherits up front: the current theme wins, then its parents, then hicolor.
    for (size_t i = 0; i < pending.size() && i < 12; ++i) {
        const std::string name = pending[i];
        if (name.empty() || !seenThemes.insert(name).second) continue;
        chain.push_back(name);
        for (const auto& root : iconRoots()) {
            const fs::path dir = root / name;
            std::error_code ec;
            if (!fs::is_directory(dir, ec)) continue;
            for (const auto& parent : splitList(inheritedThemes(dir))) {
                if (!seenThemes.contains(parent)) pending.push_back(parent);
            }
        }
    }
    if (!seenThemes.contains("hicolor")) chain.push_back("hicolor");

    for (const auto& name : chain) {
        for (const auto& root : iconRoots()) {
            const fs::path dir = root / name;
            std::error_code ec;
            if (fs::is_directory(dir, ec)) scanThemeDir(dir, *index, visited);
        }
    }
    // Loose icons (and themes that live directly in pixmaps) still count.
    for (const auto& root : iconRoots()) {
        std::error_code ec;
        if (fs::is_directory(root, ec)) scanThemeDir(root, *index, visited);
    }
    cachedTheme = theme;
    cached = index;
    return cached;
}

std::string findIconPath(const std::string& name) {
    if (name.empty()) return {};
    const fs::path direct(name);
    std::error_code ec;
    if (direct.is_absolute() && fs::is_regular_file(direct, ec)) return direct.string();
    const std::shared_ptr<const ThemeIndex> index = themeIndex(activeIconTheme());
    auto it = index->best.find(name);
    if (it != index->best.end()) return it->second;
    it = index->best.find(lowerAscii(name));
    if (it != index->best.end()) return it->second;
    return {};
}

#ifdef FORMA_HAVE_APP_ICONS
std::shared_ptr<mayo::ImageData> loadPixbuf(const std::string& path, int size) {
    if (path.empty()) return nullptr;
    GError* error = nullptr;
    GdkPixbuf* pixbuf = gdk_pixbuf_new_from_file_at_size(path.c_str(), size, size, &error);
    if (!pixbuf) {
        if (error) g_error_free(error);
        return nullptr;
    }
    if (gdk_pixbuf_get_n_channels(pixbuf) != 4) {
        GdkPixbuf* rgba = gdk_pixbuf_add_alpha(pixbuf, FALSE, 0, 0, 0);
        g_object_unref(pixbuf);
        pixbuf = rgba;
        if (!pixbuf) return nullptr;
    }
    const int width = gdk_pixbuf_get_width(pixbuf);
    const int height = gdk_pixbuf_get_height(pixbuf);
    const int stride = gdk_pixbuf_get_rowstride(pixbuf);
    const int channels = gdk_pixbuf_get_n_channels(pixbuf);
    const guchar* pixels = gdk_pixbuf_read_pixels(pixbuf);
    std::vector<std::uint8_t> rgba(static_cast<size_t>(width) * static_cast<size_t>(height) * 4);
    for (int y = 0; y < height; ++y) {
        const guchar* row = pixels + static_cast<size_t>(y) * static_cast<size_t>(stride);
        for (int x = 0; x < width; ++x) {
            std::uint8_t* out = &rgba[(static_cast<size_t>(y) * static_cast<size_t>(width) + static_cast<size_t>(x)) * 4];
            out[0] = row[static_cast<size_t>(x) * static_cast<size_t>(channels) + 0];
            out[1] = row[static_cast<size_t>(x) * static_cast<size_t>(channels) + 1];
            out[2] = row[static_cast<size_t>(x) * static_cast<size_t>(channels) + 2];
            out[3] = channels == 4 ? row[static_cast<size_t>(x) * static_cast<size_t>(channels) + 3] : 255;
        }
    }
    g_object_unref(pixbuf);
    return mayo::ImageData::fromPixels(width, height, rgba);
}
#endif

std::shared_ptr<mayo::ImageData> cachedIcon(const std::string& name, int size) {
    static std::mutex mutex;
    static std::unordered_map<std::string, std::shared_ptr<mayo::ImageData>> cache;
    const std::string path = findIconPath(name);
    if (path.empty()) return nullptr;
    const std::string key = path + "#" + std::to_string(size);
    std::lock_guard lock(mutex);
    auto found = cache.find(key);
    if (found != cache.end()) return found->second;
    std::shared_ptr<mayo::ImageData> image;
#ifdef FORMA_HAVE_APP_ICONS
    image = loadPixbuf(path, size);
#endif
    cache.emplace(key, image);
    return image;
}

struct AppIndex {
    std::unordered_map<std::string, std::vector<std::string>> byExec;
    std::unordered_map<std::string, std::vector<std::string>> byName;
    std::unordered_map<std::string, std::vector<std::string>> byId;
};

#ifdef FORMA_HAVE_APP_ICONS
std::vector<std::string> iconCandidates(GIcon* icon) {
    std::vector<std::string> out;
    if (!icon) return out;
    if (G_IS_THEMED_ICON(icon)) {
        const char* const* names = g_themed_icon_get_names(G_THEMED_ICON(icon));
        for (int pass = 0; pass < 2; ++pass) {
            for (int i = 0; names && names[i]; ++i) {
                const std::string name = names[i];
                if (name.empty()) continue;
                const bool symbolic = endsWith(name, "-symbolic");
                if ((pass == 0 && symbolic) || (pass == 1 && !symbolic)) continue;
                if (std::find(out.begin(), out.end(), name) == out.end()) out.push_back(name);
            }
        }
    } else if (G_IS_FILE_ICON(icon)) {
        GFile* file = g_file_icon_get_file(G_FILE_ICON(icon));  // owned by the icon
        if (file) {
            char* path = g_file_get_path(file);
            if (path) {
                out.emplace_back(path);
                g_free(path);
            }
        }
    }
    if (out.empty()) {
        char* text = g_icon_to_string(icon);
        if (text) {
            if (*text) out.emplace_back(text);
            g_free(text);
        }
    }
    return out;
}

std::shared_ptr<const AppIndex> appIndex() {
    static const std::shared_ptr<const AppIndex> index = [] {
        auto result = std::make_shared<AppIndex>();
        auto add = [](std::unordered_map<std::string, std::vector<std::string>>& map, const std::string& key,
                      const std::vector<std::string>& icons) {
            if (key.empty() || icons.empty()) return;
            auto& list = map[lowerAscii(key)];
            for (const auto& icon : icons) {
                if (std::find(list.begin(), list.end(), icon) == list.end()) list.push_back(icon);
            }
        };
        GList* apps = g_app_info_get_all();
        for (GList* item = apps; item; item = item->next) {
            GAppInfo* info = G_APP_INFO(item->data);
            const std::vector<std::string> icons = iconCandidates(g_app_info_get_icon(info));
            if (icons.empty()) continue;

            if (const char* executable = g_app_info_get_executable(info)) add(result->byExec, baseName(executable), icons);
            if (const char* command = g_app_info_get_commandline(info)) add(result->byExec, firstExecToken(command), icons);
            if (const char* name = g_app_info_get_display_name(info)) add(result->byName, name, icons);
            if (const char* id = g_app_info_get_id(info)) {
                std::string desktopId = id;
                if (endsWith(desktopId, ".desktop")) desktopId.resize(desktopId.size() - 8);
                add(result->byId, desktopId, icons);
                add(result->byExec, desktopId, icons);
            }
            if (G_IS_DESKTOP_APP_INFO(info)) {
                if (const char* wmClass = g_desktop_app_info_get_startup_wm_class(G_DESKTOP_APP_INFO(info)))
                    add(result->byId, wmClass, icons);
            }
        }
        g_list_free_full(apps, g_object_unref);
        return result;
    }();
    return index;
}
#endif

const std::vector<std::string>* candidateList(const std::string& key,
                                             const std::vector<const std::unordered_map<std::string, std::vector<std::string>>*>& maps) {
    const std::string needle = lowerAscii(key);
    if (needle.empty()) return nullptr;
    for (auto* map : maps) {
        auto it = map->find(needle);
        if (it != map->end() && !it->second.empty()) return &it->second;
    }
    return nullptr;
}

std::shared_ptr<mayo::ImageData> resolve(const std::string& key, const std::string& fallbackKey, int size) {
#ifndef FORMA_HAVE_APP_ICONS
    (void)key;
    (void)fallbackKey;
    (void)size;
    return nullptr;
#else
    const auto& index = *appIndex();
    const std::vector<const std::unordered_map<std::string, std::vector<std::string>>*> maps{
        &index.byExec, &index.byName, &index.byId};
    for (const std::string& candidateKey : {key, fallbackKey}) {
        if (candidateKey.empty()) continue;
        if (const auto* candidates = candidateList(candidateKey, maps)) {
            for (const auto& icon : *candidates) {
                if (auto image = cachedIcon(icon, size)) return image;
            }
        }
    }
    return nullptr;
#endif
}

}  // namespace

std::shared_ptr<mayo::ImageData> appIconForProcess(const std::string& processName, const std::string& command, int size) {
    static std::mutex mutex;
    static std::unordered_map<std::string, std::shared_ptr<mayo::ImageData>> cache;
    const std::string key = lowerAscii(processName) + "\x1f" + command + "#" + std::to_string(size);
    {
        std::lock_guard lock(mutex);
        auto found = cache.find(key);
        if (found != cache.end()) return found->second;
    }
    const std::string executable = firstExecToken(command);
    auto image = resolve(processName, executable, size);
    if (!image && !executable.empty()) image = resolve(executable, {}, size);
    if (!image) image = cachedIcon("application-x-executable", size);
    if (!image) image = cachedIcon("application-default-icon", size);
    std::lock_guard lock(mutex);
    cache.emplace(key, image);
    return image;
}

std::shared_ptr<mayo::ImageData> appIconForName(const std::string& displayName, int size) {
    static std::mutex mutex;
    static std::unordered_map<std::string, std::shared_ptr<mayo::ImageData>> cache;
    const std::string key = lowerAscii(displayName) + "#" + std::to_string(size);
    {
        std::lock_guard lock(mutex);
        auto found = cache.find(key);
        if (found != cache.end()) return found->second;
    }
    std::string name = displayName;
    if (endsWith(name, ".desktop")) name.resize(name.size() - 8);
    auto image = resolve(name, {}, size);
    if (!image) image = cachedIcon(name, size);
    std::lock_guard lock(mutex);
    cache.emplace(key, image);
    return image;
}

std::shared_ptr<mayo::ImageData> themedIcon(const std::string& iconName, int size) {
    return cachedIcon(iconName, size);
}

}  // namespace forma
