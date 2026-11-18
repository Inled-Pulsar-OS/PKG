#include "tag_manager.hpp"

#include <sys/xattr.h>
#include <algorithm>
#include <filesystem>
#include <fstream>
#include <sstream>

#include "file_model.hpp"

namespace fs = std::filesystem;

namespace finder {

namespace {

std::string getXattr(const std::string& path, const char* name) {
    char buf[1024];
    ssize_t res = getxattr(path.c_str(), name, buf, sizeof(buf) - 1);
    if (res > 0) {
        buf[res] = '\0';
        return std::string(buf);
    }
    return {};
}

void setXattr(const std::string& path, const char* name, const std::string& val) {
    setxattr(path.c_str(), name, val.c_str(), val.size(), 0);
}

void removeXattr(const std::string& path, const char* name) {
    removexattr(path.c_str(), name);
}

}  // namespace

TagManager& TagManager::instance() {
    static TagManager inst;
    return inst;
}

TagManager::TagManager() {
    m_tags = {
        {"Red", mayo::Color::hex(0xFF3B30)},
        {"Orange", mayo::Color::hex(0xFF9500)},
        {"Yellow", mayo::Color::hex(0xFFCC00)},
        {"Green", mayo::Color::hex(0x34C759)},
        {"Blue", mayo::Color::hex(0x007AFF)},
        {"Purple", mayo::Color::hex(0xAF52DE)},
        {"Gray", mayo::Color::hex(0x8E8E93)},
    };
    load();
}

const std::vector<TagInfo>& TagManager::allTags() const {
    return m_tags;
}

mayo::Color TagManager::colorForTag(const std::string& tag) const {
    for (const auto& t : m_tags) {
        if (t.name == tag) return t.color;
    }
    return mayo::Color::hex(0x8E8E93);
}

std::string TagManager::configPath() const {
    std::string home = homeDirectory();
    std::string dir = home + "/.config/mayonnaise/finder";
    return dir + "/tags.json";
}

void TagManager::load() {
    m_fileTags.clear();
    std::string path = configPath();
    std::ifstream in(path);
    if (!in.is_open()) return;

    std::string line;
    std::string currentPath;
    while (std::getline(in, line)) {
        // Simple JSON parser for "path": ["tag1", "tag2"]
        auto colon = line.find("\": [");
        if (colon != std::string::npos) {
            auto q1 = line.find('"');
            if (q1 != std::string::npos && q1 < colon) {
                currentPath = line.substr(q1 + 1, colon - q1 - 1);
                size_t pos = colon + 4;
                while (pos < line.size()) {
                    auto tq1 = line.find('"', pos);
                    if (tq1 == std::string::npos) break;
                    auto tq2 = line.find('"', tq1 + 1);
                    if (tq2 == std::string::npos) break;
                    std::string tag = line.substr(tq1 + 1, tq2 - tq1 - 1);
                    m_fileTags[currentPath].insert(tag);
                    pos = tq2 + 1;
                }
            }
        }
    }
}

void TagManager::save() {
    std::string path = configPath();
    std::error_code ec;
    fs::create_directories(fs::path(path).parent_path(), ec);
    std::ofstream out(path);
    if (!out.is_open()) return;

    out << "{\n";
    bool first = true;
    for (const auto& [p, tags] : m_fileTags) {
        if (tags.empty()) continue;
        if (!first) out << ",\n";
        first = false;
        out << "  \"" << p << "\": [";
        bool fTag = true;
        for (const auto& t : tags) {
            if (!fTag) out << ", ";
            fTag = false;
            out << "\"" << t << "\"";
        }
        out << "]";
    }
    out << "\n}\n";
}

std::vector<std::string> TagManager::tagsFor(const std::string& path) {
    std::vector<std::string> res;
    // Check xattr first if set
    std::string xattr = getXattr(path, "user.xdg.tags");
    if (!xattr.empty()) {
        std::stringstream ss(xattr);
        std::string t;
        while (std::getline(ss, t, ',')) {
            if (!t.empty() && m_fileTags[path].count(t) == 0) {
                m_fileTags[path].insert(t);
            }
        }
    }
    auto it = m_fileTags.find(path);
    if (it != m_fileTags.end()) {
        for (const auto& tag : it->second) res.push_back(tag);
    }
    return res;
}

bool TagManager::hasTag(const std::string& path, const std::string& tag) {
    auto it = m_fileTags.find(path);
    if (it == m_fileTags.end()) return false;
    return it->second.count(tag) > 0;
}

void TagManager::toggleTag(const std::string& path, const std::string& tag) {
    if (hasTag(path, tag)) {
        removeTag(path, tag);
    } else {
        addTag(path, tag);
    }
}

void TagManager::addTag(const std::string& path, const std::string& tag) {
    m_fileTags[path].insert(tag);
    save();
    // Sync xattr
    std::string all;
    for (const auto& t : m_fileTags[path]) {
        if (!all.empty()) all += ",";
        all += t;
    }
    setXattr(path, "user.xdg.tags", all);
}

void TagManager::removeTag(const std::string& path, const std::string& tag) {
    auto it = m_fileTags.find(path);
    if (it != m_fileTags.end()) {
        it->second.erase(tag);
        if (it->second.empty()) {
            m_fileTags.erase(it);
            removeXattr(path, "user.xdg.tags");
        } else {
            std::string all;
            for (const auto& t : it->second) {
                if (!all.empty()) all += ",";
                all += t;
            }
            setXattr(path, "user.xdg.tags", all);
        }
        save();
    }
}

void TagManager::clearTags(const std::string& path) {
    m_fileTags.erase(path);
    removeXattr(path, "user.xdg.tags");
    save();
}

std::vector<std::string> TagManager::filesWithTag(const std::string& tag) {
    std::vector<std::string> res;
    for (const auto& [p, tags] : m_fileTags) {
        if (tags.count(tag) > 0) {
            std::error_code ec;
            if (fs::exists(p, ec)) res.push_back(p);
        }
    }
    return res;
}

}  // namespace finder
