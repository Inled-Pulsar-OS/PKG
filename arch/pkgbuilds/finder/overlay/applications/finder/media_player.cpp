#include "media_player.hpp"

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavutil/hwcontext.h>
#include <libavutil/hwcontext_drm.h>
#include <libavutil/pixdesc.h>
#include <libavformat/avformat.h>
#include <libavutil/channel_layout.h>
#include <libswresample/swresample.h>
#include <libswscale/swscale.h>
}

#include <algorithm>
#include <cctype>
#include <chrono>
#include <cmath>
#include <cstdlib>
#include <filesystem>
#include <thread>

namespace {

constexpr std::size_t kMaxFrames = 6;          // decoded RGBA frames buffered ahead
constexpr std::size_t kMaxPacketBytes = 32u << 20;
constexpr std::size_t kMaxPackets = 240;
constexpr double kAudioAhead = 0.35;           // seconds of audio queued at the device
constexpr int kOutRate = 48000;
constexpr std::uint32_t fourcc(char a, char b, char c, char d) {
    return static_cast<std::uint32_t>(a) | (static_cast<std::uint32_t>(b) << 8) | (static_cast<std::uint32_t>(c) << 16) |
           (static_cast<std::uint32_t>(d) << 24);
}
constexpr std::uint32_t kDrmR8 = fourcc('R', '8', ' ', ' ');
constexpr std::uint32_t kDrmGR88 = fourcc('G', 'R', '8', '8');
constexpr std::uint32_t kDrmR16 = fourcc('R', '1', '6', ' ');
constexpr std::uint32_t kDrmGR1616 = fourcc('G', 'R', '3', '2');
constexpr std::uint32_t kDrmNV12 = fourcc('N', 'V', '1', '2');
constexpr std::uint32_t kDrmP010 = fourcc('P', '0', '1', '0');

std::shared_ptr<AVFrame> share(AVFrame* f) {
    return std::shared_ptr<AVFrame>(f, [](AVFrame* p) { av_frame_free(&p); });
}

mayo::VideoColor colorOf(const AVFrame* f) {
    mayo::VideoColor c;
    switch (f->colorspace) {
        case AVCOL_SPC_BT470BG:
        case AVCOL_SPC_SMPTE170M: c.matrix = mayo::YuvMatrix::BT601; break;
        case AVCOL_SPC_BT2020_NCL:
        case AVCOL_SPC_BT2020_CL: c.matrix = mayo::YuvMatrix::BT2020; break;
        case AVCOL_SPC_BT709: c.matrix = mayo::YuvMatrix::BT709; break;
        default: c.matrix = f->height >= 720 ? mayo::YuvMatrix::BT709 : mayo::YuvMatrix::BT601; break;
    }
    c.fullRange = f->color_range == AVCOL_RANGE_JPEG || f->format == AV_PIX_FMT_YUVJ420P;
    return c;
}

double wallClock() {
    using namespace std::chrono;
    return duration<double>(steady_clock::now().time_since_epoch()).count();
}

std::string upper(std::string s) {
    std::transform(s.begin(), s.end(), s.begin(), [](unsigned char c) { return static_cast<char>(std::toupper(c)); });
    return s;
}

AVCodecContext* openDecoder(AVStream* stream, const AVCodec* codec, void (*configure)(AVCodecContext*, void*) = nullptr,
                            void* user = nullptr) {
    AVCodecContext* ctx = avcodec_alloc_context3(codec);
    if (!ctx) return nullptr;
    if (avcodec_parameters_to_context(ctx, stream->codecpar) < 0) {
        avcodec_free_context(&ctx);
        return nullptr;
    }
    ctx->thread_count = 0;  // let FFmpeg pick (frame + slice threads)
    ctx->pkt_timebase = stream->time_base;
    if (configure) configure(ctx, user);
    if (avcodec_open2(ctx, codec, nullptr) < 0) avcodec_free_context(&ctx);
    return ctx;
}

/// Preferred hardware device types, from MAYO_HWDEC (auto | none | vaapi | cuda | vulkan | vdpau).
std::vector<AVHWDeviceType> wantedDevices() {
    const char* env = std::getenv("MAYO_HWDEC");
    std::string mode = env ? env : "auto";
    if (mode == "none" || mode == "no" || mode == "software") return {};
    if (mode != "auto") {
        AVHWDeviceType t = av_hwdevice_find_type_by_name(mode.c_str());
        if (t != AV_HWDEVICE_TYPE_NONE) return {t};
    }
    return {AV_HWDEVICE_TYPE_VAAPI, AV_HWDEVICE_TYPE_CUDA, AV_HWDEVICE_TYPE_VULKAN, AV_HWDEVICE_TYPE_VDPAU};
}

/// get_format callback: take the hardware surface format if offered, otherwise the first software one.
AVPixelFormat chooseFormat(AVCodecContext* ctx, const AVPixelFormat* formats) {
    auto* player = static_cast<MediaPlayer*>(ctx->opaque);
    for (const AVPixelFormat* f = formats; *f != AV_PIX_FMT_NONE; ++f) {
        if (*f == player->hwPixelFormat()) return *f;
    }
    // The GPU can't decode this stream (profile, bit depth, size): FFmpeg continues in software.
    player->noteHardwareFallback();
    for (const AVPixelFormat* f = formats; *f != AV_PIX_FMT_NONE; ++f) {
        const AVPixFmtDescriptor* d = av_pix_fmt_desc_get(*f);
        if (d && !(d->flags & AV_PIX_FMT_FLAG_HWACCEL)) return *f;
    }
    return formats[0];
}

/// Pixel format of `codec`'s hardware surfaces for device `type`, or -1.
int hwFormatFor(const AVCodec* codec, AVHWDeviceType type) {
    for (int i = 0;; ++i) {
        const AVCodecHWConfig* cfg = avcodec_get_hw_config(codec, i);
        if (!cfg) return -1;
        if ((cfg->methods & AV_CODEC_HW_CONFIG_METHOD_HW_DEVICE_CTX) && cfg->device_type == type) return cfg->pix_fmt;
    }
}

}  // namespace

