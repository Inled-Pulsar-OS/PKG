#include "file_model.hpp"
#include "applications.hpp"
#include "tag_manager.hpp"
#include "file_transaction.hpp"
#include <mayonnaise/i18n.hpp>

#include <fcntl.h>
#include <sys/stat.h>
#include <sys/statvfs.h>
#include <sys/wait.h>
#include <unistd.h>

#include <algorithm>
#include <cctype>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <map>
#include <set>
#include <sstream>

namespace fs = std::filesystem;

namespace finder {
using mayo::i18n::tr;

namespace {

std::string lower(std::string s) {
    std::transform(s.begin(), s.end(), s.begin(), [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return s;
}

struct KindInfo {
    FileKind kind;
    const char* description;
};

const std::map<std::string, KindInfo>& kindTable() {
    static const std::map<std::string, KindInfo> table = {
        {"png", {FileKind::Image, "PNG image"}}, {"jpg", {FileKind::Image, "JPEG image"}},
        {"jpeg", {FileKind::Image, "JPEG image"}}, {"gif", {FileKind::Image, "GIF image"}},
        {"webp", {FileKind::Image, "WebP image"}}, {"bmp", {FileKind::Image, "BMP image"}},
        {"tga", {FileKind::Image, "TGA image"}}, {"tif", {FileKind::Image, "TIFF image"}},
        {"tiff", {FileKind::Image, "TIFF image"}}, {"heic", {FileKind::Image, "HEIC image"}},
        {"avif", {FileKind::Image, "AVIF image"}}, {"ico", {FileKind::Image, "Windows icon"}},
        {"svg", {FileKind::Image, "SVG document"}}, {"psd", {FileKind::Image, "Photoshop document"}},
        {"xcf", {FileKind::Image, "GIMP image"}}, {"kra", {FileKind::Image, "Krita document"}},
        {"mp4", {FileKind::Video, "MPEG-4 movie"}}, {"m4v", {FileKind::Video, "MPEG-4 movie"}},
        {"mkv", {FileKind::Video, "Matroska video"}}, {"webm", {FileKind::Video, "WebM video"}},
        {"mov", {FileKind::Video, "QuickTime movie"}}, {"avi", {FileKind::Video, "AVI movie"}},
        {"wmv", {FileKind::Video, "Windows Media video"}}, {"flv", {FileKind::Video, "Flash video"}},
        {"mp3", {FileKind::Audio, "MP3 audio"}}, {"flac", {FileKind::Audio, "FLAC audio"}},
        {"wav", {FileKind::Audio, "Waveform audio"}}, {"ogg", {FileKind::Audio, "Ogg Vorbis audio"}},
        {"opus", {FileKind::Audio, "Opus audio"}}, {"m4a", {FileKind::Audio, "MPEG-4 audio"}},
        {"aac", {FileKind::Audio, "AAC audio"}}, {"aiff", {FileKind::Audio, "AIFF audio"}},
        {"pdf", {FileKind::Pdf, "PDF document"}},
        {"txt", {FileKind::Text, "Plain Text"}}, {"md", {FileKind::Text, "Markdown document"}},
        {"rst", {FileKind::Text, "reStructuredText"}}, {"log", {FileKind::Text, "Log file"}},
        {"json", {FileKind::Text, "JSON document"}}, {"xml", {FileKind::Text, "XML document"}},
        {"yaml", {FileKind::Text, "YAML document"}}, {"yml", {FileKind::Text, "YAML document"}},
        {"toml", {FileKind::Text, "TOML document"}}, {"ini", {FileKind::Text, "Configuration file"}},
        {"conf", {FileKind::Text, "Configuration file"}}, {"cfg", {FileKind::Text, "Configuration file"}},
        {"desktop", {FileKind::Text, "Desktop entry"}}, {"srt", {FileKind::Text, "Subtitles"}},
        {"c", {FileKind::Code, "C source"}}, {"h", {FileKind::Code, "C header"}},
        {"cpp", {FileKind::Code, "C++ source"}}, {"cc", {FileKind::Code, "C++ source"}},
        {"cxx", {FileKind::Code, "C++ source"}}, {"hpp", {FileKind::Code, "C++ header"}},
        {"py", {FileKind::Code, "Python script"}}, {"js", {FileKind::Code, "JavaScript"}},
        {"mjs", {FileKind::Code, "JavaScript"}}, {"ts", {FileKind::Code, "TypeScript"}},
        {"tsx", {FileKind::Code, "TypeScript JSX"}}, {"jsx", {FileKind::Code, "JavaScript JSX"}},
        {"rs", {FileKind::Code, "Rust source"}}, {"go", {FileKind::Code, "Go source"}},
        {"java", {FileKind::Code, "Java source"}}, {"kt", {FileKind::Code, "Kotlin source"}},
        {"swift", {FileKind::Code, "Swift source"}}, {"cs", {FileKind::Code, "C# source"}},
        {"rb", {FileKind::Code, "Ruby script"}}, {"lua", {FileKind::Code, "Lua script"}},
        {"sh", {FileKind::Code, "Shell script"}}, {"bash", {FileKind::Code, "Shell script"}},
        {"zsh", {FileKind::Code, "Shell script"}}, {"fish", {FileKind::Code, "Shell script"}},
        {"cmake", {FileKind::Code, "CMake script"}}, {"glsl", {FileKind::Code, "GLSL shader"}},
        {"vert", {FileKind::Code, "GLSL shader"}}, {"frag", {FileKind::Code, "GLSL shader"}},
        {"css", {FileKind::Code, "CSS style sheet"}}, {"sql", {FileKind::Code, "SQL script"}},
        {"html", {FileKind::Web, "HTML document"}}, {"htm", {FileKind::Web, "HTML document"}},
        {"zip", {FileKind::Archive, "ZIP archive"}}, {"tar", {FileKind::Archive, "tar archive"}},
        {"gz", {FileKind::Archive, "gzip archive"}}, {"tgz", {FileKind::Archive, "gzip tar archive"}},
        {"xz", {FileKind::Archive, "XZ archive"}}, {"bz2", {FileKind::Archive, "bzip2 archive"}},
        {"zst", {FileKind::Archive, "Zstandard archive"}}, {"7z", {FileKind::Archive, "7-Zip archive"}},
        {"rar", {FileKind::Archive, "RAR archive"}}, {"deb", {FileKind::Archive, "Debian package"}},
        {"rpm", {FileKind::Archive, "RPM package"}}, {"jar", {FileKind::Archive, "Java archive"}},
        {"iso", {FileKind::DiskImage, "Disk image"}}, {"img", {FileKind::DiskImage, "Disk image"}},
        {"dmg", {FileKind::DiskImage, "Apple disk image"}}, {"qcow2", {FileKind::DiskImage, "QEMU disk image"}},
        {"ttf", {FileKind::Font, "TrueType font"}}, {"otf", {FileKind::Font, "OpenType font"}},
        {"woff", {FileKind::Font, "Web font"}}, {"woff2", {FileKind::Font, "Web font"}},
        {"doc", {FileKind::Document, "Word document"}}, {"docx", {FileKind::Document, "Word document"}},
        {"odt", {FileKind::Document, "OpenDocument text"}}, {"rtf", {FileKind::Document, "Rich Text document"}},
        {"epub", {FileKind::Document, "EPUB book"}},
        {"xls", {FileKind::Spreadsheet, "Excel spreadsheet"}}, {"xlsx", {FileKind::Spreadsheet, "Excel spreadsheet"}},
        {"ods", {FileKind::Spreadsheet, "OpenDocument spreadsheet"}}, {"csv", {FileKind::Spreadsheet, "CSV document"}},
        {"ppt", {FileKind::Presentation, "PowerPoint presentation"}},
        {"pptx", {FileKind::Presentation, "PowerPoint presentation"}},
        {"odp", {FileKind::Presentation, "OpenDocument presentation"}},
        {"so", {FileKind::Executable, "Shared library"}}, {"appimage", {FileKind::Executable, "AppImage"}},
        {"exe", {FileKind::Executable, "Windows executable"}}, {"bin", {FileKind::Executable, "Binary"}},
    };
    return table;
}

std::optional<FileEntry> makeEntry(const std::string& path, std::string name) {
    struct statx sx {};
    if (statx(AT_FDCWD, path.c_str(), AT_SYMLINK_NOFOLLOW, STATX_BASIC_STATS | STATX_BTIME, &sx) != 0) return std::nullopt;
    FileEntry e;
    e.path = path;
    e.name = std::move(name);
    e.isHidden = !e.name.empty() && (e.name[0] == '.' || e.name.back() == '~');
    e.isLink = S_ISLNK(sx.stx_mode);
    unsigned mode = sx.stx_mode;
    std::uintmax_t size = sx.stx_size;
    std::time_t mtime = sx.stx_mtime.tv_sec;
    if (e.isLink) {
        struct stat target {};
        if (::stat(path.c_str(), &target) == 0) {  // broken links keep the link's own attributes
            mode = target.st_mode;
            size = static_cast<std::uintmax_t>(target.st_size);
            mtime = target.st_mtime;
        }
    }
    e.isDir = S_ISDIR(mode);
    e.mode = mode & 07777;
    e.size = e.isDir ? 0 : size;
    e.modified = mtime;
    e.created = (sx.stx_mask & STATX_BTIME) ? static_cast<std::time_t>(sx.stx_btime.tv_sec) : 0;
    if (e.isDir) {
        e.kind = FileKind::Folder;
    } else {
        // std::filesystem treats ".bashrc" as a stem without extension, which is what we want.
        e.ext = lower(fs::path(e.name).extension().string());
        if (!e.ext.empty()) e.ext.erase(0, 1);
        e.kind = kindForExtension(e.ext);
        if (e.kind == FileKind::Unknown && (mode & 0111) && S_ISREG(mode)) e.kind = FileKind::Executable;
        if (e.kind == FileKind::Unknown && (e.name == "Makefile" || e.name == "Dockerfile" || e.name == "PKGBUILD"))
            e.kind = FileKind::Code;
    }
    return e;
}

std::string percentDecode(const std::string& s) {
    std::string out;
    for (size_t i = 0; i < s.size(); ++i) {
        if (s[i] == '%' && i + 2 < s.size() && std::isxdigit(static_cast<unsigned char>(s[i + 1])) &&
            std::isxdigit(static_cast<unsigned char>(s[i + 2]))) {
            out.push_back(static_cast<char>(std::stoi(s.substr(i + 1, 2), nullptr, 16)));
            i += 2;
        } else {
            out.push_back(s[i]);
        }
    }
    return out;
}

std::string percentEncode(const std::string& s) {
    std::string out;
    char buf[4];
    for (unsigned char c : s) {
        if (std::isalnum(c) || c == '/' || c == '-' || c == '_' || c == '.' || c == '~') {
            out.push_back(static_cast<char>(c));
        } else {
            std::snprintf(buf, sizeof buf, "%%%02X", c);
            out += buf;
        }
    }
    return out;
}

std::vector<FileEntry> listRecents(bool showHidden) {
    std::ifstream in(homeDirectory() + "/.local/share/recently-used.xbel");
    std::string xml((std::istreambuf_iterator<char>(in)), {});
    // <bookmark href="file:///..." ... modified="2026-10-01T10:00:00Z" ...>
    std::vector<std::pair<std::string, std::string>> found;  // modified, path
    size_t pos = 0;
    while ((pos = xml.find("<bookmark ", pos)) != std::string::npos) {
        size_t end = xml.find('>', pos);
        std::string tag = xml.substr(pos, end - pos);
        pos = end;
        auto attr = [&](const char* name) {
            std::string key = std::string(name) + "=\"";
            size_t a = tag.find(key);
            if (a == std::string::npos) return std::string();
            a += key.size();
            return tag.substr(a, tag.find('"', a) - a);
        };
        std::string href = attr("href");
        if (href.rfind("file://", 0) != 0) continue;
        std::string when = attr("modified");
        if (when.empty()) when = attr("visited");
        found.emplace_back(when, percentDecode(href.substr(7)));
    }
    std::sort(found.begin(), found.end(), [](const auto& a, const auto& b) { return a.first > b.first; });
    std::vector<FileEntry> out;
    std::set<std::string> seen;
    for (const auto& [when, path] : found) {
        if (out.size() >= 200 || !seen.insert(path).second) continue;
        auto e = makeEntry(path, fs::path(path).filename().string());
        if (!e || e->isDir || (e->isHidden && !showHidden)) continue;
        out.push_back(std::move(*e));
    }
    return out;
}

/// The directory holding `path`'s mount ("topdir" in the Trash spec).
std::string mountTop(const std::string& path) {
    struct stat st {};
    if (::lstat(path.c_str(), &st) != 0) return "/";
    fs::path cur = fs::path(path).parent_path();
    fs::path top = cur;
    while (!cur.empty()) {
        struct stat up {};
        if (::stat(cur.c_str(), &up) != 0 || up.st_dev != st.st_dev) break;
        top = cur;
        if (cur == cur.root_path()) break;
        cur = cur.parent_path();
    }
    return top.string();
}

std::string homeTrash() {
    const char* data = std::getenv("XDG_DATA_HOME");
    return (data && *data ? std::string(data) : homeDirectory() + "/.local/share") + "/Trash";
}

std::string localTimestamp() {
    std::time_t now = std::time(nullptr);
    std::tm tm {};
    localtime_r(&now, &tm);
    char buf[32];
    std::strftime(buf, sizeof buf, "%Y-%m-%dT%H:%M:%S", &tm);
    return buf;
}

}  // namespace

/// Runs argv detached (double fork, so no zombies and no SIGCHLD handling), optionally in `cwd`.
void spawnDetached(const std::vector<std::string>& args, const std::string& cwd) {
    std::vector<char*> argv;
    for (const auto& a : args) argv.push_back(const_cast<char*>(a.c_str()));
    argv.push_back(nullptr);
    pid_t child = fork();
    if (child == 0) {
        setsid();
        if (fork() == 0) {
            if (!cwd.empty() && chdir(cwd.c_str()) != 0) _exit(127);
            int devnull = open("/dev/null", O_RDWR);
            if (devnull >= 0) {
                dup2(devnull, STDIN_FILENO);
                dup2(devnull, STDOUT_FILENO);
            }
            execvp(argv[0], argv.data());
            _exit(127);
        }
        _exit(0);
    }
    if (child > 0) waitpid(child, nullptr, 0);
}

namespace {

bool onPath(const std::string& program) {
    const char* path = std::getenv("PATH");
    if (!path) return false;
    std::stringstream ss(path);
    std::string dir;
    while (std::getline(ss, dir, ':')) {
        if (!dir.empty() && ::access((dir + "/" + program).c_str(), X_OK) == 0) return true;
    }
    return false;
}

}  // namespace

std::vector<FileEntry> listDirectory(const std::string& path, bool showHidden, std::string* error) {
    if (path.rfind("tag:", 0) == 0) {
        std::string tag = path.substr(4);
        auto files = TagManager::instance().filesWithTag(tag);
        std::vector<FileEntry> out;
        for (const auto& f : files) {
            if (auto e = statEntry(f)) out.push_back(std::move(*e));
        }
        return out;
    }
    if (path == kRecents) return listRecents(showHidden);
    std::vector<FileEntry> out;
    std::error_code ec;
    fs::directory_iterator it(path, fs::directory_options::skip_permission_denied, ec);
    if (ec) {
        if (error) *error = ec.message();
        return out;
    }
    for (; it != fs::directory_iterator(); it.increment(ec)) {
        if (ec) break;
        std::string name = it->path().filename().string();
        if (!showHidden && (name[0] == '.' || name.back() == '~')) continue;
        if (auto e = makeEntry(it->path().string(), std::move(name))) out.push_back(std::move(*e));
    }
    return out;
}

std::optional<FileEntry> statEntry(const std::string& path) {
    std::string name = fs::path(path).filename().string();
    if (name.empty()) name = path;
    return makeEntry(path, std::move(name));
}

int compareNames(const std::string& a, const std::string& b) {
    size_t i = 0;
    size_t j = 0;
    auto digit = [](char c) { return c >= '0' && c <= '9'; };
    while (i < a.size() && j < b.size()) {
        if (digit(a[i]) && digit(b[j])) {
            size_t i0 = i;
            size_t j0 = j;
            while (i0 < a.size() && a[i0] == '0') ++i0;
            while (j0 < b.size() && b[j0] == '0') ++j0;
            size_t i1 = i0;
            size_t j1 = j0;
            while (i1 < a.size() && digit(a[i1])) ++i1;
            while (j1 < b.size() && digit(b[j1])) ++j1;
            if (i1 - i0 != j1 - j0) return i1 - i0 < j1 - j0 ? -1 : 1;
            int c = a.compare(i0, i1 - i0, b, j0, j1 - j0);
            if (c != 0) return c < 0 ? -1 : 1;
            i = i1;
            j = j1;
            continue;
        }
        int ca = std::tolower(static_cast<unsigned char>(a[i]));
        int cb = std::tolower(static_cast<unsigned char>(b[j]));
        if (ca != cb) return ca < cb ? -1 : 1;
        ++i;
        ++j;
    }
    if (i < a.size()) return 1;
    if (j < b.size()) return -1;
    return a < b ? -1 : (a > b ? 1 : 0);
}

bool startsWithNoCase(const std::string& text, const std::string& prefix) {
    if (prefix.size() > text.size()) return false;
    for (size_t i = 0; i < prefix.size(); ++i) {
        if (std::tolower(static_cast<unsigned char>(text[i])) != std::tolower(static_cast<unsigned char>(prefix[i])))
            return false;
    }
    return true;
}

bool containsNoCase(const std::string& text, const std::string& needle) {
    return lower(text).find(lower(needle)) != std::string::npos;
}

void sortEntries(std::vector<FileEntry>& entries, const SortOrder& order) {
    std::stable_sort(entries.begin(), entries.end(), [&](const FileEntry& a, const FileEntry& b) {
        if (order.foldersFirst && a.isDir != b.isDir) return a.isDir;
        int c = 0;
        switch (order.key) {
            case SortKey::Name: break;
            case SortKey::Modified: c = a.modified < b.modified ? -1 : (a.modified > b.modified ? 1 : 0); break;
            case SortKey::Size: c = a.size < b.size ? -1 : (a.size > b.size ? 1 : 0); break;
            case SortKey::Kind: c = compareNames(kindDescription(a), kindDescription(b)); break;
        }
        if (c == 0) c = compareNames(a.name, b.name);
        return order.ascending ? c < 0 : c > 0;
    });
}

FileKind kindForExtension(const std::string& ext) {
    auto it = kindTable().find(ext);
    return it == kindTable().end() ? FileKind::Unknown : it->second.kind;
}

std::string kindDescription(const FileEntry& e) {
    if (e.isLink) return "Alias";
    if (e.isDir) return "Folder";
    auto it = kindTable().find(e.ext);
    if (it != kindTable().end()) return it->second.description;
    if (e.kind == FileKind::Executable) return "Unix executable";
    if (e.kind == FileKind::Code) return "Build script";
    if (!e.ext.empty()) {
        std::string up = e.ext;
        std::transform(up.begin(), up.end(), up.begin(), [](unsigned char c) { return static_cast<char>(std::toupper(c)); });
        return up + " file";
    }
    return "Document";
}

bool isImageExtension(const std::string& ext) {
    return ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "gif" || ext == "bmp" || ext == "tga";
}

bool isTextLike(const FileEntry& e) {
    return e.kind == FileKind::Text || e.kind == FileKind::Code || e.kind == FileKind::Web || e.ext == "csv" ||
           e.ext == "svg" || (e.kind == FileKind::Unknown && e.size < 512 * 1024);
}

std::string formatSize(std::uintmax_t b) {
    char buf[32];
    if (b == 0) return "Zero bytes";
    if (b < 1000) std::snprintf(buf, sizeof buf, "%ju bytes", b);
    else if (b < 1000 * 1000) std::snprintf(buf, sizeof buf, "%.0f KB", static_cast<double>(b) / 1e3);
    else if (b < 1000ull * 1000 * 1000) std::snprintf(buf, sizeof buf, "%.1f MB", static_cast<double>(b) / 1e6);
    else if (b < 1000ull * 1000 * 1000 * 1000) std::snprintf(buf, sizeof buf, "%.2f GB", static_cast<double>(b) / 1e9);
    else std::snprintf(buf, sizeof buf, "%.2f TB", static_cast<double>(b) / 1e12);
    return buf;
}

std::string formatDate(std::time_t t) {
    if (t <= 0) return "--";
    std::tm when {};
    std::tm today {};
    std::time_t now = std::time(nullptr);
    localtime_r(&t, &when);
    localtime_r(&now, &today);
    char time[16];
    std::strftime(time, sizeof time, "%H:%M", &when);
    std::tm yesterday = today;
    yesterday.tm_mday -= 1;
    std::mktime(&yesterday);
    auto sameDay = [](const std::tm& a, const std::tm& b) { return a.tm_year == b.tm_year && a.tm_yday == b.tm_yday; };
    if (sameDay(when, today)) return std::string("Today at ") + time;
    if (sameDay(when, yesterday)) return std::string("Yesterday at ") + time;
    char date[32];
    std::strftime(date, sizeof date, "%e %b %Y", &when);
    std::string d = date;
    if (!d.empty() && d[0] == ' ') d.erase(0, 1);
    return d + " at " + time;
}

std::string formatPermissions(unsigned mode, bool dir) {
    std::string s = dir ? "d" : "-";
    const char* rwx = "rwx";
    for (int i = 8; i >= 0; --i) s.push_back((mode & (1u << i)) ? rwx[(8 - i) % 3] : '-');
    return s;
}

std::string itemCount(size_t n) { return std::to_string(n) + (n == 1 ? " item" : " items"); }

std::string homeDirectory() {
    const char* home = std::getenv("HOME");
    return home && *home ? home : "/";
}

std::string userDirectory(const char* key, const char* fallback) {
    const std::string home = homeDirectory();
    std::ifstream in(home + "/.config/user-dirs.dirs");
    std::string line;
    const std::string prefix = std::string("XDG_") + key + "_DIR=";
    while (std::getline(in, line)) {
        if (line.rfind(prefix, 0) != 0) continue;
        std::string v = line.substr(prefix.size());
        if (v.size() >= 2 && v.front() == '"') v = v.substr(1, v.size() - 2);
        if (v.rfind("$HOME", 0) == 0) v = home + v.substr(5);
        if (v == home || v == home + "/") return {};  // unset directories point at $HOME
        return v;
    }
    return home + "/" + fallback;
}

std::string trashFilesDirectory() { return homeTrash() + "/files"; }

std::optional<std::uintmax_t> availableSpace(const std::string& path) {
    struct statvfs st {};
    if (path == kRecents || ::statvfs(path.c_str(), &st) != 0) return std::nullopt;
    return static_cast<std::uintmax_t>(st.f_bavail) * st.f_frsize;
}

std::string displayName(const std::string& path) {
    if (path.rfind("tag:", 0) == 0) return "Tag: " + path.substr(4);
    if (path == kRecents) return "Recents";
    if (path == "/") return "Computer";
    if (path == homeDirectory()) return "Home";
    if (path == trashFilesDirectory()) return "Trash";
    std::string name = fs::path(path).filename().string();
    return name.empty() ? path : name;
}

std::string parentPath(const std::string& path) {
    if (path == "/" || path == kRecents) return path;
    std::string p = fs::path(path).parent_path().string();
    return p.empty() ? "/" : p;
}

bool isInside(const std::string& path, const std::string& folder) {
    if (folder == "/") return !path.empty() && path[0] == '/';
    return path == folder || (path.size() > folder.size() && path.compare(0, folder.size(), folder) == 0 &&
                              path[folder.size()] == '/');
}

std::vector<Volume> mountedVolumes() {
    std::vector<Volume> out;
    std::ifstream in("/proc/self/mounts");
    std::string device, mount, type, rest;
    while (in >> device >> mount >> type && std::getline(in, rest)) {
        // Mount points escape spaces as \040.
        std::string decoded;
        for (size_t i = 0; i < mount.size(); ++i) {
            if (mount[i] == '\\' && i + 3 < mount.size()) {
                decoded.push_back(static_cast<char>(std::stoi(mount.substr(i + 1, 3), nullptr, 8)));
                i += 3;
            } else {
                decoded.push_back(mount[i]);
            }
        }
        const bool removable = decoded.rfind("/run/media/", 0) == 0 || decoded.rfind("/media/", 0) == 0;
        if (!removable && decoded.rfind("/mnt/", 0) != 0) continue;
        if (std::any_of(out.begin(), out.end(), [&](const Volume& v) { return v.mountPoint == decoded; })) continue;
        out.push_back({fs::path(decoded).filename().string(), decoded, removable});
    }
    return out;
}

std::vector<Volume> cloudVolumes() {
    std::vector<Volume> out;
    std::error_code ec;

    // 1. Check /run/user/<uid>/gvfs/
    std::string gvfsDir = "/run/user/" + std::to_string(getuid()) + "/gvfs";
    if (fs::exists(gvfsDir, ec)) {
        for (const auto& entry : fs::directory_iterator(gvfsDir, ec)) {
            if (entry.is_directory(ec)) {
                std::string name = entry.path().filename().string();
                std::string displayName = name;
                if (name.rfind("google-drive:", 0) == 0) {
                    displayName = "Google Drive";
                    size_t hostPos = name.find("host=");
                    if (hostPos != std::string::npos) {
                        std::string host = name.substr(hostPos + 5);
                        size_t comma = host.find(',');
                        if (comma != std::string::npos) host = host.substr(0, comma);
                        if (!host.empty()) displayName = "Google Drive (" + host + ")";
                    }
                } else if (name.rfind("dav:", 0) == 0 || name.rfind("davs:", 0) == 0) {
                    displayName = "Nextcloud";
                } else if (name.rfind("smb-share:", 0) == 0) {
                    displayName = "Windows Share (SMB)";
                } else if (name.rfind("mtp:", 0) == 0) {
                    displayName = "Mobile Device (MTP)";
                }
                out.push_back({displayName, entry.path().string(), false});
            }
        }
    }

    // 2. Check ~/Cloud/*
    std::string cloudDir = homeDirectory() + "/Cloud";
    if (fs::exists(cloudDir, ec)) {
        for (const auto& entry : fs::directory_iterator(cloudDir, ec)) {
            if (entry.is_directory(ec)) {
                out.push_back({entry.path().filename().string(), entry.path().string(), false});
            }
        }
    }

    // 3. Check ~/Google Drive, ~/Nextcloud, etc.
    for (const auto& name : {"Google Drive", "Nextcloud", "OwnCloud", "Dropbox"}) {
        std::string p = homeDirectory() + "/" + name;
        if (fs::exists(p, ec)) {
            out.push_back({name, p, false});
        }
    }

    return out;
}

std::string uniqueName(const std::string& folder, const std::string& name) {
    std::error_code ec;
    if (!fs::exists(fs::path(folder) / name, ec)) return name;
    const fs::path p(name);
    std::string stem = p.stem().string();
    std::string ext = p.extension().string();
    if (stem.empty()) {  // ".hidden"
        stem = name;
        ext.clear();
    }
    for (int i = 2;; ++i) {
        std::string candidate = stem + " " + std::to_string(i) + ext;
        if (!fs::exists(fs::path(folder) / candidate, ec)) return candidate;
    }
}

std::string copyName(const std::string& folder, const std::string& name) {
    const fs::path p(name);
    std::string stem = p.stem().string();
    std::string ext = p.extension().string();
    if (stem.empty()) {
        stem = name;
        ext.clear();
    }
    std::error_code ec;
    std::string candidate = stem + " copy" + ext;
    for (int i = 2; fs::exists(fs::path(folder) / candidate, ec); ++i) candidate = stem + " copy " + std::to_string(i) + ext;
    return candidate;
}

// --- Trash -------------------------------------------------------------------------------------------

std::optional<TrashedItem> moveToTrash(const std::string& path, std::string* error) {
    struct stat st {};
    if (::lstat(path.c_str(), &st) != 0) {
        if (error) *error = std::strerror(errno);
        return std::nullopt;
    }
    std::string trash = homeTrash();
    std::error_code ec;
    fs::create_directories(trash + "/files", ec);
    fs::create_directories(trash + "/info", ec);
    struct stat trashStat {};
    bool relative = false;
    std::string top;
    if (::stat(trash.c_str(), &trashStat) != 0 || trashStat.st_dev != st.st_dev) {
        // Another file system (USB stick): use its own $topdir/.Trash-$uid so nothing is copied.
        top = mountTop(path);
        trash = top + "/.Trash-" + std::to_string(::getuid());
        fs::create_directories(trash + "/files", ec);
        fs::create_directories(trash + "/info", ec);
        fs::permissions(trash, fs::perms::owner_all, ec);
        relative = true;
    }
    const std::string base = fs::path(path).filename().string();
    std::string name = base;
    std::string infoPath;
    for (int i = 2;; ++i) {
        infoPath = trash + "/info/" + name + ".trashinfo";
        // O_EXCL claims the name atomically, as the spec asks.
        int fd = ::open(infoPath.c_str(), O_WRONLY | O_CREAT | O_EXCL, 0600);
        if (fd >= 0 && !fs::exists(trash + "/files/" + name, ec)) {
            std::string original = relative ? path.substr(top.size() + (top == "/" ? 0 : 1)) : path;
            std::string info = "[Trash Info]\nPath=" + percentEncode(original) + "\nDeletionDate=" + localTimestamp() + "\n";
            bool ok = ::write(fd, info.data(), info.size()) == static_cast<ssize_t>(info.size());
            ::close(fd);
            if (!ok) {
                ::unlink(infoPath.c_str());
                if (error) *error = "Couldn't write to the Trash.";
                return std::nullopt;
            }
            break;
        }
        if (fd >= 0) {
            ::close(fd);
            ::unlink(infoPath.c_str());
        } else if (errno != EEXIST) {
            if (error) *error = std::strerror(errno);
            return std::nullopt;
        }
        const fs::path p(base);
        name = p.stem().string() + "." + std::to_string(i) + p.extension().string();
    }
    const std::string target = trash + "/files/" + name;
    if (::rename(path.c_str(), target.c_str()) != 0) {
        if (error) *error = std::strerror(errno);
        ::unlink(infoPath.c_str());
        return std::nullopt;
    }
    return TrashedItem{path, target, infoPath};
}

std::optional<std::string> trashOriginalPath(const std::string& trashedPath) {
    const fs::path p(trashedPath);
    const fs::path trash = p.parent_path().parent_path();
    std::ifstream in((trash / "info" / (p.filename().string() + ".trashinfo")).string());
    std::string line;
    while (std::getline(in, line)) {
        if (line.rfind("Path=", 0) != 0) continue;
        std::string original = percentDecode(line.substr(5));
        if (!original.empty() && original[0] != '/') original = (trash.parent_path() / original).string();
        return original;
    }
    return std::nullopt;
}

bool isInTrash(const std::string& path) { return isInside(path, trashFilesDirectory()) && path != trashFilesDirectory(); }

void emptyTrash() {
    std::error_code ec;
    for (const char* sub : {"/files", "/info"}) {
        for (const auto& entry : fs::directory_iterator(homeTrash() + sub, ec)) fs::remove_all(entry.path(), ec);
    }
}

std::string kindLabel(FileKind kind, bool link) {
    if (link) return tr("Symbolic Link");
    switch (kind) {
        case FileKind::Folder: return tr("Folder");
        case FileKind::Image: return tr("Image");
        case FileKind::Video: return tr("Video");
        case FileKind::Audio: return tr("Audio");
        case FileKind::Pdf: return tr("PDF Document");
        case FileKind::Text: return tr("Plain Text");
        case FileKind::Code: return tr("Source Code");
        case FileKind::Archive: return tr("Archive");
        case FileKind::Executable: return tr("Unix Executable");
        case FileKind::Document: return tr("Document");
        case FileKind::Spreadsheet: return tr("Spreadsheet");
        case FileKind::Presentation: return tr("Presentation");
        case FileKind::DiskImage: return tr("Disk Image");
        case FileKind::Font: return tr("Font");
        case FileKind::Web: return tr("Web Page");
        case FileKind::Unknown: return tr("Document");
    }
    return tr("Document");
}

bool isExecutableType(const std::string& mimeType) {
    static const char* const types[] = {"application/x-executable", "application/x-sharedlib",
                                        "application/x-pie-executable", "application/x-msdos-program",
                                        "application/vnd.microsoft.portable-executable", "application/x-msi",
                                        "application/x-shared-library-launcher", "application/x-iso9660-appimage",
                                        "application/vnd.appimage", "application/x-appimage"};
    for (const char* t : types) {
        if (mimeType == t) return true;
    }
    return false;
}

std::string setExecutable(const std::string& path) {
    struct stat st {};
    if (::stat(path.c_str(), &st) != 0) return tr("Item not found:") + " " + std::strerror(errno) + ".";
    const mode_t mode = (st.st_mode & 07777) | 0111;
    if (::chmod(path.c_str(), mode) != 0) {
        if (errno == EPERM) return tr("Only the owner can add execute permission.");
        return tr("Cannot change permissions:") + " " + std::strerror(errno) + ".";
    }
    return {};
}

OpenResult launchWithDefaultApp(const std::string& path) {
    OpenResult result;
    struct stat st {};
    if (::stat(path.c_str(), &st) != 0) {
        result.error = tr("Cannot find this item. It may have been moved or deleted.");
        return result;
    }
    const std::string type = mimeTypeFor(path);
    const bool isProgram = (st.st_mode & 0111) != 0;
    if (S_ISREG(st.st_mode) && !isProgram && (isExecutableType(type) || lower(fs::path(path).extension().string()) == ".appimage")) {
        result.needsExecutableBit = true;
        return result;
    }
    const std::string id = defaultApplicationId(path);
    if (!id.empty()) {
        const Application* app = findApplication(id);
        result.application = app ? app->name : id;
        if (launchApplication(id, path, &result.error)) {
            result.launched = true;
            return result;
        }
        if (!result.error.empty()) return result;
    }
    if (S_ISREG(st.st_mode) && isProgram) {
        if (spawnDetachedProcess({path}, &result.error)) {
            result.launched = true;
            result.application = displayName(path);
            return result;
        }
        if (result.error.empty()) result.error = tr("Cannot run") + " “" + displayName(path) + "”.";
        return result;
    }
    result.noDefaultApp = true;
    result.type = type;
    return result;
}

void openWithDefaultApp(const std::string& path) {
    std::error_code ec;
    if (fs::is_directory(path, ec)) {
        spawnDetached({"finder", path});
        return;
    }
    spawnDetached({"xdg-open", path});
}

std::string openTerminal(const std::string& folder) {
    // Our own Terminal first, like Finder opens Terminal.app. It ships next to us in /usr/bin; when running from
    // a build tree, look for the sibling build directory.
    std::error_code ec;
    const fs::path self = fs::read_symlink("/proc/self/exe", ec);
    for (const fs::path& own : {self.parent_path() / "terminal", self.parent_path().parent_path() / "terminal" / "terminal"}) {
        if (!ec && fs::exists(own, ec) && own != self) {
            spawnDetached({own.string(), folder}, folder);
            return {};
        }
    }
    if (onPath("terminal")) {
        spawnDetached({"terminal", folder}, folder);
        return {};
    }
    if (const char* t = std::getenv("TERMINAL"); t && *t && onPath(t)) {
        spawnDetached({t}, folder);
        return {};
    }
    // Most terminals run as a single process per user, so a new window starts in *that* process's directory
    // unless it's passed explicitly; the working directory of the child alone isn't enough.
    const std::vector<std::vector<std::string>> known = {
        {"xdg-terminal-exec", "--dir=" + folder},
        {"ptyxis", "--new-window", "--working-directory=" + folder},
        {"kgx", "--working-directory=" + folder},
        {"gnome-terminal", "--working-directory=" + folder},
        {"konsole", "--workdir", folder},
        {"kitty", "--directory", folder},
        {"alacritty", "--working-directory", folder},
        {"foot", "--working-directory=" + folder},
        {"wezterm", "start", "--cwd", folder},
        {"ghostty", "--working-directory=" + folder},
        {"xfce4-terminal", "--working-directory=" + folder},
        {"tilix", "--working-directory=" + folder},
        {"xterm"},
    };
    for (const auto& cmd : known) {
        if (onPath(cmd[0])) {
            spawnDetached(cmd, folder);
            return {};
        }
    }
    return "No terminal found.";
}

}  // namespace finder
