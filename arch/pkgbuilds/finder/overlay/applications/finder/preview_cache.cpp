#include "preview_cache.hpp"

#include <algorithm>
#include <cstring>
#include <fstream>
#include <filesystem>
#include <iomanip>
#include <sstream>

namespace fs = std::filesystem;
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
    return lines;
}

// --- Minimal, self-contained MD5 implementation for FreeDesktop specification ---
struct MD5Context {
    uint32_t state[4];
    uint32_t count[2];
    uint8_t buffer[64];
};

#define F(x, y, z) (((x) & (y)) | ((~x) & (z)))
#define G(x, y, z) (((x) & (z)) | ((y) & (~z)))
#define H(x, y, z) ((x) ^ (y) ^ (z))
#define I(x, y, z) ((y) ^ ((x) | (~z)))
#define ROTATE_LEFT(x, n) (((x) << (n)) | ((x) >> (32 - (n))))
#define FF(a, b, c, d, x, s, ac) { \
    (a) += F((b), (c), (d)) + (x) + (uint32_t)(ac); \
    (a) = ROTATE_LEFT((a), (s)); \
    (a) += (b); \
}
#define GG(a, b, c, d, x, s, ac) { \
    (a) += G((b), (c), (d)) + (x) + (uint32_t)(ac); \
    (a) = ROTATE_LEFT((a), (s)); \
    (a) += (b); \
}
#define HH(a, b, c, d, x, s, ac) { \
    (a) += H((b), (c), (d)) + (x) + (uint32_t)(ac); \
    (a) = ROTATE_LEFT((a), (s)); \
    (a) += (b); \
}
#define II(a, b, c, d, x, s, ac) { \
    (a) += I((b), (c), (d)) + (x) + (uint32_t)(ac); \
    (a) = ROTATE_LEFT((a), (s)); \
    (a) += (b); \
}

void md5_transform(uint32_t state[4], const uint8_t block[64]) {
    uint32_t a = state[0], b = state[1], c = state[2], d = state[3], x[16];
    for (int i = 0, j = 0; i < 16; ++i, j += 4) {
        x[i] = ((uint32_t)block[j]) | (((uint32_t)block[j + 1]) << 8) |
               (((uint32_t)block[j + 2]) << 16) | (((uint32_t)block[j + 3]) << 24);
    }
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
    II(a, b, c, d, x[ 4], 6, 0xf7537e82); II(d, a, b, c, x[11], 10, 0xbd3af235);
    II(c, d, a, b, x[ 2], 15, 0x2ad7d2bb); II(b, c, d, a, x[ 9], 21, 0xeb86d391);

    state[0] += a; state[1] += b; state[2] += c; state[3] += d;
}

void md5_init(MD5Context* ctx) {
    ctx->count[0] = ctx->count[1] = 0;
    ctx->state[0] = 0x67452301;
    ctx->state[1] = 0xefcdab89;
    ctx->state[2] = 0x98badcfe;
    ctx->state[3] = 0x10325476;
}

void md5_update(MD5Context* ctx, const uint8_t* input, size_t inputLen) {
    uint32_t i = 0, index = (ctx->count[0] >> 3) & 0x3F;
    if ((ctx->count[0] += ((uint32_t)inputLen << 3)) < ((uint32_t)inputLen << 3)) ctx->count[1]++;
    ctx->count[1] += ((uint32_t)inputLen >> 29);
    uint32_t partLen = 64 - index;
    if (inputLen >= partLen) {
        std::memcpy(&ctx->buffer[index], input, partLen);
        md5_transform(ctx->state, ctx->buffer);
        for (i = partLen; i + 63 < inputLen; i += 64) md5_transform(ctx->state, &input[i]);
        index = 0;
    }
    std::memcpy(&ctx->buffer[index], &input[i], inputLen - i);
}

void md5_final(MD5Context* ctx, uint8_t digest[16]) {
    static const uint8_t PADDING[64] = { 0x80 };
    uint8_t bits[8];
    for (int i = 0; i < 4; ++i) {
        bits[i] = (uint8_t)((ctx->count[0] >> (i * 8)) & 0xFF);
        bits[i + 4] = (uint8_t)((ctx->count[1] >> (i * 8)) & 0xFF);
    }
    uint32_t index = (ctx->count[0] >> 3) & 0x3F;
    uint32_t padLen = (index < 56) ? (56 - index) : (120 - index);
    md5_update(ctx, PADDING, padLen);
    md5_update(ctx, bits, 8);
    for (int i = 0; i < 4; ++i) {
        digest[i * 4] = (uint8_t)(ctx->state[i] & 0xFF);
        digest[i * 4 + 1] = (uint8_t)((ctx->state[i] >> 8) & 0xFF);
        digest[i * 4 + 2] = (uint8_t)((ctx->state[i] >> 16) & 0xFF);
        digest[i * 4 + 3] = (uint8_t)((ctx->state[i] >> 24) & 0xFF);
    }
}

