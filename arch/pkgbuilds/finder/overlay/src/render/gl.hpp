#pragma once

// RAII wrappers for the handful of GL object types the renderer uses.

#include <glad/gl.h>

#include <string_view>
#include <utility>

namespace mayo::gl {

/// Loads GL entry points through the platform layer and installs the KHR_debug callback in debug builds.
bool load();

template <void (*Deleter)(GLuint)>
class Handle {
public:
    Handle() = default;
    explicit Handle(GLuint id) : m_id(id) {}
    ~Handle() { reset(); }
    Handle(const Handle&) = delete;
    Handle& operator=(const Handle&) = delete;
    Handle(Handle&& o) noexcept : m_id(std::exchange(o.m_id, 0)) {}
    Handle& operator=(Handle&& o) noexcept {
        if (this != &o) {
            reset();
            m_id = std::exchange(o.m_id, 0);
        }
        return *this;
    }
    void reset() {
        if (m_id) Deleter(m_id);
        m_id = 0;
    }
    [[nodiscard]] GLuint id() const { return m_id; }
    explicit operator bool() const { return m_id != 0; }

private:
    GLuint m_id = 0;
};

namespace detail {
inline void deleteBuffer(GLuint id) { glDeleteBuffers(1, &id); }
inline void deleteVertexArray(GLuint id) { glDeleteVertexArrays(1, &id); }
inline void deleteTexture(GLuint id) { glDeleteTextures(1, &id); }
inline void deleteFramebuffer(GLuint id) { glDeleteFramebuffers(1, &id); }
inline void deleteProgram(GLuint id) { glDeleteProgram(id); }
}  // namespace detail

using Buffer = Handle<detail::deleteBuffer>;
using VertexArray = Handle<detail::deleteVertexArray>;
using Texture = Handle<detail::deleteTexture>;
using Framebuffer = Handle<detail::deleteFramebuffer>;
using Program = Handle<detail::deleteProgram>;

Buffer makeBuffer();
VertexArray makeVertexArray();
Texture makeTexture();
Framebuffer makeFramebuffer();

/// Compiles and links a program; aborts with the info log on failure (shaders are embedded, so a
/// failure is a bug, not a runtime condition).
Program makeProgram(std::string_view vertexSource, std::string_view fragmentSource, std::string_view name);

}  // namespace mayo::gl
