use calloop::EventLoop;
use smithay::reexports::*;

//

mod handlers;
mod socket;
mod state;
mod winit;

pub use state::*;

pub struct Options {}

pub struct LoopData {
    state: crate::State,
    display: wayland_server::Display<crate::State>,
}

pub fn run(_options: Options) -> anyhow::Result<()> {
    let mut event_loop: EventLoop<LoopData> = calloop::EventLoop::try_new()?;
    let loop_handle = event_loop.handle();
    let mut display: wayland_server::Display<crate::State> = wayland_server::Display::new()?;
    let display_handle = display.handle();

    let mut state = crate::State::new(&display_handle, event_loop.get_signal());

    crate::socket::init_socket(&loop_handle, &mut display)?;
    crate::winit::init_winit(&loop_handle, &display_handle, &mut state)?;

    let mut loop_data = LoopData { state, display };
    event_loop.run(None, &mut loop_data, |_| {})?;

    Ok(())
}
