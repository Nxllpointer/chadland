use iced_widget::*;

#[derive(Debug, Default)]
pub struct Shell {}

#[derive(Debug, Clone)]
pub enum Message {}

impl<B: crate::Backend> crate::iced::Program<B> for Shell {
    type Message = Message;

    fn view(&self) -> impl Into<crate::iced::Element<'_, Self::Message>> {
        button(text!("Hello world!"))
    }

    fn update(
        &mut self,
        comp: &mut crate::state::Compositor<B>,
        message: Self::Message,
    ) -> impl Into<iced_runtime::Task<Self::Message>> {
        
    }

}
