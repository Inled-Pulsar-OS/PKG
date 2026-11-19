#pragma once

#include <cstdint>
#include <memory>
#include <span>

#include "geometry.hpp"

namespace mayo {

/// YUV → RGB conversion parameters of a video frame.
enum class YuvMatrix : std::uint8_t { BT601, BT709, BT2020 };
struct VideoColor {
    YuvMatrix matrix = YuvMatrix::BT709;
    bool fullRange = false;  ///< false: "TV" range (16–235 luma), the usual case for video
};

/// One CPU plane: pointer to the first row and the byte distance between rows.
struct VideoPlane {
    const std::uint8_t* data = nullptr;
    int stride = 0;
};

/// One single-plane layer of a Linux dma-buf (as exported by VAAPI with separate layers).
struct DmaBufLayer {
    std::uint32_t drmFormat = 0;  ///< DRM fourcc: R8 / GR88 (8-bit), R16 / GR1616 (10/16-bit)
    int width = 0;
    int height = 0;
    int fd = -1;
    std::uint32_t offset = 0;
    std::uint32_t pitch = 0;
    std::uint64_t modifier = ~0ull >> 8;  ///< DRM_FORMAT_MOD_INVALID unless the exporter says otherwise
};

/// GPU-resident YUV video frame, converted to RGB in the shader when drawn with Painter::drawVideo.
/// All methods must be called on the UI thread (they touch GL).
class VideoTexture {
public:
    static std::shared_ptr<VideoTexture> create();
    ~VideoTexture();
    VideoTexture(const VideoTexture&) = delete;
    VideoTexture& operator=(const VideoTexture&) = delete;

    /// 8-bit planar 4:2:0 (yuv420p / I420).
    void uploadI420(int width, int height, VideoPlane y, VideoPlane u, VideoPlane v, VideoColor color);
    /// Semi-planar 4:2:0 (NV12), or P010/P016 when `sixteenBit`.
    void uploadNV12(int width, int height, VideoPlane y, VideoPlane uv, VideoColor color, bool sixteenBit = false);
    /// Zero-copy import of a hardware-decoded frame: two layers, luma (R8/R16) then chroma (GR88/GR1616).
    /// The caller keeps the underlying buffer alive (and unmodified) while the frame is displayed.
    /// Returns false if the platform can't import it; fall back to an upload then.
    bool importDmaBuf(int width, int height, std::span<const DmaBufLayer> layers, VideoColor color);
    /// Whether importDmaBuf can work in this process (EGL with EXT_image_dma_buf_import).
    static bool dmaBufSupported();

    [[nodiscard]] int width() const;
    [[nodiscard]] int height() const;
    [[nodiscard]] bool isEmpty() const;

    struct Impl;
    [[nodiscard]] Impl& impl() const { return *m_impl; }

private:
    VideoTexture();
    std::unique_ptr<Impl> m_impl;
};

}  // namespace mayo
