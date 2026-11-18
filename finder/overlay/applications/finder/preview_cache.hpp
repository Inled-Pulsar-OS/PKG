#pragma once

// Asynchronous thumbnails (pictures) and text previews with a bounded memory budget.
//
// Views ask for previews while drawing; misses are queued for worker threads (newest request first, so
// whatever is on screen right now wins) and results come back on the UI thread. Picture thumbnails live
// only as GPU textures (ImageData frees its CPU copy after upload) and the least recently drawn ones are
// evicted once the budget is exceeded.

#include <mayonnaise/mayonnaise.hpp>

#include <condition_variable>
#include <deque>
#include <functional>
#include <list>
#include <mutex>
#include <string>
#include <thread>
#include <unordered_map>
#include <vector>

#include "file_icons.hpp"
#include "file_model.hpp"

namespace finder {

class PreviewCache {
public:
    PreviewCache(mayo::App& app, std::function<void()> onReady, unsigned threads = 2);
    ~PreviewCache();
    PreviewCache(const PreviewCache&) = delete;
    PreviewCache& operator=(const PreviewCache&) = delete;

    /// Preview for `e` drawn `pixels` device pixels wide; nullptr until loaded (and for files without one).
    const Preview* get(const FileEntry& e, float pixels);
    /// Decodes a large version (Quick Look, preview column). `done` runs on the UI thread.
    void loadLarge(const std::string& path, int maxDimension,
                   std::function<void(std::shared_ptr<mayo::ImageData>)> done);
    /// Drops queued work (the user left the folder); finished results still land in the cache.
    void cancelPending();

    [[nodiscard]] size_t bytesUsed() const { return m_bytes; }

private:
    struct Job {
        std::string key;
        std::string path;
        bool text = false;
        int pixels = 0;   // 0 = large image job
        std::function<void(std::shared_ptr<mayo::ImageData>)> done;
    };
    struct Entry {
        Preview preview;
        int pixels = 0;   // thumbnail size loaded (or being loaded)
        bool loading = false;
        size_t bytes = 0;
        std::list<std::string>::iterator lru;
    };

    void worker();
    void enqueue(Job job);
    void insert(const std::string& key, Preview preview, int pixels);
    void evict();

    mayo::App& m_app;
    std::function<void()> m_onReady;
    std::unordered_map<std::string, Entry> m_entries;  // UI thread only
    std::list<std::string> m_lru;                       // front = most recently drawn
    size_t m_bytes = 0;

    std::mutex m_mutex;
    std::condition_variable m_cv;
    std::deque<Job> m_queue;  // back = newest
    bool m_stop = false;
    std::vector<std::thread> m_threads;
    std::shared_ptr<bool> m_alive = std::make_shared<bool>(true);  // guards posted results after destruction
};

}  // namespace finder
