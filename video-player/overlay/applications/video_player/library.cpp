#include "library.hpp"

#include <mayonnaise/app.hpp>

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/imgutils.h>
#include <libavutil/md5.h>
#include <libswscale/swscale.h>
}

#include <algorithm>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <set>
#include <sstream>

namespace fs = std::filesystem;

namespace {

constexpr int kThumbWidth = 480;  // cards are 240 points wide; 2x for HiDPI

std::string lower(std::string s) {
    for (auto& c : s) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
    return s;
}

std::string homeDir() {
    const char* h = std::getenv("HOME");
    return h ? h : "/";
}

std::string cacheDir() {
    const char* x = std::getenv("XDG_CACHE_HOME");
    return (x && *x) ? x : homeDir() + "/.cache";
}

std::string hex(std::uint64_t v) {
    char buf[17];
    std::snprintf(buf, sizeof buf, "%016llx", static_cast<unsigned long long>(v));
    return buf;
}

/// Cache key: the file's identity, so an edited file gets a new thumbnail.
std::string cacheKey(const std::string& path, std::uintmax_t size, std::int64_t modified) {
    std::uint64_t h = 1469598103934665603ULL;
    auto mix = [&](const std::string& s) {
        for (unsigned char c : s) h = (h ^ c) * 1099511628211ULL;
    };
    mix(path);
    mix(std::to_string(size));
    mix(std::to_string(modified));
    return hex(h);
}

/// file:// URI as the thumbnail spec hashes it (RFC 3986 escaping of everything but unreserved and '/').
std::string fileUri(const std::string& path) {
    std::string out = "file://";
    for (unsigned char c : path) {
        if (std::isalnum(c) || c == '/' || c == '-' || c == '_' || c == '.' || c == '~') {
            out += static_cast<char>(c);
        } else {
            char buf[4];
            std::snprintf(buf, sizeof buf, "%%%02X", c);
            out += buf;
        }
    }
    return out;
}

/// A thumbnail another app (Files, Videos) already made, from the freedesktop.org cache.
std::shared_ptr<mayo::ImageData> sharedThumbnail(const std::string& path) {
    const std::string uri = fileUri(path);
    std::uint8_t digest[16];
    av_md5_sum(digest, reinterpret_cast<const std::uint8_t*>(uri.data()), static_cast<int>(uri.size()));
    std::string md5;
    char buf[3];
    for (auto b : digest) {
        std::snprintf(buf, sizeof buf, "%02x", b);
        md5 += buf;
    }
    std::error_code ec;
    for (const char* size : {"x-large", "large", "normal"}) {
        const std::string t = cacheDir() + "/thumbnails/" + size + "/" + md5 + ".png";
        if (fs::exists(t, ec)) {
            if (auto img = mayo::ImageData::load(t, kThumbWidth)) return img;
        }
    }
    return nullptr;
}

/// Writes RGBA pixels as PNG with FFmpeg's encoder (no extra image library needed).
void writePng(const std::string& file, int w, int h, const std::vector<std::uint8_t>& rgba) {
    const AVCodec* codec = avcodec_find_encoder(AV_CODEC_ID_PNG);
    if (!codec) return;
    AVCodecContext* ctx = avcodec_alloc_context3(codec);
    ctx->width = w;
    ctx->height = h;
    ctx->pix_fmt = AV_PIX_FMT_RGBA;
    ctx->time_base = {1, 25};
    AVFrame* frame = av_frame_alloc();
    AVPacket* pkt = av_packet_alloc();
    if (avcodec_open2(ctx, codec, nullptr) == 0) {
        frame->format = AV_PIX_FMT_RGBA;
        frame->width = w;
        frame->height = h;
        av_image_fill_arrays(frame->data, frame->linesize, rgba.data(), AV_PIX_FMT_RGBA, w, h, 1);
        if (avcodec_send_frame(ctx, frame) == 0 && avcodec_receive_packet(ctx, pkt) == 0) {
            const std::string tmp = file + ".tmp";
            {
                std::ofstream out(tmp, std::ios::binary);
                out.write(reinterpret_cast<const char*>(pkt->data), pkt->size);
            }
            std::error_code ec;
            fs::rename(tmp, file, ec);
        }
    }
    av_packet_free(&pkt);
    av_frame_free(&frame);
    avcodec_free_context(&ctx);
}

/// Scales a decoded frame to kThumbWidth (keeping the display aspect) as RGBA.
bool frameToRgba(const AVFrame* f, AVRational sar, int& outW, int& outH, std::vector<std::uint8_t>& rgba) {
    double aspect = static_cast<double>(f->width) / std::max(1, f->height);
    if (sar.num > 0 && sar.den > 0) aspect *= av_q2d(sar);
    outW = std::min(kThumbWidth, std::max(1, f->width));
    outH = std::max(1, static_cast<int>(outW / aspect + 0.5));
    SwsContext* sws = sws_getContext(f->width, f->height, static_cast<AVPixelFormat>(f->format), outW, outH, AV_PIX_FMT_RGBA,
                                     SWS_BILINEAR, nullptr, nullptr, nullptr);
    if (!sws) return false;
    rgba.assign(static_cast<size_t>(outW) * static_cast<size_t>(outH) * 4, 0);
    std::uint8_t* dst[4] = {rgba.data(), nullptr, nullptr, nullptr};
    int stride[4] = {outW * 4, 0, 0, 0};
    sws_scale(sws, f->data, f->linesize, 0, f->height, dst, stride);
    sws_freeContext(sws);
    return true;
}

/// Decodes one frame about 10% into the video (past black intros).
bool grabFrame(AVFormatContext* fmt, int stream, int& w, int& h, std::vector<std::uint8_t>& rgba) {
    AVStream* st = fmt->streams[stream];
    const AVCodec* codec = avcodec_find_decoder(st->codecpar->codec_id);
    if (!codec) return false;
    AVCodecContext* ctx = avcodec_alloc_context3(codec);
    avcodec_parameters_to_context(ctx, st->codecpar);
    ctx->thread_count = 2;
    bool ok = false;
    if (avcodec_open2(ctx, codec, nullptr) == 0) {
        if (fmt->duration > 0) {
            const std::int64_t target = fmt->duration / 10;
            av_seek_frame(fmt, -1, target, AVSEEK_FLAG_BACKWARD);
        }
        AVPacket* pkt = av_packet_alloc();
        AVFrame* frame = av_frame_alloc();
        for (int packets = 0; packets < 600 && !ok && av_read_frame(fmt, pkt) >= 0; ++packets) {
            if (pkt->stream_index == stream && avcodec_send_packet(ctx, pkt) == 0) {
                if (avcodec_receive_frame(ctx, frame) == 0) ok = frameToRgba(frame, st->sample_aspect_ratio.num ? st->sample_aspect_ratio : frame->sample_aspect_ratio, w, h, rgba);
            }
            av_packet_unref(pkt);
        }
        if (!ok && avcodec_send_packet(ctx, nullptr) == 0 && avcodec_receive_frame(ctx, frame) == 0)
            ok = frameToRgba(frame, frame->sample_aspect_ratio, w, h, rgba);
        av_frame_free(&frame);
        av_packet_free(&pkt);
    }
    avcodec_free_context(&ctx);
    return ok;
}

}  // namespace

