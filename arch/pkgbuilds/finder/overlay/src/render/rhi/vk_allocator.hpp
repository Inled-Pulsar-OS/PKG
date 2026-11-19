#pragma once

// A small sub-allocator for Vulkan memory. Drivers cap the number of live allocations (4096 on some), and a
// photo grid easily has more thumbnails than that, so images and buffers share 16 MB blocks per memory type.
// Requests above a quarter block get memory of their own. Empty blocks are freed at once: RAM on integrated
// GPUs is system RAM, and keeping it around for a burst that may never come costs every other program.

#include <cstdint>
#include <vector>

#include "vk_functions.hpp"

namespace mayo::rhi::vk {

struct Allocation {
    VkDeviceMemory memory = VK_NULL_HANDLE;
    VkDeviceSize offset = 0;
    VkDeviceSize size = 0;
    void* mapped = nullptr;  // host-visible memory is mapped for its whole lifetime
    std::uint32_t block = ~0u;
    explicit operator bool() const { return memory != VK_NULL_HANDLE; }
};

enum class Usage : std::uint8_t {
    Device,    // device local, for images
    Upload,    // host visible + coherent: staging, instances
    Readback,  // host visible, cached if possible
};

class Allocator {
public:
    void init(VkPhysicalDevice physicalDevice, VkDevice device);
    void destroy();

    /// `linear`: buffers and linear images; kept apart from optimal-tiling images so the
    /// bufferImageGranularity rule never applies inside a block.
    bool allocate(const VkMemoryRequirements& requirements, Usage usage, bool linear, Allocation& out);
    void free(Allocation& allocation);

    [[nodiscard]] int findType(std::uint32_t typeBits, VkMemoryPropertyFlags required,
                               VkMemoryPropertyFlags preferred = 0) const;

private:
    struct Range {
        VkDeviceSize offset;
        VkDeviceSize size;
    };
    struct Block {
        VkDeviceMemory memory = VK_NULL_HANDLE;
        VkDeviceSize size = 0;
        void* mapped = nullptr;
        std::uint32_t type = 0;
        bool linear = false;
        bool dedicated = false;
        std::vector<Range> free;  // sorted by offset
        VkDeviceSize used = 0;
    };

    bool allocateFrom(Block& block, std::uint32_t index, const VkMemoryRequirements& req, Allocation& out);
    std::uint32_t newBlock(std::uint32_t type, VkDeviceSize size, bool linear, bool dedicated);

    VkDevice m_device = VK_NULL_HANDLE;
    VkPhysicalDeviceMemoryProperties m_props{};
    std::vector<Block> m_blocks;  // freed blocks keep their slot (memory == null) so indices stay valid
};

}  // namespace mayo::rhi::vk
