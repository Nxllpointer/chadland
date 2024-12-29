use smithay::backend::allocator::{dmabuf::Dmabuf, format::get_bpp, Buffer};
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
    let dma_fd = dmabuf
        .handles()
        .last()
        .expect("No dmabuf fd")
        .try_clone_to_owned()
        .expect("Cant clone dmabuf fd");

    let vk_instance = device.shared_instance().raw_instance();
    let vk_device = device.raw_device();
    let vk_physical_device = device.raw_physical_device();

    let memory_properties = vk_instance.get_physical_device_memory_properties(vk_physical_device);
    let memory_type_index = memory_properties
        .memory_types
        .into_iter()
        .enumerate()
        .find(|(_, mem)| {
            mem.property_flags
                .contains(ash::vk::MemoryPropertyFlags::DEVICE_LOCAL)
        })
        .expect("Unable to find memory type index")
        .0;

    let mut import_memory_fd_info = ash::vk::ImportMemoryFdInfoKHR::default()
        .handle_type(ash::vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT)
        .fd(dma_fd.into_raw_fd());

    let bytes_per_pixel = get_bpp(dmabuf.format().code).expect("Cant get bpp for dmabuf") / 8;
    let size = dmabuf.width() * dmabuf.height() * bytes_per_pixel as u32;

    let allocate_info = ash::vk::MemoryAllocateInfo::default()
        .push_next(&mut import_memory_fd_info)
        .allocation_size(size as u64)
        .memory_type_index(memory_type_index as u32);

    let memory = vk_device
        .allocate_memory(&allocate_info, None)
        .expect("Unable to import memory");

    let image_info = ash::vk::ImageCreateInfo::default()
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
        .initial_layout(ash::vk::ImageLayout::UNDEFINED);

    let image = vk_device
        .create_image(&image_info, None)
        .expect("Cant create image");

    vk_device
        .bind_image_memory(
            image,
            memory,
            dmabuf.offsets().last().expect("No offset") as u64,
        )
        .expect("Unable to bind memory to image");

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

    let hal_texture = wgpu::hal::vulkan::Device::texture_from_raw(image, &texture_descriptor, None);

    (hal_texture, texture_descriptor)
}
