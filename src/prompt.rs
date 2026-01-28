use iced::{
    Alignment, Element,
    widget::{Container, container, horizontal_space, image, row, text_input},
};

use crate::preferences::theme::CustomTheme;

pub struct Prompt<'a, Message> {
    prompt: &'a str,
    magnifier: Option<&'a image::Handle>,
    style: &'a CustomTheme,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_submit: Option<Message>,
    id: Option<text_input::Id>,
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
            id: None,
        }
    }

    pub fn id(mut self, id: impl Into<text_input::Id>) -> Self {
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

    pub fn magnifier(mut self, image: Option<&'a image::Handle>) -> Self {
        self.magnifier = image;
        self
    }

    pub fn view(self) -> Container<'a, Message, CustomTheme> {
        let magnifier: Element<Message, CustomTheme> = match self.magnifier {
            Some(handle) => image(handle)
                .width(self.style.prompt.icon_size)
                .height(self.style.prompt.icon_size)
                .into(),
            None => horizontal_space()
                .width(self.style.prompt.icon_size)
                .height(self.style.prompt.icon_size)
                .into(),
        };

        let mut input = text_input("Search...", self.prompt)
            .padding(8)
            .size(self.style.prompt.font_size)
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
            .align_y(iced::Alignment::Center)
            .spacing(2)
            .into();

        container(prompt)
            .padding(iced::Padding::from(&self.style.prompt.margin))
            .align_y(Alignment::Center)
    }
}
