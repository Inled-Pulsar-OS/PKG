#include "gl.hpp"

#include <cstdio>
#include <cstdlib>
#include <string>

#include "../platform/platform.hpp"

namespace mayo::gl {

namespace {

#ifndef NDEBUG
void GLAD_API_PTR debugCallback(GLenum /*source*/, GLenum type, GLuint id, GLenum severity, GLsizei /*length*/,
                                const GLchar* message, const void* /*user*/) {
    if (severity == GL_DEBUG_SEVERITY_NOTIFICATION) return;
    std::fprintf(stderr, "mayonnaise GL %s (id %u, type 0x%x): %s\n",
                 severity == GL_DEBUG_SEVERITY_HIGH ? "error" : "warning", id, type, message);
}
#endif

GLuint compile(GLenum stage, std::string_view source, std::string_view name) {
    GLuint shader = glCreateShader(stage);
    const char* src = source.data();
    auto len = static_cast<GLint>(source.size());
    glShaderSource(shader, 1, &src, &len);
    glCompileShader(shader);
    GLint ok = 0;
    glGetShaderiv(shader, GL_COMPILE_STATUS, &ok);
    if (!ok) {
        char log[4096];
        glGetShaderInfoLog(shader, sizeof log, nullptr, log);
        std::fprintf(stderr, "mayonnaise: failed to compile %.*s (%s):\n%s\n", static_cast<int>(name.size()),
                     name.data(), stage == GL_VERTEX_SHADER ? "vertex" : "fragment", log);
        std::abort();
    }
    return shader;
}

}  // namespace

bool load() {
    int version = gladLoadGL(reinterpret_cast<GLADloadfunc>(&platform::Platform::glProcAddress));
    if (!version) return false;
    bool debugOutput = false;
#ifndef NDEBUG
    if (GLAD_GL_KHR_debug) {
        glEnable(GL_DEBUG_OUTPUT);
        glEnable(GL_DEBUG_OUTPUT_SYNCHRONOUS);
        glDebugMessageCallback(debugCallback, nullptr);
        debugOutput = true;
    }
#endif
    if (const char* v = std::getenv("MAYO_VERBOSE"); v && *v && *v != '0') {
        std::fprintf(stderr, "mayonnaise: GL %s on %s (KHR_debug output %s)\n",
                     reinterpret_cast<const char*>(glGetString(GL_VERSION)),
                     reinterpret_cast<const char*>(glGetString(GL_RENDERER)), debugOutput ? "on" : "off");
    }
    return true;
}

Buffer makeBuffer() {
    GLuint id = 0;
    glGenBuffers(1, &id);
    return Buffer(id);
}
VertexArray makeVertexArray() {
    GLuint id = 0;
    glGenVertexArrays(1, &id);
    return VertexArray(id);
}
Texture makeTexture() {
    GLuint id = 0;
    glGenTextures(1, &id);
    return Texture(id);
}
Framebuffer makeFramebuffer() {
    GLuint id = 0;
    glGenFramebuffers(1, &id);
    return Framebuffer(id);
}

Program makeProgram(std::string_view vertexSource, std::string_view fragmentSource, std::string_view name) {
    GLuint vs = compile(GL_VERTEX_SHADER, vertexSource, name);
    GLuint fs = compile(GL_FRAGMENT_SHADER, fragmentSource, name);
    GLuint program = glCreateProgram();
    glAttachShader(program, vs);
    glAttachShader(program, fs);
    glLinkProgram(program);
    glDeleteShader(vs);
    glDeleteShader(fs);
    GLint ok = 0;
    glGetProgramiv(program, GL_LINK_STATUS, &ok);
    if (!ok) {
        char log[4096];
        glGetProgramInfoLog(program, sizeof log, nullptr, log);
        std::fprintf(stderr, "mayonnaise: failed to link %.*s:\n%s\n", static_cast<int>(name.size()), name.data(), log);
        std::abort();
    }
    return Program(program);
}

}  // namespace mayo::gl
