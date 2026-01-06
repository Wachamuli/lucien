use std::sync::LazyLock;

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use iced::{
    Alignment, Border, Element, Event, Length, Subscription, Task, event,
    keyboard::{self, Key},
    widget::{
        Column, Container, button, container, row,
        scrollable::{self, Rail, RelativeOffset, Viewport},
        text, text_input,
    },
};
use iced_layershell::to_layer_message;

use crate::app::{App, all_apps};

static TEXT_INPUT_ID: LazyLock<text_input::Id> = std::sync::LazyLock::new(text_input::Id::unique);
static SCROLLABLE_ID: LazyLock<scrollable::Id> = std::sync::LazyLock::new(scrollable::Id::unique);

// #EBECF2
static MAGNIFIER: &[u8] = include_bytes!("../assets/magnifier.png");
static CUBE_ACTIVE: &[u8] = include_bytes!("../assets/tabler--cube-active.png");
static TERMINAL_PROMPT_ACTIVE: &[u8] = include_bytes!("../assets/mynaui--terminal-active.png");

// #808080
static CUBE_INACTIVE: &[u8] = include_bytes!("../assets/tabler--cube.png");
static TERMINAL_PROMPT_INACTIVE: &[u8] = include_bytes!("../assets/mynaui--terminal.png");
// static FOLDER_INACTIVE: &[u8] = include_bytes!("../assets/proicons--folder.png");
// static CLIPBOARD_INACTIVE: &[u8] = include_bytes!("../assets/tabler--clipboard.png");

#[derive(Debug, Default)]
enum Mode {
    #[default]
    Launcher,
    Terminal,
}

#[derive(Debug, Default)]
pub struct Lucien {
    mode: Mode,
    prompt: String,
    keyboard_modifiers: keyboard::Modifiers,
    all_apps: Vec<App>,
    filtered_apps: Vec<App>,
    scroll_position: usize,
    last_viewport: Option<Viewport>,
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    PromptChange(String),
    LaunchApp(usize),
    SystemEvent(iced::Event),
    ScrollableViewport(Viewport),
    Exit,
}

