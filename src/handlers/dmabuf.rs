use smithay::wayland::dmabuf::DmabufHandler;

impl<B: crate::Backend> DmabufHandler for crate::App<B> {
    fn dmabuf_state(&mut self) -> &mut smithay::wayland::dmabuf::DmabufState {
        &mut self.common.wl.dmabuf
    }

    fn dmabuf_imported(
        &mut self,
        _global: &smithay::wayland::dmabuf::DmabufGlobal,
        dmabuf: smithay::backend::allocator::dmabuf::Dmabuf,
        notifier: smithay::wayland::dmabuf::ImportNotifier,
    ) {
        match self.backend.import_dmabuf(&dmabuf) {
            Ok(_) => {
                notifier.successful::<crate::App<B>>().ok();
            }
            Err(_) => {
                notifier.failed();
            }
        };
    }
}