bool isMediaFile(const std::string& path) {
    static const std::set<std::string> kExt = {".mp4", ".m4v", ".mov", ".mkv", ".webm", ".avi", ".ogv", ".mpg", ".mpeg",
                                               ".ts",  ".m2ts", ".wmv", ".flv", ".3gp", ".mp3", ".m4a", ".aac", ".flac",
                                               ".wav", ".ogg", ".opus"};
    return kExt.contains(lower(fs::path(path).extension().string()));
}

std::string videosDirectory() {
    const char* cfg = std::getenv("XDG_CONFIG_HOME");
    const std::string config = (cfg && *cfg) ? cfg : homeDir() + "/.config";
    std::ifstream in(config + "/user-dirs.dirs");
    std::string line;
    while (std::getline(in, line)) {
        if (line.rfind("XDG_VIDEOS_DIR=", 0) != 0) continue;
        std::string v = line.substr(15);
        if (v.size() >= 2 && v.front() == '"') v = v.substr(1, v.size() - 2);
        if (v.rfind("$HOME", 0) == 0) v = homeDir() + v.substr(5);
        return v;
    }
    return homeDir() + "/Videos";
}

VideoPtr makeItem(const std::string& path) {
    auto v = std::make_shared<VideoItem>();
    const fs::path p(path);
    v->path = path;
    v->title = p.stem().string();
    v->folder = p.parent_path().filename().string();
    std::error_code ec;
    v->size = fs::file_size(p, ec);
    const auto t = fs::last_write_time(p, ec);
    if (!ec) v->modified = std::chrono::duration_cast<std::chrono::seconds>(t.time_since_epoch()).count();
    return v;
}

std::vector<VideoPtr> scanFolder(const std::string& dir, int depth) {
    std::vector<VideoPtr> out;
    std::error_code ec;
    fs::recursive_directory_iterator it(dir, fs::directory_options::skip_permission_denied, ec), end;
    for (; !ec && it != end && out.size() < 5000; it.increment(ec)) {
        const auto& e = *it;
        const std::string name = e.path().filename().string();
        std::error_code ec2;
        if (e.is_directory(ec2)) {
            if (!name.empty() && name[0] == '.') it.disable_recursion_pending();
            else if (it.depth() >= depth) it.disable_recursion_pending();
            continue;
        }
        if (!name.empty() && name[0] != '.' && isMediaFile(e.path().string())) out.push_back(makeItem(e.path().string()));
    }
    return out;
}