// --- PacketQueue -----------------------------------------------------------------------------

void MediaPlayer::PacketQueue::push(AVPacket* p, int serial) {
    {
        std::lock_guard lock(mutex);
        entries.push_back({p, serial});
        bytes += p ? static_cast<std::size_t>(p->size) : 0;
    }
    cv.notify_all();
}

bool MediaPlayer::PacketQueue::pop(Entry& out, const std::atomic<bool>& stop) {
    std::unique_lock lock(mutex);
    cv.wait(lock, [&] { return stop.load() || !entries.empty(); });
    if (stop) return false;
    out = entries.front();
    entries.pop_front();
    bytes -= out.packet ? static_cast<std::size_t>(out.packet->size) : 0;
    return true;
}

void MediaPlayer::PacketQueue::clear() {
    std::lock_guard lock(mutex);
    for (auto& e : entries) av_packet_free(&e.packet);
    entries.clear();
    bytes = 0;
}

// --- lifecycle -------------------------------------------------------------------------------

MediaPlayer::MediaPlayer() = default;
MediaPlayer::~MediaPlayer() { close(); }

bool MediaPlayer::open(const std::string& path, std::string& error) {
    close();
    if (avformat_open_input(&m_format, path.c_str(), nullptr, nullptr) < 0) {
        error = "The file couldn't be opened.";
        m_format = nullptr;
        return false;
    }
    if (avformat_find_stream_info(m_format, nullptr) < 0) {
        error = "The file doesn't look like a media file.";
        close();
        return false;
    }

    const AVCodec* vcodec = nullptr;
    m_videoStream = av_find_best_stream(m_format, AVMEDIA_TYPE_VIDEO, -1, -1, &vcodec, 0);
    if (m_videoStream >= 0 && (m_format->streams[m_videoStream]->disposition & AV_DISPOSITION_ATTACHED_PIC)) {
        m_videoStream = -1;  // cover art, not a video track
    }
    if (m_videoStream >= 0) {
        AVStream* st = m_format->streams[m_videoStream];
        const bool hw = setupHardware(vcodec, st->codecpar->codec_id);
        m_video = openDecoder(st, vcodec, [](AVCodecContext* ctx, void* user) {
            auto* self = static_cast<MediaPlayer*>(user);
            if (!self->m_hwDevice) return;
            ctx->hw_device_ctx = av_buffer_ref(self->m_hwDevice);
            ctx->opaque = self;
            ctx->get_format = &chooseFormat;
            // Frames queued for display (and the one on screen) pin decoder surfaces.
            ctx->extra_hw_frames = static_cast<int>(kMaxFrames) + 4;
        }, this);
        if (!m_video && hw) {
            // The hardware path refused to open: try plain software decoding.
            av_buffer_unref(&m_hwDevice);
            m_hwFormat = -1;
            vcodec = avcodec_find_decoder(st->codecpar->codec_id);
            m_video = vcodec ? openDecoder(st, vcodec) : nullptr;
        }
        if (!m_video) {
            m_videoStream = -1;
        } else {
            m_outW = m_video->width;
            m_outH = m_video->height;
            AVRational fr = av_guess_frame_rate(m_format, st, nullptr);
            m_frameRate = fr.num > 0 && fr.den > 0 ? av_q2d(fr) : 0;
            AVRational sar = av_guess_sample_aspect_ratio(m_format, st, nullptr);
            double pixel = sar.num > 0 && sar.den > 0 ? av_q2d(sar) : 1.0;
            m_aspect = m_video->height > 0 ? m_video->width * pixel / m_video->height : 16.0 / 9.0;
        }
    }

    const AVCodec* acodec = nullptr;
    m_audioStream = av_find_best_stream(m_format, AVMEDIA_TYPE_AUDIO, -1, m_videoStream, &acodec, 0);
    if (m_audioStream >= 0) {
        m_audio = openDecoder(m_format->streams[m_audioStream], acodec);
        if (m_audio) m_output = std::make_unique<mayo::AudioOutput>(kOutRate, 2);
        if (!m_audio || !m_output->isOpen()) {
            // No decoder or no sound device: play silently on the wall clock.
            avcodec_free_context(&m_audio);
            m_output.reset();
            m_audioStream = -1;
        } else {
            m_output->pause();
        }
    }

    if (m_videoStream < 0 && m_audioStream < 0) {
        error = "No playable video or audio streams were found.";
        close();
        return false;
    }

    m_duration = m_format->duration != AV_NOPTS_VALUE ? static_cast<double>(m_format->duration) / AV_TIME_BASE : 0;
    if (const AVDictionaryEntry* t = av_dict_get(m_format->metadata, "title", nullptr, 0); t && *t->value)
        m_title = t->value;
    else
        m_title = std::filesystem::path(path).filename().string();

    m_codecInfo.clear();
    if (m_video) {
        m_codecInfo = upper(avcodec_get_name(m_video->codec_id)) + " · " + std::to_string(m_video->width) + "×" +
                      std::to_string(m_video->height);
    }
    if (m_audio) {
        if (!m_codecInfo.empty()) m_codecInfo += " · ";
        m_codecInfo += upper(avcodec_get_name(m_audio->codec_id));
    }

    m_serial = 0;
    m_shownSerial = -1;
    m_seekTarget = 0;
    m_audioEnd = 0;
    m_demuxEof = false;
    m_videoDone = m_videoStream < 0;
    m_audioDone = m_audioStream < 0;
    m_playing = false;
    {
        std::lock_guard lock(m_clockMutex);
        m_clockBase = 0;
        m_clockStart = wallClock();
    }

    m_demuxThread = std::thread([this] { demuxLoop(); });
    if (m_video) m_videoThread = std::thread([this] { videoLoop(); });
    if (m_audio) m_audioThread = std::thread([this] { audioLoop(); });
    return true;
}

