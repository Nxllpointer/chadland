use std::marker::PhantomData;

use futures::{FutureExt, StreamExt};

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
            renderer,
            task_scheduler,
            event_sender,
            _b: Default::default(),
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
}