std::string md5_hex(const std::string& input) {
    MD5Context ctx;
    md5_init(&ctx);
    md5_update(&ctx, (const uint8_t*)input.data(), input.size());
    uint8_t digest[16];
    md5_final(&ctx, digest);
    char buf[33];
    for (int i = 0; i < 16; ++i) std::snprintf(buf + i * 2, 3, "%02x", digest[i]);
    return std::string(buf, 32);
}

std::string urlEncodePath(const std::string& path) {
    std::ostringstream escaped;
    escaped.fill('0');
    escaped << std::hex;
    for (char c : path) {
        if (std::isalnum((unsigned char)c) || c == '-' || c == '_' || c == '.' || c == '~' || c == '/') {
            escaped << c;
        } else {
            escaped << '%' << std::setw(2) << ((int)(unsigned char)c);
        }
    }
    return escaped.str();
}

std::shared_ptr<ImageData> loadOrGenerateThumbnail(const std::string& path, int pixels) {
    std::error_code ec;
    std::string canonical = fs::canonical(path, ec).string();
    if (canonical.empty()) canonical = path;

    std::string ext = fs::path(canonical).extension().string();
    if (!ext.empty() && ext[0] == '.') ext = ext.substr(1);
    std::transform(ext.begin(), ext.end(), ext.begin(), [](unsigned char c) { return std::tolower(c); });

    if (isImageExtension(ext) && ext != "svg") {
        if (auto img = ImageData::load(path, pixels)) return img;
    }

    std::string uri = "file://" + urlEncodePath(canonical);
    std::string hash = md5_hex(uri);

    const char* homeEnv = std::getenv("HOME");
    std::string home = homeEnv ? homeEnv : "/home/jaime";
    std::string thumbDir = home + "/.cache/thumbnails";

    std::string xl = thumbDir + "/x-large/" + hash + ".png";
    std::string l = thumbDir + "/large/" + hash + ".png";
    std::string n = thumbDir + "/normal/" + hash + ".png";

    if (fs::exists(xl, ec)) { if (auto img = ImageData::load(xl, pixels)) return img; }
    if (fs::exists(l, ec)) { if (auto img = ImageData::load(l, pixels)) return img; }
    if (fs::exists(n, ec)) { if (auto img = ImageData::load(n, pixels)) return img; }

    fs::create_directories(thumbDir + "/large", ec);

    if (ext == "mp4" || ext == "mkv" || ext == "mov" || ext == "avi" || ext == "webm" ||
        ext == "flv" || ext == "wmv" || ext == "m4v" || ext == "mpg" || ext == "mpeg") {
        std::string cmd = "ffmpeg -y -ss 00:00:01 -i \"" + canonical + "\" -vframes 1 -vf \"scale=256:256:force_original_aspect_ratio=decrease\" \"" + l + "\" >/dev/null 2>&1";
        if (std::system(cmd.c_str()) == 0 && fs::exists(l, ec)) {
            return ImageData::load(l, pixels);
        }
    } else if (ext == "pdf") {
        std::string tmpPrefix = "/tmp/finder_p_" + hash;
        std::string cmd = "pdftoppm -png -r 100 -f 1 -l 1 -scale-to 256 \"" + canonical + "\" \"" + tmpPrefix + "\" >/dev/null 2>&1";
        if (std::system(cmd.c_str()) == 0) {
            std::string gen = tmpPrefix + "-1.png";
            if (fs::exists(gen, ec)) {
                fs::rename(gen, l, ec);
                return ImageData::load(l, pixels);
            }
        }
    } else if (ext == "svg") {
        std::string cmd = "rsvg-convert -w 256 -h 256 -a -o \"" + l + "\" \"" + canonical + "\" >/dev/null 2>&1";
        if (std::system(cmd.c_str()) == 0 && fs::exists(l, ec)) {
            return ImageData::load(l, pixels);
        }
    }

    return ImageData::load(path, pixels);
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
    const bool isImg = isImageExtension(e.ext) && e.size > 0 && e.size <= kMaxImageFile;
    const bool isVid = e.kind == FileKind::Video && e.size > 0;
    const bool isPdf = e.kind == FileKind::Pdf && e.size > 0;
    const bool isSvg = (e.ext == "svg" || e.ext == "SVG") && e.size > 0;
    const bool image = isImg || isVid || isPdf || isSvg;
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
            auto image = loadOrGenerateThumbnail(job.path, -job.pixels);
            m_app.post([alive = m_alive, done = std::move(job.done), image] {
                if (*alive && done) done(image);
            });
            continue;
        }
        Preview preview;
        if (job.text) {
            preview.lines = readTextLines(job.path);
        } else {
            preview.image = loadOrGenerateThumbnail(job.path, job.pixels);
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
