use std::{
    path::PathBuf,
    sync::{Arc, LazyLock},
};

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

use crate::{
    app::{App, all_apps},
    preferences::{self, Action, HexColor, Preferences},
};

static TEXT_INPUT_ID: LazyLock<text_input::Id> = std::sync::LazyLock::new(text_input::Id::unique);
static SCROLLABLE_ID: LazyLock<scrollable::Id> = std::sync::LazyLock::new(scrollable::Id::unique);

// #EBECF2
static MAGNIFIER: &[u8] = include_bytes!("../assets/magnifier.png");
// static CUBE_ACTIVE: &[u8] = include_bytes!("../assets/tabler--cube-active.png");
// static TERMINAL_PROMPT_ACTIVE: &[u8] = include_bytes!("../assets/mynaui--terminal-active.png");

// // #808080
// static CUBE_INACTIVE: &[u8] = include_bytes!("../assets/tabler--cube.png");
// static TERMINAL_PROMPT_INACTIVE: &[u8] = include_bytes!("../assets/mynaui--terminal.png");
// static FOLDER_INACTIVE: &[u8] = include_bytes!("../assets/proicons--folder.png");
// static CLIPBOARD_INACTIVE: &[u8] = include_bytes!("../assets/tabler--clipboard.png");

// #[derive(Debug)]
pub struct Lucien {
    prompt: String,
    matcher: SkimMatcherV2,
    keyboard_modifiers: keyboard::Modifiers,
    cached_apps: Vec<App>,
    ranked_apps: Vec<usize>,
    preferences: Preferences,
    selected_entry: usize,
    last_viewport: Option<Viewport>,
    magnifier_icon: iced::widget::image::Handle,
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    PromptChange(String),
    LaunchApp(usize),
    MarkFavorite(usize),

    Keybinding(keyboard::Key, keyboard::Modifiers),

    ScrollableViewport(Viewport),
    SystemEvent(iced::Event),
    SaveIntoDisk(Result<PathBuf, Arc<tokio::io::Error>>),
}

impl Lucien {
    pub fn init(preferences: Preferences) -> (Self, Task<Message>) {
        let magnifier_icon = iced::widget::image::Handle::from_bytes(MAGNIFIER);
        let cached_apps = all_apps();
        let mut ranked_apps: Vec<usize> = (0..cached_apps.len()).collect();
        // TODO: Note: sort_by_key is executed even if favorite_apps is empty.
        ranked_apps
            .sort_by_key(|index| !preferences.favorite_apps.contains(&cached_apps[*index].id));

        let auto_focus_prompt_task = text_input::focus(TEXT_INPUT_ID.clone());

        let initial_values = Self {
            // mode: Mode::Launcher,
            prompt: String::new(),
            matcher: SkimMatcherV2::default(),
            keyboard_modifiers: keyboard::Modifiers::empty(),
            cached_apps,
            ranked_apps,
            preferences,
            selected_entry: 0,
            last_viewport: None,
            magnifier_icon: magnifier_icon,
        };

        (initial_values, auto_focus_prompt_task)
    }

    fn update_ranked_apps(&mut self) {
        let mut ranked: Vec<(i64, usize)> = self
            .cached_apps
            .iter()
            .enumerate()
            .filter_map(|(index, app)| {
                let score = self.matcher.fuzzy_match(&app.name, &self.prompt)?;
                Some((score, index))
            })
            .collect();

        ranked.sort_by(|(score_a, index_a), (score_b, index_b)| {
            let app_a = &self.cached_apps[*index_a];
            let app_b = &self.cached_apps[*index_b];
            let a_is_fav = self.preferences.favorite_apps.contains(&app_a.id);
            let b_is_fav = self.preferences.favorite_apps.contains(&app_b.id);

            match (a_is_fav, b_is_fav) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => score_b.cmp(score_a),
            }
        });

