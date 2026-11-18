#include "preview_cache.hpp"

#include <algorithm>
#include <array>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <unistd.h>

using namespace mayo;
namespace fs = std::filesystem;

namespace finder {

namespace {

constexpr size_t kBudget = 64u << 20;  // bytes of thumbnail textures kept around
constexpr size_t kMaxQueue = 400;      // older requests are dropped (scrolled past long ago)
constexpr std::uintmax_t kMaxImageFile = 100u << 20;

/// Thumbnail sizes are bucketed so a slightly different icon size doesn't reload everything.
int bucketFor(float pixels) { return pixels <= 40 ? 64 : pixels <= 110 ? 128 : 256; }

// --- Compact standard RFC 1321 MD5 for FreeDesktop thumbnail lookup ---
struct MD5Context {
    uint32_t state[4] = {0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476};
    uint32_t count[2] = {0, 0};
    uint8_t buffer[64] = {};
};

void md5Transform(uint32_t state[4], const uint8_t block[64]) {
    uint32_t a = state[0], b = state[1], c = state[2], d = state[3], x[16];
    for (int i = 0, j = 0; i < 16; ++i, j += 4)
        x[i] = (uint32_t)block[j] | ((uint32_t)block[j+1] << 8) | ((uint32_t)block[j+2] << 16) | ((uint32_t)block[j+3] << 24);

    #define S(x, n) (((x) << (n)) | ((x) >> (32 - (n))))
    #define FF(a, b, c, d, x, s, ac) { a += ((b & c) | (~b & d)) + x + ac; a = S(a, s) + b; }
    #define GG(a, b, c, d, x, s, ac) { a += ((b & d) | (c & ~d)) + x + ac; a = S(a, s) + b; }
    #define HH(a, b, c, d, x, s, ac) { a += (b ^ c ^ d) + x + ac; a = S(a, s) + b; }
    #define II(a, b, c, d, x, s, ac) { a += (c ^ (b | ~d)) + x + ac; a = S(a, s) + b; }

    FF(a, b, c, d, x[ 0], 7, 0xd76aa478); FF(d, a, b, c, x[ 1], 12, 0xe8c7b756);
    FF(c, d, a, b, x[ 2], 17, 0x242070db); FF(b, c, d, a, x[ 3], 22, 0xc1bdceee);
    FF(a, b, c, d, x[ 4], 7, 0xf57c0faf); FF(d, a, b, c, x[ 5], 12, 0x4787c62a);
    FF(c, d, a, b, x[ 6], 17, 0xa8304613); FF(b, c, d, a, x[ 7], 22, 0xfd469501);
    FF(a, b, c, d, x[ 8], 7, 0x698098d8); FF(d, a, b, c, x[ 9], 12, 0x8b44f7af);
    FF(c, d, a, b, x[10], 17, 0xffff5bb1); FF(b, c, d, a, x[11], 22, 0x895cd7be);
    FF(a, b, c, d, x[12], 7, 0x6b901122); FF(d, a, b, c, x[13], 12, 0xfd987193);
    FF(c, d, a, b, x[14], 17, 0xa679438e); FF(b, c, d, a, x[15], 22, 0x49b40821);

    GG(a, b, c, d, x[ 1], 5, 0xf61e2562); GG(d, a, b, c, x[ 6], 9, 0xc040b340);
    GG(c, d, a, b, x[11], 14, 0x265e5a51); GG(b, c, d, a, x[ 0], 20, 0xe9b6c7aa);
    GG(a, b, c, d, x[ 5], 5, 0xd62f105d); GG(d, a, b, c, x[10], 9, 0x02441453);
    GG(c, d, a, b, x[15], 14, 0xd8a1e681); GG(b, c, d, a, x[ 4], 20, 0xe7d3fbc8);
    GG(a, b, c, d, x[ 9], 5, 0x21e1cde6); GG(d, a, b, c, x[14], 9, 0xc33707d6);
    GG(c, d, a, b, x[ 3], 14, 0xf4d50d87); GG(b, c, d, a, x[ 8], 20, 0x455a14ed);
    GG(a, b, c, d, x[13], 5, 0xa9e3e905); GG(d, a, b, c, x[ 2], 9, 0xfcefa3f8);
    GG(c, d, a, b, x[ 7], 14, 0x676f02d9); GG(b, c, d, a, x[12], 20, 0x8d2a4c8a);

    HH(a, b, c, d, x[ 5], 4, 0xfffa3942); HH(d, a, b, c, x[ 8], 11, 0x8771f681);
    HH(c, d, a, b, x[11], 16, 0x6d9d6122); HH(b, c, d, a, x[14], 23, 0xfde5380c);
    HH(a, b, c, d, x[ 1], 4, 0xa4beea44); HH(d, a, b, c, x[ 4], 11, 0x4bdecfa9);
    HH(c, d, a, b, x[ 7], 16, 0xf6bb4b60); HH(b, c, d, a, x[10], 23, 0xbebfbc70);
    HH(a, b, c, d, x[13], 4, 0x289b7ec6); HH(d, a, b, c, x[ 0], 11, 0xeaa127fa);
    HH(c, d, a, b, x[ 3], 16, 0xd4ef3085); HH(b, c, d, a, x[ 6], 23, 0x04881d05);
    HH(a, b, c, d, x[ 9], 4, 0xd9d4d039); HH(d, a, b, c, x[12], 11, 0xe6db99e5);
    HH(c, d, a, b, x[15], 16, 0x1fa27cf8); HH(b, c, d, a, x[ 2], 23, 0xc4ac5665);

    II(a, b, c, d, x[ 0], 6, 0xf4292244); II(d, a, b, c, x[ 7], 10, 0x432aff97);
    II(c, d, a, b, x[14], 15, 0xab9423a7); II(b, c, d, a, x[ 5], 21, 0xfc93a039);
    II(a, b, c, d, x[12], 6, 0x655b59c3); II(d, a, b, c, x[ 3], 10, 0x8f0ccc92);
    II(c, d, a, b, x[10], 15, 0xffeff47d); II(b, c, d, a, x[ 1], 21, 0x85845dd1);
    II(a, b, c, d, x[ 8], 6, 0x6fa87e4f); II(d, a, b, c, x[15], 10, 0xfe2ce6e0);
    II(c, d, a, b, x[ 6], 15, 0xa3014314); II(b, c, d, a, x[13], 21, 0x4e0811a1);
    II(a, b, c, d, x[ 4], 6, 0xf7537e82); II(d, a, b, c, x[ 2], 10, 0xbd3af235);
    II(c, d, a, b, x[ 9], 15, 0x2ad7d2bb); II(b, c, d, a, x[14], 21, 0xeb86d391);
    #undef S
    #undef FF
    #undef GG
    #undef HH
    #undef II
    state[0] += a; state[1] += b; state[2] += c; state[3] += d;
}

std::string md5Hex(const std::string& input) {
    MD5Context ctx;
    size_t len = input.size();
    const uint8_t* data = (const uint8_t*)input.data();
    uint32_t index = (ctx.count[0] >> 3) & 0x3f;
    if ((ctx.count[0] += ((uint32_t)len << 3)) < ((uint32_t)len << 3)) ctx.count[1]++;
    ctx.count[1] += (uint32_t)(len >> 29);
    uint32_t partLen = 64 - index;
    size_t i = 0;
    if (len >= partLen) {
        memcpy(&ctx.buffer[index], data, partLen);
        md5Transform(ctx.state, ctx.buffer);
        for (i = partLen; i + 63 < len; i += 64) md5Transform(ctx.state, &data[i]);
        index = 0;
    }
    memcpy(&ctx.buffer[index], &data[i], len - i);

    uint8_t bits[8];
    for (int k = 0; k < 8; ++k) bits[k] = (uint8_t)((ctx.count[k >= 4 ? 1 : 0] >> ((k % 4) * 8)) & 0xff);
    static const uint8_t PADDING[64] = {0x80};
    index = (ctx.count[0] >> 3) & 0x3f;
    uint32_t padLen = (index < 56) ? (56 - index) : (120 - index);
    // Pad
    uint32_t idx = (ctx.count[0] >> 3) & 0x3f;
    if ((ctx.count[0] += (padLen << 3)) < (padLen << 3)) ctx.count[1]++;
    ctx.count[1] += (padLen >> 29);
    uint32_t pLen = 64 - idx;
    size_t pi = 0;
    if (padLen >= pLen) {
        memcpy(&ctx.buffer[idx], PADDING, pLen);
        md5Transform(ctx.state, ctx.buffer);
        for (pi = pLen; pi + 63 < padLen; pi += 64) md5Transform(ctx.state, &PADDING[pi]);
        idx = 0;
    }
    memcpy(&ctx.buffer[idx], &PADDING[pi], padLen - pi);
    // Append bits
    idx = (ctx.count[0] >> 3) & 0x3f;
    if ((ctx.count[0] += (8 << 3)) < (8 << 3)) ctx.count[1]++;
    ctx.count[1] += (8 >> 29);
    pLen = 64 - idx;
    pi = 0;
    if (8 >= pLen) {
        memcpy(&ctx.buffer[idx], bits, pLen);
        md5Transform(ctx.state, ctx.buffer);
        for (pi = pLen; pi + 63 < 8; pi += 64) md5Transform(ctx.state, &bits[pi]);
        idx = 0;
    }
    memcpy(&ctx.buffer[idx], &bits[pi], 8 - pi);

    char hex[33];
    for (int k = 0; k < 4; ++k) {
        snprintf(&hex[k * 8], 9, "%02x%02x%02x%02x",
                 ctx.state[k] & 0xff, (ctx.state[k] >> 8) & 0xff,
                 (ctx.state[k] >> 16) & 0xff, (ctx.state[k] >> 24) & 0xff);
    }
    return std::string(hex);
}

std::string toFileUri(const std::string& path) {
    std::string uri = "file://";
    for (unsigned char c : path) {
        if ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') ||
            c == '/' || c == '.' || c == '_' || c == '-' || c == '~') {
            uri.push_back(static_cast<char>(c));
        } else {
            char buf[4];
            snprintf(buf, sizeof(buf), "%%%02X", c);
            uri += buf;
        }
    }
    return uri;
}

std::shared_ptr<ImageData> checkFreeDesktopThumb(const std::string& path, int pixels) {
    std::string uri = toFileUri(path);
    std::string hash = md5Hex(uri);
    std::string home = homeDirectory();
    std::vector<std::string> candidates = {
        home + "/.cache/thumbnails/x-large/" + hash + ".png",
        home + "/.cache/thumbnails/large/" + hash + ".png",
        home + "/.cache/thumbnails/normal/" + hash + ".png"
    };
    for (const auto& c : candidates) {
        std::error_code ec;
        if (fs::exists(c, ec) && fs::file_size(c, ec) > 0) {
            auto img = ImageData::load(c, pixels);
            if (img && img->width() > 0) return img;
        }
    }
    return nullptr;
}

std::shared_ptr<ImageData> generateOrFetchThumbnail(const std::string& path, FileKind kind, int pixels) {
    // 1. Check system thumbnail cache first
    auto cached = checkFreeDesktopThumb(path, pixels);
    if (cached) return cached;

    // 2. Format-specific handling
    if (kind == FileKind::Pdf) {
        char tmpPrefix[] = "/tmp/finder_pdf_XXXXXX";
        int fd = mkstemp(tmpPrefix);
        if (fd >= 0) {
            close(fd);
            unlink(tmpPrefix);
            int scale = std::max(128, pixels);
            std::string cmd = "pdftoppm -png -f 1 -l 1 -singlefile -scale-to " + std::to_string(scale) +
                              " \"" + path + "\" " + tmpPrefix + " >/dev/null 2>&1";
            if (std::system(cmd.c_str()) == 0) {
                std::string genPng = std::string(tmpPrefix) + ".png";
                auto img = ImageData::load(genPng, pixels);
                unlink(genPng.c_str());
                if (img) return img;
            }
        }
    } else if (kind == FileKind::Video) {
        char tmpPng[] = "/tmp/finder_vid_XXXXXX.png";
        int fd = mkstemps(tmpPng, 4);
        if (fd >= 0) {
            close(fd);
            unlink(tmpPng);
            int scale = std::max(256, pixels);
            std::string cmd = "totem-video-thumbnailer -s " + std::to_string(scale) +
                              " \"" + path + "\" \"" + tmpPng + "\" >/dev/null 2>&1";
            if (std::system(cmd.c_str()) != 0) {
                cmd = "ffmpeg -y -ss 00:00:01 -i \"" + path + "\" -vframes 1 -vf \"scale='min(" +
                      std::to_string(scale) + ",iw)':-1\" \"" + tmpPng + "\" >/dev/null 2>&1";
                std::system(cmd.c_str());
            }
            auto img = ImageData::load(tmpPng, pixels);
            unlink(tmpPng);
            if (img) return img;
        }
    }

    // Default image loader
    return ImageData::load(path, pixels);
}

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
    const bool video = (e.kind == FileKind::Video) && e.size > 0;
    const bool pdf = (e.kind == FileKind::Pdf) && e.size > 0;
    const bool text = !image && !video && !pdf && pixels >= 40 && isTextLike(e) && e.size > 0;
    if (!image && !video && !pdf && !text) return nullptr;
    const int want = (image || video || pdf) ? bucketFor(pixels) : 1;

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
        enqueue({key, e.path, e.kind, text, want, {}});
    }
    const Preview& p = entry.preview;
    return (p.image || !p.lines.empty()) ? &p : nullptr;
}

