use smithay::{backend::allocator, wayland::dmabuf::DmabufFeedback};

pub mod winit;

/// Trait for handling input and output
// Instead of adding the 'static requirement everywhere like anvil does we require it on the trait level to reduce boilerplate. Seems to work just fine
pub trait Backend: 'static {
    /// The struct implementing this trait. Required for [crate::run<Backend>] to work
    type SelfType: Backend;

    fn new(common: &mut crate::state::Common<Self::SelfType>) -> Self;

    fn default_dmabuf_feedback(&mut self) -> Option<DmabufFeedback>;
    fn dmabuf_formats(&mut self) -> allocator::format::FormatSet;
    fn import_dmabuf(&mut self, dmabuf: &allocator::dmabuf::Dmabuf) -> Result<(), ()>;
}