void MediaPlayer::stopThreads() {
    m_stop = true;
    m_videoPackets.cv.notify_all();
    m_audioPackets.cv.notify_all();
    m_frameCv.notify_all();
    m_demuxCv.notify_all();
    for (auto* t : {&m_demuxThread, &m_videoThread, &m_audioThread}) {
        if (t->joinable()) t->join();
    }
    m_stop = false;
    m_videoPackets.clear();
    m_audioPackets.clear();
    std::lock_guard lock(m_frameMutex);
    m_frames.clear();
}

bool MediaPlayer::setupHardware(const AVCodec*& codec, int codecId) {
    if (!m_hwAllowed) return false;
    for (AVHWDeviceType type : wantedDevices()) {
        // The default decoder may be software-only (libdav1d for AV1); find one that supports `type`.
        const AVCodec* candidate = hwFormatFor(codec, type) >= 0 ? codec : nullptr;
        void* it = nullptr;
        for (const AVCodec* c = nullptr; !candidate && (c = av_codec_iterate(&it));) {
            if (av_codec_is_decoder(c) && c->id == codecId && hwFormatFor(c, type) >= 0) candidate = c;
        }
        if (!candidate) continue;
        if (av_hwdevice_ctx_create(&m_hwDevice, type, nullptr, nullptr, 0) < 0) continue;
        codec = candidate;
        m_hwFormat = hwFormatFor(candidate, type);
        m_hwName = av_hwdevice_get_type_name(type);
        // Zero-copy display needs VAAPI surfaces exported as dma-bufs and EGL to import them.
        m_zeroCopy = type == AV_HWDEVICE_TYPE_VAAPI && mayo::VideoTexture::dmaBufSupported() &&
                     !std::getenv("MAYO_NO_ZEROCOPY");
        return true;
    }
    return false;
}

