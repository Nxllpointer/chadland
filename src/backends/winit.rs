use smithay::{
    backend::{
        allocator,
        egl::EGLDevice,
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer, ImportDma,
        },
        winit::{WinitEvent, WinitGraphicsBackend},
    },
    desktop, output,
    wayland::dmabuf::DmabufFeedbackBuilder,
};
use tracing::error;

const REFRESH_RATE: i32 = 60_000;

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
            .loop_handle
            .insert_source(event_source, |winit_event, _, app| {
                Self::event_handler(winit_event, app)
            })
            .expect("Unable to insert winit event source");

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
                refresh: REFRESH_RATE,
            }),
            // Everything is upside down without transform
            Some(smithay::utils::Transform::Flipped180),
            Some(output::Scale::Integer(1)),
            Some((0, 0).into()),
        );
        output.set_preferred(output.current_mode().expect("Output has no current mode"));

        output.create_global::<WinitApp>(&common.display_handle);

        common.space.map_output(&output, (0, 0));

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

impl WinitBackend {
    fn event_handler(winit_event: WinitEvent, app: &mut WinitApp) {
        match winit_event {
            smithay::backend::winit::WinitEvent::Resized {
                size,
                scale_factor: _,
            } => app.backend.output.change_current_state(
                Some(output::Mode {
                    size,
                    refresh: REFRESH_RATE,
                }),
                None,
                None,
                None,
            ),
            smithay::backend::winit::WinitEvent::Focus(_) => {}
            smithay::backend::winit::WinitEvent::Input(_) => {}
            smithay::backend::winit::WinitEvent::CloseRequested => app.common.loop_signal.stop(),
            smithay::backend::winit::WinitEvent::Redraw => Self::render(app),
        }
    }

    fn render(app: &mut WinitApp) {
        app.backend.winit.bind().expect("Unable to bind backend");

        let damage =
            desktop::space::render_output::<_, WaylandSurfaceRenderElement<GlesRenderer>, _, _>(
                &app.backend.output,
                app.backend.winit.renderer(),
                1.0,
                0,
                [&app.common.space],
                &[],
                &mut app.backend.damage_tracker,
                [1.0, 0.0, 1.0, 1.0],
            )
            .expect("Error rendering output")
            .damage
            .map(|d| d.as_slice());

        app.backend.winit.submit(damage).unwrap();

        app.common.space.elements().for_each(|window| {
            // TODO this *should* only be run for visible surfaces
            window.send_frame(
                &app.backend.output,
                app.common.start_time.elapsed(),
                Some(std::time::Duration::ZERO),
                |_, _| Some(app.backend.output.clone()),
            )
        });

        // Ask for redraw to schedule new frame.
        app.backend.winit.window().request_redraw();
    }
}
