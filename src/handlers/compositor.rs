use smithay::desktop::Space;
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
        if !wayland::compositor::is_sync_subsurface(surface) {
            let root_surface = get_root_surface(surface);
            if let Some(window) = find_window(&root_surface, &self.common.space) {
                window.on_commit();
                window.toplevel().unwrap().send_pending_configure(); //TODO Is it ok to send this every commit?
            };
        };
    }
}

/// Returns the root parent [WlSurface]
fn get_root_surface(surface: &WlSurface) -> WlSurface {
    if let Some(parent) = wayland::compositor::get_parent(surface) {
        get_root_surface(&parent)
    } else {
        surface.clone()
    }
}

/// Tries finding the [smithay::desktop::Window] that the given [WlSurface] belongs to
fn find_window(
    surface: &WlSurface,
    space: &Space<smithay::desktop::Window>,
) -> Option<smithay::desktop::Window> {
    space
        .elements()
        .find(|window| window.toplevel().map(|s| s.wl_surface()) == Some(surface))
        .cloned()
}