std::string MediaPlayer::decoderDescription() const {
    if (!m_video) return "No video";
    if (m_hwDevice && !m_hwFellBack) {
        std::string name = m_hwName;
        for (auto& ch : name) ch = static_cast<char>(std::toupper(static_cast<unsigned char>(ch)));
        return "Hardware · " + name + (m_zeroCopy ? " · zero-copy" : " · copy-back");
    }
    std::string why = m_hwFellBack ? " (GPU can't decode this stream)" : "";
    return "Software · " + std::to_string(std::max(1, m_video->thread_count)) + " threads" + why;
}

void MediaPlayer::close() {
    stopThreads();
    avcodec_free_context(&m_video);
    avcodec_free_context(&m_audio);
    if (m_format) avformat_close_input(&m_format);
    m_output.reset();
    av_buffer_unref(&m_hwDevice);
    m_hwFormat = -1;
    m_hwFellBack = false;
    m_zeroCopy = false;
    m_dropped = 0;
    m_videoStream = m_audioStream = -1;
    m_outW = m_outH = 0;
    m_duration = 0;
    m_playing = false;
}

// --- transport ---------------------------------------------------------------------------------

void MediaPlayer::play() {
    if (!isOpen()) return;
    if (ended()) seek(0);
    {
        std::lock_guard lock(m_clockMutex);
        m_clockStart = wallClock();
    }
    m_playing = true;
    if (m_output) m_output->resume();
}

void MediaPlayer::pause() {
    if (!m_playing) return;
    double p = position();
    m_playing = false;
    if (m_output) m_output->pause();
    std::lock_guard lock(m_clockMutex);
    m_clockBase = p;
}

