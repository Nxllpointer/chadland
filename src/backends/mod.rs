use smithay::reexports::calloop;

pub mod winit;

/// Trait for handling input and output
// Instead of adding the 'static requirement everywhere like anvil does we require it on the trait level to reduce boilerplate. Seems to work just fine
pub trait Backend: 'static {
    /// The struct implementing this trait. Required for [crate::run<Backend>] to work
    type SelfType: Backend;

    fn new() -> Self;

    fn init(
        loop_handle: &calloop::LoopHandle<crate::LoopData<Self::SelfType>>,
        app: &mut crate::App<Self::SelfType>,
    );
}
