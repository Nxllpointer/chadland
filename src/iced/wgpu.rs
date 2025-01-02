pub use wgpu::*;

const ADDITIONAL_EXTENSIONS: [&str; 3] = [
    "VK_KHR_external_memory_fd\0",
    "VK_EXT_external_memory_dma_buf\0",
    "VK_EXT_image_drm_format_modifier\0",
];

pub struct Objects {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Objects {
    pub async fn new() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .expect("No matching adapter found");

        let device_desc = wgpu::DeviceDescriptor {
            label: Some("Iced program device"),
            ..Default::default()
        };

        let hal_device = unsafe {
            adapter.as_hal::<wgpu::hal::vulkan::Api, _, _>(|hal_adapter| {
                hal_adapter.map(|hal_adapter| create_hal_device(hal_adapter, &device_desc))
            })
        }
        .expect("No vulkan hal adapter");

        let (device, queue) = unsafe {
            adapter
                .create_device_from_hal(hal_device, &device_desc, None)
                .expect("Unable to create device")
        };

        Self {
            instance,
            adapter,
            device,
            queue,
        }
    }
}

fn create_hal_device(
    adapter: &wgpu::hal::vulkan::Adapter,
    desc: &wgpu::DeviceDescriptor<'_>,
) -> hal::OpenDevice<hal::vulkan::Api> {
    let vk_instance = adapter.shared_instance().raw_instance();
    let vk_physical_device = adapter.raw_physical_device();

    let mut extensions = adapter.required_device_extensions(desc.required_features);
    extensions.extend(ADDITIONAL_EXTENSIONS.map(|e| {
        std::ffi::CStr::from_bytes_with_nul(e.as_bytes())
            .expect(&format!("Extension {e} has no null terminator"))
    }));

    let mut physical_features =
        adapter.physical_device_features(&extensions, desc.required_features);

    let extension_names: Vec<*const std::ffi::c_char> =
        extensions.iter().map(|ext| ext.as_ptr()).collect();

    let queue_info = ash::vk::DeviceQueueCreateInfo::default()
        .queue_family_index(0)
        .queue_priorities(&[1.0]);
    let queue_infos = [queue_info];

    let device_info = ash::vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_infos)
        .enabled_extension_names(extension_names.as_slice());
    let device_info = physical_features.add_to_device_create(device_info);

    unsafe {
        let vk_device = vk_instance
            .create_device(vk_physical_device, &device_info, None)
            .expect("Unable to create vulkan device");

        adapter
            .device_from_raw(
                vk_device,
                None,
                &extensions,
                desc.required_features,
                &desc.memory_hints,
                queue_info.queue_family_index,
                0,
            )
            .expect("Unable to create hal device")
    }
}
