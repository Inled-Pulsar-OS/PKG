#include "vk_allocator.hpp"

#include <algorithm>

namespace mayo::rhi::vk {

namespace {
constexpr VkDeviceSize kBlockSize = 16ull << 20;

VkDeviceSize alignUp(VkDeviceSize v, VkDeviceSize a) { return (v + a - 1) / a * a; }
}  // namespace

// --- function loading ------------------------------------------------------------------------

namespace {
PFN_vkGetInstanceProcAddr g_getInstanceProcAddr = nullptr;
}

bool loadGlobal(PFN_vkGetInstanceProcAddr getInstanceProcAddr) {
    g_getInstanceProcAddr = getInstanceProcAddr;
    if (!g_getInstanceProcAddr) return false;
    bool ok = true;
#define MAYO_VK_LOAD(name) \
    name = reinterpret_cast<PFN_##name>(g_getInstanceProcAddr(nullptr, #name)); \
    ok = ok && name;
    MAYO_VK_GLOBAL_FUNCTIONS(MAYO_VK_LOAD)
#undef MAYO_VK_LOAD
    return ok;
}

bool loadInstance(VkInstance instance) {
    bool ok = true;
#define MAYO_VK_LOAD(name) \
    name = reinterpret_cast<PFN_##name>(g_getInstanceProcAddr(instance, #name)); \
    ok = ok && name;
    MAYO_VK_INSTANCE_FUNCTIONS(MAYO_VK_LOAD)
#undef MAYO_VK_LOAD
    return ok;
}

bool loadDevice(VkDevice device) {
    bool ok = true;
#define MAYO_VK_LOAD(name) \
    name = reinterpret_cast<PFN_##name>(vkGetDeviceProcAddr(device, #name)); \
    ok = ok && name;
    MAYO_VK_DEVICE_FUNCTIONS(MAYO_VK_LOAD)
#undef MAYO_VK_LOAD
#define MAYO_VK_LOAD(name) name = reinterpret_cast<PFN_##name>(vkGetDeviceProcAddr(device, #name));
    MAYO_VK_OPTIONAL_DEVICE_FUNCTIONS(MAYO_VK_LOAD)
#undef MAYO_VK_LOAD
    return ok;
}

// --- allocator -------------------------------------------------------------------------------

void Allocator::init(VkPhysicalDevice physicalDevice, VkDevice device) {
    m_device = device;
    vkGetPhysicalDeviceMemoryProperties(physicalDevice, &m_props);
}

void Allocator::destroy() {
    for (Block& b : m_blocks) {
        if (b.memory) vkFreeMemory(m_device, b.memory, nullptr);
    }
    m_blocks.clear();
}

int Allocator::findType(std::uint32_t typeBits, VkMemoryPropertyFlags required, VkMemoryPropertyFlags preferred) const {
    int fallback = -1;
    for (std::uint32_t i = 0; i < m_props.memoryTypeCount; ++i) {
        if (!(typeBits & (1u << i))) continue;
        const VkMemoryPropertyFlags flags = m_props.memoryTypes[i].propertyFlags;
        if ((flags & required) != required) continue;
        if ((flags & preferred) == preferred) return static_cast<int>(i);
        if (fallback < 0) fallback = static_cast<int>(i);
    }
    return fallback;
}

std::uint32_t Allocator::newBlock(std::uint32_t type, VkDeviceSize size, bool linear, bool dedicated) {
    VkMemoryAllocateInfo info{VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO};
    info.allocationSize = size;
    info.memoryTypeIndex = type;
    VkDeviceMemory memory = VK_NULL_HANDLE;
    if (vkAllocateMemory(m_device, &info, nullptr, &memory) != VK_SUCCESS) return ~0u;
    Block block;
    block.memory = memory;
    block.size = size;
    block.type = type;
    block.linear = linear;
    block.dedicated = dedicated;
    block.free.push_back({0, size});
    if (m_props.memoryTypes[type].propertyFlags & VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT) {
        if (vkMapMemory(m_device, memory, 0, VK_WHOLE_SIZE, 0, &block.mapped) != VK_SUCCESS) {
            vkFreeMemory(m_device, memory, nullptr);
            return ~0u;
        }
    }
    for (std::uint32_t i = 0; i < m_blocks.size(); ++i) {
        if (!m_blocks[i].memory) {
            m_blocks[i] = std::move(block);
            return i;
        }
    }
    m_blocks.push_back(std::move(block));
    return static_cast<std::uint32_t>(m_blocks.size() - 1);
}

bool Allocator::allocateFrom(Block& block, std::uint32_t index, const VkMemoryRequirements& req, Allocation& out) {
    for (size_t i = 0; i < block.free.size(); ++i) {
        Range& r = block.free[i];
        const VkDeviceSize start = alignUp(r.offset, std::max<VkDeviceSize>(req.alignment, 1));
        if (start + req.size > r.offset + r.size) continue;
        const Range before{r.offset, start - r.offset};
        const Range after{start + req.size, r.offset + r.size - (start + req.size)};
        block.free.erase(block.free.begin() + static_cast<std::ptrdiff_t>(i));
        auto at = block.free.begin() + static_cast<std::ptrdiff_t>(i);
        if (after.size) at = block.free.insert(at, after);
        if (before.size) block.free.insert(at, before);
        out.memory = block.memory;
        out.offset = start;
        out.size = req.size;  // the alignment gap went back to the free list as `before`
        out.mapped = block.mapped ? static_cast<char*>(block.mapped) + start : nullptr;
        out.block = index;
        block.used += req.size;
        return true;
    }
    return false;
}

bool Allocator::allocate(const VkMemoryRequirements& req, Usage usage, bool linear, Allocation& out) {
    VkMemoryPropertyFlags required = 0;
    VkMemoryPropertyFlags preferred = 0;
    switch (usage) {
        case Usage::Device: preferred = VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT; break;
        case Usage::Upload:
            required = VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT;
            break;
        case Usage::Readback:
            required = VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT;
            preferred = VK_MEMORY_PROPERTY_HOST_CACHED_BIT;
            break;
    }
    const int type = findType(req.memoryTypeBits, required, preferred);
    if (type < 0) return false;
    const auto t = static_cast<std::uint32_t>(type);

    if (req.size > kBlockSize / 4) {
        const std::uint32_t index = newBlock(t, req.size, linear, true);
        return index != ~0u && allocateFrom(m_blocks[index], index, req, out);
    }
    for (std::uint32_t i = 0; i < m_blocks.size(); ++i) {
        Block& b = m_blocks[i];
        if (!b.memory || b.dedicated || b.type != t || b.linear != linear) continue;
        if (allocateFrom(b, i, req, out)) return true;
    }
    const std::uint32_t index = newBlock(t, kBlockSize, linear, false);
    return index != ~0u && allocateFrom(m_blocks[index], index, req, out);
}

void Allocator::free(Allocation& a) {
    if (!a || a.block >= m_blocks.size()) return;
    Block& b = m_blocks[a.block];
    // Insert the range back in offset order and merge it with its neighbours.
    auto it = std::lower_bound(b.free.begin(), b.free.end(), a.offset,
                               [](const Range& r, VkDeviceSize offset) { return r.offset < offset; });
    it = b.free.insert(it, {a.offset, a.size});
    if (it + 1 != b.free.end() && it->offset + it->size == (it + 1)->offset) {
        it->size += (it + 1)->size;
        b.free.erase(it + 1);
    }
    if (it != b.free.begin() && (it - 1)->offset + (it - 1)->size == it->offset) {
        (it - 1)->size += it->size;
        b.free.erase(it);
    }
    b.used -= std::min(b.used, a.size);
    if (b.used == 0) {
        vkFreeMemory(m_device, b.memory, nullptr);
        b = Block{};
    }
    a = Allocation{};
}

}  // namespace mayo::rhi::vk
