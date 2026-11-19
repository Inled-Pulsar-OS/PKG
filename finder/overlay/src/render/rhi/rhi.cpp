#include "rhi.hpp"

namespace mayo::rhi {

namespace {
Device* g_device = nullptr;
}

Device* device() { return g_device; }
void setDevice(Device* device) { g_device = device; }

int bytesPerPixel(Format format) {
    switch (format) {
        case Format::R8: return 1;
        case Format::RG8:
        case Format::R16: return 2;
        case Format::RG16:
        case Format::RGBA8: return 4;
    }
    return 4;
}

#ifndef MAYONNAISE_HAS_VULKAN
std::unique_ptr<Device> createVulkanDevice() { return nullptr; }
#endif

}  // namespace mayo::rhi
