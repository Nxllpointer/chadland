use smithay::{
    backend::renderer::{
        damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
        gles::GlesRenderer,
    },
    desktop, output,
};

pub struct WinitBackend;

impl super::Backend for WinitBackend {
    type SelfType = WinitBackend;

    fn new(common: &mut crate::state::Common<Self::SelfType>) -> Self {
        let (mut graphics_backend, winit_source) =
            smithay::backend::winit::init::<GlesRenderer>().expect("Unable to initialize winit");

        let output = output::Output::new(
            "winit".to_string(),
            output::PhysicalProperties {
                size: (0, 0).into(),
                subpixel: output::Subpixel::Unknown,
                make: "Chadland".to_string(),
                model: "Super".to_string(),
            },
        );

        let mode = output::Mode {
            size: graphics_backend.window_size(),
            refresh: 60_000,
        };

        output.create_global::<crate::App<WinitBackend>>(&common.display_handle);
        output.change_current_state(
            Some(mode),
            Some(smithay::utils::Transform::Flipped180),
            Some(output::Scale::Integer(1)),
            Some((0, 0).into()),
        );
        output.set_preferred(mode);

        common.space.map_output(&output, (0, 0));

        let mut damage_tracker = OutputDamageTracker::from_output(&output);

        common
            .loop_handle
            .insert_source(winit_source, move |event, _, app| match event {
                smithay::backend::winit::WinitEvent::Resized {
                    size,
                    scale_factor: _,
                } => output.change_current_state(
                    Some(output::Mode {
                        size,
                        refresh: output
                            .preferred_mode()
                            .expect("No preferred output mode")
                            .refresh,
                    }),
                    None,
                    None,
                    None,
                ),
                smithay::backend::winit::WinitEvent::Focus(_) => {}
                smithay::backend::winit::WinitEvent::Input(_) => {}
                smithay::backend::winit::WinitEvent::CloseRequested => {
                    app.common.loop_signal.stop()
                }
                smithay::backend::winit::WinitEvent::Redraw => {
                    graphics_backend.bind().unwrap();

                    let damage = desktop::space::render_output::<
                        _,
                        WaylandSurfaceRenderElement<GlesRenderer>,
                        _,
                        _,
                    >(
                        &output,
                        graphics_backend.renderer(),
                        1.0,
                        0,
                        [&app.common.space],
                        &[],
                        &mut damage_tracker,
                        [1.0, 0.0, 1.0, 1.0],
                    )
                    .expect("Error rendering output")
                    .damage
                    .map(|d| d.as_slice());

                    graphics_backend.submit(damage).unwrap();
                    app.common.space.elements().for_each(|window| {
                        window.send_frame(
                            &output,
                            app.common.start_time.elapsed(),
                            Some(std::time::Duration::ZERO),
                            |_, _| Some(output.clone()),
                        )
                    });

                    // Ask for redraw to schedule new frame.
                    graphics_backend.window().request_redraw();
                }
            })
            .expect("Unable to insert winit event source");

        Self {}
    }
}
