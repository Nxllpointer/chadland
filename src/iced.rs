use futures::{FutureExt, StreamExt};
use smithay::backend::allocator::dmabuf::Dmabuf;
use std::marker::PhantomData;

pub type Renderer = iced_wgpu::Renderer;
pub type Theme = iced_core::Theme;
pub type Element<'a, Message> = iced_core::Element<'a, Message, Theme, Renderer>;

pub trait Program<B: crate::Backend> {
    type Message: iced_runtime::futures::MaybeSend + 'static;

    fn view(&self) -> impl Into<Element<'_, Self::Message>>;
    fn update(
        &mut self,
        comp: &mut crate::state::Compositor<B>,
        message: Self::Message,
    ) -> impl Into<iced_runtime::Task<Self::Message>>;
}

pub struct State<B: crate::Backend, P: Program<B>> {
    program: P,
    bounds: iced_core::Size,
    cache: iced_runtime::user_interface::Cache,
    device: wgpu::Device,
    renderer: Renderer,
    task_scheduler: calloop::futures::Scheduler<Option<iced_runtime::Action<P::Message>>>,
    event_sender: calloop::channel::Sender<iced_core::Event>,
    _b: PhantomData<B>,
}

impl<B: crate::Backend, P: Program<B, Message = crate::shell::Message>> State<B, P> {
    pub async fn new(
        program: P,
        bounds: iced_core::Size,
        loop_handle: calloop::LoopHandle<'_, crate::App<B>>,
    ) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });

        dbg!(instance.enumerate_adapters(wgpu::Backends::all()));

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .expect("No matching adapter found");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Iced program device"),
                    ..Default::default()
                },
                None,
            )
            .await
            .expect("Unable to create device");

        let engine = iced_wgpu::Engine::new(
            &adapter,
            &device,
            &queue,
            wgpu::TextureFormat::Rgba8Unorm,
            None,
        );
        let renderer =
            iced_wgpu::Renderer::new(&device, &engine, iced_core::Font::default(), 16.into());

        let (task_executor, task_scheduler) =
            calloop::futures::executor::<Option<iced_runtime::Action<P::Message>>>()
                .expect("Unable to create executor");

        let (event_sender, event_receiver) = calloop::channel::channel::<iced_core::Event>();

        let _ = loop_handle.insert_source(task_executor, |action, _, app| {
            let Some(action) = action else {
                return;
            };

            match action {
                iced_runtime::Action::Output(message) => app
                    .common
                    .iced
                    .process_message(&mut app.common.comp, message),
                _ => {}
            }
        });

        let _ = loop_handle.insert_source(event_receiver, |event, _, app| {
            let calloop::channel::Event::Msg(event) = event else {
                return;
            };

            app.common
                .iced
                .process_event(event, Some(&mut app.common.comp));
        });

        Self {
            program,
            bounds,
            cache: Default::default(),
            device,
            renderer,
            task_scheduler,
            event_sender,
            _b: Default::default(),
        }
    }

    pub fn bounds(&self) -> iced_core::Size {
        self.bounds
    }

    pub fn set_bounds(&mut self, bounds: impl Into<iced_core::Size>) {
        self.bounds = bounds.into();
    }

    pub fn schedule_task(&self, task: iced_runtime::Task<P::Message>) {
        iced_runtime::task::into_stream(task)
            .map(|stream| stream.into_future())
            .map(|future| future.map(|(result, _)| result))
            .map(|future| self.task_scheduler.schedule(future));
    }

    pub fn schedule_message(&self, message: P::Message) {
        self.schedule_task(iced_runtime::Task::done(message));
    }

    pub fn process_message(&mut self, comp: &mut crate::state::Compositor<B>, message: P::Message) {
        let task = self.program.update(comp, message).into();
        self.schedule_task(task);
    }

    pub fn schedule_event(&self, event: iced_core::Event) {
        let _ = self.event_sender.send(event);
    }

    pub fn process_event(
        &mut self,
        event: iced_core::Event,
        mut comp: Option<&mut crate::state::Compositor<B>>,
    ) {
        let mut messages: Vec<P::Message> = Vec::new();

        self.with_ui(|ui, renderer| {
            ui.update(
                &[event],
                iced_core::mouse::Cursor::Unavailable,
                renderer,
                &mut iced_core::clipboard::Null,
                &mut messages,
            );
        });

        while let Some(message) = messages.pop() {
            match comp {
                Some(ref mut comp) => self.process_message(comp, message),
                None => self.schedule_message(message),
            }
        }
    }

    pub fn with_ui<T>(
        &mut self,
        func: impl FnOnce(
            &mut iced_runtime::UserInterface<P::Message, Theme, Renderer>,
            &mut Renderer,
        ) -> T,
    ) -> T {
        let mut ui = iced_runtime::UserInterface::build(
            self.program.view(),
            self.bounds,
            std::mem::take(&mut self.cache),
            &mut self.renderer,
        );
        let result = func(&mut ui, &mut self.renderer);
        self.cache = ui.into_cache();

        result
    }

    pub fn import_dmabuf(&self, dmabuf: &Dmabuf) -> wgpu::Texture {
        unsafe { dmabuf::texture_from_dmabuf(&self.device, dmabuf) }
    }
}

