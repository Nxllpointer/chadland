use smithay::{
    desktop, input,
    reexports::{calloop, wayland_server},
    wayland::{self, shm::ShmState},
};

pub struct WaylandState {
    pub compositor: wayland::compositor::CompositorState,
    pub seat: input::SeatState<State>,
    pub shm: ShmState,
    pub xdg_shell: wayland::shell::xdg::XdgShellState,
}

pub struct State {
    pub loop_signal: calloop::LoopSignal,

    pub start_time: std::time::Instant,

    pub wl: WaylandState,

    pub seat: input::Seat<Self>,
    pub space: desktop::Space<desktop::Window>,
}

impl State {
    pub fn new(
        display_handle: &wayland_server::DisplayHandle,
        loop_signal: calloop::LoopSignal,
    ) -> Self {
        let mut wl = WaylandState {
            compositor: wayland::compositor::CompositorState::new::<Self>(display_handle),
            seat: input::SeatState::new(),
            shm: ShmState::new::<Self>(display_handle, []),
            xdg_shell: wayland::shell::xdg::XdgShellState::new::<Self>(display_handle),
        };

        Self {
            loop_signal,
            start_time: std::time::Instant::now(),
            seat: wl.seat.new_wl_seat(display_handle, "default"),
            space: desktop::Space::default(),
            wl,
        }
    }
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_client_state: wayland::compositor::CompositorClientState,
}

impl wayland_server::backend::ClientData for ClientState {}
