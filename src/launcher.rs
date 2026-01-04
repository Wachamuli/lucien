use std::sync::LazyLock;

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use iced::{
    Alignment, Border, Element, Event, Length, Subscription, Task, event,
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
pub struct Lucien {
    input: String,
    all_apps: Vec<App>,
    filtered_apps: Vec<App>,
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
    Close,
}

static TEXT_INPUT_ID: LazyLock<text_input::Id> = std::sync::LazyLock::new(text_input::Id::unique);
static SCROLLABLE_ID: LazyLock<scrollable::Id> = std::sync::LazyLock::new(scrollable::Id::unique);

// #EBECF2
static MAGNIFIER: &[u8] = include_bytes!("../assets/magnifier.png");
// static TERMINAL_PROMPT: &[u8] = include_bytes!("../assets/terminal-prompt.png");
// static FOLDER: &[u8] = include_bytes!("../assets/folder.png");
// static CLIPBOARD: &[u8] = include_bytes!("../assets/clipboard.png");

impl Lucien {
    pub fn init() -> (Self, Task<Message>) {
        let auto_focus_task = text_input::focus(TEXT_INPUT_ID.clone());
        let all = all_apps();
        let initial_values = Self {
            input: String::new(),
            all_apps: all.clone(),
            filtered_apps: all,
            scroll_position: 0,
            last_viewport: None,
        };

        (initial_values, auto_focus_task)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenApp(index) | Message::AltDigitShortcut(index) => {
                let Some(app) = self.filtered_apps.get(index) else {
                    return Task::none();
                };

                match app.launch() {
                    Ok(_) => iced::exit(),
                    Err(_) => Task::none(), // TODO: Handle this error
                }
            }
            Message::InputChange(input) => {
                let matcher = SkimMatcherV2::default();

                let mut ranked_apps: Vec<(i64, App)> = self
                    .all_apps
                    .iter()
                    .filter_map(|app| {
                        matcher
                            .fuzzy_match(&app.name, &input)
                            .map(|score| (score, app.clone()))
                    })
                    .collect();

                ranked_apps.sort_by(|a, b| b.0.cmp(&a.0));

                self.filtered_apps = ranked_apps.into_iter().map(|(_score, app)| app).collect();
                self.input = input;
                self.scroll_position = 0;

                if let Some(viewport) = self.last_viewport {
                    if viewport.absolute_offset().y > 0.0 {
                        return scrollable::snap_to(
                            SCROLLABLE_ID.clone(),
                            RelativeOffset { x: 0.0, y: 0.0 },
                        );
                    }
                }

                Task::none()
            }
            Message::ScrollableViewport(viewport) => {
                self.last_viewport = Some(viewport);
                Task::none()
            }
            Message::SystemEvent(iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(keyboard::key::Named::Tab),
                modifiers,
                ..
            })) if modifiers.shift() => {
                let old_pos = self.scroll_position;

                self.scroll_position =
                    wrapped_index(self.scroll_position, self.filtered_apps.len(), -1);

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
                    self.scroll_position =
                        wrapped_index(self.scroll_position, self.filtered_apps.len(), 1);
                }

                if let kp::ArrowUp = key_pressed {
                    self.scroll_position =
                        wrapped_index(self.scroll_position, self.filtered_apps.len(), -1);
                }

                if old_pos != self.scroll_position {
                    return self.snap_if_needed();
                }

                Task::none()
            }
            Message::Close
            | Message::EscPressed
            | Message::SystemEvent(iced::Event::Mouse(iced::mouse::Event::ButtonPressed(_))) => {
                iced::exit()
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
        // Lighter variant
        // let background = iced::Color::from_rgba(0.12, 0.12, 0.12, 0.85);
        // let border_color = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.15);
        // let inner_glow = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08);
        // let active_selection = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.12);
        // let text_main = iced::Color::from_rgba(0.95, 0.95, 0.95, 1.0);
        // let text_dim = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.5);

        let background = iced::Color::from_rgba(0.12, 0.12, 0.12, 0.95);
        let border_color = iced::Color::from_rgba(0.65, 0.65, 0.65, 0.15);
        let inner_glow = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08);
        let active_selection = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.12);
        let text_main = iced::Color::from_rgba(0.95, 0.95, 0.95, 1.0);
        let text_dim = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.5);

        let app_items: Vec<Element<Message>> = self
            .filtered_apps
            .iter()
            .enumerate()
            .map(|(index, app)| {
                app.itemlist(self.scroll_position, index)
                    .style(move |_, status| {
                        let is_selected = self.scroll_position == index;

                        let bg = if is_selected {
                            active_selection
                        } else if status == button::Status::Hovered {
                            inner_glow
                        } else {
                            iced::Color::TRANSPARENT
                        };

                        button::Style {
                            background: Some(iced::Background::Color(bg)),
                            text_color: if is_selected { text_main } else { text_dim },
                            border: iced::Border {
                                radius: iced::border::radius(10),
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    })
                    .into()
            })
            .collect();

        let app_list_content: Element<_> = if !app_items.is_empty() {
            Column::with_children(app_items)
                .padding(10)
                .spacing(4)
                .width(Length::Fill)
                .into()
        } else {
            container(text("No Results Found").size(14).color(text_dim))
                .width(Length::Fill)
                .align_x(Alignment::Center)
                .padding(25)
                .into()
        };

        let prompt = row![
            iced::widget::image(iced::widget::image::Handle::from_bytes(MAGNIFIER))
                .width(28)
                .height(28),
            iced::widget::text_input("Search...", &self.input)
                .id(TEXT_INPUT_ID.clone())
                .on_input(Message::InputChange)
                .on_submit(Message::OpenApp(self.scroll_position))
                .padding(8)
                .size(18)
                .style(move |_, _| {
                    iced::widget::text_input::Style {
                        background: iced::Background::Color(iced::Color::TRANSPARENT),
                        border: Border {
                            width: 0.0,
                            ..Default::default()
                        },
                        icon: text_main,
                        placeholder: text_dim,
                        value: text_main,
                        selection: active_selection,
                    }
                }),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let results = iced::widget::scrollable(app_list_content)
            .on_scroll(Message::ScrollableViewport)
            .id(SCROLLABLE_ID.clone())
            .style(move |_, _| scrollable::Style {
                container: iced::widget::container::Style {
                    background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
                    border: Border {
                        radius: iced::border::radius(20),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                vertical_rail: Rail {
                    background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
                    scroller: scrollable::Scroller {
                        color: iced::Color::TRANSPARENT,
                        border: Border {
                            width: 0.0,
                            ..Default::default()
                        },
                    },
                    border: Border::default(),
                },
                horizontal_rail: Rail {
                    background: None,
                    scroller: scrollable::Scroller {
                        color: border_color,
                        border: Border {
                            radius: iced::border::radius(5),
                            ..Default::default()
                        },
                    },
                    border: Border::default(),
                },
                gap: None,
            });

        // let mode_circle = |bytes: &'static [u8]| {
        //     container(
        //         iced::widget::image(iced::widget::image::Handle::from_bytes(bytes))
        //             .width(28)
        //             .height(28),
        //     )
        //     .padding(18)
        //     .style(move |_| container::Style {
        //         background: Some(iced::Background::Color(background)),
        //         border: Border {
        //             width: 1.0,
        //             color: border_color,
        //             radius: iced::border::radius(36),
        //         },
        //         ..Default::default()
        //     })
        // };

        // let modes_island = container(
        //     row![
        //         mode_circle(MAGNIFIER),
        //         mode_circle(FOLDER),
        //         mode_circle(TERMINAL_PROMPT),
        //         mode_circle(CLIPBOARD),
        //     ]
        //     .spacing(10),
        // )
        // .align_y(Alignment::Center)
        // .style(move |_| container::Style {
        //     background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
        //     ..Default::default()
        // });

        container(
            iced::widget::column![
                row![
                    container(prompt)
                        .padding(15)
                        .style(move |_| container::Style {
                            background: Some(iced::Background::Color(background)),
                            border: Border {
                                width: 1.0,
                                color: border_color,
                                radius: iced::border::radius(20),
                            },
                            ..Default::default()
                        }),
                    // modes_island
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                container(results).style(move |_| container::Style {
                    background: Some(iced::Background::Color(background)),
                    border: Border {
                        width: 1.0,
                        color: border_color,
                        radius: iced::border::radius(20),
                    },
                    ..Default::default()
                })
            ]
            .spacing(10),
        )
    }

    fn snap_if_needed(&self) -> Task<Message> {
        let Some(viewport) = &self.last_viewport else {
            return Task::none();
        };

        let item_height = 62.0;

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
