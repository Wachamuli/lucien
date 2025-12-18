use gio::{AppInfo, AppLaunchContext, prelude::AppInfoExt};
use iced::{
    Element, Length, Task,
    widget::{
        Column, Container, Scrollable, Text, button, column, row,
        scrollable::{self, Rail},
        text,
    },
};
// use iced_layershell::to_layer_message;

use crate::app::{App, all_apps};

#[derive(Debug, Default)]
pub struct Launcher {
    input: String,
    apps: Vec<App>,
}

// #[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    InputChange(String),
    Open(usize),
}

impl Launcher {
    pub fn init() -> (Self, Task<Message>) {
        let launcher = Self {
            input: String::new(),
            apps: all_apps(),
        };
        (launcher, Task::none())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Open(index) => {
                self.apps[index].launch();
                iced::window::get_latest().and_then(iced::window::close)
            }
            Message::InputChange(input) => {
                self.input = input;
                iced::Task::none()
            }
        }
    }

    pub fn view<'a>(&'a self) -> Column<'a, Message> {
        let app_items: Vec<Element<Message>> = self
            .apps
            .iter()
            .enumerate()
            .map(|(index, app)| {
                let file_ext = app
                    .icon
                    .as_ref()
                    .and_then(|path| path.extension())
                    .and_then(|ext| ext.to_str())
                    .unwrap_or_default();

                let icon_view: Element<Message> = match file_ext {
                    "svg" => iced::widget::svg(iced::widget::svg::Handle::from_path(
                        app.icon.clone().unwrap_or_default(),
                    ))
                    .width(64)
                    .height(64)
                    .into(),
                    _ => iced::widget::image(iced::widget::image::Handle::from_path(
                        app.icon.clone().unwrap_or_default(),
                    ))
                    .width(64)
                    .height(64)
                    .into(),
                };

                button(iced::widget::column![
                    icon_view,
                    text(app.name.clone()),
                    text(app.description.clone())
                        .width(Length::Fill)
                        .wrapping(text::Wrapping::Glyph)
                        .line_height(1.0)
                ])
                .on_press(Message::Open(index))
                .style(|_, status| match status {
                    button::Status::Hovered => button::Style {
                        background: Some(iced::Background::Color(iced::Color::from_rgb(
                            0.3, 0.3, 0.3,
                        ))),
                        text_color: iced::Color::WHITE,
                        border: iced::border::rounded(20),
                        shadow: Default::default(),
                    },
                    _ => button::Style {
                        background: Some(iced::Background::Color(iced::color!(0, 0, 0))),
                        text_color: iced::Color::WHITE,
                        border: iced::border::rounded(20),
                        shadow: Default::default(),
                    },
                })
                .width(Length::Fill)
                .into()
            })
            .collect();

        iced::widget::column![
            iced::widget::text_input("Type ", &self.input).on_input(Message::InputChange),
            iced::widget::scrollable(
                Column::with_children(app_items)
                    .spacing(10)
                    .width(Length::Fill),
            )
            .style(|_, _| scrollable::Style {
                container: iced::widget::container::Style {
                    background: Some(iced::Background::Color(iced::Color::BLACK)),
                    ..Default::default()
                },
                vertical_rail: Rail {
                    background: Some(iced::Background::Color(iced::Color::BLACK)),
                    scroller: scrollable::Scroller {
                        color: iced::Color::WHITE,
                        border: iced::Border {
                            color: iced::Color::WHITE,
                            width: 20.0,
                            radius: iced::border::Radius::new(20.0),
                        },
                    },
                    border: iced::Border {
                        color: iced::Color::WHITE,
                        width: 0.0,
                        radius: iced::border::Radius::new(20.0),
                    },
                },
                horizontal_rail: Rail {
                    background: Some(iced::Background::Color(iced::Color::BLACK)),
                    scroller: scrollable::Scroller {
                        color: iced::Color::WHITE,
                        border: iced::Border {
                            color: iced::Color::WHITE,
                            width: 20.0,
                            radius: iced::border::Radius::new(20.0),
                        },
                    },
                    border: iced::Border {
                        color: iced::Color::WHITE,
                        width: 0.0,
                        radius: iced::border::Radius::new(20.0),
                    },
                },
                gap: None,
            })
        ]
        .padding(10)
    }
}
