//! Methods related to Vulkano devices

use crate::renderer::vulkan::VulkanRendererError;
use std::sync::Arc;
use vulkano::VulkanLibrary;
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceExtensions, DeviceFeatures, Queue, QueueCreateInfo, QueueFlags,
};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::swapchain::Surface;

/// Creates a `Device` and a `Queue`.
/// The device and queue support graphical calculations.
/// Can return a `VulkanRendererError` if the `Device` creation fails
pub fn get_device(
    required_extensions: DeviceExtensions,
) -> Result<(Arc<Device>, Arc<Queue>), VulkanRendererError> {
    let library = VulkanLibrary::new()?;
    let instance = Instance::new(
        library,
        InstanceCreateInfo {
            flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
            ..Default::default()
        },
    )?;

    // We then choose which physical device to use. First, we enumerate all the available
    // physical devices, then apply filters to narrow them down to those that can support our
    // needs.
    let (physical_device, queue_family_index) = instance
        .enumerate_physical_devices()?
        .filter(|p| {
            // Some devices may not support the extensions or features that your application,
            // or report properties and limits that are not sufficient for your application.
            // These should be filtered out here.
            p.supported_extensions().contains(&required_extensions)
        })
        .filter_map(|p| {
            // For each physical device, we try to find a suitable queue family that will
            // execute our draw commands.
            //
            // Devices can provide multiple queues to run commands in parallel (for example a
            // draw queue and a compute queue), similar to CPU threads. This is
            // something you have to have to manage manually in Vulkan. Queues
            // of the same type belong to the same queue family.
            //
            // Here, we look for a single queue family that is suitable for our purposes. In a
            // real-world application, you may want to use a separate dedicated transfer queue
            // to handle data transfers in parallel with graphics operations.
            // You may also need a separate queue for compute operations, if
            // your application uses those.
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(_i, q)| {
                    // We select a queue family that supports graphics operations. When drawing
                    // to a window surface, as we do in this example, we also need to check
                    // that queues in this queue family are capable of presenting images to the
                    // surface.
                    q.queue_flags.intersects(QueueFlags::GRAPHICS)
                })
                // The code here searches for the first queue family that is suitable. If none
                // is found, `None` is returned to `filter_map`, which
                // disqualifies this physical device.
                .map(|i| (p, i as u32))
        })
        // All the physical devices that pass the filters above are suitable for the
        // application. However, not every device is equal, some are preferred over others.
        // Now, we assign each physical device a score, and pick the device with the lowest
        // ("best") score.
        //
        // In this example, we simply select the best-scoring device to use in the application.
        // In a real-world setting, you may want to use the best-scoring device only as a
        // "default" or "recommended" device, and let the user choose the device themself.
        .min_by_key(|(p, _)| {
            // We assign a lower score to device types that are likely to be faster/better.
            match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
                _ => 5,
            }
        })
        .expect("no suitable physical device found");

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
            enabled_extensions: required_extensions,
            ..Default::default()
        },
    )?;

    Ok((device, queues.next().unwrap()))
}

/// Like `get_device`, but selects a physical device and queue family that can also
/// present to `surface`. The caller is responsible for creating the `Instance` with
/// the surface extensions already enabled.
pub fn get_device_for_surface(
    instance: Arc<Instance>,
    surface: &Arc<Surface>,
    required_extensions: DeviceExtensions,
) -> Result<(Arc<Device>, Arc<Queue>), VulkanRendererError> {
    let (physical_device, queue_family_index) = instance
        .enumerate_physical_devices()?
        .filter(|p| p.supported_extensions().contains(&required_extensions))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(idx, q)| {
                    q.queue_flags.intersects(QueueFlags::GRAPHICS)
                        && p.surface_support(idx as u32, surface).unwrap_or(false)
                })
                .map(|i| (p, i as u32))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            PhysicalDeviceType::Other => 4,
            _ => 5,
        })
        .expect("no suitable physical device found");

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
            enabled_extensions: required_extensions,
            ..Default::default()
        },
    )?;

    Ok((device, queues.next().unwrap()))
}
