use smithay::{
    desktop, input,
    reexports::{calloop, wayland_server},
    wayland,
};

pub struct App<B: crate::Backend> {
    pub common: Common<B>,
    pub backend: B,
}

pub struct Common<B: crate::Backend> {
    pub comp: Compositor<B>,
    pub iced: crate::iced::State<B, crate::shell::Shell>,
}

pub struct Compositor<B: crate::Backend> {
    pub display_handle: wayland_server::DisplayHandle,
    pub loop_handle: calloop::LoopHandle<'static, crate::App<B>>,
    pub loop_signal: calloop::LoopSignal,

    pub wl: WaylandState<B>,

    pub start_time: std::time::Instant,
    pub seat: input::Seat<App<B>>,
    pub space: desktop::Space<desktop::Window>,
}

pub struct WaylandState<B: crate::Backend> {
    pub compositor: wayland::compositor::CompositorState,
    pub seat: input::SeatState<App<B>>,
    pub shm: wayland::shm::ShmState,
    pub xdg_shell: wayland::shell::xdg::XdgShellState,
    pub dmabuf: wayland::dmabuf::DmabufState,
}

impl<B: crate::Backend> Compositor<B> {
    pub fn new(
        display_handle: wayland_server::DisplayHandle,
        loop_handle: calloop::LoopHandle<'static, App<B>>,
        loop_signal: calloop::LoopSignal,
    ) -> Self {
        let mut wl = WaylandState {
            compositor: wayland::compositor::CompositorState::new::<App<B>>(&display_handle),
            seat: input::SeatState::new(),
            shm: wayland::shm::ShmState::new::<App<B>>(&display_handle, []),
            xdg_shell: wayland::shell::xdg::XdgShellState::new::<App<B>>(&display_handle),
            dmabuf: wayland::dmabuf::DmabufState::new(),
        };

        let seat = wl.seat.new_wl_seat(&display_handle, "default");

        Self {
            display_handle,
            loop_handle,
            loop_signal,
            wl,
            start_time: std::time::Instant::now(),
            seat,
            space: desktop::Space::default(),
        }
    }
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_client_state: wayland::compositor::CompositorClientState,
}

impl wayland_server::backend::ClientData for ClientState {}