void PreviewCache::loadLarge(const std::string& path, int maxDimension,
                             std::function<void(std::shared_ptr<ImageData>)> done,
                             FileKind kind) {
    enqueue({{}, path, kind, false, -maxDimension, std::move(done)});
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
            auto image = generateOrFetchThumbnail(job.path, job.kind, -job.pixels);
            m_app.post([alive = m_alive, done = std::move(job.done), image] {
                if (*alive && done) done(image);
            });
            continue;
        }
        Preview preview;
        if (job.text) {
            preview.lines = readTextLines(job.path);
        } else {
            preview.image = generateOrFetchThumbnail(job.path, job.kind, job.pixels);
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
    e.bytes = e.preview.image ? static_cast<size_t>(e.preview.image->width()) * e.preview.image->height() * 16 / 3
                              : e.preview.lines.size() + 64;
    m_bytes += e.bytes;
    evict();
    if (m_onReady) m_onReady();
}

void PreviewCache::evict() {
    while (m_bytes > kBudget && m_lru.size() > 1) {
        const std::string key = m_lru.back();
        auto it = m_entries.find(key);
        if (it != m_entries.end()) {
            if (it->second.loading) break;
            m_bytes -= it->second.bytes;
            m_entries.erase(it);
        }
        m_lru.pop_back();
    }
}

}  // namespace finder
