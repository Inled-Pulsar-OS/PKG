#pragma once

#include <mayonnaise/color.hpp>
#include <string>
#include <vector>
#include <unordered_map>
#include <set>

namespace finder {

struct TagInfo {
    std::string name;
    mayo::Color color;
};

class TagManager {
public:
    static TagManager& instance();

    const std::vector<TagInfo>& allTags() const;
    mayo::Color colorForTag(const std::string& tag) const;

    std::vector<std::string> tagsFor(const std::string& path);
    bool hasTag(const std::string& path, const std::string& tag);
    void toggleTag(const std::string& path, const std::string& tag);
    void addTag(const std::string& path, const std::string& tag);
    void removeTag(const std::string& path, const std::string& tag);
    void clearTags(const std::string& path);

    std::vector<std::string> filesWithTag(const std::string& tag);

private:
    TagManager();
    void load();
    void save();
    std::string configPath() const;

    std::vector<TagInfo> m_tags;
    std::unordered_map<std::string, std::set<std::string>> m_fileTags; // path -> set of tag names
};

}  // namespace finder
