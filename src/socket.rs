use std::sync::Arc;

use smithay::{
    reexports::{
        calloop::{self, generic::Generic},
        wayland_server,
    },
    wayland::socket::ListeningSocketSource,
};
use tracing::error;

/// Create and initialize the wayland socket
pub fn init_socket<B: crate::Backend>(
    app: &mut crate::App<B>,
    mut display: wayland_server::Display<crate::App<B>>,
) {
    let listening_socket =
        ListeningSocketSource::new_auto().expect("Unable to create wayland socket");

    app.common
        .loop_handle
        .insert_source(listening_socket, |client, _, app| {
            let client_dbg = format!("{client:?}");

            if let Err(err) = app
                .common
                .display_handle
                .insert_client(client, Arc::new(crate::state::ClientState::default()))
            {
                error!("Unable to insert client into display!");
                error!("Client: {client_dbg}");
                error!("Error: {err}");
            }
        })
        .expect("Unable to insert wayland listening socket source");

    let poll_source = Generic::new(
        display
            .backend()
            .poll_fd()
            .try_clone_to_owned()
            .expect("Unable to duplicate display poll-fd"),
        calloop::Interest::READ,
        calloop::Mode::Level,
    );

    app.common
        .loop_handle
        .insert_source(poll_source, move |_, _, app| {
            display.dispatch_clients(app)?;
            Ok(calloop::PostAction::Continue)
        })
        .expect("Unable to insert poll-fd source");
}
