use smithay::utils::SERIAL_COUNTER;
use smithay_input::{Event as _, KeyboardKeyEvent as _};

mod smithay_input {
    pub use smithay::{
        backend::input::{Event, InputBackend, InputEvent, KeyboardKeyEvent},
        input::{keyboard::FilterResult, SeatHandler},
    };
}

/// Wrapper for extending [smithay_input::InputEvent]
pub enum InputEvent<IB: smithay_input::InputBackend> {
    /// Regular predefined event
    Basic(smithay_input::InputEvent<IB>),
    /// Additional custom event
    Extra(ExtraInputEvent),
}

pub enum ExtraInputEvent {}

impl<B: crate::Backend> crate::App<B> {
    pub fn process_input<IB: smithay_input::InputBackend>(&mut self, event: InputEvent<IB>) {
        match event {
            InputEvent::Basic(b_event) => match b_event {
                smithay_input::InputEvent::Keyboard { event: k_event } => {
                    if let Some(keyboard) = self.common.comp.seat.get_keyboard() {
                        keyboard.input(
                            self,
                            k_event.key_code(),
                            k_event.state(),
                            SERIAL_COUNTER.next_serial(),
                            k_event.time_msec(),
                            |_, _, _| smithay_input::FilterResult::Forward::<()>,
                        );
                    }
                }
                _ => {}
            },
            InputEvent::Extra(event) => match event {},
        }
    }

    pub fn set_focus(&mut self, focus: <Self as smithay_input::SeatHandler>::KeyboardFocus) {
        if let Some(keyboard) = self.common.comp.seat.get_keyboard() {
            keyboard.set_focus(self, Some(focus), SERIAL_COUNTER.next_serial());
        }
    }
}
