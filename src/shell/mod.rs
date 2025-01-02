use iced_core::{alignment::Vertical, Element, Length};
use iced_widget::{button, column, horizontal_space, row, text, vertical_space};
use std::marker::PhantomData;

mod window;

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Default)]
pub struct Shell<B: crate::Backend>(PhantomData<B>);
impl<B: crate::Backend> crate::iced::Program for Shell<B> {
    type Data = crate::state::Compositor<B>;
    type Message = Message;

    fn view(data: &Self::Data) -> impl Into<crate::iced::Element<'_, Self::Message>> {
        Element::new(
            column![
                row(data.space.elements().map(|window| {
                    iced_widget::column![
                        text!("Top"),
                        window::Window(window.clone()),
                        text!("Bottom")
                    ]
                    .into()
                })),
                vertical_space(),
                iced_widget::row![
                    button(text!("Active windows: {}", data.space.elements().len())),
                    horizontal_space(),
                    text!(
                        "Running for {} seconds",
                        data.start_time.elapsed().as_secs()
                    )
                ]
                .width(Length::Fill)
                .align_y(Vertical::Center)
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .explain(iced_core::color!(0xFF0000))
    }

    fn update(
        _data: &mut Self::Data,
        _message: Self::Message,
    ) -> impl Into<iced_runtime::Task<Self::Message>> {
    }
}