ProbeResult probeVideo(const std::string& path, std::uintmax_t size, std::int64_t modified) {
    ProbeResult r;
    const fs::path dir = fs::path(cacheDir()) / "mayonnaise" / "video_player";
    const std::string key = cacheKey(path, size, modified);
    const fs::path info = dir / (key + ".info");
    const fs::path png = dir / (key + ".png");
    std::error_code ec;
    if (fs::exists(info, ec)) {
        std::ifstream in(info);
        int audio = 0;
        int failed = 0;
        in >> r.duration >> r.width >> r.height >> audio >> failed;
        r.audioOnly = audio != 0;
        r.failed = failed != 0;
        if (fs::exists(png, ec)) r.thumb = mayo::ImageData::load(png.string(), kThumbWidth);
        if (!r.thumb && !r.failed) r.thumb = sharedThumbnail(path);
        return r;
    }

    AVFormatContext* fmt = nullptr;
    if (avformat_open_input(&fmt, path.c_str(), nullptr, nullptr) < 0 || avformat_find_stream_info(fmt, nullptr) < 0) {
        if (fmt) avformat_close_input(&fmt);
        r.failed = true;
    } else {
        if (fmt->duration > 0) r.duration = static_cast<double>(fmt->duration) / AV_TIME_BASE;
        int video = -1;
        int cover = -1;
        for (unsigned i = 0; i < fmt->nb_streams; ++i) {
            AVStream* st = fmt->streams[i];
            if (st->codecpar->codec_type != AVMEDIA_TYPE_VIDEO) continue;
            if (st->disposition & AV_DISPOSITION_ATTACHED_PIC) cover = static_cast<int>(i);
            else if (video < 0) video = static_cast<int>(i);
        }
        if (video >= 0) {
            r.width = fmt->streams[video]->codecpar->width;
            r.height = fmt->streams[video]->codecpar->height;
        } else {
            r.audioOnly = true;
        }
        r.thumb = sharedThumbnail(path);
        if (!r.thumb && cover >= 0) {
            // Album art embedded in music files.
            const AVPacket& pic = fmt->streams[cover]->attached_pic;
            r.thumb = mayo::ImageData::decode({pic.data, static_cast<size_t>(pic.size)});
            if (r.thumb) r.thumb = r.thumb->scaledToFit(kThumbWidth);
        }
        if (!r.thumb && video >= 0) {
            int w = 0;
            int h = 0;
            std::vector<std::uint8_t> rgba;
            if (grabFrame(fmt, video, w, h, rgba)) {
                r.thumb = mayo::ImageData::fromPixels(w, h, rgba);
                fs::create_directories(dir, ec);
                writePng(png.string(), w, h, rgba);
            }
        }
        avformat_close_input(&fmt);
    }
    fs::create_directories(dir, ec);
    std::ofstream out(info);
    out << r.duration << ' ' << r.width << ' ' << r.height << ' ' << (r.audioOnly ? 1 : 0) << ' ' << (r.failed ? 1 : 0) << '\n';
    return r;
}

// Prober ---------------------------------------------------------------------------------------------------

Prober::Prober(mayo::App& app, unsigned threads) : m_app(app) {
    for (unsigned i = 0; i < threads; ++i) m_threads.emplace_back([this] { run(); });
}

Prober::~Prober() {
    {
        std::lock_guard lock(m_mutex);
        m_stop = true;
    }
    m_cv.notify_all();
    for (auto& t : m_threads) t.join();
}

void Prober::probe(VideoPtr item, std::function<void()> done, bool urgent) {
    {
        std::lock_guard lock(m_mutex);
        Job job{std::move(item), std::move(done)};
        if (urgent) m_queue.push_front(std::move(job));
        else m_queue.push_back(std::move(job));
    }
    m_cv.notify_one();
}

void Prober::clear() {
    std::lock_guard lock(m_mutex);
    m_queue.clear();
}

void Prober::run() {
    av_log_set_level(AV_LOG_QUIET);
    for (;;) {
        Job job;
        {
            std::unique_lock lock(m_mutex);
            m_cv.wait(lock, [&] { return m_stop || !m_queue.empty(); });
            if (m_stop) return;
            job = std::move(m_queue.front());
            m_queue.pop_front();
        }
        // Only immutable fields are read here; the item is updated on the UI thread.
        ProbeResult r = probeVideo(job.item->path, job.item->size, job.item->modified);
        m_app.post([job = std::move(job), r = std::move(r)] {
            VideoItem& v = *job.item;
            v.probed = true;
            v.duration = r.duration;
            v.width = r.width;
            v.height = r.height;
            v.audioOnly = r.audioOnly;
            v.failed = r.failed;
            v.thumb = r.thumb;
            if (job.done) job.done();
        });
    }
}
