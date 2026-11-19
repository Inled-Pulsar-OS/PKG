#pragma once

// Vulkan entry points, loaded at run time through SDL (no link-time dependency on libvulkan, so the library
// still starts where Vulkan isn't installed). Declared in mayo::rhi::vk under the real names, so code in that
// namespace calls them as usual.

#define VK_NO_PROTOTYPES
#include <vulkan/vulkan_core.h>

namespace mayo::rhi::vk {

#define MAYO_VK_GLOBAL_FUNCTIONS(X) \
    X(vkCreateInstance)             \
    X(vkEnumerateInstanceVersion)   \
    X(vkEnumerateInstanceExtensionProperties) \
    X(vkEnumerateInstanceLayerProperties)

#define MAYO_VK_INSTANCE_FUNCTIONS(X)              \
    X(vkDestroyInstance)                           \
    X(vkEnumeratePhysicalDevices)                  \
    X(vkGetPhysicalDeviceProperties)               \
    X(vkGetPhysicalDeviceFeatures2)                \
    X(vkGetPhysicalDeviceQueueFamilyProperties)    \
    X(vkGetPhysicalDeviceMemoryProperties)         \
    X(vkGetPhysicalDeviceFormatProperties)         \
    X(vkGetPhysicalDeviceImageFormatProperties2)   \
    X(vkEnumerateDeviceExtensionProperties)        \
    X(vkCreateDevice)                              \
    X(vkGetDeviceProcAddr)                         \
    X(vkDestroySurfaceKHR)                         \
    X(vkGetPhysicalDeviceSurfaceSupportKHR)        \
    X(vkGetPhysicalDeviceSurfaceCapabilitiesKHR)   \
    X(vkGetPhysicalDeviceSurfaceFormatsKHR)

#define MAYO_VK_DEVICE_FUNCTIONS(X)       \
    X(vkDestroyDevice)                    \
    X(vkGetDeviceQueue)                   \
    X(vkDeviceWaitIdle)                   \
    X(vkQueueSubmit2)                     \
    X(vkQueueWaitIdle)                    \
    X(vkQueuePresentKHR)                  \
    X(vkCreateSwapchainKHR)               \
    X(vkDestroySwapchainKHR)              \
    X(vkGetSwapchainImagesKHR)            \
    X(vkAcquireNextImageKHR)              \
    X(vkCreateCommandPool)                \
    X(vkDestroyCommandPool)               \
    X(vkResetCommandPool)                 \
    X(vkAllocateCommandBuffers)           \
    X(vkBeginCommandBuffer)               \
    X(vkEndCommandBuffer)                 \
    X(vkCreateFence)                      \
    X(vkDestroyFence)                     \
    X(vkWaitForFences)                    \
    X(vkResetFences)                      \
    X(vkCreateSemaphore)                  \
    X(vkDestroySemaphore)                 \
    X(vkCreateImage)                      \
    X(vkDestroyImage)                     \
    X(vkGetImageMemoryRequirements)       \
    X(vkBindImageMemory)                  \
    X(vkCreateImageView)                  \
    X(vkDestroyImageView)                 \
    X(vkCreateBuffer)                     \
    X(vkDestroyBuffer)                    \
    X(vkGetBufferMemoryRequirements)      \
    X(vkBindBufferMemory)                 \
    X(vkAllocateMemory)                   \
    X(vkFreeMemory)                       \
    X(vkMapMemory)                        \
    X(vkUnmapMemory)                      \
    X(vkInvalidateMappedMemoryRanges)     \
    X(vkCreateSampler)                    \
    X(vkDestroySampler)                   \
    X(vkCreateDescriptorSetLayout)        \
    X(vkDestroyDescriptorSetLayout)       \
    X(vkCreatePipelineLayout)             \
    X(vkDestroyPipelineLayout)            \
    X(vkCreateShaderModule)               \
    X(vkDestroyShaderModule)              \
    X(vkCreateGraphicsPipelines)          \
    X(vkDestroyPipeline)                  \
    X(vkCreateDescriptorPool)             \
    X(vkDestroyDescriptorPool)            \
    X(vkResetDescriptorPool)              \
    X(vkAllocateDescriptorSets)           \
    X(vkUpdateDescriptorSets)             \
    X(vkCmdBeginRendering)                \
    X(vkCmdEndRendering)                  \
    X(vkCmdPipelineBarrier2)              \
    X(vkCmdBindPipeline)                  \
    X(vkCmdBindDescriptorSets)            \
    X(vkCmdBindVertexBuffers)             \
    X(vkCmdPushConstants)                 \
    X(vkCmdSetViewport)                   \
    X(vkCmdSetScissor)                    \
    X(vkCmdDraw)                          \
    X(vkCmdCopyBufferToImage)             \
    X(vkCmdCopyImageToBuffer)             \
    X(vkCmdBlitImage)                     \
    X(vkCmdClearColorImage)

// Only present when the device enables the dma-buf import extensions.
#define MAYO_VK_OPTIONAL_DEVICE_FUNCTIONS(X) X(vkGetMemoryFdPropertiesKHR)

#define MAYO_VK_DECLARE(name) inline PFN_##name name = nullptr;
MAYO_VK_GLOBAL_FUNCTIONS(MAYO_VK_DECLARE)
MAYO_VK_INSTANCE_FUNCTIONS(MAYO_VK_DECLARE)
MAYO_VK_DEVICE_FUNCTIONS(MAYO_VK_DECLARE)
MAYO_VK_OPTIONAL_DEVICE_FUNCTIONS(MAYO_VK_DECLARE)
#undef MAYO_VK_DECLARE

/// Each returns false if a required entry point is missing.
bool loadGlobal(PFN_vkGetInstanceProcAddr getInstanceProcAddr);
bool loadInstance(VkInstance instance);
bool loadDevice(VkDevice device);

}  // namespace mayo::rhi::vk
