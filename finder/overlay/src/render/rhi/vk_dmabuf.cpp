// Zero-copy video for the Vulkan backend: VAAPI surfaces exported as dma-bufs become sampled images through
// VK_EXT_external_memory_dma_buf + VK_EXT_image_drm_format_modifier, one image per plane (R8 / GR88, or
// R16 / GR1616 for 10-bit), the same split the GL backend imports through EGL.

#include <unistd.h>

#include <mutex>

#include "vk_device.hpp"

namespace mayo::rhi::vk {

namespace {

constexpr std::uint64_t kModInvalid = 0x00ffffffffffffffull;

constexpr std::uint32_t fourcc(char a, char b, char c, char d) {
    return static_cast<std::uint32_t>(a) | (static_cast<std::uint32_t>(b) << 8) | (static_cast<std::uint32_t>(c) << 16) |
           (static_cast<std::uint32_t>(d) << 24);
}

bool formatFor(std::uint32_t drm, Format& format, VkFormat& vkFormat) {
    switch (drm) {
        case fourcc('R', '8', ' ', ' '): format = Format::R8; vkFormat = VK_FORMAT_R8_UNORM; return true;
        case fourcc('G', 'R', '8', '8'): format = Format::RG8; vkFormat = VK_FORMAT_R8G8_UNORM; return true;
        case fourcc('R', '1', '6', ' '): format = Format::R16; vkFormat = VK_FORMAT_R16_UNORM; return true;
        case fourcc('G', 'R', '3', '2'): format = Format::RG16; vkFormat = VK_FORMAT_R16G16_UNORM; return true;
        default: return false;
    }
}

}  // namespace

std::unique_ptr<Texture> VulkanDevice::importDmaBuf(const DmaBufPlane& plane) {
    // Without a modifier the layout of the buffer is unknown to Vulkan (EGL guesses; we fall back to copies).
    if (!m_dmaBuf || plane.fd < 0 || plane.modifier == kModInvalid) return nullptr;
    Format format{};
    VkFormat vkFormat{};
    if (!formatFor(plane.drmFormat, format, vkFormat)) return nullptr;

    VkPhysicalDeviceImageDrmFormatModifierInfoEXT modifierInfo{
        VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_IMAGE_DRM_FORMAT_MODIFIER_INFO_EXT};
    modifierInfo.drmFormatModifier = plane.modifier;
    modifierInfo.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
    VkPhysicalDeviceExternalImageFormatInfo externalInfo{VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_EXTERNAL_IMAGE_FORMAT_INFO};
    externalInfo.pNext = &modifierInfo;
    externalInfo.handleType = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT;
    VkPhysicalDeviceImageFormatInfo2 formatInfo{VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_IMAGE_FORMAT_INFO_2};
    formatInfo.pNext = &externalInfo;
    formatInfo.format = vkFormat;
    formatInfo.type = VK_IMAGE_TYPE_2D;
    formatInfo.tiling = VK_IMAGE_TILING_DRM_FORMAT_MODIFIER_EXT;
    formatInfo.usage = VK_IMAGE_USAGE_SAMPLED_BIT;
    VkExternalImageFormatProperties externalProps{VK_STRUCTURE_TYPE_EXTERNAL_IMAGE_FORMAT_PROPERTIES};
    VkImageFormatProperties2 props{VK_STRUCTURE_TYPE_IMAGE_FORMAT_PROPERTIES_2};
    props.pNext = &externalProps;
    if (vkGetPhysicalDeviceImageFormatProperties2(m_physical, &formatInfo, &props) != VK_SUCCESS) return nullptr;
    if (!(externalProps.externalMemoryProperties.externalMemoryFeatures & VK_EXTERNAL_MEMORY_FEATURE_IMPORTABLE_BIT))
        return nullptr;

    TextureDesc desc{.width = plane.width, .height = plane.height, .format = format, .filter = Filter::Linear};
    auto t = std::make_unique<VulkanTexture>(desc, *this);

    VkSubresourceLayout layout{};
    layout.offset = plane.offset;
    layout.rowPitch = plane.pitch;
    VkImageDrmFormatModifierExplicitCreateInfoEXT explicitInfo{
        VK_STRUCTURE_TYPE_IMAGE_DRM_FORMAT_MODIFIER_EXPLICIT_CREATE_INFO_EXT};
    explicitInfo.drmFormatModifier = plane.modifier;
    explicitInfo.drmFormatModifierPlaneCount = 1;
    explicitInfo.pPlaneLayouts = &layout;
    VkExternalMemoryImageCreateInfo externalImage{VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO};
    externalImage.pNext = &explicitInfo;
    externalImage.handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT;
    VkImageCreateInfo ici{VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO};
    ici.pNext = &externalImage;
    ici.imageType = VK_IMAGE_TYPE_2D;
    ici.format = vkFormat;
    ici.extent = {static_cast<std::uint32_t>(plane.width), static_cast<std::uint32_t>(plane.height), 1};
    ici.mipLevels = 1;
    ici.arrayLayers = 1;
    ici.samples = VK_SAMPLE_COUNT_1_BIT;
    ici.tiling = VK_IMAGE_TILING_DRM_FORMAT_MODIFIER_EXT;
    ici.usage = VK_IMAGE_USAGE_SAMPLED_BIT;
    ici.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
    ici.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    if (vkCreateImage(m_device, &ici, nullptr, &t->image) != VK_SUCCESS) return nullptr;

    VkMemoryRequirements req;
    vkGetImageMemoryRequirements(m_device, t->image, &req);
    VkMemoryFdPropertiesKHR fdProps{VK_STRUCTURE_TYPE_MEMORY_FD_PROPERTIES_KHR};
    vkGetMemoryFdPropertiesKHR(m_device, VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT, plane.fd, &fdProps);
    const int type = m_allocator.findType(req.memoryTypeBits & fdProps.memoryTypeBits, 0);
    const int fd = type >= 0 ? dup(plane.fd) : -1;  // a successful import takes ownership of the descriptor
    if (fd < 0) {
        vkDestroyImage(m_device, t->image, nullptr);
        t->image = VK_NULL_HANDLE;
        return nullptr;
    }
    VkMemoryDedicatedAllocateInfo dedicated{VK_STRUCTURE_TYPE_MEMORY_DEDICATED_ALLOCATE_INFO};
    dedicated.image = t->image;
    VkImportMemoryFdInfoKHR import{VK_STRUCTURE_TYPE_IMPORT_MEMORY_FD_INFO_KHR};
    import.pNext = &dedicated;
    import.handleType = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT;
    import.fd = fd;
    VkMemoryAllocateInfo ai{VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO};
    ai.pNext = &import;
    ai.allocationSize = req.size;
    ai.memoryTypeIndex = static_cast<std::uint32_t>(type);
    if (vkAllocateMemory(m_device, &ai, nullptr, &t->ownMemory) != VK_SUCCESS) {
        close(fd);
        vkDestroyImage(m_device, t->image, nullptr);
        t->image = VK_NULL_HANDLE;
        return nullptr;
    }
    vkBindImageMemory(m_device, t->image, t->ownMemory, 0);
    VkImageViewCreateInfo vci{VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO};
    vci.image = t->image;
    vci.viewType = VK_IMAGE_VIEW_TYPE_2D;
    vci.format = vkFormat;
    vci.subresourceRange = {VK_IMAGE_ASPECT_COLOR_BIT, 0, 1, 0, 1};
    vkCreateImageView(m_device, &vci, nullptr, &t->view);
    t->foreign = true;
    registerTexture(*t);
    return t;
}

}  // namespace mayo::rhi::vk