impl Lucien {
    pub fn init() -> (Self, Task<Message>) {
        let all_apps = all_apps();
        let auto_focus_prompt_task = text_input::focus(TEXT_INPUT_ID.clone());
        let initial_values = Self {
            mode: Mode::Launcher,
            prompt: String::new(),
            keyboard_modifiers: keyboard::Modifiers::empty(),
            all_apps: all_apps.clone(),
            filtered_apps: all_apps,
            scroll_position: 0,
            last_viewport: None,
        };

        (initial_values, auto_focus_prompt_task)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LaunchApp(index) => {
                let Some(app) = self.filtered_apps.get(index) else {
                    return Task::none();
                };

                match app.launch() {
                    Ok(_) => iced::exit(),
                    Err(_) => Task::none(),
                }
            }
            Message::PromptChange(prompt) => {
                if self.keyboard_modifiers.alt() {
                    return Task::none();
                }

                let matcher = SkimMatcherV2::default();

                let mut ranked_apps: Vec<(i64, App)> = self
                    .all_apps
                    .iter()
                    .filter_map(|app| {
                        matcher
                            .fuzzy_match(&app.name, &prompt)
                            .map(|score| (score, app.clone()))
                    })
                    .collect();

                ranked_apps.sort_by(|a, b| b.0.cmp(&a.0));

                self.filtered_apps = ranked_apps.into_iter().map(|(_score, app)| app).collect();
                self.prompt = prompt;
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
                key: Key::Named(key_pressed),
                modifiers,
                ..
            })) => {
                use iced::keyboard::key::Named as kp;

                let old_pos = self.scroll_position;

                match (key_pressed, modifiers.shift()) {
                    (kp::ArrowDown | kp::Tab, false) => {
                        self.scroll_position =
                            wrapped_index(self.scroll_position, self.filtered_apps.len(), 1);
                    }
                    (kp::ArrowUp, false) | (kp::Tab, true) => {
                        self.scroll_position =
                            wrapped_index(self.scroll_position, self.filtered_apps.len(), -1);
                    }
                    _ => {}
                }

                if old_pos != self.scroll_position {
                    return self.snap_if_needed();
                }

                Task::none()
            }
            Message::Exit
            | Message::SystemEvent(iced::Event::Mouse(iced::mouse::Event::ButtonPressed(_))) => {
                iced::exit()
            }
            Message::SystemEvent(iced::Event::Keyboard(keyboard::Event::ModifiersChanged(
                modifiers,
            ))) => {
                self.keyboard_modifiers = modifiers;
                Task::none()
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
                ) => Some(Message::Exit),
                (
                    Event::Keyboard(keyboard::Event::KeyPressed {
                        physical_key: keyboard::key::Physical::Code(physical_key_pressed),
                        modifiers,
                        ..
                    }),
                    _,
                ) if modifiers.alt() => match physical_key_pressed {
                    keyboard::key::Code::Digit1 => Some(Message::LaunchApp(0)),
                    keyboard::key::Code::Digit2 => Some(Message::LaunchApp(1)),
                    keyboard::key::Code::Digit3 => Some(Message::LaunchApp(2)),
                    keyboard::key::Code::Digit4 => Some(Message::LaunchApp(3)),
                    keyboard::key::Code::Digit5 => Some(Message::LaunchApp(4)),
                    _ => None,
                },
                _ => None,
            }),
        ])
    }

    pub fn view(&self) -> Container<'_, Message> {
        let background = iced::Color::from_rgba(0.12, 0.12, 0.12, 0.95);
        let border_color = iced::Color::from_rgba(0.65, 0.65, 0.65, 0.10);
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
            container(
                text("No Results Found")
                    .size(14)
                    .color(text_dim)
                    .width(Length::Fill)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .font(iced::Font {
                        style: iced::font::Style::Italic,
                        ..Default::default()
                    }),
            )
            .padding(25)
            .into()
        };

        let prompt = row![
            iced::widget::image(iced::widget::image::Handle::from_bytes(MAGNIFIER))
                .width(28)
                .height(28),
            iced::widget::text_input("Search...", &self.prompt)
                .id(TEXT_INPUT_ID.clone())
                .on_input(Message::PromptChange)
                .on_submit(Message::LaunchApp(self.scroll_position))
                .padding(8)
                .size(18)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
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
            self.status_indicator()
        ]
        .spacing(2)
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

        container(iced::widget::column![
            container(prompt).padding(15).align_y(Alignment::Center),
            iced::widget::horizontal_rule(1).style(move |_| iced::widget::rule::Style {
                color: border_color,
                width: 1,
                fill_mode: iced::widget::rule::FillMode::Padded(10),
                radius: Default::default(),
            }),
            container(results),
            iced::widget::horizontal_rule(1).style(move |_| iced::widget::rule::Style {
                color: border_color,
                width: 1,
                fill_mode: iced::widget::rule::FillMode::Padded(10),
                radius: Default::default(),
            }),
        ])
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(background)),
            border: Border {
                width: 1.0,
                color: border_color,
                radius: iced::border::radius(20),
            },
            ..Default::default()
        })
    }

    fn status_indicator<'a>(&'a self) -> Container<'a, Message> {
        use iced::widget::image;

        let launcher_icon = match self.mode {
            Mode::Launcher => CUBE_ACTIVE,
            _ => CUBE_INACTIVE,
        };

        let terminal_icon = match self.mode {
            Mode::Terminal => TERMINAL_PROMPT_ACTIVE,
            _ => TERMINAL_PROMPT_INACTIVE,
        };

        container(
            row![
                image(image::Handle::from_bytes(launcher_icon))
                    .width(18)
                    .height(18),
                image(image::Handle::from_bytes(terminal_icon))
                    .width(18)
                    .height(18),
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
        let i_bottom = i_top + item_height + 15.0;

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
