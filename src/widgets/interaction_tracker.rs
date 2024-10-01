use iced::advanced::graphics::core::event;
use iced::advanced::renderer;
use iced::advanced::widget::Tree;
use iced::advanced::{self, layout, Widget};
use iced::{mouse, touch, Event};
use iced::{Element, Length, Rectangle, Size};

#[derive(Debug)]
pub struct InteractionTracker<Message> {
    on_interaction: Option<Message>,
}

impl<Message> InteractionTracker<Message> {
    pub const fn new() -> Self {
        Self {
            on_interaction: None,
        }
    }

    pub fn on_interaction(mut self, message: Message) -> Self {
        self.on_interaction = Some(message);
        self
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for InteractionTracker<Message>
where
    Renderer: advanced::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, Length::Fill, Length::Fill)
    }

    fn draw(
        &self,
        _state: &Tree,
        _renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        _layout: advanced::Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
    }

    fn on_event(
        &mut self,
        _state: &mut Tree,
        event: iced::Event,
        _layout: layout::Layout<'_>,
        _cursor: iced::advanced::mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(_))
            | Event::Mouse(mouse::Event::ButtonReleased(_))
            | Event::Keyboard(_)
            | Event::Touch(_) => {
                if let Event::Touch(touch::Event::FingerMoved { .. }) = event {
                    return event::Status::Ignored;
                }

                if let Some(message) = self.on_interaction.take() {
                    shell.publish(message);
                }
            }
            _ => {}
        }

        event::Status::Ignored
    }
}

impl<'a, Message, Theme, Renderer> From<InteractionTracker<Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer,
    Message: 'a,
{
    fn from(tracker: InteractionTracker<Message>) -> Self {
        Element::new(tracker)
    }
}
