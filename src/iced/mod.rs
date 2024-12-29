use futures::{FutureExt, StreamExt};
use smithay::{
    backend::{
        allocator::{
            dmabuf::{AsDmabuf, Dmabuf},
            gbm::GbmAllocator,
            Swapchain,
        },
        drm::DrmDeviceFd,
    },
    reexports::gbm,
};
use std::marker::PhantomData;

mod drm;
pub mod texture;
pub mod wgpu;

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
    wgpu_objects: wgpu::Objects,
    engine: iced_wgpu::Engine,
    renderer: Renderer,
    swapchain: Swapchain<GbmAllocator<DrmDeviceFd>>,
    task_scheduler: calloop::futures::Scheduler<Option<iced_runtime::Action<P::Message>>>,
    event_sender: calloop::channel::Sender<iced_core::Event>,
    _b: PhantomData<B>,
}

impl<B: crate::Backend, P: Program<B, Message = crate::shell::Message>> State<B, P> {
    pub fn new(
        program: P,
        bounds: iced_core::Size,
        wgpu_objects: wgpu::Objects,
        loop_handle: calloop::LoopHandle<'_, crate::App<B>>,
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

        let gbm_device = gbm::Device::new(
            drm::get_render_node(&wgpu_objects.device).expect("Unable to get render node"),
        )
        .expect("Cant create gbm device");

        let swapchain = Swapchain::new(
            GbmAllocator::new(gbm_device, gbm::BufferObjectFlags::RENDERING),
            bounds.width as u32,
            bounds.height as u32,
            texture::properties::TEXTURE_FORMAT.2,
            vec![drm_fourcc::DrmModifier::Linear],
        );

        Self {
            program,
            bounds,
            cache: Default::default(),
            wgpu_objects,
            engine,
            renderer,
            swapchain,
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
        self.swapchain
            .resize(self.bounds.width as u32, self.bounds.height as u32);
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

    pub fn render(&mut self, dmabuf: &Dmabuf) {
        let texture = self.import_dmabuf(dmabuf);
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.with_ui(|ui, renderer| {
            ui.draw(
                renderer,
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
            None,
            texture::properties::TEXTURE_FORMAT.1,
            &texture_view,
            &iced_wgpu::graphics::Viewport::with_physical_size(
                (self.bounds.width as u32, self.bounds.height as u32).into(),
                1.0,
            ),
            &[] as &[String],
        );

        let submission_index = self.engine.submit(&self.wgpu_objects.queue, encoder);
        self.wgpu_objects
            .device
            .poll(wgpu::Maintain::WaitForSubmissionIndex(submission_index));
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
        unsafe { texture::from_dmabuf(&self.wgpu_objects.device, dmabuf) }
    }

    pub fn get_buffer(&mut self) -> Dmabuf {
        self.swapchain
            .acquire()
            .expect("allocation error")
            .expect("No free slot")
            .export()
            .expect("Cant export dambuf")
    }
}
