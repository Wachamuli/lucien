use std::sync::LazyLock;

use gio::{AppInfo, AppLaunchContext, prelude::AppInfoExt};
use iced::{
    Element, Event, Font, Length, Pixels, Subscription, Task, event,
    keyboard::{self, Key},
    widget::{
        Column, Container, Scrollable, Text, button, column, container, row,
        scrollable::{self, Rail},
        text, text_input,
    },
    window,
};
use iced_layershell::to_layer_message;

use crate::app::{App, all_apps};

#[derive(Debug, Default)]
pub struct Launcher {
    input: String,
    apps: Vec<App>,
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    InputChange(String),
    OpenApp(usize),
    SystemEventOccurred(Event),
}

static TEXT_INPUT_ID: LazyLock<text_input::Id> =
    std::sync::LazyLock::new(|| text_input::Id::unique());

impl Launcher {
    pub fn init() -> (Self, Task<Message>) {
        let launcher = Self {
            input: String::new(),
            apps: all_apps(),
        };
        (launcher, text_input::focus(TEXT_INPUT_ID.clone()))
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenApp(index) => {
                self.apps[index].launch();
                std::process::exit(0)
            }
            Message::InputChange(input) => {
                self.input = input;
                iced::Task::none()
            }
            Message::SystemEventOccurred(Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(keyboard::key::Named::Escape),
                ..
            })) => {
                std::process::exit(0);
            }
            Message::SystemEventOccurred(_) => Task::none(),
            Message::AnchorChange(anchor) => todo!(),
            Message::SetInputRegion(action_callback) => todo!(),
            Message::AnchorSizeChange(anchor, _) => todo!(),
            Message::LayerChange(layer) => todo!(),
            Message::MarginChange(_) => todo!(),
            Message::SizeChange(_) => todo!(),
            Message::VirtualKeyboardPressed { time, key } => todo!(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        event::listen().map(Message::SystemEventOccurred)
    }

    pub fn view<'a>(&'a self) -> Column<'a, Message> {
        let app_items: Vec<Element<Message>> = self
            .apps
            .iter()
            .filter(|app| {
                regex::RegexBuilder::new(&self.input)
                    .case_insensitive(true)
                    .ignore_whitespace(true)
                    .build()
                    .unwrap()
                    .is_match(&app.name)
            })
            .enumerate()
            .map(|(index, app)| {
                // let file_ext = app
                //     .icon
                //     .as_ref()
                //     .and_then(|path| path.extension())
                //     .and_then(|ext| ext.to_str())
                //     .unwrap_or_default();

                // let icon_view: Element<Message> = match file_ext {
                //     "svg" => iced::widget::svg(iced::widget::svg::Handle::from_path(
                //         app.icon.clone().unwrap_or_default(),
                //     ))
                //     .width(32)
                //     .height(32)
                //     .into(),
                //     _ => iced::widget::image(iced::widget::image::Handle::from_path(
                //         app.icon.clone().unwrap_or_default(),
                //     ))
                //     .width(32)
                //     .height(32)
                //     .into(),
                // };

                button(
                    row![
                        // icon_view,
                        iced::widget::column![
                            text(&app.name),
                            text(&app.description)
                                .width(Length::Fill)
                                .size(12)
                                .wrapping(text::Wrapping::Glyph)
                                .line_height(1.0)
                        ],
                    ]
                    .spacing(10),
                )
                .on_press(Message::OpenApp(index))
                .padding(10)
                .style(|_, status| match status {
                    button::Status::Hovered => button::Style {
                        background: Some(iced::Background::Color(iced::Color::from_rgb(
                            0.3, 0.3, 0.3,
                        ))),
                        text_color: iced::Color::WHITE,
                        border: iced::border::rounded(10),
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
            container(
                iced::widget::text_input("Type to search...", &self.input)
                    .id(TEXT_INPUT_ID.clone())
                    .on_input(Message::InputChange)
                    .on_submit(Message::OpenApp(0))
                    .padding(10)
                    .style(|_, _| {
                        iced::widget::text_input::Style {
                            background: iced::Background::Color(iced::Color::BLACK),
                            border: iced::border::Border {
                                radius: iced::border::radius(10),
                                ..Default::default()
                            },
                            icon: iced::Color::WHITE,
                            placeholder: iced::Color::WHITE,
                            value: iced::Color::WHITE,
                            selection: iced::Color::WHITE,
                        }
                    })
            )
            .style(|_| container::Style {
                background: Some(iced::Background::Color(iced::Color::BLACK)),
                border: iced::border::Border {
                    radius: iced::border::top(20),
                    ..Default::default()
                },
                ..Default::default()
            })
            .padding(10),
            iced::widget::scrollable(
                Column::with_children(app_items)
                    .padding(10)
                    .width(Length::Fill),
            )
            .style(|_, _| scrollable::Style {
                container: iced::widget::container::Style {
                    background: Some(iced::Background::Color(iced::Color::BLACK)),
                    border: iced::Border {
                        radius: iced::border::bottom(20),
                        ..Default::default()
                    },
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
                        radius: iced::border::bottom(20.0),
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
                        radius: iced::border::bottom(20.0),
                    },
                },
                gap: None,
            }),
        ]
    }
}
