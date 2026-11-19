// OpenGL 3.3 core implementation of the RHI. One context is shared by every window.

#include <algorithm>
#include <bit>
#include <cstdio>
#include <cstring>
#include <optional>
#include <string>

#include "../../platform/platform.hpp"
#include "../gl.hpp"
#include "rhi.hpp"

namespace mayo::rhi {

namespace {

constexpr size_t kStreamBytes = 512 * 1024;  // instance ring buffer; grows for huge batches

struct GLFormat {
    GLenum internal;
    GLenum format;
    GLenum type;
};

GLFormat glFormat(Format f) {
    switch (f) {
        case Format::R8: return {GL_R8, GL_RED, GL_UNSIGNED_BYTE};
        case Format::RG8: return {GL_RG8, GL_RG, GL_UNSIGNED_BYTE};
        case Format::R16: return {GL_R16, GL_RED, GL_UNSIGNED_SHORT};
        case Format::RG16: return {GL_RG16, GL_RG, GL_UNSIGNED_SHORT};
        case Format::RGBA8: return {GL_RGBA8, GL_RGBA, GL_UNSIGNED_BYTE};
    }
    return {GL_RGBA8, GL_RGBA, GL_UNSIGNED_BYTE};
}

// EGL entry points and constants for dma-buf import, declared here so the build doesn't need EGL headers.
using EGLDisplay = void*;
using EGLImage = void*;
using EGLint = std::int32_t;
using CreateImageFn = EGLImage (*)(EGLDisplay, void* context, unsigned target, void* buffer, const EGLint* attribs);
using DestroyImageFn = unsigned (*)(EGLDisplay, EGLImage);
using QueryStringFn = const char* (*)(EGLDisplay, EGLint);
using ImageTargetFn = void (*)(GLenum target, void* image);

constexpr EGLint kEglNone = 0x3038;
constexpr EGLint kEglExtensions = 0x3055;
constexpr EGLint kEglWidth = 0x3057;
constexpr EGLint kEglHeight = 0x3056;
constexpr unsigned kEglLinuxDmaBuf = 0x3270;
constexpr EGLint kEglFourcc = 0x3271;
constexpr EGLint kEglPlane0Fd = 0x3272;
constexpr EGLint kEglPlane0Offset = 0x3273;
constexpr EGLint kEglPlane0Pitch = 0x3274;
constexpr EGLint kEglPlane0ModLo = 0x3443;
constexpr EGLint kEglPlane0ModHi = 0x3444;
constexpr std::uint64_t kModInvalid = 0x00ffffffffffffffull;

constexpr std::uint32_t fourcc(char a, char b, char c, char d) {
    return static_cast<std::uint32_t>(a) | (static_cast<std::uint32_t>(b) << 8) | (static_cast<std::uint32_t>(c) << 16) |
           (static_cast<std::uint32_t>(d) << 24);
}

struct Egl {
    bool supported = false;
    bool modifiers = false;
    EGLDisplay display = nullptr;
    CreateImageFn createImage = nullptr;
    DestroyImageFn destroyImage = nullptr;
    ImageTargetFn imageTarget = nullptr;