        self.ranked_apps = ranked
            .into_iter()
            .map(|(_score, app_index)| app_index)
            .collect();
    }

    fn mark_favorite(&mut self, index: usize) -> Task<Message> {
        let Some(app) = self.ranked_apps.get(index) else {
            return Task::none();
        };

        let Some(ref path) = self.preferences.path else {
            tracing::warn!("In-memory defaults. Settings will not be saved");
            return Task::none();
        };

        let id = self.cached_apps[*app].id.clone();
        let path = path.clone();
        // Toggle_favorite is a very opaque function. It actually
        // modifies the in-memory favorite_apps variable.
        // Maybe I should expose this assignnment operation at this level.
        let favorite_apps = self.preferences.toggle_favorite(id);
        self.update_ranked_apps();

        Task::perform(
            preferences::save_into_disk(path, "favorite_apps", favorite_apps),
            Message::SaveIntoDisk,
        )
    }

    fn go_to_entry(&mut self, step: isize) -> Task<Message> {
        if self.ranked_apps.len() <= 0 {
            return Task::none();
        }

        let old_pos = self.selected_entry;
        self.selected_entry = wrapped_index(self.selected_entry, self.ranked_apps.len(), step);

        if old_pos != self.selected_entry {
            return self.snap_if_needed();
        }

        Task::none()
    }

    fn handle_action(&mut self, action: Action) -> Task<Message> {
        match action {
            Action::Mark => self.mark_favorite(self.selected_entry),
            Action::Exit => iced::exit(),
            Action::GoNextEntry => self.go_to_entry(1),
            Action::GoPreviousEntry => self.go_to_entry(-1),
        }
    }

    fn snap_if_needed(&self) -> Task<Message> {
        let Some(viewport) = &self.last_viewport else {
            return Task::none();
        };

        let content_height = viewport.content_bounds().height;
        let view_height = viewport.bounds().height;

        let item_height = content_height / self.ranked_apps.len() as f32;

        let selection_top = self.selected_entry as f32 * item_height;
        let scalar = if self.prompt.is_empty() && !self.preferences.favorite_apps.is_empty() {
            1.0
        } else {
            0.2
        };
        let extra_bottom_padding = item_height * scalar;
        let selection_bottom = selection_top + item_height + extra_bottom_padding;

        let scroll_top = viewport.absolute_offset().y;
        let scroll_bottom = scroll_top + view_height;

        if selection_top < scroll_top {
            scrollable::snap_to(
                SCROLLABLE_ID.clone(),
                RelativeOffset {
                    x: 0.0,
                    y: selection_top / (content_height - view_height),
                },
            )
        } else if selection_bottom > scroll_bottom {
            scrollable::snap_to(
                SCROLLABLE_ID.clone(),
                RelativeOffset {
                    x: 0.0,
                    y: (selection_bottom - view_height) / (content_height - view_height),
                },
            )
        } else {
            Task::none()
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SaveIntoDisk(result) => {
                match result {
                    Ok(path) => tracing::debug!("Preference saved into disk: {:?}", path),
                    Err(e) => tracing::error!("Failed to save preferences to disk: {}", e),
                }

                Task::none()
            }
            Message::Keybinding(current_key_pressed, current_modifiers) => {
                self.keyboard_modifiers = current_modifiers;
                for (action, keystroke) in &self.preferences.keybindings {
                    if keystroke.matches(&current_key_pressed, current_modifiers) {
                        return self.handle_action(*action);
                    }
                }

                Task::none()
            }
            Message::LaunchApp(index) => {
                let Some(app_index) = self.ranked_apps.get(index) else {
                    return Task::none();
                };

                let app = &self.cached_apps[*app_index];

                match app.launch() {
                    Ok(_) => iced::exit(),
                    Err(_) => Task::none(),
                }
            }
            Message::PromptChange(prompt) => {
                if self.keyboard_modifiers.alt() {
                    return Task::none();
                }

                self.prompt = prompt;
                self.selected_entry = 0;
                self.update_ranked_apps();

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
            Message::MarkFavorite(index) => self.mark_favorite(index),
            Message::ScrollableViewport(viewport) => {
                self.last_viewport = Some(viewport);
                Task::none()
            }
            Message::SystemEvent(Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(keyboard::key::Named::ArrowUp),
                ..
            })) => self.go_to_entry(-1),
            Message::SystemEvent(Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(keyboard::key::Named::ArrowDown),
                ..
            })) => self.go_to_entry(1),
            Message::SystemEvent(iced::Event::Keyboard(keyboard::Event::ModifiersChanged(
                modifiers,
            ))) => {
                self.keyboard_modifiers = modifiers;
                Task::none()
            }
            Message::SystemEvent(iced::Event::Mouse(iced::mouse::Event::ButtonPressed(_))) => {
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
            event::listen_with(move |event, status, _id| match (event, status) {
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
                (Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }), _) => {
                    Some(Message::Keybinding(key, modifiers))
                }
                _ => None,
            }),
        ])
    }

    pub fn view(&self) -> Container<'_, Message> {
        let theme = &self.preferences.theme;
        let background = &theme.background;
        // Maybe we can get rid of this conversion on the ui.
        let border_style = iced::Border::from(&theme.border);
        let focus_highlight = &theme.focus_highlight;
        let hover_highlight = &theme.hover_highlight;

        let text_main = iced::Color::from_rgba(0.95, 0.95, 0.95, 1.0);
        let text_dim = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.5);

        fn section(name: &str, color: iced::Color) -> Container<'_, Message> {
            container(
                text(name)
                    .size(14)
                    .color(color)
                    .width(Length::Fill)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    }),
            )
            .padding(iced::Padding {
                top: 10.,
                right: 0.,
                bottom: 5.,
                left: 10.,
            })
        }

        let mut starred_column;
        let mut general_column;

        if self.prompt.is_empty() && !self.preferences.favorite_apps.is_empty() {
            starred_column = Column::new().push(section("Starred", text_dim));
            general_column = Column::new().push(section("General", text_dim));
        } else {
            starred_column = Column::new();
            general_column = Column::new();
        }

        for (rank_pos, app_index) in self.ranked_apps.iter().enumerate() {
            let app = &self.cached_apps[*app_index];
            let is_selected = self.selected_entry == rank_pos;
            let is_favorite = self.preferences.favorite_apps.contains(&app.id);

            let element: Element<Message> = app
                .itemlist(self.selected_entry, rank_pos, is_favorite)
                .style(move |_, status| {
                    let bg = if is_selected {
                        focus_highlight
                    } else if status == button::Status::Hovered {
                        hover_highlight
                    } else {
                        &HexColor(iced::Color::TRANSPARENT)
                    };

                    button::Style {
                        background: Some(iced::Background::Color(**bg)),
                        text_color: if is_selected { text_main } else { text_dim },
                        border: Border {
                            radius: iced::border::radius(20),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
                .into();

            if is_favorite {
                starred_column = starred_column.push(element);
            } else {
                general_column = general_column.push(element);
            }
        }

        let results_not_found: Container<_> = container(
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
        .padding(19.8);

        let content = Column::new()
            .push(starred_column)
            .push(general_column)
            .push_maybe(self.ranked_apps.is_empty().then(|| results_not_found))
            .padding(10)
            .width(Length::Fill);

        let magnifier = iced::widget::image(&self.magnifier_icon)
            .width(28)
            .height(28);
        let promp_input = iced::widget::text_input("Search...", &self.prompt)
            .id(TEXT_INPUT_ID.clone())
            .on_input(Message::PromptChange)
            .on_submit(Message::LaunchApp(self.selected_entry))
            .padding(8)
            .size(18)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            })
            .style(move |_, _| iced::widget::text_input::Style {
                background: iced::Background::Color(iced::Color::TRANSPARENT),
                border: Border {
                    width: 0.0,
                    ..Default::default()
                },
                icon: text_main,
                placeholder: text_dim,
                value: text_main,
                selection: **focus_highlight,
            });

        let prompt_view = row![]
            .push(magnifier)
            .push(promp_input)
            // .push(self.status_indicator())
            .align_y(iced::Alignment::Center)
            .spacing(2);

        let results = iced::widget::scrollable(content)
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
                        color: text_dim,
                        // color: border_style.color,
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
            container(prompt_view)
                .padding(15)
                .align_y(Alignment::Center),
            iced::widget::horizontal_rule(1).style(move |_| iced::widget::rule::Style {
                color: border_style.color,
                width: theme.border.width as u16,
                fill_mode: iced::widget::rule::FillMode::Padded(10),
                radius: Default::default(),
            }),
            container(results),
            iced::widget::horizontal_rule(1).style(move |_| iced::widget::rule::Style {
                color: border_style.color,
                width: 1,
                fill_mode: iced::widget::rule::FillMode::Padded(10),
                radius: Default::default(),
            }),
        ])
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(**background)),
            border: border_style,
            ..Default::default()
        })
    }

    // fn status_indicator<'a>(&'a self) -> Container<'a, Message> {
    //     use iced::widget::image;

    //     let launcher_icon = match self.mode {
    //         Mode::Launcher => CUBE_ACTIVE,
    //         _ => CUBE_INACTIVE,
    //     };

    //     let terminal_icon = match self.mode {
    //         Mode::Terminal => TERMINAL_PROMPT_ACTIVE,
    //         _ => TERMINAL_PROMPT_INACTIVE,
    //     };

    //     container(
    //         row![
    //             image(image::Handle::from_bytes(launcher_icon))
    //                 .width(18)
    //                 .height(18),
    //             image(image::Handle::from_bytes(terminal_icon))
    //                 .width(18)
    //                 .height(18),
    //         ]
    //         .spacing(10),
    //     )
    // }
}

fn wrapped_index(index: usize, array_len: usize, step: isize) -> usize {
    if array_len == 0 {
        return 0;
    }

    if step >= 0 {
        return (index + step as usize) % array_len;
    }

    let abs_offset = step.unsigned_abs();
    (index + array_len - (abs_offset % array_len)) % array_len
}