void MediaPlayer::seek(double seconds) {
    if (!isOpen()) return;
    seconds = std::clamp(seconds, 0.0, m_duration > 0 ? m_duration : seconds);
    m_seekTarget = seconds;
    ++m_serial;  // everything decoded before this point is now stale
    m_audioEnd = seconds;
    if (m_output) m_output->clear();
    {
        std::lock_guard lock(m_clockMutex);
        m_clockBase = seconds;
        m_clockStart = wallClock();
    }
    m_seekRequested = true;
    m_demuxCv.notify_all();
    m_frameCv.notify_all();
}

double MediaPlayer::position() const {
    if (m_output) {
        double p = m_audioEnd.load() - m_output->queuedSeconds();
        return std::max(0.0, p);
    }
    std::lock_guard lock(m_clockMutex);
    return m_playing ? m_clockBase + (wallClock() - m_clockStart) : m_clockBase;
}

bool MediaPlayer::ended() const {
    if (!isOpen() || !m_demuxEof || !m_videoDone || !m_audioDone) return false;
    {
        std::lock_guard lock(const_cast<std::mutex&>(m_frameMutex));
        if (!m_frames.empty()) return false;
    }
    return !m_output || m_output->queuedSeconds() < 0.01;
}

void MediaPlayer::setVolume(float volume) {
    if (m_output) m_output->setVolume(volume);
}

bool MediaPlayer::takeFrame(Frame& out) {
    std::lock_guard lock(m_frameMutex);
    const int serial = m_serial;
    while (!m_frames.empty() && m_frames.front().serial != serial) m_frames.pop_front();
    if (m_frames.empty()) return false;
    bool got = false;
    if (m_shownSerial != serial) {
        // First frame after open/seek: show it right away, even while paused.
        out = std::move(m_frames.front());
        m_frames.pop_front();
        got = true;
    } else {
        const double clock = position();
        // Skip frames that are already late; keep the newest one that is due.
        while (!m_frames.empty() && m_frames.front().pts <= clock + 0.004) {
            if (got) ++m_dropped;  // a newer frame was already due: this one never reached the screen
            out = std::move(m_frames.front());
            m_frames.pop_front();
            got = true;
        }
    }
    if (got) {
        m_shownSerial = serial;
        m_shownPts = out.pts;
        m_frameCv.notify_all();
    }
    return got;
}

// --- worker threads ------------------------------------------------------------------------------

void MediaPlayer::demuxLoop() {
    AVPacket* pkt = av_packet_alloc();
    while (!m_stop) {
        if (m_seekRequested.exchange(false)) {
            const double t = m_seekTarget;
            av_seek_frame(m_format, -1, static_cast<int64_t>(t * AV_TIME_BASE), AVSEEK_FLAG_BACKWARD);
            m_videoPackets.clear();
            m_audioPackets.clear();
            {
                std::lock_guard lock(m_frameMutex);
                m_frames.clear();
            }
            m_frameCv.notify_all();
            m_demuxEof = false;
            m_videoDone = m_videoStream < 0;
            m_audioDone = m_audioStream < 0;
        }
        bool full = false;
        {
            std::scoped_lock lock(m_videoPackets.mutex, m_audioPackets.mutex);
            const bool videoFull = m_videoStream < 0 || m_videoPackets.entries.size() > kMaxPackets;
            const bool audioFull = m_audioStream < 0 || m_audioPackets.entries.size() > kMaxPackets;
            full = (videoFull && audioFull) || m_videoPackets.bytes + m_audioPackets.bytes > kMaxPacketBytes;
        }
        if (full || m_demuxEof) {
            std::unique_lock lock(m_seekMutex);
            m_demuxCv.wait_for(lock, std::chrono::milliseconds(20));
            continue;
        }
        const int serial = m_serial;
        const int r = av_read_frame(m_format, pkt);
        if (r < 0) {
            if (r == AVERROR_EOF || avio_feof(m_format->pb)) {
                if (m_videoStream >= 0) m_videoPackets.push(nullptr, serial);
                if (m_audioStream >= 0) m_audioPackets.push(nullptr, serial);
                m_demuxEof = true;
            }
            continue;
        }
        if (pkt->stream_index == m_videoStream || pkt->stream_index == m_audioStream) {
            AVPacket* owned = av_packet_alloc();
            av_packet_move_ref(owned, pkt);
            (owned->stream_index == m_videoStream ? m_videoPackets : m_audioPackets).push(owned, serial);
        } else {
            av_packet_unref(pkt);
        }
    }
    av_packet_free(&pkt);
}

