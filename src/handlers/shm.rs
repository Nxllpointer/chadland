use smithay::wayland;

impl<B: crate::Backend> wayland::shm::ShmHandler for crate::App<B> {
    fn shm_state(&self) -> &wayland::shm::ShmState {
        &self.wl.shm
    }
}

impl<B: crate::Backend> wayland::buffer::BufferHandler for crate::App<B> {
    fn buffer_destroyed(
        &mut self,
        _buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer,
    ) {
    }
}
