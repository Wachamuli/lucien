use std::sync::LazyLock;

use gio::{AppInfo, AppLaunchContext, prelude::AppInfoExt};
use iced::{
    Border, Element, Event, Font, Length, Pixels, Subscription, Task,
    alignment::Vertical,
    event,
    keyboard::{self, Key},
    widget::{
        self, Column, Container, Scrollable, Text, button, column, container, row,
        scrollable::{self, Anchor, Rail},
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
    scroll_position: usize,
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    InputChange(String),
    OpenApp(usize),
    SystemEvent(iced::Event),
    EscPressed,
    AltDigitShortcut(usize),
}

static TEXT_INPUT_ID: LazyLock<text_input::Id> = std::sync::LazyLock::new(text_input::Id::unique);
static SCROLLABLE_ID: LazyLock<scrollable::Id> = std::sync::LazyLock::new(scrollable::Id::unique);

impl Launcher {
    pub fn init() -> (Self, Task<Message>) {
        let auto_focus_task = text_input::focus(TEXT_INPUT_ID.clone());
        let initial_values = Self {
            input: String::new(),
            apps: all_apps(),
            scroll_position: 0,
        };

        (initial_values, auto_focus_task)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenApp(index) => {
                self.apps[index].launch();
                std::process::exit(0)
            }
            Message::InputChange(input) => {
                let regex_builder = regex::RegexBuilder::new(&input)
                    .case_insensitive(true)
                    .ignore_whitespace(true)
                    .build()
                    .unwrap();

                self.apps = all_apps()
                    .into_iter()
                    .filter(|app| regex_builder.is_match(&app.name))
                    .collect();
                self.input = input;
                self.scroll_position = 0;

                iced::Task::none()
            }
            Message::EscPressed => {
                std::process::exit(0);
            }
            Message::AltDigitShortcut(n) => {
                self.apps[n - 1].launch();
                std::process::exit(0);
            }
            Message::SystemEvent(iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(keyboard::key::Named::Tab),
                modifiers,
                ..
            })) if modifiers.shift() => {
                self.scroll_position = wrapped_index(self.scroll_position, self.apps.len(), -1);
                Task::none()
            }
            Message::SystemEvent(iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(key_pressed),
                ..
            })) => {
                use iced::keyboard::key::Named as kp;

                if let kp::ArrowDown | kp::Tab = key_pressed {
                    self.scroll_position = wrapped_index(self.scroll_position, self.apps.len(), 1);
                }

                if let keyboard::key::Named::ArrowUp = key_pressed {
                    self.scroll_position = wrapped_index(self.scroll_position, self.apps.len(), -1);
                }

                Task::none()
            }
            Message::SystemEvent(iced::Event::Mouse(iced::mouse::Event::ButtonPressed(_))) => {
                std::process::exit(0)
            }
            Message::SystemEvent(_) => Task::none(),
            Message::AnchorChange(_anchor) => todo!(),
            Message::SetInputRegion(_action_callback) => todo!(),
            Message::AnchorSizeChange(_anchor, _) => todo!(),
            Message::LayerChange(_layer) => todo!(),
            Message::MarginChange(_) => todo!(),
            Message::SizeChange(_) => todo!(),
            Message::VirtualKeyboardPressed { .. } => todo!(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            event::listen().map(Message::SystemEvent),
            event::listen_with(|event, status, _id| match (event, status) {
                (
                    Event::Keyboard(keyboard::Event::KeyPressed {
                        key: keyboard::Key::Named(keyboard::key::Named::Escape),
                        ..
                    }),
                    _,
                ) => Some(Message::EscPressed),
                (
                    Event::Keyboard(keyboard::Event::KeyPressed {
                        physical_key: keyboard::key::Physical::Code(physical_key_pressed),
                        modifiers,
                        ..
                    }),
                    _,
                ) if modifiers.alt() => match physical_key_pressed {
                    keyboard::key::Code::Digit1 => Some(Message::AltDigitShortcut(1)),
                    keyboard::key::Code::Digit2 => Some(Message::AltDigitShortcut(2)),
                    keyboard::key::Code::Digit3 => Some(Message::AltDigitShortcut(3)),
                    keyboard::key::Code::Digit4 => Some(Message::AltDigitShortcut(4)),
                    keyboard::key::Code::Digit5 => Some(Message::AltDigitShortcut(5)),
                    _ => None,
                },
                _ => None,
            }),
        ])
    }

    pub fn view<'a>(&'a self) -> Container<'a, Message> {
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
                    .width(32)
                    .height(32)
                    .into(),
                    _ => iced::widget::image(iced::widget::image::Handle::from_path(
                        app.icon.clone().unwrap_or_default(),
                    ))
                    .width(32)
                    .height(32)
                    .into(),
                };

                let shortcut_label = match index {
                    n if self.scroll_position == n => "Enter".to_string(),
                    n @ 0..5 => format!("Alt+{}", n + 1),
                    _ => "".to_string(),
                };

                button(
                    row![
                        icon_view,
                        iced::widget::column![
                            text(&app.name),
                            text(&app.description)
                                .width(Length::Fill)
                                .size(12)
                                .wrapping(text::Wrapping::Glyph)
                                .line_height(1.0)
                        ],
                        text(shortcut_label)
                            .size(12)
                            .color(iced::Color::from_rgba(255., 255., 255., 0.2))
                    ]
                    .align_y(Vertical::Center)
                    .spacing(10),
                )
                .on_press(Message::OpenApp(index))
                .padding(10)
                .style(move |_, status| {
                    if index == self.scroll_position {
                        return button::Style {
                            background: Some(iced::Background::Color(iced::Color::from_rgb(
                                0.3, 0.3, 0.3,
                            ))),
                            text_color: iced::Color::WHITE,
                            border: iced::border::rounded(10),
                            shadow: Default::default(),
                        };
                    }

                    match status {
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
                    }
                })
                .width(Length::Fill)
                .into()
            })
            .collect();

        container(iced::widget::column![
            container(
                iced::widget::text_input("Type to search...", &self.input)
                    .id(TEXT_INPUT_ID.clone())
                    .on_input(Message::InputChange)
                    .on_submit(Message::OpenApp(self.scroll_position))
                    .padding(10)
                    .style(|_, _| {
                        iced::widget::text_input::Style {
                            background: iced::Background::Color(iced::Color::WHITE),
                            border: iced::border::Border {
                                radius: iced::border::radius(10),
                                ..Default::default()
                            },
                            icon: iced::Color::WHITE,
                            placeholder: iced::Color::BLACK,
                            value: iced::Color::BLACK,
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
            .id(SCROLLABLE_ID.clone())
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
                            color: iced::Color::from_rgb(255., 0., 0.),
                            width: 0.0,
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
        ])
        .padding(2)
        .style(|_| container::Style {
            border: Border {
                width: 2.,
                color: iced::Color::WHITE,
                radius: iced::border::radius(20),
            },
            ..Default::default()
        })
    }
}

fn wrapped_index(index: usize, array_len: usize, step: isize) -> usize {
    if step >= 0 {
        return (index + step as usize) % array_len;
    }
    let abs_offset = step.unsigned_abs();
    (index + array_len - (abs_offset % array_len)) % array_len
}