void MediaPlayer::videoLoop() {
    AVFrame* frame = av_frame_alloc();
    SwsContext* sws = nullptr;
    const double tb = av_q2d(m_format->streams[m_videoStream]->time_base);
    int current = -1;
    // Turns whatever the decoder produced into something VideoTexture can display.
    auto present = [&](AVFrame* src) -> std::shared_ptr<AVFrame> {
        if (src->format == m_hwFormat) {
            if (m_zeroCopy) {
                AVFrame* mapped = av_frame_alloc();
                mapped->format = AV_PIX_FMT_DRM_PRIME;
                if (av_hwframe_map(mapped, src, AV_HWFRAME_MAP_READ) == 0) {
                    av_frame_copy_props(mapped, src);
                    return share(mapped);  // keeps the surface referenced
                }
                av_frame_free(&mapped);
                m_zeroCopy = false;  // export not possible on this driver: copy back from now on
            }
            AVFrame* sw = av_frame_alloc();
            if (av_hwframe_transfer_data(sw, src, 0) < 0) {
                av_frame_free(&sw);
                return nullptr;
            }
            av_frame_copy_props(sw, src);
            auto owned = share(sw);
            if (sw->format == AV_PIX_FMT_NV12 || sw->format == AV_PIX_FMT_P010 || sw->format == AV_PIX_FMT_P016) return owned;
            // Unusual transfer formats are normalised below.
            AVFrame* conv = av_frame_alloc();
            conv->format = AV_PIX_FMT_YUV420P;
            conv->width = sw->width;
            conv->height = sw->height;
            av_frame_get_buffer(conv, 0);
            sws = sws_getCachedContext(sws, sw->width, sw->height, static_cast<AVPixelFormat>(sw->format), sw->width,
                                       sw->height, AV_PIX_FMT_YUV420P, SWS_BILINEAR, nullptr, nullptr, nullptr);
            sws_scale(sws, sw->data, sw->linesize, 0, sw->height, conv->data, conv->linesize);
            av_frame_copy_props(conv, sw);
            return share(conv);
        }
        const auto fmt = static_cast<AVPixelFormat>(src->format);
        if (fmt == AV_PIX_FMT_YUV420P || fmt == AV_PIX_FMT_YUVJ420P || fmt == AV_PIX_FMT_NV12 || fmt == AV_PIX_FMT_P010) {
            AVFrame* ref = av_frame_alloc();
            av_frame_ref(ref, src);  // no copy: the decoder's buffer is shared
            return share(ref);
        }
        // Other software formats (4:2:2, 4:4:4, 10-bit planar, RGB...): convert on the CPU.
        const bool deep = av_pix_fmt_desc_get(fmt) && av_pix_fmt_desc_get(fmt)->comp[0].depth > 8;
        AVFrame* conv = av_frame_alloc();
        conv->format = deep ? AV_PIX_FMT_P010 : AV_PIX_FMT_YUV420P;
        conv->width = src->width;
        conv->height = src->height;
        av_frame_get_buffer(conv, 0);
        sws = sws_getCachedContext(sws, src->width, src->height, fmt, src->width, src->height,
                                   static_cast<AVPixelFormat>(conv->format), SWS_BILINEAR, nullptr, nullptr, nullptr);
        sws_scale(sws, src->data, src->linesize, 0, src->height, conv->data, conv->linesize);
        av_frame_copy_props(conv, src);
        return share(conv);
    };

    while (!m_stop) {
        PacketQueue::Entry e{};
        if (!m_videoPackets.pop(e, m_stop)) break;
        if (e.serial != m_serial) {
            av_packet_free(&e.packet);
            continue;
        }
        if (e.serial != current) {
            avcodec_flush_buffers(m_video);
            current = e.serial;
        }
        avcodec_send_packet(m_video, e.packet);  // nullptr drains at end of stream
        const bool eos = e.packet == nullptr;
        av_packet_free(&e.packet);
        while (!m_stop && avcodec_receive_frame(m_video, frame) == 0) {
            const double pts = frame->best_effort_timestamp != AV_NOPTS_VALUE ? frame->best_effort_timestamp * tb : 0;
            if (pts < m_seekTarget - 0.001 || current != m_serial) {  // decode up to the exact seek time
                av_frame_unref(frame);
                continue;
            }
            Frame out;
            out.frame = present(frame);
            out.pts = pts;
            out.serial = current;
            av_frame_unref(frame);
            if (!out.frame) continue;

            std::unique_lock lock(m_frameMutex);
            m_frameCv.wait(lock, [&] { return m_stop || m_frames.size() < kMaxFrames || current != m_serial; });
            if (m_stop || current != m_serial) break;
            m_frames.push_back(std::move(out));
        }
        if (eos && current == m_serial) m_videoDone = true;
    }
    sws_freeContext(sws);
    av_frame_free(&frame);
}

