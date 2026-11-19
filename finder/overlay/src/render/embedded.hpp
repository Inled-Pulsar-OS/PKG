#pragma once

// Files embedded by cmake/Embed.cmake. Each symbol is named after the file (non-identifier chars -> '_').

#include <span>
#include <string_view>

namespace mayo::embedded {
extern const std::span<const unsigned char> ui_vert;
extern const std::span<const unsigned char> ui_frag;
extern const std::span<const unsigned char> fullscreen_vert;
extern const std::span<const unsigned char> blur_down_frag;
extern const std::span<const unsigned char> blur_up_frag;
extern const std::span<const unsigned char> InterVariable_ttf;
#ifdef MAYONNAISE_HAS_VULKAN
// The same shaders compiled to SPIR-V (cmake/Embed.cmake, mayonnaise_embed_spirv).
extern const std::span<const unsigned char> ui_vert_spv;
extern const std::span<const unsigned char> ui_frag_spv;
extern const std::span<const unsigned char> fullscreen_vert_spv;
extern const std::span<const unsigned char> blur_down_frag_spv;
extern const std::span<const unsigned char> blur_up_frag_spv;
#endif

inline std::string_view text(std::span<const unsigned char> bytes) {
    return {reinterpret_cast<const char*>(bytes.data()), bytes.size()};
}
}  // namespace mayo::embedded
