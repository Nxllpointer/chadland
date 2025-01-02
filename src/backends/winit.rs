use smithay::{
    backend::{
        allocator,
        egl::EGLDevice,
        renderer::{damage::OutputDamageTracker, gles::GlesRenderer, Frame, ImportDma, Renderer},
        winit::{WinitEvent, WinitGraphicsBackend},
    },
    output,
    reexports::calloop,
    utils::{Physical, Rectangle, Transform},
    wayland::dmabuf::DmabufFeedbackBuilder,
};
use std::time::Duration;
use tracing::error;

const REFRESH_RATE: i32 = 60;

pub type WinitApp = crate::App<WinitBackend>;

pub struct WinitBackend {
    pub winit: WinitGraphicsBackend<GlesRenderer>,
    pub output: output::Output,
    pub damage_tracker: OutputDamageTracker,
}

impl super::Backend for WinitBackend {
    type SelfType = WinitBackend;

    fn new(common: &mut crate::state::Common<Self::SelfType>) -> Self {
        let (winit, event_source) =
            smithay::backend::winit::init::<GlesRenderer>().expect("Unable to initialize winit");

        common
            .comp
            .loop_handle
            .insert_source(event_source, |winit_event, _, app| {
                app.event_handler(winit_event)
            })
            .expect("Unable to insert winit event source");

        let redraw_delay = Duration::from_millis(1000 / REFRESH_RATE as u64);
        common
            .comp
            .loop_handle
            .insert_source(
                calloop::timer::Timer::from_duration(redraw_delay),
                move |_, _, app| {
                    app.event_handler(WinitEvent::Redraw);
                    calloop::timer::TimeoutAction::ToDuration(redraw_delay)
                },
            )
            .expect("Unable to insert redraw timer event source");

        common
            .comp
            .seat
            .add_keyboard(smithay::input::keyboard::XkbConfig::default(), 500, 100)
            .expect("Unable to initialize keyboard");

        let output = output::Output::new(
            "winit".to_string(),
            output::PhysicalProperties {
                size: (0, 0).into(),
                subpixel: output::Subpixel::Unknown,
                make: "Chadland".to_string(),
                model: "Super".to_string(),
            },
        );

        output.change_current_state(
            Some(output::Mode {
                size: winit.window_size(),
                refresh: REFRESH_RATE * 1000,
            }),
            // Everything is upside down without transform
            Some(smithay::utils::Transform::Flipped180),
            Some(output::Scale::Integer(1)),
            Some((0, 0).into()),
        );
        output.set_preferred(output.current_mode().expect("Output has no current mode"));

        output.create_global::<WinitApp>(&common.comp.display_handle);

        common.comp.space.map_output(&output, (0, 0));

        let damage_tracker = OutputDamageTracker::from_output(&output);

        Self {
            winit,
            output,
            damage_tracker,
        }
    }

    fn default_dmabuf_feedback(&mut self) -> Option<smithay::wayland::dmabuf::DmabufFeedback> {
        let display = self.winit.renderer().egl_context().display();
        let device = EGLDevice::device_for_display(display).ok()?;
        let render_node = device.try_get_render_node().ok()??;
        DmabufFeedbackBuilder::new(render_node.dev_id(), self.winit.renderer().dmabuf_formats())
            .build()
            .ok()
    }

    fn dmabuf_formats(&mut self) -> allocator::format::FormatSet {
        self.winit.renderer().dmabuf_formats()
    }

    fn import_dmabuf(&mut self, dmabuf: &allocator::dmabuf::Dmabuf) -> std::result::Result<(), ()> {
        match self.winit.renderer().import_dmabuf(dmabuf, None) {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed importing dmabuf: {e:?}");
                Err(())
            }
        }
    }
}

impl WinitApp {
    fn event_handler(&mut self, winit_event: WinitEvent) {
        match winit_event {
            smithay::backend::winit::WinitEvent::Resized {
                size,
                scale_factor: _,
            } => self.backend.output.change_current_state(
                Some(output::Mode {
                    size,
                    refresh: REFRESH_RATE * 1000,
                }),
                None,
                None,
                None,
            ),
            smithay::backend::winit::WinitEvent::Focus(_) => {}
            smithay::backend::winit::WinitEvent::Input(event) => {
                self.process_input(crate::input::InputEvent::Basic(event));
            }
            smithay::backend::winit::WinitEvent::CloseRequested => {
                self.common.comp.loop_signal.stop()
            }
            smithay::backend::winit::WinitEvent::Redraw => self.render(),
        }
    }

    fn render(&mut self) {
        let win_size = self.backend.winit.window_size();
        let win_rect =
            Rectangle::<_, Physical>::from_loc_and_size((0, 0), (win_size.w, win_size.h));

        let iced_dmabuf = self.common.shell_driver.render(
            &self.common.comp,
            (win_size.w as u32, win_size.h as u32).into(),
        );

        let renderer = self.backend.winit.renderer();
        let iced_texture = renderer
            .import_dmabuf(
                &iced_dmabuf,
                Some(&[win_rect.to_logical(1).to_buffer(
                    1,
                    Transform::Normal,
                    &win_size.to_logical(1),
                )]),
            )
            .expect("Cant import iced dmabuf into gles");

        self.backend.winit.bind().expect("Unable to bind backend");

        let mut frame = self
            .backend
            .winit
            .renderer()
            .render(win_size, Transform::Flipped180)
            .expect("Unable to create render frame");

        frame
            .render_texture_at(
                &iced_texture,
                (0, 0).into(),
                1,
                1.0,
                Transform::Normal,
                &[win_rect],
                &[win_rect],
                1.0,
            )
            .expect("Unable to render iced texture");
        drop(frame);

        self.backend
            .winit
            .submit(Some(&[win_rect]))
            .expect("Unable to submit back buffer");

        self.common.comp.space.elements().for_each(|window| {
            // TODO this *should* only be run for visible surfaces
            window.send_frame(
                &self.backend.output,
                self.common.comp.start_time.elapsed(),
                Some(std::time::Duration::ZERO),
                |_, _| Some(self.backend.output.clone()),
            )
        });
    }
}
