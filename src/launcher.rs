use std::{
    path::PathBuf,
    sync::{Arc, LazyLock},
};

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use iced::{
    Alignment, Element, Event, Length, Subscription, Task, event,
    keyboard::{self, Key},
    widget::{
        Column, Container, container, horizontal_space, image, row,
        scrollable::{self, RelativeOffset, Viewport},
        text, text_input,
    },
};
use iced_layershell::to_layer_message;

use crate::{
    app::{App, IconState, all_apps, process_icon},
    preferences::{self, Action, Preferences},
    theme::{ContainerClass, CustomTheme, TextClass},
};

const SECTION_HEIGHT: f32 = 36.0;

static TEXT_INPUT_ID: LazyLock<text_input::Id> = std::sync::LazyLock::new(text_input::Id::unique);
static SCROLLABLE_ID: LazyLock<scrollable::Id> = std::sync::LazyLock::new(scrollable::Id::unique);
// static DEBOUNCER_ID: LazyLock<task::Id> = std::sync::LazyLock::new(scrollable::Id::unique);

// #EBECF2
static MAGNIFIER: &[u8] = include_bytes!("../assets/magnifier.png");
static ENTER: &[u8] = include_bytes!("../assets/enter.png");
static STAR_ACTIVE: &[u8] = include_bytes!("../assets/star-fill.png");
static STAR_INACTIVE: &[u8] = include_bytes!("../assets/star-line.png");

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
    search_handle: Option<iced::task::Handle>,
    layout: AppLayout,
    icons: BakedIcons,
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    AppsLoaded(Vec<App>),
    IconProcessed(usize, IconState),
    PromptChange(String),
    DebouncedFilter,
    LaunchApp(usize),
    MarkFavorite(usize),

    Keybinding(keyboard::Key, keyboard::Modifiers),

    ScrollableViewport(Viewport),
    SystemEvent(iced::Event),
    SaveIntoDisk(Result<PathBuf, Arc<tokio::io::Error>>),
}

#[derive(Debug, Default, Clone)]
pub struct BakedIcons {
    pub magnifier: Option<image::Handle>,
    pub star_active: Option<image::Handle>,
    pub star_inactive: Option<image::Handle>,
    pub enter: Option<image::Handle>,
}

impl Lucien {
    pub fn init(preferences: Preferences) -> (Self, Task<Message>) {
        let auto_focus_prompt_task = text_input::focus(TEXT_INPUT_ID.clone());
        let scan_apps_task = Task::perform(async { all_apps() }, Message::AppsLoaded);
        let initial_tasks = Task::batch([auto_focus_prompt_task, scan_apps_task]);

        let initial_values = Self {
            // mode: Mode::Launcher,
            prompt: String::new(),
            matcher: SkimMatcherV2::default(),
            keyboard_modifiers: keyboard::Modifiers::empty(),
            cached_apps: Vec::new(),
            ranked_apps: Vec::new(),
            layout: AppLayout::new(&preferences, ""),
            preferences,
            selected_entry: 0,
            last_viewport: None,
            search_handle: None,
            icons: BakedIcons::default(),
        };

        (initial_values, initial_tasks)
    }

