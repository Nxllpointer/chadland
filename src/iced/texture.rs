use smithay::backend::allocator::{dmabuf::Dmabuf, Buffer};
use std::os::fd::IntoRawFd;

/// Equivalent properties for different contexts
/// https://github.com/gfx-rs/wgpu/blob/trunk/wgpu-hal/src/vulkan/conv.rs
/// https://github.com/gfx-rs/wgpu/blob/trunk/wgpu-core/src/conv.rs
/// https://smithay.github.io/smithay/src/smithay/backend/allocator/vulkan/format.rs.html
pub mod properties {
    pub const MIP_LEVEL_COUNT: u32 = 1;

    pub const SAMPLE_COUNT: (ash::vk::SampleCountFlags, u32) =
        (ash::vk::SampleCountFlags::TYPE_1, 1);

    pub const TEXTURE_DIMENSION: (ash::vk::ImageType, wgpu::TextureDimension) =
        (ash::vk::ImageType::TYPE_2D, wgpu::TextureDimension::D2);

    pub const ARRAY_LAYERS: u32 = 1;

    pub const TEXTURE_FORMAT: (ash::vk::Format, wgpu::TextureFormat, drm_fourcc::DrmFourcc) = (
        ash::vk::Format::R8G8B8A8_UNORM,
        wgpu::TextureFormat::Rgba8Unorm,
        drm_fourcc::DrmFourcc::Abgr8888,
    );

    pub const USAGE: (
        ash::vk::ImageUsageFlags,
        wgpu::hal::TextureUses,
        wgpu::TextureUsages,
    ) = (
        ash::vk::ImageUsageFlags::COLOR_ATTACHMENT,
        wgpu::hal::TextureUses::COLOR_TARGET,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
}

// TODO error handling
pub unsafe fn from_dmabuf(device: &wgpu::Device, dmabuf: &Dmabuf) -> wgpu::Texture {
    let (hal_texture, hal_descriptor) = device
        .as_hal::<wgpu::hal::vulkan::Api, _, _>(|device| {
            device.map(|device| hal_from_dmabuf(device, dmabuf))
        })
        .flatten()
        .expect("Unable to create hal texture");

    device.create_texture_from_hal::<wgpu::hal::vulkan::Api>(
        hal_texture,
        &wgpu::TextureDescriptor {
            label: hal_descriptor.label,
            dimension: hal_descriptor.dimension,
            size: hal_descriptor.size,
            format: hal_descriptor.format,
            usage: properties::USAGE.2,
            mip_level_count: hal_descriptor.mip_level_count,
            sample_count: hal_descriptor.sample_count,
            view_formats: hal_descriptor.view_formats.as_slice(),
        },
    )
}

unsafe fn hal_from_dmabuf(
    device: &wgpu::hal::vulkan::Device,
    dmabuf: &Dmabuf,
) -> (
    wgpu::hal::vulkan::Texture,
    wgpu::hal::TextureDescriptor<'static>,
) {
    let vk_device = device.raw_device();

    let dma_fd = dmabuf
        .handles()
        .last()
        .expect("No dmabuf fd")
        .try_clone_to_owned()
        .expect("Cant clone dmabuf fd");

    let mut external_memory_info = ash::vk::ExternalMemoryImageCreateInfo::default()
        .handle_types(ash::vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);

    let planes = dmabuf.offsets().zip(dmabuf.strides());
    let plane_layouts: Vec<_> = planes
        .map(|(offset, stride)| {
            ash::vk::SubresourceLayout::default()
                .offset(offset as u64)
                .row_pitch(stride as u64)
        })
        .collect();

    let mut drm_format_info = ash::vk::ImageDrmFormatModifierExplicitCreateInfoEXT::default()
        .drm_format_modifier(dmabuf.format().modifier.into())
        .plane_layouts(plane_layouts.as_slice());

    let image_info = ash::vk::ImageCreateInfo::default()
        .push_next(&mut external_memory_info)
        .push_next(&mut drm_format_info)
        .image_type(properties::TEXTURE_DIMENSION.0)
        .format(properties::TEXTURE_FORMAT.0)
        .mip_levels(properties::MIP_LEVEL_COUNT)
        .array_layers(properties::ARRAY_LAYERS)
        .samples(properties::SAMPLE_COUNT.0)
        .flags(ash::vk::ImageCreateFlags::empty())
        .extent(ash::vk::Extent3D {
            width: dmabuf.width(),
            height: dmabuf.height(),
            depth: properties::ARRAY_LAYERS,
        })
        .tiling(ash::vk::ImageTiling::LINEAR)
        .usage(properties::USAGE.0)
        .sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
        .initial_layout(ash::vk::ImageLayout::PREINITIALIZED);

    let image = vk_device
        .create_image(&image_info, None)
        .expect("Unable to create image");

    let image_requirements = vk_device.get_image_memory_requirements(image);

    let memory_type_index = image_requirements.memory_type_bits.trailing_zeros();

    let mut import_memory_info = ash::vk::ImportMemoryFdInfoKHR::default()
        .handle_type(ash::vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT)
        .fd(dma_fd.into_raw_fd());

    let mut dedicated_allocate_info = ash::vk::MemoryDedicatedAllocateInfo::default().image(image);

    let allocate_info = ash::vk::MemoryAllocateInfo::default()
        .push_next(&mut import_memory_info)
        .push_next(&mut dedicated_allocate_info)
        .allocation_size(image_requirements.size)
        .memory_type_index(memory_type_index);

    let memory = vk_device
        .allocate_memory(&allocate_info, None)
        .expect("Unable to allocate memory");

    vk_device
        .bind_image_memory(image, memory, 0)
        .expect("Unable to bind image memory");

    let texture_descriptor = wgpu::hal::TextureDescriptor {
        label: Some("Iced dmabuf imported texture"),
        dimension: properties::TEXTURE_DIMENSION.1,
        mip_level_count: properties::MIP_LEVEL_COUNT,
        sample_count: properties::SAMPLE_COUNT.1,
        format: properties::TEXTURE_FORMAT.1,
        usage: properties::USAGE.1,
        size: wgpu::Extent3d {
            width: dmabuf.width(),
            height: dmabuf.height(),
            depth_or_array_layers: properties::ARRAY_LAYERS,
        },
        memory_flags: wgpu::hal::MemoryFlags::empty(),
        view_formats: Vec::new(),
    };

    // ash::Device is very large, so we only copy the relevant handles instead of cloning
    // Maybe ash will introduces an Arc or wgpu exposes the device in the drop callback
    let drop_callback = {
        let vk_device_handle = vk_device.handle();
        let vk_destroy_image = vk_device.fp_v1_0().destroy_image;
        let vk_free_memory = vk_device.fp_v1_0().free_memory;
        move || {
            vk_destroy_image(vk_device_handle, image, std::ptr::null());
            vk_free_memory(vk_device_handle, memory, std::ptr::null());
        }
    };

    let hal_texture = wgpu::hal::vulkan::Device::texture_from_raw(
        image,
        &texture_descriptor,
        Some(Box::new(drop_callback)),
    );

    (hal_texture, texture_descriptor)
}
