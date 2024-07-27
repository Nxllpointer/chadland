use smithay::{reexports::wayland_server::protocol::wl_surface::WlSurface, wayland};

/// Tries finding the [smithay::desktop::Window] that the given root [WlSurface] belongs to
pub fn find_window(
    root_surface: &WlSurface,
    space: &smithay::desktop::Space<smithay::desktop::Window>,
) -> Option<smithay::desktop::Window> {
    space
        .elements()
        .find(|window| window.toplevel().map(|s| s.wl_surface()) == Some(root_surface))
        .cloned()
}

/// Returns the root parent [WlSurface]
pub fn get_root_surface(surface: &WlSurface) -> WlSurface {
    if let Some(parent) = wayland::compositor::get_parent(surface) {
        get_root_surface(&parent)
    } else {
        surface.clone()
    }
}

/// Provides access to the specified surface data while in the closure.
/// Returns [None] if the requested data does not exist.
pub fn with_surface_data<D: 'static, T>(surface: &WlSurface, f: impl FnOnce(&D) -> T) -> Option<T> {
    wayland::compositor::with_states(surface, |states| states.data_map.get::<D>().map(f))
}
