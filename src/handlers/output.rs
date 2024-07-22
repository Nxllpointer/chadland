use smithay::wayland::output::OutputHandler;

impl<B: crate::Backend> OutputHandler for crate::App<B> {}
