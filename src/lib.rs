use smithay::reexports::*;

//

pub mod backends;
pub mod handlers;
pub mod socket;
pub mod state;

pub use backends::Backend;
pub use state::App;
use tracing::info;

/// Run the compositor using the specified [crate::Backend]
pub fn run<B: crate::Backend<SelfType = B>>() {
    let mut event_loop: calloop::EventLoop<crate::App<B>> =
        calloop::EventLoop::try_new().expect("Unable to create event loop");
    let display: wayland_server::Display<crate::App<B>> =
        wayland_server::Display::new().expect("Unable to create wayland display");

    let mut common = crate::state::Common::new(
        display.handle(),
        event_loop.handle(),
        event_loop.get_signal(),
    );
    let backend = B::new(&mut common);

    let mut app = crate::App { common, backend };

    crate::socket::init_socket(&mut app, display);
    init_dmabuf(&mut app);

    event_loop
        .run(None, &mut app, |app| {
            app.common.space.refresh();
            app.common
                .display_handle
                .flush_clients()
                .expect("Unable to flush clients");
        })
        .expect("Error while running event loop");
}

fn init_dmabuf<B: crate::Backend>(app: &mut crate::App<B>) {
    if let Some(default_feedback) = app.backend.default_dmabuf_feedback() {
        app.common
            .wl
            .dmabuf
            .create_global_with_default_feedback::<crate::App<B>>(
                &app.common.display_handle,
                &default_feedback,
            );
        info!("Using DMA-Buf version >=4 with default feedback");
    } else {
        app.common.wl.dmabuf.create_global::<crate::App<B>>(
            &app.common.display_handle,
            app.backend.dmabuf_formats(),
        );
        info!("Using DMA-Buf version <=3");
    };
}
