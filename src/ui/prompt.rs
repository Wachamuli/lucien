use iced::{
    Alignment, Element,
    widget::{Container, Id, container, image, row, space, text_input},
};

use crate::preferences::theme::CustomTheme;

pub struct Prompt<'a, Message> {
    prompt: &'a str,
    magnifier: Option<image::Handle>,
    style: &'a CustomTheme,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_submit: Option<Message>,
    indicator: Option<Container<'a, Message, CustomTheme>>,
    id: Option<Id>,
}

impl<'a, Message> Prompt<'a, Message>
where
    Message: Clone + 'a,
{
    pub fn new(prompt: &'a str, style: &'a CustomTheme) -> Self {
        Self {
            prompt,
            style,
            magnifier: None,
            on_input: None,
            on_submit: None,
            indicator: None,
            id: None,
        }
    }

    pub fn id(mut self, id: impl Into<Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn on_input(mut self, message: impl Fn(String) -> Message + 'a) -> Self {
        self.on_input = Some(Box::new(message));
        self
    }

    pub fn on_submit(mut self, message: Message) -> Self {
        self.on_submit = Some(message);
        self
    }

    pub fn magnifier(mut self, image: image::Handle) -> Self {
        self.magnifier = Some(image);
        self
    }

    pub fn indicator(mut self, content: Container<'a, Message, CustomTheme>) -> Self {
        self.indicator = Some(content);
        self
    }

    pub fn view(self) -> Container<'a, Message, CustomTheme> {
        let magnifier: Element<Message, CustomTheme> = match self.magnifier {
            Some(handle) => image(handle)
                .width(iced::Length::Fixed(self.style.prompt.icon_size as f32))
                .height(iced::Length::Fixed(self.style.prompt.icon_size as f32))
                .into(),
            None => space::horizontal()
                .width(iced::Length::Fixed(self.style.prompt.icon_size as f32))
                .height(iced::Length::Fixed(self.style.prompt.icon_size as f32))
                .into(),
        };

        let mut input = text_input("Search...", self.prompt)
            .padding(8)
            .size(self.style.prompt.font_size as u32)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            });

        if let Some(id) = self.id {
            input = input.id(id);
        }

        if let Some(on_input) = self.on_input {
            input = input.on_input(on_input);
        }

        if let Some(on_submit) = self.on_submit {
            input = input.on_submit(on_submit);
        }

        let prompt: Element<Message, CustomTheme> = row![]
            .push(magnifier)
            .push(input)
            .extend(self.indicator.map(Element::from))
            .align_y(iced::Alignment::Center)
            .spacing(2)
            .into();

        container(prompt)
            .padding(iced::Padding::from(&self.style.prompt.margin))
            .align_y(Alignment::Center)
    }
}