// TODO error handling
mod dmabuf {
    use smithay::{
        backend::allocator::{dmabuf::Dmabuf, format::get_bpp, Buffer},
        reexports::ash,
    };
    use std::os::fd::IntoRawFd;

    const MIP_LEVEL_COUNT: u32 = 1;
    const SAMPLE_COUNT: (ash::vk::SampleCountFlags, u32) = (ash::vk::SampleCountFlags::TYPE_1, 1);
    const TEXTURE_DIMENSION: (ash::vk::ImageType, wgpu::TextureDimension) =
        (ash::vk::ImageType::TYPE_2D, wgpu::TextureDimension::D2);
    const ARRAY_LAYERS: u32 = 1;
    const TEXTURE_FORMAT: (ash::vk::Format, wgpu::TextureFormat) = (
        ash::vk::Format::R8G8B8A8_UNORM,
        wgpu::TextureFormat::Rgba8Unorm,
    );
    const USAGE: (
        ash::vk::ImageUsageFlags,
        wgpu::hal::TextureUses,
        wgpu::TextureUsages,
    ) = (
        ash::vk::ImageUsageFlags::empty(),
        wgpu::hal::TextureUses::empty(),
        wgpu::TextureUsages::empty(),
    );

    pub unsafe fn texture_from_dmabuf(device: &wgpu::Device, dmabuf: &Dmabuf) -> wgpu::Texture {
        let (hal_texture, hal_descriptor) = device
            .as_hal::<wgpu::hal::vulkan::Api, _, _>(|device| {
                if let Some(device) = device {
                    Some(hal_texture_from_dmabuf(device, dmabuf))
                } else {
                    None
                }
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
                usage: USAGE.2,
                mip_level_count: hal_descriptor.mip_level_count,
                sample_count: hal_descriptor.sample_count,
                view_formats: hal_descriptor.view_formats.as_slice(),
            },
        )
    }

    unsafe fn hal_texture_from_dmabuf(
        device: &wgpu::hal::vulkan::Device,
        dmabuf: &Dmabuf,
    ) -> (
        wgpu::hal::vulkan::Texture,
        wgpu::hal::TextureDescriptor<'static>,
    ) {
        let vk_instance = device.shared_instance().raw_instance();
        let vk_device = device.raw_device();
        let vk_physical_device = device.raw_physical_device();

        let mem_properties = vk_instance.get_physical_device_memory_properties(vk_physical_device);
        let memory_type_index = mem_properties
            .memory_types
            .into_iter()
            .enumerate()
            .find(|(_, mem)| {
                mem.property_flags
                    .contains(ash::vk::MemoryPropertyFlags::DEVICE_LOCAL)
            })
            .expect("Unable to find memory type index")
            .0;

        let dma_fd = dmabuf
            .handles()
            .last()
            .expect("No dmabuf fd")
            .try_clone_to_owned()
            .expect("Cant clone dmabuf fd");

        let mut import_mem_fd_info = ash::vk::ImportMemoryFdInfoKHR::default()
            .handle_type(ash::vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT)
            .fd(dma_fd.into_raw_fd());

        let bytes_per_pixel = get_bpp(dmabuf.format().code).expect("Cant get bpp for dmabuf") / 8;
        let size = dmabuf.width() * dmabuf.height() * bytes_per_pixel as u32;

        let allocate_info = ash::vk::MemoryAllocateInfo::default()
            .push_next(&mut import_mem_fd_info)
            .allocation_size(size as u64)
            .memory_type_index(memory_type_index as u32);

        let memory = vk_device
            .allocate_memory(&allocate_info, None)
            .expect("Unable to import memory");

        let image_info = ash::vk::ImageCreateInfo::default()
            .flags(ash::vk::ImageCreateFlags::empty())
            .image_type(TEXTURE_DIMENSION.0)
            .format(TEXTURE_FORMAT.0)
            .extent(ash::vk::Extent3D {
                width: dmabuf.width(),
                height: dmabuf.height(),
                depth: 0,
            })
            .mip_levels(MIP_LEVEL_COUNT)
            .array_layers(ARRAY_LAYERS)
            .samples(SAMPLE_COUNT.0)
            .tiling(ash::vk::ImageTiling::OPTIMAL)
            .usage(USAGE.0)
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
            label: Some("Iced DMABUF imported texture"),
            dimension: TEXTURE_DIMENSION.1,
            size: wgpu::Extent3d {
                width: dmabuf.width(),
                height: dmabuf.height(),
                depth_or_array_layers: ARRAY_LAYERS,
            },
            mip_level_count: MIP_LEVEL_COUNT,
            sample_count: SAMPLE_COUNT.1,
            format: TEXTURE_FORMAT.1,
            usage: USAGE.1,
            memory_flags: wgpu::hal::MemoryFlags::empty(),
            view_formats: Vec::new(),
        };

        let hal_texture =
            wgpu::hal::vulkan::Device::texture_from_raw(image, &texture_descriptor, None);

        (hal_texture, texture_descriptor)
    }
}
