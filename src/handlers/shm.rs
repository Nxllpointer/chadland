use smithay::wayland;
use tracing::warn;

impl wayland::shm::ShmHandler for crate::State {
    fn shm_state(&self) -> &wayland::shm::ShmState {
        &self.wl.shm
    }
}

impl wayland::buffer::BufferHandler for crate::State {
    fn buffer_destroyed(
        &mut self,
        buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer,
    ) {
        warn!("Buffer destroyed. Not implemented");
    }
}
