use futures::{FutureExt, StreamExt};
use smithay::{
    backend::{
        allocator::{
            dmabuf::{AsDmabuf, Dmabuf},
            gbm::GbmAllocator,
            Allocator,
        },
        drm::DrmDeviceFd,
    },
    reexports::gbm,
};
use std::sync::Arc;

mod drm;
pub mod texture;
pub mod wgpu;

pub type Renderer = iced_wgpu::Renderer;
pub type Theme = iced_core::Theme;
pub type Element<'a, Message> = iced_core::Element<'a, Message, Theme, Renderer>;
pub type Bounds = iced_core::Size<u32>;

pub trait Program {
    type Data;
    type Message: iced_runtime::futures::MaybeSend + 'static;

    fn view(data: &Self::Data) -> impl Into<Element<'_, Self::Message>>;
    fn update(
        data: &mut Self::Data,
        message: Self::Message,
    ) -> impl Into<iced_runtime::Task<Self::Message>>;
}

pub struct Driver<P: Program> {
    cache: iced_runtime::user_interface::Cache,
    wgpu_objects: Arc<wgpu::Objects>,
    engine: iced_wgpu::Engine,
    renderer: Renderer,
    allocator: GbmAllocator<DrmDeviceFd>,
    cached_buffer: Option<(Bounds, Dmabuf, Arc<wgpu::Texture>)>,
    task_scheduler: calloop::futures::Scheduler<Option<iced_runtime::Action<P::Message>>>,
    event_sender: calloop::channel::Sender<iced_core::Event>,
}

impl<P: Program + 'static> Driver<P> {
    pub fn new<B: crate::Backend>(
        wgpu_objects: Arc<wgpu::Objects>,
        loop_handle: calloop::LoopHandle<'_, crate::App<B>>,
        object_provider: fn(&mut crate::App<B>) -> (&mut Self, &mut P::Data, Bounds),
    ) -> Self {
        let engine = iced_wgpu::Engine::new(
            &wgpu_objects.adapter,
            &wgpu_objects.device,
            &wgpu_objects.queue,
            wgpu::TextureFormat::Rgba8Unorm,
            None,
        );
        let renderer = iced_wgpu::Renderer::new(
            &wgpu_objects.device,
            &engine,
            iced_core::Font::default(),
            16.into(),
        );

        let (task_executor, task_scheduler) =
            calloop::futures::executor::<Option<iced_runtime::Action<P::Message>>>()
                .expect("Unable to create executor");

        let (event_sender, event_receiver) = calloop::channel::channel::<iced_core::Event>();

        let _ = loop_handle.insert_source(task_executor, move |action, _, app| {
            let Some(action) = action else {
                return;
            };

            let (driver, data, _bounds) = object_provider(app);
            match action {
                iced_runtime::Action::Output(message) => driver.process_message(data, message),
                _ => tracing::warn!("Iced runtime action not implemented!"),
            }
        });

        let _ = loop_handle.insert_source(event_receiver, move |event, _, app| {
            let calloop::channel::Event::Msg(event) = event else {
                return;
            };

            let (driver, data, bounds) = object_provider(app);
            driver.process_event(event, data, bounds, true);
        });

        let gbm_device = gbm::Device::new(
            drm::get_render_node(&wgpu_objects.device).expect("Unable to get drm render node"),
        )
        .expect("Cant create gbm device");

        let allocator = GbmAllocator::new(gbm_device, gbm::BufferObjectFlags::RENDERING);

        Self {
            cache: Default::default(),
            wgpu_objects,
            engine,
            renderer,
            allocator,
            cached_buffer: None,
            task_scheduler,
            event_sender,
        }
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

    pub fn process_message(&mut self, data: &mut P::Data, message: P::Message) {
        let task = P::update(data, message).into();
        self.schedule_task(task);
    }

    pub fn schedule_event(&self, event: iced_core::Event) {
        let _ = self.event_sender.send(event);
    }

    pub fn process_event(
        &mut self,
        event: iced_core::Event,
        data: &mut P::Data,
        bounds: Bounds,
        process_messages_immediately: bool,
    ) {
        let mut messages: Vec<P::Message> = Vec::new();

        self.with_ui(data, bounds, |ui, driver| {
            ui.update(
                &[event],
                iced_core::mouse::Cursor::Unavailable,
                &mut driver.renderer,
                &mut iced_core::clipboard::Null,
                &mut messages,
            );
        });

        while let Some(message) = messages.pop() {
            if process_messages_immediately {
                self.process_message(data, message);
            } else {
                self.schedule_message(message);
            }
        }
    }

    pub fn render(&mut self, data: &P::Data, bounds: Bounds) -> Dmabuf {
        let (dmabuf, texture) = self.get_buffer(bounds);
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.with_ui(data, bounds, |ui, driver| {
            ui.draw(
                &mut driver.renderer,
                &iced_core::Theme::CatppuccinMocha,
                &iced_core::renderer::Style::default(),
                iced_core::mouse::Cursor::Unavailable,
            );
        });

        let mut encoder =
            self.wgpu_objects
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Iced render encoder"),
                });

        self.renderer.present(
            &mut self.engine,
            &self.wgpu_objects.device,
            &self.wgpu_objects.queue,
            &mut encoder,
            Some(iced_core::color!(0x6666aa)),
            texture::properties::TEXTURE_FORMAT.1,
            &texture_view,
            &iced_wgpu::graphics::Viewport::with_physical_size(bounds, 1.0),
            &[] as &[String],
        );

        let submission_index = self.engine.submit(&self.wgpu_objects.queue, encoder);
        self.wgpu_objects
            .device
            .poll(wgpu::Maintain::WaitForSubmissionIndex(submission_index));

        dmabuf
    }

    fn get_buffer(&mut self, bounds: Bounds) -> (Dmabuf, Arc<wgpu::Texture>) {
        let new_cached_buffer = self
            .cached_buffer
            .take_if(|(cached_bounds, _, _)| cached_bounds == &bounds)
            .unwrap_or_else(|| {
                let gbm_buffer = self
                    .allocator
                    .create_buffer(
                        bounds.width,
                        bounds.height,
                        texture::properties::TEXTURE_FORMAT.2,
                        &[drm_fourcc::DrmModifier::Linear],
                    )
                    .expect("Unable to allocate gbm buffer");

                let dmabuf = gbm_buffer
                    .export()
                    .expect("Unable to export gbm buffer as dmabuf");

                let texture = unsafe { texture::from_dmabuf(&self.wgpu_objects.device, &dmabuf) };

                (bounds, dmabuf, Arc::new(texture))
            });

        self.cached_buffer = Some(new_cached_buffer.clone());

        (new_cached_buffer.1, new_cached_buffer.2)
    }

    fn with_ui<T>(
        &mut self,
        data: &P::Data,
        bounds: Bounds,
        func: impl FnOnce(&mut iced_runtime::UserInterface<P::Message, Theme, Renderer>, &mut Self) -> T,
    ) -> T {
        let mut ui = iced_runtime::UserInterface::build(
            P::view(data),
            (bounds.width as f32, bounds.height as f32).into(),
            std::mem::take(&mut self.cache),
            &mut self.renderer,
        );
        let result = func(&mut ui, self);
        self.cache = ui.into_cache();

        result
    }
}
