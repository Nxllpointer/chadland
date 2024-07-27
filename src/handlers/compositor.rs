use smithay::wayland::compositor::CompositorHandler;
use smithay::{reexports::wayland_server::protocol::wl_surface::WlSurface, wayland};

impl<B: crate::Backend> CompositorHandler for crate::App<B> {
    fn compositor_state(&mut self) -> &mut smithay::wayland::compositor::CompositorState {
        &mut self.common.wl.compositor
    }

    fn client_compositor_state<'a>(
        &self,
        client: &'a smithay::reexports::wayland_server::Client,
    ) -> &'a smithay::wayland::compositor::CompositorClientState {
        &client
            .get_data::<crate::state::ClientState>()
            .expect("Client has no ClientState")
            .compositor_client_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        smithay::backend::renderer::utils::on_commit_buffer_handler::<crate::App<B>>(surface);

        super::xdg_shell::handle_commit(self, surface);
    }
}
