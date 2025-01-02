use smithay::{
    backend::drm::{DrmDeviceFd, DrmNode},
    reexports::rustix::fs::makedev,
};
use std::os::fd::OwnedFd;

pub fn get_render_node(device: &wgpu::Device) -> Option<DrmDeviceFd> {
    unsafe {
        device
            .as_hal::<wgpu::hal::vulkan::Api, _, _>(|device| {
                device.and_then(|device| {
                    let vk_instance = device.shared_instance().raw_instance();
                    let vk_physical_device = device.raw_physical_device();

                    let mut drm_properties = ash::vk::PhysicalDeviceDrmPropertiesEXT::default();
                    let mut device_properties = ash::vk::PhysicalDeviceProperties2::default()
                        .push_next(&mut drm_properties);

                    vk_instance.get_physical_device_properties2(
                        vk_physical_device,
                        &mut device_properties,
                    );

                    if drm_properties.has_render == ash::vk::TRUE {
                        let dev_id = makedev(
                            drm_properties.render_major as u32,
                            drm_properties.render_minor as u32,
                        );
                        DrmNode::from_dev_id(dev_id)
                            .ok()
                            .and_then(|node| node.dev_path())
                            .and_then(|path| std::fs::File::open(path).ok())
                            .map(|file| DrmDeviceFd::new(OwnedFd::from(file).into()))
                    } else {
                        None
                    }
                })
            })
            .flatten()
    }
}