    pub fn theme(&self) -> CustomTheme {
        self.preferences.theme.clone()
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

        let id = &self.cached_apps[*app].id;
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
        let total = self.ranked_apps.len();
        if total <= 0 {
            return Task::none();
        }

        let old_pos = self.selected_entry;
        self.selected_entry = wrapped_index(self.selected_entry, total, step);

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

    pub fn snap_if_needed(&self) -> Task<Message> {
        let Some(viewport) = &self.last_viewport else {
            return Task::none();
        };

        // 1. Get coordinates from injected layout
        let app_idx = self.ranked_apps[self.selected_entry];
        let is_fav = self
            .preferences
            .favorite_apps
            .contains(&self.cached_apps[app_idx].id);
        let selection_top = self.layout.y_for_index(self.selected_entry, is_fav);
        let selection_bottom = selection_top + self.layout.item_height;

        // 2. Viewport state
        let scroll_top = viewport.absolute_offset().y;
        let view_height = viewport.bounds().height;
        let scroll_bottom = scroll_top + view_height;
        let max_scroll = viewport.content_bounds().height - view_height;

        if max_scroll <= 0.0 {
            return Task::none();
        }

        const PADDING: f32 = 12.0; // Comfort buffer
        let mut target_y = None;

        if selection_top < scroll_top + PADDING {
            // Scroll Up
            let mut top = selection_top - PADDING;
            // Adjust for headers if at the start of a section
            if !self.layout.is_filtered
                && (self.selected_entry == 0 || self.selected_entry == self.layout.fav_count)
            {
                top -= SECTION_HEIGHT;
            }
            target_y = Some(top);
        } else if selection_bottom > scroll_bottom - PADDING {
            // Scroll Down
            target_y = Some(selection_bottom + PADDING - view_height);
        }

        target_y
            .map(|y| {
                scrollable::snap_to(
                    SCROLLABLE_ID.clone(),
                    RelativeOffset {
                        x: 0.0,
                        y: (y.clamp(0.0, max_scroll)) / max_scroll,
                    },
                )
            })
            .unwrap_or(Task::none())
    }

    fn preload_specific_range(&mut self, indices: Vec<usize>) -> Task<Message> {
        let mut tasks = Vec::new();

        for rank_pos in indices {
            if let Some(&app_idx) = self.ranked_apps.get(rank_pos) {
                let app = &mut self.cached_apps[app_idx];

                if matches!(app.icon_state, IconState::Empty) {
                    app.icon_state = IconState::Loading;

                    tasks.push(Task::perform(
                        process_icon(app_idx, app.icon_name.clone()),
                        |(app_idx, state)| Message::IconProcessed(app_idx, state),
                    ));
                }
            }
        }

        Task::batch(tasks)
    }

    fn preload_visible_icons(&mut self) -> Task<Message> {
        let Some(viewport) = &self.last_viewport else {
            return Task::none();
        };
        let scroll_top = viewport.absolute_offset().y;
        let scroll_bottom = scroll_top + viewport.bounds().height;

        let mut indices = Vec::new();

        // Calculate visible Starred items
        if scroll_top < self.layout.starred_end_y {
            let s = ((scroll_top - self.layout.starred_start_y).max(0.0) / self.layout.item_height)
                .floor() as usize;
            let e = ((scroll_bottom - self.layout.starred_start_y).max(0.0)
                / self.layout.item_height)
                .ceil() as usize;
            indices.extend(s..e.min(self.layout.fav_count));
        }

        // Calculate visible General items
        if scroll_bottom > self.layout.general_start_y {
            let s = ((scroll_top - self.layout.general_start_y).max(0.0) / self.layout.item_height)
                .floor() as usize;
            let e = ((scroll_bottom - self.layout.general_start_y).max(0.0)
                / self.layout.item_height)
                .ceil() as usize;
            let total = self.ranked_apps.len();
            indices.extend((s + self.layout.fav_count)..(e + self.layout.fav_count).min(total));
        }

        self.preload_specific_range(indices)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AppsLoaded(apps) => {
                self.cached_apps = apps;
                self.ranked_apps = (0..self.cached_apps.len()).collect();

                self.icons = BakedIcons {
                    enter: Some(image::Handle::from_bytes(ENTER)),
                    magnifier: Some(image::Handle::from_bytes(MAGNIFIER)),
                    star_active: Some(image::Handle::from_bytes(STAR_ACTIVE)),
                    star_inactive: Some(image::Handle::from_bytes(STAR_INACTIVE)),
                };

                if !self.preferences.favorite_apps.is_empty() {
                    self.ranked_apps.sort_by_key(|index| {
                        let app = &self.cached_apps[*index];
                        !self.preferences.favorite_apps.contains(&app.id)
                    });
                }

                Task::none()
            }
            Message::IconProcessed(app_index, state) => {
                if let Some(app) = self.cached_apps.get_mut(app_index) {
                    app.icon_state = if matches!(state, IconState::Empty) {
                        IconState::NotFound
                    } else {
                        state
                    };
                }

                Task::none()
            }
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
                    Err(e) => {
                        tracing::error!("Failed to launch {}, due to: {}", app.id, e);
                        Task::none()
                    }
                }
            }
            Message::PromptChange(prompt) => {
                if self.keyboard_modifiers.alt() {
                    return Task::none();
                }

                self.prompt = prompt;

                if let Some(handle) = self.search_handle.take() {
                    handle.abort();
                }

                if self.prompt.is_empty() {
                    return Task::done(Message::DebouncedFilter);
                }

                let (task, handle) = Task::future(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    Message::DebouncedFilter
                })
                .abortable();