    void load() {
        using platform::Platform;
        display = Platform::eglDisplay();
        if (!display) return;  // GLX context: no dma-buf import
        auto query = reinterpret_cast<QueryStringFn>(Platform::eglProcAddress("eglQueryString"));
        createImage = reinterpret_cast<CreateImageFn>(Platform::eglProcAddress("eglCreateImageKHR"));
        destroyImage = reinterpret_cast<DestroyImageFn>(Platform::eglProcAddress("eglDestroyImageKHR"));
        imageTarget = reinterpret_cast<ImageTargetFn>(Platform::glProcAddress("glEGLImageTargetTexture2DOES"));
        const char* ext = query ? query(display, kEglExtensions) : nullptr;
        supported = ext && std::strstr(ext, "EGL_EXT_image_dma_buf_import") && createImage && destroyImage && imageTarget;
        modifiers = ext && std::strstr(ext, "EGL_EXT_image_dma_buf_import_modifiers");
    }
};

class GLDevice;

class GLTexture final : public Texture {
public:
    GLTexture(const TextureDesc& desc, Device* owner) : Texture(desc), m_owner(owner) {
        glGenTextures(1, &m_id);
        glBindTexture(GL_TEXTURE_2D, m_id);
        const auto filter = static_cast<GLint>(desc.filter == Filter::Nearest ? GL_NEAREST : GL_LINEAR);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, desc.mipmaps ? GL_LINEAR_MIPMAP_LINEAR : filter);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, filter);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    }
    ~GLTexture() override {
        // The context goes with the device; late destruction (or from another thread) must not call GL.
        if (device() != m_owner) return;
        if (m_fbo) glDeleteFramebuffers(1, &m_fbo);
        glDeleteTextures(1, &m_id);
    }

    [[nodiscard]] GLuint id() const { return m_id; }
    /// A framebuffer with this texture attached, created on first use.
    GLuint fbo() {
        if (!m_fbo) {
            glGenFramebuffers(1, &m_fbo);
            glBindFramebuffer(GL_FRAMEBUFFER, m_fbo);
            glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, m_id, 0);
        }
        return m_fbo;
    }

private:
    Device* m_owner;
    GLuint m_id = 0;
    GLuint m_fbo = 0;
};

class GLPipeline final : public Pipeline {
public:
    gl::Program program;
    gl::VertexArray vao;
    std::vector<GLint> uniformLocations;
    std::vector<UniformDesc> uniforms;
    int textures = 0;
    int instanceVec4s = 0;
    Blend blend = Blend::None;
    GLenum mode = GL_TRIANGLES;
};

