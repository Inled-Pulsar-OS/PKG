#include "mayonnaise/video.hpp"

#include <cstring>

#include "mayonnaise/painter.hpp"
#include "renderer2d.hpp"
#include "rhi/rhi.hpp"

namespace mayo {

struct VideoTexture::Impl {
    // Planes: Y, then UV (NV12) or U and V (I420). Destroyed on any thread; the RHI defers the GPU side.
    std::unique_ptr<rhi::Texture> planes[3];
    int width = 0;
    int height = 0;
    bool planar = false;  // I420 (three planes) vs NV12 (two)
    VideoColor color;

    /// Uploads one plane, honouring its stride; reallocates when the size or format changed (or the plane
    /// was an import).
    void upload(int index, int w, int h, VideoPlane plane, rhi::Format format) {
        rhi::Device* device = rhi::device();
        if (!device) return;
        auto& t = planes[index];
        if (!t || t->width() != w || t->height() != h || t->desc().format != format || imported[index]) {
            t = device->createTexture({.width = w, .height = h, .format = format});
            imported[index] = false;
        }
        device->upload(*t, 0, 0, w, h, plane.data, plane.stride);
    }
    bool imported[3] = {false, false, false};
};

VideoTexture::VideoTexture() : m_impl(std::make_unique<Impl>()) {}
VideoTexture::~VideoTexture() = default;

std::shared_ptr<VideoTexture> VideoTexture::create() { return std::shared_ptr<VideoTexture>(new VideoTexture()); }

int VideoTexture::width() const { return m_impl->width; }
int VideoTexture::height() const { return m_impl->height; }
bool VideoTexture::isEmpty() const { return m_impl->width == 0; }

bool VideoTexture::dmaBufSupported() { return rhi::device() && rhi::device()->supportsDmaBuf(); }

void VideoTexture::uploadI420(int w, int h, VideoPlane y, VideoPlane u, VideoPlane v, VideoColor color) {
    auto& d = *m_impl;
    const int cw = (w + 1) / 2;
    const int ch = (h + 1) / 2;
    d.upload(0, w, h, y, rhi::Format::R8);
    d.upload(1, cw, ch, u, rhi::Format::R8);
    d.upload(2, cw, ch, v, rhi::Format::R8);
    d.width = w;
    d.height = h;
    d.planar = true;
    d.color = color;
}

void VideoTexture::uploadNV12(int w, int h, VideoPlane y, VideoPlane uv, VideoColor color, bool sixteenBit) {
    auto& d = *m_impl;
    d.upload(0, w, h, y, sixteenBit ? rhi::Format::R16 : rhi::Format::R8);
    d.upload(1, (w + 1) / 2, (h + 1) / 2, uv, sixteenBit ? rhi::Format::RG16 : rhi::Format::RG8);
    d.width = w;
    d.height = h;
    d.planar = false;
    d.color = color;
}

bool VideoTexture::importDmaBuf(int w, int h, std::span<const DmaBufLayer> layers, VideoColor color) {
    rhi::Device* device = rhi::device();
    if (!device || !device->supportsDmaBuf() || layers.size() != 2) return false;
    std::unique_ptr<rhi::Texture> imported[2];
    for (size_t i = 0; i < 2; ++i) {
        const DmaBufLayer& l = layers[i];
        imported[i] = device->importDmaBuf({.drmFormat = l.drmFormat, .width = l.width, .height = l.height, .fd = l.fd,
                                            .offset = l.offset, .pitch = l.pitch, .modifier = l.modifier});
        if (!imported[i]) return false;
    }
    auto& d = *m_impl;
    for (size_t i = 0; i < 2; ++i) {
        d.planes[i] = std::move(imported[i]);
        d.imported[i] = true;
    }
    d.width = w;
    d.height = h;
    d.planar = false;
    d.color = color;
    return true;
}

void Painter::drawVideo(const Rect& r, const VideoTexture& video, const CornerRadii& radii, float opacity) {
    const auto& d = video.impl();
    if (d.width == 0) return;
    rhi::Texture* const tex[3] = {d.planes[0].get(), d.planes[1].get(), d.planar ? d.planes[2].get() : nullptr};
    m_renderer.video(r, tex, d.planar, static_cast<int>(d.color.matrix), d.color.fullRange, radii, opacity);
}

}  // namespace mayo
