#include "preview_cache.hpp"

#include <algorithm>
#include <cstring>
#include <fstream>

using namespace mayo;

namespace finder {

namespace {

constexpr size_t kBudget = 48u << 20;  // bytes of thumbnail textures kept around
constexpr size_t kMaxQueue = 400;      // older requests are dropped (scrolled past long ago)
constexpr std::uintmax_t kMaxImageFile = 80u << 20;

/// Thumbnail sizes are bucketed so a slightly different icon size doesn't reload everything.
int bucketFor(float pixels) { return pixels <= 40 ? 64 : pixels <= 110 ? 128 : 256; }

std::vector<std::uint8_t> readTextLines(const std::string& path) {
    std::ifstream in(path, std::ios::binary);
    char buf[3072];
    in.read(buf, sizeof buf);
    const auto n = static_cast<size_t>(in.gcount());
    if (std::memchr(buf, 0, n)) return {};  // binary
    std::vector<std::uint8_t> lines;
    int len = 0;
    for (size_t i = 0; i < n && lines.size() < 28; ++i) {
        const auto c = static_cast<unsigned char>(buf[i]);
        if (c == '\n') {
            lines.push_back(static_cast<std::uint8_t>(std::min(len, 80)));
            len = 0;
        } else if (c == '\t') {
            len += 4;
        } else if ((c & 0xC0) != 0x80) {  // count UTF-8 lead bytes only
            ++len;
        }
    }
    if (len > 0 && lines.size() < 28) lines.push_back(static_cast<std::uint8_t>(std::min(len, 80)));
    // A text file whose first lines are all empty would draw a blank page; that's fine and honest.
    return lines;
}

}  // namespace

PreviewCache::PreviewCache(App& app, std::function<void()> onReady, unsigned threads)
    : m_app(app), m_onReady(std::move(onReady)) {
    for (unsigned i = 0; i < threads; ++i) m_threads.emplace_back([this] { worker(); });
}

PreviewCache::~PreviewCache() {
    *m_alive = false;
    {
        std::lock_guard lock(m_mutex);
        m_stop = true;
        m_queue.clear();
    }
    m_cv.notify_all();
    for (auto& t : m_threads) t.join();
}

const Preview* PreviewCache::get(const FileEntry& e, float pixels) {
    if (e.isDir) return nullptr;
    const bool image = isImageExtension(e.ext) && e.size > 0 && e.size <= kMaxImageFile;
    const bool text = !image && pixels >= 40 && isTextLike(e) && e.size > 0;  // too small to show lines
    if (!image && !text) return nullptr;
    const int want = image ? bucketFor(pixels) : 1;
    // The modification time is part of the key so edited files get a fresh preview.
    std::string key = e.path;
    key += '#';
    key += std::to_string(e.modified);

    auto it = m_entries.find(key);
    if (it == m_entries.end()) {
        m_lru.push_front(key);
        it = m_entries.emplace(key, Entry{}).first;
        it->second.lru = m_lru.begin();
    } else {
        m_lru.splice(m_lru.begin(), m_lru, it->second.lru);
    }
    Entry& entry = it->second;
    if (!entry.loading && !entry.preview.failed && entry.pixels < want) {
        entry.loading = true;
        entry.pixels = want;
        enqueue({key, e.path, text, want, {}});
    }
    const Preview& p = entry.preview;
    return (p.image || !p.lines.empty()) ? &p : nullptr;
}

void PreviewCache::loadLarge(const std::string& path, int maxDimension,
                             std::function<void(std::shared_ptr<ImageData>)> done) {
    enqueue({{}, path, false, -maxDimension, std::move(done)});
}

void PreviewCache::cancelPending() {
    std::vector<std::string> dropped;
    {
        std::lock_guard lock(m_mutex);
        for (auto& job : m_queue) {
            if (job.pixels > 0 || job.text) dropped.push_back(job.key);
        }
        std::erase_if(m_queue, [](const Job& j) { return j.pixels > 0 || j.text; });
    }
    for (const auto& key : dropped) {
        // Not loaded after all: let a later get() ask again.
        auto it = m_entries.find(key);
        if (it == m_entries.end()) continue;
        it->second.loading = false;
        if (!it->second.preview.image) it->second.pixels = 0;
    }
}

void PreviewCache::enqueue(Job job) {
    {
        std::lock_guard lock(m_mutex);
        m_queue.push_back(std::move(job));
        while (m_queue.size() > kMaxQueue) {
            // Drop the oldest thumbnail request; large (Quick Look) requests are never dropped.
            auto old = std::find_if(m_queue.begin(), m_queue.end(), [](const Job& j) { return j.pixels >= 0; });
            if (old == m_queue.end()) break;
            std::string key = old->key;
            m_queue.erase(old);
            m_app.post([this, alive = m_alive, key] {
                if (!*alive) return;
                if (auto it = m_entries.find(key); it != m_entries.end()) {
                    it->second.loading = false;
                    if (!it->second.preview.image) it->second.pixels = 0;
                }
            });
        }
    }
    m_cv.notify_one();
}

void PreviewCache::worker() {
    for (;;) {
        Job job;
        {
            std::unique_lock lock(m_mutex);
            m_cv.wait(lock, [&] { return m_stop || !m_queue.empty(); });
            if (m_stop) return;
            // Large requests first (the user is waiting for them), then the newest thumbnail.
            auto large = std::find_if(m_queue.begin(), m_queue.end(), [](const Job& j) { return j.pixels < 0; });
            if (large != m_queue.end()) {
                job = std::move(*large);
                m_queue.erase(large);
            } else {
                job = std::move(m_queue.back());
                m_queue.pop_back();
            }
        }
        if (job.pixels < 0) {
            auto image = ImageData::load(job.path, -job.pixels);
            m_app.post([alive = m_alive, done = std::move(job.done), image] {
                if (*alive && done) done(image);
            });
            continue;
        }
        Preview preview;
        if (job.text) {
            preview.lines = readTextLines(job.path);
        } else {
            preview.image = ImageData::load(job.path, job.pixels);
        }
        preview.failed = !preview.image && preview.lines.empty();
        m_app.post([this, alive = m_alive, key = std::move(job.key), preview = std::move(preview), px = job.pixels] {
            if (*alive) insert(key, preview, px);
        });
    }
}

void PreviewCache::insert(const std::string& key, Preview preview, int pixels) {
    auto it = m_entries.find(key);
    if (it == m_entries.end()) {
        m_lru.push_front(key);
        it = m_entries.emplace(key, Entry{}).first;
        it->second.lru = m_lru.begin();
    }
    Entry& e = it->second;
    m_bytes -= e.bytes;
    e.loading = false;
    e.pixels = std::max(e.pixels, pixels);
    e.preview = std::move(preview);
    // RGBA8 plus mipmaps (+1/3).
    e.bytes = e.preview.image ? static_cast<size_t>(e.preview.image->width()) * e.preview.image->height() * 16 / 3
                              : e.preview.lines.size() + 64;
    m_bytes += e.bytes;
    evict();
    if (m_onReady) m_onReady();
}

void PreviewCache::evict() {
    // Keep the most recently drawn ones; the visible set is always far below the budget.
    while (m_bytes > kBudget && m_lru.size() > 1) {
        const std::string key = m_lru.back();
        auto it = m_entries.find(key);
        if (it != m_entries.end()) {
            if (it->second.loading) break;  // in flight; try again after it lands
            m_bytes -= it->second.bytes;
            m_entries.erase(it);
        }
        m_lru.pop_back();
    }
}

}  // namespace finder
