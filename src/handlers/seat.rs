use smithay::input;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;

impl<B: crate::Backend> input::SeatHandler for crate::App<B> {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut input::SeatState<Self> {
        &mut self.common.comp.wl.seat
    }
}
