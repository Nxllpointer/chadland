use std::ops::Not;

use smithay::{
    desktop,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::shell::xdg::{XdgShellHandler, XdgToplevelSurfaceData},
};
use tracing::error;

impl<B: crate::Backend> XdgShellHandler for crate::App<B> {
    fn xdg_shell_state(&mut self) -> &mut smithay::wayland::shell::xdg::XdgShellState {
        &mut self.common.comp.wl.xdg_shell
    }

    fn new_toplevel(&mut self, surface: smithay::wayland::shell::xdg::ToplevelSurface) {
        let wl_surface = surface.wl_surface().clone();
        let window = desktop::Window::new_wayland_window(surface);
        self.common.comp.space.map_element(window, (0, 0), true);
        self.set_focus(wl_surface);
    }

    fn new_popup(
        &mut self,
        surface: smithay::wayland::shell::xdg::PopupSurface,
        positioner: smithay::wayland::shell::xdg::PositionerState,
    ) {
        todo!()
    }

    fn grab(
        &mut self,
        surface: smithay::wayland::shell::xdg::PopupSurface,
        seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        serial: smithay::utils::Serial,
    ) {
        todo!()
    }

    fn reposition_request(
        &mut self,
        surface: smithay::wayland::shell::xdg::PopupSurface,
        positioner: smithay::wayland::shell::xdg::PositionerState,
        token: u32,
    ) {
        todo!()
    }
}

/// Needs to be called on [CompositorHandler::commit] as a wl_surface can also be a xdg_surface
pub fn handle_commit<B: crate::Backend>(app: &mut crate::App<B>, surface: &WlSurface) {
    // Sync subsurfaces have their changes cached
    // until the parent's state is committed
    if !smithay::wayland::compositor::is_sync_subsurface(surface) {
        let root_surface = crate::util::surface::get_root_surface(surface);

        if let Some(window) =
            crate::util::surface::find_window(&root_surface, &app.common.comp.space)
        {
            window.on_commit();
        }
    }

    if let Some(window) = crate::util::surface::find_window(surface, &app.common.comp.space) {
        if let Some(top_level) = window.toplevel() {
            let initial_configure_pending: Option<bool> = crate::util::surface::with_surface_data(
                surface,
                |mutex: &XdgToplevelSurfaceData| mutex.lock().unwrap().initial_configure_sent.not(),
            );

            if let Some(true) = initial_configure_pending {
                // Send the initial configure on the initial
                // commit as required by the specification
                top_level.send_configure();
            } else if initial_configure_pending.is_none() {
                error!("Unable to obtain XdgToplevelSurfaceData from top-level surface");
            }
        }
    }
}