bool MediaPlayer::Frame::uploadTo(mayo::VideoTexture& texture) const {
    const AVFrame* f = frame.get();
    if (!f) return false;
    const mayo::VideoColor color = colorOf(f);
    switch (f->format) {
        case AV_PIX_FMT_DRM_PRIME: {
            const auto* desc = reinterpret_cast<const AVDRMFrameDescriptor*>(f->data[0]);
            std::vector<mayo::DmaBufLayer> layers;
            auto plane = [&](const AVDRMPlaneDescriptor& p, std::uint32_t format, int w, int h) {
                const AVDRMObjectDescriptor& obj = desc->objects[p.object_index];
                mayo::DmaBufLayer l;
                l.drmFormat = format;
                l.width = w;
                l.height = h;
                l.fd = obj.fd;
                l.offset = static_cast<std::uint32_t>(p.offset);
                l.pitch = static_cast<std::uint32_t>(p.pitch);
                l.modifier = obj.format_modifier;
                layers.push_back(l);
            };
            const int cw = (f->width + 1) / 2;
            const int ch = (f->height + 1) / 2;
            if (desc->nb_layers == 2) {
                // Separate layers (VAAPI's default export): R8 + GR88, or R16 + GR1616 for 10-bit.
                plane(desc->layers[0].planes[0], desc->layers[0].format, f->width, f->height);
                plane(desc->layers[1].planes[0], desc->layers[1].format, cw, ch);
            } else if (desc->nb_layers == 1 && desc->layers[0].nb_planes == 2) {
                // One NV12/P010 layer: import each plane as its own single-channel image.
                const bool deep = desc->layers[0].format == kDrmP010;
                if (!deep && desc->layers[0].format != kDrmNV12) return false;
                plane(desc->layers[0].planes[0], deep ? kDrmR16 : kDrmR8, f->width, f->height);
                plane(desc->layers[0].planes[1], deep ? kDrmGR1616 : kDrmGR88, cw, ch);
            } else {
                return false;
            }
            return texture.importDmaBuf(f->width, f->height, layers, color);
        }
        case AV_PIX_FMT_NV12:
        case AV_PIX_FMT_P010:
        case AV_PIX_FMT_P016:
            texture.uploadNV12(f->width, f->height, {f->data[0], f->linesize[0]}, {f->data[1], f->linesize[1]}, color,
                               f->format != AV_PIX_FMT_NV12);
            return true;
        case AV_PIX_FMT_YUV420P:
        case AV_PIX_FMT_YUVJ420P:
            texture.uploadI420(f->width, f->height, {f->data[0], f->linesize[0]}, {f->data[1], f->linesize[1]},
                               {f->data[2], f->linesize[2]}, color);
            return true;
        default: return false;
    }
}

