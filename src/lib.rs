use smithay::reexports::*;

//

pub mod backends;
pub mod handlers;
pub mod socket;
pub mod state;

pub use backends::Backend;
pub use state::App;

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
