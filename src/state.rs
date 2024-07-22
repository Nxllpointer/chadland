use smithay::{
    desktop, input,
    reexports::{calloop, wayland_server},
    wayland::{self, shm::ShmState},
};

pub struct WaylandState<B: crate::Backend> {
    pub compositor: wayland::compositor::CompositorState,
    pub seat: input::SeatState<App<B>>,
    pub shm: ShmState,
    pub xdg_shell: wayland::shell::xdg::XdgShellState,
}

pub struct App<B: crate::Backend> {
    pub display_handle: wayland_server::DisplayHandle,
    pub loop_signal: calloop::LoopSignal,

    pub backend: B,

    pub wl: WaylandState<B>,

    pub start_time: std::time::Instant,
    pub seat: input::Seat<Self>,
    pub space: desktop::Space<desktop::Window>,
}

impl<B: crate::Backend> App<B> {
    pub fn new(
        display_handle: wayland_server::DisplayHandle,
        loop_signal: calloop::LoopSignal,
        backend: B,
    ) -> Self {
        let mut wl = WaylandState {
            compositor: wayland::compositor::CompositorState::new::<Self>(&display_handle),
            seat: input::SeatState::new(),
            shm: ShmState::new::<Self>(&display_handle, []),
            xdg_shell: wayland::shell::xdg::XdgShellState::new::<Self>(&display_handle),
        };

        Self {
            loop_signal,
            backend,
            start_time: std::time::Instant::now(),
            seat: wl.seat.new_wl_seat(&display_handle, "default"),
            space: desktop::Space::default(),
            display_handle,
            wl,
        }
    }
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_client_state: wayland::compositor::CompositorClientState,
}

impl wayland_server::backend::ClientData for ClientState {}
