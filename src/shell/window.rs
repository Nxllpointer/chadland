use iced_core::{Element, Widget};

pub struct Window(pub smithay::desktop::Window);

impl Window {
    fn width(&self) -> f32 {
        self.0.bbox().size.w as f32
    }
    fn height(&self) -> f32 {
        self.0.bbox().size.h as f32
    }
}

impl<Message, Theme, Renderer: iced_core::Renderer> Widget<Message, Theme, Renderer> for Window {
    fn size(&self) -> iced_core::Size<iced_core::Length> {
        iced_core::Size::new(self.width().into(), self.height().into())
    }

    fn layout(
        &self,
        _tree: &mut iced_core::widget::Tree,
        _renderer: &Renderer,
        _limits: &iced_core::layout::Limits,
    ) -> iced_core::layout::Node {
        iced_core::layout::Node::new((self.width(), self.height()).into())
    }

    fn draw(
        &self,
        _tree: &iced_core::widget::Tree,
        _renderer: &mut Renderer,
        _theme: &Theme,
        _style: &iced_core::renderer::Style,
        _layout: iced_core::Layout<'_>,
        _cursor: iced_core::mouse::Cursor,
        _viewport: &iced_core::Rectangle,
    ) {
    }
}

impl<Message, Theme, Renderer: iced_core::Renderer> From<Window>
    for Element<'_, Message, Theme, Renderer>
{
    fn from(value: Window) -> Self {
        Self::new(value)
    }
}
