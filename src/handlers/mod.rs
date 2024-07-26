use smithay::*;

mod compositor;
mod dmabuf;
mod output;
mod seat;
mod shm;
mod xdg_shell;

/// Run `delegate_xxxx!` for each argument disregarding the backend type
macro_rules! delegate_for_all_backends {
    ($($macro_name:ident)*) => {
        use crate::Backend;
        $(
            paste::paste! {
               [<delegate_ $macro_name>]!(@<B: Backend> crate::App<B>);
            }
        )*
    };
}
delegate_for_all_backends!(
    compositor
    dmabuf
    output
    seat
    shm
    xdg_shell
);