class GLDevice final : public Device {
public:
    GLDevice() {
        m_egl.load();
        glGetIntegerv(GL_MAX_TEXTURE_SIZE, &m_maxTextureSize);
        m_instanceBuffer = gl::makeBuffer();
        m_white = std::make_unique<GLTexture>(TextureDesc{.filter = Filter::Nearest}, this);
        const std::uint8_t white[4] = {255, 255, 255, 255};
        glBindTexture(GL_TEXTURE_2D, m_white->id());
        glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA8, 1, 1, 0, GL_RGBA, GL_UNSIGNED_BYTE, white);
    }

    [[nodiscard]] Backend backend() const override { return Backend::OpenGL; }
    [[nodiscard]] std::string description() const override {
        return std::string("OpenGL ") + reinterpret_cast<const char*>(glGetString(GL_VERSION)) + " on " +
               reinterpret_cast<const char*>(glGetString(GL_RENDERER));
    }
    [[nodiscard]] int maxTextureSize() const override { return m_maxTextureSize; }

    std::unique_ptr<Texture> createTexture(const TextureDesc& desc) override {
        auto texture = std::make_unique<GLTexture>(desc, this);
        const GLFormat f = glFormat(desc.format);
        glTexImage2D(GL_TEXTURE_2D, 0, static_cast<GLint>(f.internal), desc.width, desc.height, 0, f.format, f.type,
                     nullptr);
        // glTexImage2D with null data leaves contents undefined. Clearing through a framebuffer avoids uploading
        // a zero buffer (up to 16 MB for the glyph atlas) through the driver's staging memory.
        glBindFramebuffer(GL_FRAMEBUFFER, texture->fbo());
        glDisable(GL_SCISSOR_TEST);
        glViewport(0, 0, desc.width, desc.height);
        glClearColor(0, 0, 0, 0);
        glClear(GL_COLOR_BUFFER_BIT);
        restorePass();
        return texture;
    }

    void upload(Texture& texture, int x, int y, int w, int h, const void* data, int rowBytes) override {
        auto& t = static_cast<GLTexture&>(texture);
        const GLFormat f = glFormat(t.desc().format);
        glBindTexture(GL_TEXTURE_2D, t.id());
        glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
        if (rowBytes) glPixelStorei(GL_UNPACK_ROW_LENGTH, rowBytes / bytesPerPixel(t.desc().format));
        glTexSubImage2D(GL_TEXTURE_2D, 0, x, y, w, h, f.format, f.type, data);
        if (rowBytes) glPixelStorei(GL_UNPACK_ROW_LENGTH, 0);
        if (t.desc().mipmaps) glGenerateMipmap(GL_TEXTURE_2D);
    }

    [[nodiscard]] bool supportsDmaBuf() const override { return m_egl.supported; }

    std::unique_ptr<Texture> importDmaBuf(const DmaBufPlane& plane) override {
        if (!m_egl.supported) return nullptr;
        TextureDesc desc{.width = plane.width, .height = plane.height};
        switch (plane.drmFormat) {
            case fourcc('R', '8', ' ', ' '): desc.format = Format::R8; break;
            case fourcc('G', 'R', '8', '8'): desc.format = Format::RG8; break;
            case fourcc('R', '1', '6', ' '): desc.format = Format::R16; break;
            case fourcc('G', 'R', '3', '2'): desc.format = Format::RG16; break;
            default: return nullptr;
        }
        EGLint attribs[20];
        int n = 0;
        attribs[n++] = kEglWidth;
        attribs[n++] = plane.width;
        attribs[n++] = kEglHeight;
        attribs[n++] = plane.height;
        attribs[n++] = kEglFourcc;
        attribs[n++] = static_cast<EGLint>(plane.drmFormat);
        attribs[n++] = kEglPlane0Fd;
        attribs[n++] = plane.fd;
        attribs[n++] = kEglPlane0Offset;
        attribs[n++] = static_cast<EGLint>(plane.offset);
        attribs[n++] = kEglPlane0Pitch;
        attribs[n++] = static_cast<EGLint>(plane.pitch);
        if (m_egl.modifiers && plane.modifier != kModInvalid) {
            attribs[n++] = kEglPlane0ModLo;
            attribs[n++] = static_cast<EGLint>(plane.modifier & 0xffffffffu);
            attribs[n++] = kEglPlane0ModHi;
            attribs[n++] = static_cast<EGLint>(plane.modifier >> 32);
        }
        attribs[n++] = kEglNone;
        EGLImage image = m_egl.createImage(m_egl.display, nullptr, kEglLinuxDmaBuf, nullptr, attribs);
        if (!image) return nullptr;
        auto texture = std::make_unique<GLTexture>(desc, this);
        m_egl.imageTarget(GL_TEXTURE_2D, image);
        // The texture keeps the buffer referenced; the EGLImage handle itself isn't needed any more.
        m_egl.destroyImage(m_egl.display, image);
        return texture;
    }

    std::unique_ptr<Pipeline> createPipeline(const PipelineDesc& desc) override {
        auto p = std::make_unique<GLPipeline>();
        p->program = gl::makeProgram(desc.vertexGlsl, desc.fragmentGlsl, desc.name);
        p->vao = gl::makeVertexArray();
        p->uniforms = desc.uniforms;
        p->textures = static_cast<int>(desc.textures.size());
        p->instanceVec4s = desc.instanceVec4s;
        p->blend = desc.blend;
        p->mode = desc.topology == Topology::TriangleStrip ? GL_TRIANGLE_STRIP : GL_TRIANGLES;
        glUseProgram(p->program.id());
        for (size_t i = 0; i < desc.textures.size(); ++i) {
            const std::string name(desc.textures[i]);
            glUniform1i(glGetUniformLocation(p->program.id(), name.c_str()), static_cast<GLint>(i));
        }
        for (const auto& u : desc.uniforms) {
            const std::string name(u.name);
            p->uniformLocations.push_back(glGetUniformLocation(p->program.id(), name.c_str()));
        }
        if (desc.instanceVec4s > 0) {
            glBindVertexArray(p->vao.id());
            glBindBuffer(GL_ARRAY_BUFFER, m_instanceBuffer.id());
            for (GLuint i = 0; i < static_cast<GLuint>(desc.instanceVec4s); ++i) {
                glEnableVertexAttribArray(i);
                glVertexAttribDivisor(i, 1);
            }
            glBindVertexArray(0);
        }
        return p;
    }

    void beginFrame(platform::PlatformWindow& window) override { window.makeCurrent(); }

    void endFrame(platform::PlatformWindow& window, Texture& scene) override {
        auto& s = static_cast<GLTexture&>(scene);
        glBindFramebuffer(GL_READ_FRAMEBUFFER, s.fbo());
        glBindFramebuffer(GL_DRAW_FRAMEBUFFER, 0);
        glDisable(GL_SCISSOR_TEST);
        glBlitFramebuffer(0, 0, s.width(), s.height(), 0, 0, s.width(), s.height(), GL_COLOR_BUFFER_BIT, GL_NEAREST);
        glBindFramebuffer(GL_FRAMEBUFFER, 0);
        m_target = nullptr;
        window.swapBuffers();
    }

    void beginPass(Texture& target, const float* clear) override {
        m_target = &static_cast<GLTexture&>(target);
        m_scissor.reset();
        restorePass();
        if (clear) {
            glClearColor(clear[0], clear[1], clear[2], clear[3]);
            glClear(GL_COLOR_BUFFER_BIT);
        }
    }

    void setScissor(const Scissor* scissor) override {
        if (scissor) {
            m_scissor = *scissor;
            glEnable(GL_SCISSOR_TEST);
            glScissor(scissor->x, scissor->y, scissor->width, scissor->height);
        } else {
            m_scissor.reset();
            glDisable(GL_SCISSOR_TEST);
        }
    }

    void draw(Pipeline& pipeline, std::span<Texture* const> textures, const void* uniforms, std::size_t uniformBytes,
              std::span<const std::byte> instances, int vertexCount, int instanceCount) override {
        auto& p = static_cast<GLPipeline&>(pipeline);
        if (p.blend == Blend::Premultiplied) {
            glEnable(GL_BLEND);
            glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA);
        } else {
            glDisable(GL_BLEND);
        }
        glUseProgram(p.program.id());
        const auto* bytes = static_cast<const unsigned char*>(uniforms);
        for (size_t i = 0; i < p.uniforms.size(); ++i) {
            const UniformDesc& u = p.uniforms[i];
            if (static_cast<size_t>(u.offset + u.floats * 4) > uniformBytes) continue;
            float v[4] = {};
            std::memcpy(v, bytes + u.offset, static_cast<size_t>(u.floats) * 4);
            const GLint loc = p.uniformLocations[i];
            if (u.floats == 1) glUniform1f(loc, v[0]);
            else if (u.floats == 2) glUniform2f(loc, v[0], v[1]);
            else glUniform4f(loc, v[0], v[1], v[2], v[3]);
        }
        for (int i = p.textures - 1; i >= 0; --i) {
            const auto idx = static_cast<size_t>(i);
            Texture* t = idx < textures.size() && textures[idx] ? textures[idx] : m_white.get();
            glActiveTexture(static_cast<GLenum>(GL_TEXTURE0 + i));
            glBindTexture(GL_TEXTURE_2D, static_cast<GLTexture*>(t)->id());
        }

        glBindVertexArray(p.vao.id());
        if (p.instanceVec4s > 0) {
            streamInstances(p, instances);
            glDrawArraysInstanced(p.mode, 0, vertexCount, instanceCount);
        } else {
            glDrawArrays(p.mode, 0, vertexCount);
        }
        glBindVertexArray(0);
    }

    std::vector<std::uint8_t> readPixels(Texture& texture) override {
        auto& t = static_cast<GLTexture&>(texture);
        const int width = t.width();
        const int height = t.height();
        std::vector<std::uint8_t> pixels(static_cast<size_t>(width) * static_cast<size_t>(height) * 4);
        glBindFramebuffer(GL_FRAMEBUFFER, t.fbo());
        glPixelStorei(GL_PACK_ALIGNMENT, 1);
        glReadPixels(0, 0, width, height, GL_RGBA, GL_UNSIGNED_BYTE, pixels.data());
        restorePass();
        // GL rows are bottom-up.
        const size_t stride = static_cast<size_t>(width) * 4;
        for (int y = 0; y < height / 2; ++y) {
            std::swap_ranges(pixels.begin() + static_cast<std::ptrdiff_t>(y * stride),
                             pixels.begin() + static_cast<std::ptrdiff_t>((y + 1) * stride),
                             pixels.begin() + static_cast<std::ptrdiff_t>((height - 1 - y) * stride));
        }
        return pixels;
    }

