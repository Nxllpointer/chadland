use smithay::*;

mod compositor;
mod output;
mod seat;
mod shm;
mod xdg_shell;

/// Run `delegate_xxxx!` with the arguments being provided automatically
macro_rules! chadland_delegate {
    ($($macro_name:ident)*) => {
        $(
            paste::paste! {
               [<delegate_ $macro_name>]!(crate::State);
            }
        )*
    };
}
chadland_delegate!(
    compositor
    shm
    output
    seat
    xdg_shell
);