void MediaPlayer::audioLoop() {
    AVFrame* frame = av_frame_alloc();
    SwrContext* swr = nullptr;
    const double tb = av_q2d(m_format->streams[m_audioStream]->time_base);
    AVChannelLayout outLayout = AV_CHANNEL_LAYOUT_STEREO;
    AVChannelLayout inLayout{};
    if (m_audio->ch_layout.order == AV_CHANNEL_ORDER_UNSPEC)
        av_channel_layout_default(&inLayout, m_audio->ch_layout.nb_channels);
    else
        av_channel_layout_copy(&inLayout, &m_audio->ch_layout);
    auto resetResampler = [&] {
        swr_free(&swr);
        swr_alloc_set_opts2(&swr, &outLayout, AV_SAMPLE_FMT_FLT, kOutRate, &inLayout, m_audio->sample_fmt,
                            m_audio->sample_rate, 0, nullptr);
        if (swr && swr_init(swr) < 0) swr_free(&swr);
    };
    std::vector<float> buffer;
    int current = -1;
    while (!m_stop) {
        PacketQueue::Entry e{};
        if (!m_audioPackets.pop(e, m_stop)) break;
        if (e.serial != m_serial) {
            av_packet_free(&e.packet);
            continue;
        }
        if (e.serial != current) {
            avcodec_flush_buffers(m_audio);
            resetResampler();  // drop samples buffered inside the resampler from before the seek
            current = e.serial;
        }
        avcodec_send_packet(m_audio, e.packet);
        const bool eos = e.packet == nullptr;
        av_packet_free(&e.packet);
        while (!m_stop && swr && avcodec_receive_frame(m_audio, frame) == 0) {
            double pts = frame->best_effort_timestamp != AV_NOPTS_VALUE ? frame->best_effort_timestamp * tb
                                                                         : m_audioEnd.load();
            const int capacity = swr_get_out_samples(swr, frame->nb_samples);
            buffer.resize(static_cast<std::size_t>(std::max(capacity, 0)) * 2);
            auto* out = reinterpret_cast<uint8_t*>(buffer.data());
            const int n = swr_convert(swr, &out, capacity, const_cast<const uint8_t**>(frame->extended_data),
                                      frame->nb_samples);
            av_frame_unref(frame);
            if (n <= 0) continue;
            std::size_t first = 0;
            const double target = m_seekTarget;
            if (pts < target) {  // trim to the exact seek position
                first = static_cast<std::size_t>(std::min<double>(n, (target - pts) * kOutRate));
                pts += static_cast<double>(first) / kOutRate;
            }
            if (first >= static_cast<std::size_t>(n)) continue;
            // Keep only a short buffer at the device so pausing and seeking stay responsive.
            while (!m_stop && current == m_serial && m_output->queuedSeconds() > kAudioAhead) {
                std::this_thread::sleep_for(std::chrono::milliseconds(15));
            }
            if (m_stop || current != m_serial) break;
            m_output->write(std::span<const float>(buffer.data() + first * 2, (static_cast<std::size_t>(n) - first) * 2));
            m_audioEnd = pts + static_cast<double>(static_cast<std::size_t>(n) - first) / kOutRate;
        }
        if (eos && current == m_serial) m_audioDone = true;
    }
    av_channel_layout_uninit(&inLayout);
    swr_free(&swr);
    av_frame_free(&frame);
}
