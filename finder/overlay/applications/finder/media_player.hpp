#pragma once

// FFmpeg-backed playback engine for the video player example. Demuxing, video decoding and audio
// decoding each run on their own thread; the UI thread asks for "the frame to show now" every vsync.
// Audio is the master clock (falls back to the wall clock for silent videos).

#include <mayonnaise/audio.hpp>
#include <mayonnaise/video.hpp>

#include <atomic>
#include <condition_variable>
#include <cstdint>
#include <deque>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

struct AVFormatContext;
struct AVCodecContext;
struct AVPacket;
struct AVFrame;
struct AVBufferRef;

class MediaPlayer {
public:
    /// A decoded frame: CPU planes (NV12 / P010 / yuv420p) or a dma-buf mapping of a GPU surface.
    /// Holding it keeps the decoder from reusing the surface, so keep the displayed frame alive.
    struct Frame {
        std::shared_ptr<AVFrame> frame;
        double pts = 0;
        int serial = 0;
        /// Uploads (or, for dma-buf frames, imports zero-copy) into `texture`. UI thread.
        bool uploadTo(mayo::VideoTexture& texture) const;
    };

    MediaPlayer();
    ~MediaPlayer();
    MediaPlayer(const MediaPlayer&) = delete;
    MediaPlayer& operator=(const MediaPlayer&) = delete;

    /// Opens a file and starts buffering (paused). Returns false with a human-readable error.
    bool open(const std::string& path, std::string& error);
    void close();
    [[nodiscard]] bool isOpen() const { return m_format != nullptr; }

    [[nodiscard]] bool hasVideo() const { return m_videoStream >= 0; }
    [[nodiscard]] bool hasAudio() const { return m_audioStream >= 0; }
    [[nodiscard]] int width() const { return m_outW; }
    [[nodiscard]] int height() const { return m_outH; }
    /// "Hardware · VAAPI · zero-copy", "Software · 16 threads", ...
    [[nodiscard]] std::string decoderDescription() const;
    [[nodiscard]] double frameRate() const { return m_frameRate; }
    [[nodiscard]] int droppedFrames() const { return m_dropped; }
    /// Allow GPU decoding for files opened from now on (MAYO_HWDEC still picks the API).
    void setHardwareDecoding(bool enabled) { m_hwAllowed = enabled; }
    /// Called when a dma-buf import failed: decode into system memory from now on.
    void disableZeroCopy() { m_zeroCopy = false; }
    // Used by the decoder's get_format callback.
    [[nodiscard]] int hwPixelFormat() const { return m_hwFormat; }
    void noteHardwareFallback() { m_hwFellBack = true; }
    /// Display aspect ratio (accounts for non-square pixels).
    [[nodiscard]] double aspect() const { return m_aspect; }
    [[nodiscard]] double duration() const { return m_duration; }
    [[nodiscard]] const std::string& title() const { return m_title; }
    [[nodiscard]] const std::string& codecDescription() const { return m_codecInfo; }

    void play();
    void pause();
    [[nodiscard]] bool isPlaying() const { return m_playing; }
    /// Accurate seek (decodes forward to the exact time).
    void seek(double seconds);
    [[nodiscard]] double position() const;
    [[nodiscard]] bool ended() const;
    void setVolume(float volume);

    /// Returns the frame due at the current clock if it differs from the last one returned.
    bool takeFrame(Frame& out);
    /// True right after a seek until the first frame of the new position arrived.
    [[nodiscard]] bool isSeeking() const { return m_shownSerial != m_serial.load(); }

private:
    struct PacketQueue {
        struct Entry {
            AVPacket* packet;  // nullptr = end of stream
            int serial;
        };
        std::mutex mutex;
        std::condition_variable cv;
        std::deque<Entry> entries;
        std::size_t bytes = 0;
        void push(AVPacket* p, int serial);
        bool pop(Entry& out, const std::atomic<bool>& stop);
        void clear();
    };

    bool setupHardware(const struct AVCodec*& codec, int codecId);
    void demuxLoop();
    void videoLoop();
    void audioLoop();
    void stopThreads();

    AVFormatContext* m_format = nullptr;
    AVCodecContext* m_video = nullptr;
    AVCodecContext* m_audio = nullptr;
    int m_videoStream = -1;
    int m_audioStream = -1;
    AVBufferRef* m_hwDevice = nullptr;
    int m_hwFormat = -1;          // AVPixelFormat of hardware surfaces
    std::string m_hwName;
    bool m_hwAllowed = true;
    std::atomic<bool> m_hwFellBack{false};
    std::atomic<bool> m_zeroCopy{false};
    std::atomic<int> m_dropped{0};
    double m_frameRate = 0;
    int m_outW = 0;
    int m_outH = 0;
    double m_aspect = 16.0 / 9.0;
    double m_duration = 0;
    std::string m_title;
    std::string m_codecInfo;

    std::unique_ptr<mayo::AudioOutput> m_output;
    PacketQueue m_videoPackets;
    PacketQueue m_audioPackets;

    std::mutex m_frameMutex;
    std::condition_variable m_frameCv;
    std::deque<Frame> m_frames;

    std::atomic<bool> m_stop{false};
    std::atomic<bool> m_playing{false};
    std::atomic<int> m_serial{0};
    std::atomic<double> m_seekTarget{0};
    std::atomic<bool> m_seekRequested{false};
    std::mutex m_seekMutex;
    std::condition_variable m_demuxCv;
    std::atomic<bool> m_demuxEof{false};
    std::atomic<bool> m_videoDone{false};
    std::atomic<bool> m_audioDone{false};
    std::atomic<double> m_audioEnd{0};  // pts just past the last sample handed to the device
    int m_shownSerial = -1;
    double m_shownPts = -1;

    // Wall clock for files without audio.
    mutable std::mutex m_clockMutex;
    double m_clockBase = 0;
    double m_clockStart = 0;

    std::thread m_demuxThread;
    std::thread m_videoThread;
    std::thread m_audioThread;
};
