#include "archives.hpp"

#include "applications.hpp"
#include "file_model.hpp"
#include <mayonnaise/i18n.hpp>

#include <algorithm>
#include <filesystem>
#include <map>
#include <set>

namespace fs = std::filesystem;
using mayo::i18n::tr;

namespace finder {

bool canCompressFiles(const std::vector<std::string>& paths) {
    return !paths.empty() && std::all_of(paths.begin(), paths.end(), [](const std::string& path) {
        if (isInTrash(path) || path == trashFilesDirectory()) return false;
        std::error_code ec;
        const auto status = fs::status(path, ec);
        return !ec && (fs::is_regular_file(status) || fs::is_directory(status));
    });
}

std::string compressFiles(const std::vector<std::string>& paths) {
    if (!canCompressFiles(paths)) return tr("Please select files or folders outside the trash.");
    std::string parentDir = fs::path(paths.front()).parent_path().string();
    std::string zipName;
    if (paths.size() == 1) {
        std::string stem = fs::path(paths.front()).stem().string();
        zipName = uniqueName(parentDir, stem + ".zip");
    } else {
        zipName = uniqueName(parentDir, "Archive.zip");
    }
    std::string zipPath = (fs::path(parentDir) / zipName).string();

    std::vector<std::string> command{"bsdtar", "-acf", zipPath, "-C", parentDir};
    for (const auto& path : paths) {
        command.push_back(fs::path(path).filename().string());
    }
    std::string error;
    if (!spawnDetachedProcess(command, &error)) {
        return tr("Failed to compress:") + "\n" + error;
    }
    return {};
}

bool canExtractArchives(const std::vector<std::string>& paths) {
    return !paths.empty() && std::all_of(paths.begin(), paths.end(), [](const std::string& path) {
        std::error_code ec;
        if (isInTrash(path) || !fs::is_regular_file(path, ec)) return false;
        const auto entry = statEntry(path);
        if (!entry || entry->isDir) return false;
        if (entry->kind == FileKind::Archive) return true;
        static const std::set<std::string> types{
            "application/zip",
            "application/x-zip",
            "application/x-zip-compressed",
            "application/vnd.rar",
            "application/x-rar",
            "application/x-rar-compressed",
            "application/x-7z-compressed",
            "application/x-tar",
            "application/x-compressed-tar",
            "application/gzip",
            "application/x-gzip",
            "application/x-bzip",
            "application/x-bzip2",
            "application/x-bzip-compressed-tar",
            "application/x-xz",
            "application/x-xz-compressed-tar",
            "application/zstd",
            "application/x-zstd-compressed-tar",
            "application/x-lzma",
            "application/x-lzma-compressed-tar",
            "application/x-lzip",
            "application/x-lzip-compressed-tar",
        };
        return types.contains(mimeTypeFor(path));
    });
}

std::string extractArchives(const std::vector<std::string>& paths, const std::string& destination) {
    if (!canExtractArchives(paths)) return tr("Please select an archive outside the trash.");
    if (destination.empty()) {
        std::map<std::string, std::vector<std::string>> groups;
        for (const auto& path : paths) groups[fs::absolute(path).parent_path().string()].push_back(path);
        for (const auto& [folder, archives] : groups) {
            if (const auto error = extractArchives(archives, folder); !error.empty()) return error;
        }
        return {};
    }
    std::error_code ec;
    if (!fs::is_directory(destination, ec)) return tr("Destination directory does not exist or cannot be accessed.");

    for (const auto& path : paths) {
        std::vector<std::string> command{"bsdtar", "-xf", fs::absolute(path).string(), "-C", fs::absolute(destination).string()};
        std::string error;
        if (!spawnDetachedProcess(command, &error)) {
            return tr("Failed to extract:") + "\n" + error;
        }
    }
    return {};
}

}  // namespace finder
