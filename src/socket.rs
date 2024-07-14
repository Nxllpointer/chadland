use std::sync::Arc;

use smithay::{
    reexports::{
        calloop::{self, generic::Generic},
        wayland_server,
    },
    wayland::socket::ListeningSocketSource,
};
use tracing::error;

pub fn init_socket(
    loop_handle: &calloop::LoopHandle<crate::LoopData>,
    display: &mut wayland_server::Display<crate::State>,
) -> anyhow::Result<()> {
    let listening_socket = ListeningSocketSource::new_auto()?;

    loop_handle.insert_source(listening_socket, |client, _, data| {
        let client_dbg = format!("{client:?}");

        if let Err(err) = data
            .display
            .handle()
            .insert_client(client, Arc::new(crate::ClientState::default()))
        {
            error!("Unable to insert client into display!");
            error!("Client: {client_dbg}");
            error!("Error: {err}");
        }
    })?;

    let poll_source = Generic::new(
        display
            .backend()
            .poll_fd()
            .try_clone_to_owned()
            .expect("Unable to duplicate display poll-fd"),
        calloop::Interest::READ,
        calloop::Mode::Level,
    );

    loop_handle.insert_source(poll_source, |_, _, data| {
        data.display.dispatch_clients(&mut data.state)?;
        Ok(calloop::PostAction::Continue)
    })?;

    Ok(())
}
