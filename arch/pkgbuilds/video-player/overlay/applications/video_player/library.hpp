// The video library behind the gallery: finding videos, probing duration and a thumbnail with FFmpeg on
// worker threads, and caching both on disk so a second start is instant.
#pragma once

#include <mayonnaise/widgets/image.hpp>

#include <condition_variable>
#include <cstdint>
#include <deque>
#include <functional>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

namespace mayo {
class App;
}

struct VideoItem {
    std::string path;
    std::string title;   ///< file name without extension
    std::string folder;  ///< containing folder's name
    std::uintmax_t size = 0;
    std::int64_t modified = 0;  ///< seconds since the epoch
    double duration = -1;       ///< unknown until probed
    int width = 0;
    int height = 0;
    bool audioOnly = false;
    bool probed = false;
    bool failed = false;
    std::shared_ptr<mayo::ImageData> thumb;
};
using VideoPtr = std::shared_ptr<VideoItem>;

[[nodiscard]] bool isMediaFile(const std::string& path);
/// $XDG_VIDEOS_DIR (from user-dirs.dirs), or ~/Videos.
[[nodiscard]] std::string videosDirectory();
/// Media files under `dir`, `depth` folders deep, skipping hidden folders. Any thread.
std::vector<VideoPtr> scanFolder(const std::string& dir, int depth = 4);
/// Item for a single file (size and date filled in, not probed).
VideoPtr makeItem(const std::string& path);

/// Duration, dimensions and a thumbnail for `item`, from the disk cache or by decoding one frame ~10% in.
/// Worker threads only; the result is applied on the UI thread by Prober.
struct ProbeResult {
    double duration = -1;
    int width = 0;
    int height = 0;
    bool audioOnly = false;
    bool failed = false;
    std::shared_ptr<mayo::ImageData> thumb;
};
ProbeResult probeVideo(const std::string& path, std::uintmax_t size, std::int64_t modified);

/// A small pool of worker threads that probe items and hand results back on the UI thread.
class Prober {
public:
    Prober(mayo::App& app, unsigned threads);
    ~Prober();
    Prober(const Prober&) = delete;
    Prober& operator=(const Prober&) = delete;

    /// `done` runs on the UI thread after the item has been updated.
    void probe(VideoPtr item, std::function<void()> done, bool urgent = false);
    /// Forgets queued work (folder switched); running probes still finish.
    void clear();

private:
    struct Job {
        VideoPtr item;
        std::function<void()> done;
    };
    void run();

    mayo::App& m_app;
    std::mutex m_mutex;
    std::condition_variable m_cv;
    std::deque<Job> m_queue;
    bool m_stop = false;
    std::vector<std::thread> m_threads;
};