private:
    /// Rebinds the current pass after something else used the framebuffer binding.
    void restorePass() {
        if (!m_target) {
            glBindFramebuffer(GL_FRAMEBUFFER, 0);
            return;
        }
        glBindFramebuffer(GL_FRAMEBUFFER, m_target->fbo());
        glViewport(0, 0, m_target->width(), m_target->height());
        setScissor(m_scissor ? &*m_scissor : nullptr);
    }

    // Instances stream into one buffer used as a ring: each draw writes after the previous one (unsynchronized,
    // that range is unused since the last orphan) and points the attributes at it. Re-specifying the whole
    // buffer on every draw made the driver allocate and cache a fresh buffer per batch, tens of MB of GTT
    // (system RAM on integrated GPUs) with many batches per frame.
    void streamInstances(const GLPipeline& p, std::span<const std::byte> instances) {
        glBindBuffer(GL_ARRAY_BUFFER, m_instanceBuffer.id());
        const size_t bytes = instances.size();
        if (bytes > m_streamSize) {
            m_streamSize = std::max<size_t>(kStreamBytes, std::bit_ceil(bytes));
            m_streamOffset = m_streamSize;  // forces the orphan below
        }
        if (m_streamOffset + bytes > m_streamSize) {
            glBufferData(GL_ARRAY_BUFFER, static_cast<GLsizeiptr>(m_streamSize), nullptr, GL_STREAM_DRAW);
            m_streamOffset = 0;
        }
        void* dst = glMapBufferRange(GL_ARRAY_BUFFER, static_cast<GLintptr>(m_streamOffset),
                                     static_cast<GLsizeiptr>(bytes),
                                     GL_MAP_WRITE_BIT | GL_MAP_UNSYNCHRONIZED_BIT | GL_MAP_INVALIDATE_RANGE_BIT);
        if (dst) {
            std::memcpy(dst, instances.data(), bytes);
            glUnmapBuffer(GL_ARRAY_BUFFER);
        } else {
            glBufferSubData(GL_ARRAY_BUFFER, static_cast<GLintptr>(m_streamOffset), static_cast<GLsizeiptr>(bytes),
                            instances.data());
        }
        const auto stride = static_cast<GLsizei>(p.instanceVec4s * 4 * sizeof(float));
        for (GLuint i = 0; i < static_cast<GLuint>(p.instanceVec4s); ++i) {
            glVertexAttribPointer(i, 4, GL_FLOAT, GL_FALSE, stride,
                                  reinterpret_cast<const void*>(m_streamOffset + i * 4 * sizeof(float)));
        }
        m_streamOffset += (bytes + 255) & ~size_t{255};
    }

    Egl m_egl;
    GLint m_maxTextureSize = 4096;
    gl::Buffer m_instanceBuffer;
    size_t m_streamSize = 0;  // bytes
    size_t m_streamOffset = 0;
    std::unique_ptr<GLTexture> m_white;
    GLTexture* m_target = nullptr;
    std::optional<Scissor> m_scissor;
};

}  // namespace

std::unique_ptr<Device> createGLDevice() {
    if (!gl::load()) return nullptr;
    return std::make_unique<GLDevice>();
}

}  // namespace mayo::rhi
