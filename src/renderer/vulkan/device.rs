//! Methods related to Vulkano devices

use crate::renderer::vulkan::VulkanRendererError;
use std::sync::Arc;
use vulkano::VulkanLibrary;
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceFeatures, Queue, QueueCreateInfo, QueueFlags,
};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};

/// Creates a `Device` and a `Queue`.
/// The device and queue support graphical calculations.
/// Can return a `VulkanRendererError` if the `Device` creation fails
pub fn get_device() -> Result<(Arc<Device>, Arc<Queue>), VulkanRendererError> {
    let library = VulkanLibrary::new()?;
    let instance = Instance::new(
        library,
        InstanceCreateInfo {
            flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
            ..Default::default()
        },
    )?;

    let physical_device = instance
        .enumerate_physical_devices()?
        .next()
        .ok_or(VulkanRendererError::NoPhysicalDevice)?;

    let queue_family_index = physical_device
        .queue_family_properties()
        .iter()
        .position(|q| q.queue_flags.contains(QueueFlags::GRAPHICS))
        .ok_or(VulkanRendererError::NoGraphicalQueueFamily)? as u32;

    let (device, mut queues) = Device::new(
        physical_device,
        DeviceCreateInfo {
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            enabled_features: DeviceFeatures {
                fill_mode_non_solid: true,
                ..Default::default()
            },
            ..Default::default()
        },
    )?;

    Ok((device, queues.next().unwrap()))
}