                self.search_handle = Some(handle);
                return task;
            }
            Message::DebouncedFilter => {
                self.selected_entry = 0;
                self.update_ranked_apps();

                let scroll_task =
                    scrollable::snap_to(SCROLLABLE_ID.clone(), RelativeOffset { x: 0.0, y: 0.0 });

                self.layout = AppLayout::new(&self.preferences, &self.prompt);
                let preload_task = self.preload_visible_icons();

                Task::batch([scroll_task, preload_task])
            }
            Message::MarkFavorite(index) => self.mark_favorite(index),
            Message::ScrollableViewport(viewport) => {
                self.last_viewport = Some(viewport);
                self.layout = AppLayout::new(&self.preferences, &self.prompt);
                self.preload_visible_icons()
            }
            Message::SystemEvent(Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(key),
                ..
            })) => match key {
                keyboard::key::Named::ArrowUp => self.go_to_entry(-1),
                keyboard::key::Named::ArrowDown => self.go_to_entry(1),
                _ => Task::none(),
            },
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

    pub fn view(&self) -> Container<'_, Message, CustomTheme> {
        let theme = &self.preferences.theme;

        fn section(name: &str) -> Container<'_, Message, CustomTheme> {
            container(
                text(name)
                    .size(14)
                    .class(TextClass::TextDim)
                    .width(Length::Fill)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    }),
            )
            .height(SECTION_HEIGHT)
            .padding(iced::Padding {
                top: 10.,
                right: 0.,
                bottom: 5.,
                left: 10.,
            })
            .into()
        }

        let mut starred_column;
        let mut general_column;

        if self.prompt.is_empty() && !self.preferences.favorite_apps.is_empty() {
            starred_column = Column::new().push(section("Starred"));
            general_column = Column::new().push(section("General"));
        } else {
            starred_column = Column::new();
            general_column = Column::new();
        }

        for (rank_pos, app_index) in self.ranked_apps.iter().enumerate() {
            let app = &self.cached_apps[*app_index];
            let is_favorite = self.preferences.favorite_apps.contains(&app.id);
            let is_selected = self.selected_entry == rank_pos;

            let icon_status = app.icon_state.status();
            let item_height = theme.launchpad.entry.height;
            let style = &self.preferences.theme.launchpad.entry;
            let icons = &self.icons;

            let element: Element<Message, CustomTheme> = container(iced::widget::lazy(
                (*app_index, is_selected, is_favorite, icon_status),
                move |_| app.entry(&icons, style, rank_pos, self.selected_entry, is_favorite),
            ))
            .height(item_height)
            .width(Length::Fill)
            .into();

            if is_favorite && self.prompt.is_empty() {
                starred_column = starred_column.push(element);
            } else {
                general_column = general_column.push(element);
            }
        }

        let results_not_found: Container<Message, CustomTheme> = container(
            text("No Results Found")
                .size(14)
                .class(TextClass::TextDim)
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
            .padding(theme.launchpad.padding)
            .width(Length::Fill);
        let magnifier: Element<Message, CustomTheme> = match &self.icons.magnifier {
            Some(handle) => image(handle)
                .width(theme.prompt.icon_size)
                .height(theme.prompt.icon_size)
                .into(),
            None => horizontal_space()
                .width(theme.prompt.icon_size)
                .height(theme.prompt.icon_size)
                .into(),
        };
        let promp_input: Element<Message, CustomTheme> =
            iced::widget::text_input("Search...", &self.prompt)
                .id(TEXT_INPUT_ID.clone())
                .on_input(Message::PromptChange)
                .on_submit(Message::LaunchApp(self.selected_entry))
                .padding(8)
                .size(theme.prompt.font_size)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .into();
        let prompt_view: Element<Message, CustomTheme> = row![]
            .push(magnifier)
            .push(promp_input)
            // .push(self.status_indicator())
            .align_y(iced::Alignment::Center)
            .spacing(2)
            .into();
        let results = iced::widget::scrollable(content)
            .on_scroll(Message::ScrollableViewport)
            .id(SCROLLABLE_ID.clone());

        container(iced::widget::column![
            container(prompt_view)
                .padding(iced::Padding::from(&theme.prompt.margin))
                .align_y(Alignment::Center),
            iced::widget::horizontal_rule(1),
            container(results),
        ])
        .class(ContainerClass::MainContainer)
        .into()
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

pub struct AppLayout {
    pub item_height: f32,
    pub padding: f32,
    pub fav_count: usize,
    pub is_filtered: bool,
    pub has_favorites: bool,
    pub starred_start_y: f32,
    pub starred_end_y: f32,
    pub general_start_y: f32,
}

impl AppLayout {
    pub fn new(preferences: &Preferences, prompt: &str) -> Self {
        let style = &preferences.theme.launchpad;
        let item_height = style.entry.height;
        let fav_count = preferences.favorite_apps.len();
        let is_filtered = !prompt.is_empty();
        let has_favorites = fav_count > 0;

        // If filtered, headers disappear (height = 0)
        let header_h = if !is_filtered && has_favorites {
            SECTION_HEIGHT
        } else {
            0.0
        };

        let starred_start_y = style.padding + header_h;
        let starred_end_y = starred_start_y + (fav_count as f32 * item_height);
        let general_start_y = starred_end_y + header_h;

        Self {
            item_height,
            padding: style.padding,
            fav_count,
            is_filtered,
            has_favorites,
            starred_start_y,
            starred_end_y,
            general_start_y,
        }
    }

    /// Maps a global list index to its top Y coordinate
    pub fn y_for_index(&self, index: usize, is_favorite: bool) -> f32 {
        if !self.is_filtered && self.has_favorites {
            if is_favorite {
                self.starred_start_y + (index as f32 * self.item_height)
            } else {
                let local_idx = index - self.fav_count;
                self.general_start_y + (local_idx as f32 * self.item_height)
            }
        } else {
            self.padding + (index as f32 * self.item_height)
        }
    }
}
