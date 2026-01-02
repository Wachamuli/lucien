use std::sync::LazyLock;

use iced::{
    Alignment, Border, Element, Event, Length, Subscription, Task,
    alignment::Vertical,
    event,
    keyboard::{self, Key},
    widget::{
        Column, Container, button, container, row,
        scrollable::{self, Rail, RelativeOffset},
        text, text_input,
    },
};
use iced_layershell::to_layer_message;

use crate::app::{App, all_apps};

#[derive(Debug, Default)]
pub struct Launcher {
    input: String,
    apps: Vec<App>,
    scroll_position: usize,
    last_viewport: Option<iced::widget::scrollable::Viewport>,
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    InputChange(String),
    OpenApp(usize),
    SystemEvent(iced::Event),
    EscPressed,
    AltDigitShortcut(usize),
    ScrollableViewport(iced::widget::scrollable::Viewport),
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
            last_viewport: None,
        };

        (initial_values, auto_focus_task)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ScrollableViewport(viewport) => {
                self.last_viewport = Some(viewport);
                Task::none()
            }
            Message::OpenApp(index) => {
                if let Some(app) = self.apps.get(index) {
                    app.launch();
                    std::process::exit(0)
                }

                Task::none()
            }
            Message::InputChange(input) => {
                let regex_builder = regex::RegexBuilder::new(&input)
                    .case_insensitive(true)
                    .ignore_whitespace(true)
                    .build();

                if let Ok(regex) = regex_builder {
                    self.apps = all_apps()
                        .into_iter()
                        .filter(|app| regex.is_match(&app.name))
                        .collect();
                }

                self.input = input;
                self.scroll_position = 0;
                iced::Task::none()
            }
            Message::EscPressed => {
                std::process::exit(0);
            }
            Message::AltDigitShortcut(n) => {
                if let Some(app) = self.apps.get(n - 1) {
                    app.launch();
                    std::process::exit(0);
                }

                Task::none()
            }
            Message::SystemEvent(iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(keyboard::key::Named::Tab),
                modifiers,
                ..
            })) if modifiers.shift() => {
                let old_pos = self.scroll_position;

                self.scroll_position = wrapped_index(self.scroll_position, self.apps.len(), -1);

                if old_pos != self.scroll_position {
                    return self.snap_if_needed();
                }

                Task::none()
            }
            Message::SystemEvent(iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(key_pressed),
                ..
            })) => {
                use iced::keyboard::key::Named as kp;
                let old_pos = self.scroll_position;

                if let kp::ArrowDown | kp::Tab = key_pressed {
                    self.scroll_position = wrapped_index(self.scroll_position, self.apps.len(), 1);
                }

                if let keyboard::key::Named::ArrowUp = key_pressed {
                    self.scroll_position = wrapped_index(self.scroll_position, self.apps.len(), -1);
                }

                if old_pos != self.scroll_position {
                    return self.snap_if_needed();
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
                    // FIXME: Fix alt keys bugs
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
        let base_black = iced::Color::from_rgba(0.01, 0.01, 0.01, 0.82);
        let border_color = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.18);
        let transparent_gray = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.04);
        let highlight_gray = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.10);

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
                    .width(28)
                    .height(28)
                    .into(),
                    _ => iced::widget::image(iced::widget::image::Handle::from_path(
                        app.icon.clone().unwrap_or_default(),
                    ))
                    .width(28)
                    .height(28)
                    .into(),
                };

                let shortcut_label = match index {
                    n if self.scroll_position == n => "Enter".to_string(),
                    n @ 0..7 => format!("Alt+{}", n + 1),
                    _ => "".to_string(),
                };

                button(
                    row![
                        icon_view,
                        iced::widget::column![
                            text(&app.name).size(14),
                            text(&app.description)
                                .width(Length::Fill)
                                .size(11)
                                .color(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.3)) // Dimmer description
                                .wrapping(text::Wrapping::Glyph)
                                .line_height(1.0)
                        ],
                        text(shortcut_label)
                            .size(11)
                            .color(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.2))
                    ]
                    .align_y(Vertical::Center)
                    .spacing(12),
                )
                .on_press(Message::OpenApp(index))
                .padding(8)
                .style(move |_, status| {
                    let is_selected = index == self.scroll_position;

                    let bg = if is_selected {
                        highlight_gray
                    } else if status == button::Status::Hovered {
                        transparent_gray
                    } else {
                        iced::Color::TRANSPARENT
                    };

                    button::Style {
                        background: Some(iced::Background::Color(bg)),
                        text_color: iced::Color::WHITE,
                        border: Border {
                            radius: iced::border::radius(8),
                            width: 0.0,
                            color: iced::Color::TRANSPARENT,
                        },
                        shadow: Default::default(),
                    }
                })
                .width(Length::Fill)
                .into()
            })
            .collect();

        let app_list_content: Element<_> = if !app_items.is_empty() {
            Column::with_children(app_items)
                .padding(10)
                .width(Length::Fill)
                .into()
        } else {
            container(
                text("No results")
                    .size(13)
                    .color(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.4)),
            )
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .padding(20)
            .into()
        };

        container(iced::widget::column![
            // Search bar section
            container(
                iced::widget::text_input("Search...", &self.input)
                    .id(TEXT_INPUT_ID.clone())
                    .on_input(Message::InputChange)
                    .on_submit(Message::OpenApp(self.scroll_position))
                    .padding(10)
                    .size(14)
                    .style(move |_, _| {
                        iced::widget::text_input::Style {
                            background: iced::Background::Color(iced::Color::from_rgba(
                                1.0, 1.0, 1.0, 0.03,
                            )),
                            border: Border {
                                radius: iced::border::radius(8),
                                width: 1.0,
                                color: border_color,
                            },
                            icon: iced::Color::WHITE,
                            placeholder: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.2),
                            value: iced::Color::WHITE,
                            selection: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.1),
                        }
                    })
            )
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(base_black)),
                border: Border {
                    radius: iced::border::top(18),
                    ..Default::default()
                },
                ..Default::default()
            })
            .padding(15),
            // Results list section
            iced::widget::scrollable(app_list_content)
                .on_scroll(Message::ScrollableViewport)
                .id(SCROLLABLE_ID.clone())
                .style(move |_, _| scrollable::Style {
                    container: iced::widget::container::Style {
                        background: Some(iced::Background::Color(base_black)),
                        border: Border {
                            radius: iced::border::bottom(18),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    vertical_rail: Rail {
                        background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
                        scroller: scrollable::Scroller {
                            color: iced::Color::TRANSPARENT,
                            border: Border {
                                radius: iced::border::Radius::new(10.0),
                                width: 0.0,
                                color: iced::Color::TRANSPARENT,
                            },
                        },
                        border: Border::default(),
                    },

                    horizontal_rail: Rail {
                        background: None,
                        scroller: scrollable::Scroller {
                            color: border_color,
                            border: Border {
                                radius: iced::border::Radius::new(10.0),
                                width: 0.0,
                                color: iced::Color::TRANSPARENT,
                            },
                        },
                        border: Border::default(),
                    },
                    gap: None
                }),
        ])
        .padding(1)
        .style(move |_| container::Style {
            // Subtle outer border ring
            border: Border {
                width: 1.0,
                color: border_color,
                radius: iced::border::radius(20),
            },
            ..Default::default()
        })
    }

    fn snap_if_needed(&self) -> Task<Message> {
        let Some(viewport) = &self.last_viewport else {
            return Task::none();
        };

        let item_height = 56.0;

        let v_top = viewport.absolute_offset().y;
        let v_height = viewport.bounds().height;
        let v_bottom = v_top + v_height;

        let content_height = viewport.content_bounds().height;
        let max_scroll = content_height - v_height;

        if max_scroll <= 0.0 {
            return Task::none();
        }

        let i_top = self.scroll_position as f32 * item_height;
        let i_bottom = i_top + item_height;

        let mut target_y = None;

        if i_top < v_top {
            target_y = Some(i_top);
        } else if i_bottom > v_bottom {
            target_y = Some(i_bottom - v_height);
        }

        if let Some(y_px) = target_y {
            let relative_y = (y_px / max_scroll).clamp(0.0, 1.0);
            return scrollable::snap_to(
                SCROLLABLE_ID.clone(),
                RelativeOffset {
                    x: 0.0,
                    y: relative_y,
                },
            );
        }

        Task::none()
    }
}

fn wrapped_index(index: usize, array_len: usize, step: isize) -> usize {
    if step >= 0 {
        return (index + step as usize) % array_len;
    }
    let abs_offset = step.unsigned_abs();
    (index + array_len - (abs_offset % array_len)) % array_len
}
