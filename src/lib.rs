use smithay::reexports::*;

//

pub mod backends;
pub mod handlers;
pub mod socket;
pub mod state;

pub use backends::Backend;
pub use state::*;

pub struct LoopData<B: crate::Backend> {
    pub app: crate::App<B>,
    pub display: wayland_server::Display<crate::App<B>>,
}

pub fn run<B: crate::Backend<SelfType = B>>() {
    let mut event_loop: calloop::EventLoop<crate::LoopData<B>> =
        calloop::EventLoop::try_new().expect("Unable to create event loop");
    let display: wayland_server::Display<crate::App<B>> =
        wayland_server::Display::new().expect("Unable to create wayland display");
    let backend = B::new();

    let app = crate::App::new(display.handle(), event_loop.get_signal(), backend);
    let mut loop_data = crate::LoopData { app, display };

    B::init(&event_loop.handle(), &mut loop_data.app);

    crate::socket::init_socket(&event_loop.handle(), &mut loop_data.display);

    event_loop
        .run(None, &mut loop_data, |data| {
            data.app.space.refresh();
            data.app
                .display_handle
                .flush_clients()
                .expect("Unable to flush clients");
        })
        .expect("Error while running event loop");
}
