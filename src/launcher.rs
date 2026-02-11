use std::{
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
};

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use iced::{
    Alignment, Event, Length, Subscription, Task, event, keyboard, mouse,
    widget::{
        Column, Container, container, image, row,
        scrollable::{self, RelativeOffset, Viewport},
        text, text_input,
    },
};
use iced_layershell::to_layer_message;

use crate::{
    preferences::{
        self, Preferences,
        keybindings::{Action, KeyStroke},
        theme::{ContainerClass, CustomTheme, TextClass},
    },
    providers::{Entry, ProviderKind, app::AppProvider, file::FileProvider},
    ui::{
        entry,
        icon::{
            BakedIcons, CUBE_ACTIVE, CUBE_INACTIVE, ENTER, FOLDER_ACTIVE, FOLDER_INACTIVE,
            MAGNIFIER, STAR_ACTIVE, STAR_INACTIVE,
        },
        prompt::Prompt,
    },
};

const SECTION_HEIGHT: f32 = 36.0;

static TEXT_INPUT_ID: LazyLock<text_input::Id> = std::sync::LazyLock::new(text_input::Id::unique);
static SCROLLABLE_ID: LazyLock<scrollable::Id> = std::sync::LazyLock::new(scrollable::Id::unique);

pub struct Lucien {
    provider: ProviderKind,
    prompt: String,
    matcher: SkimMatcherV2,
    cached_entries: Vec<Entry>,
    ranked_entries: Vec<usize>,
    preferences: Preferences,
    selected_entry: usize,
    last_viewport: Option<Viewport>,
    search_handle: Option<iced::task::Handle>,
    icons: BakedIcons,
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    PopulateEntries(Vec<Entry>),
    PromptChange(String),
    DebouncedFilter,
    TriggerAction(Action),
    TriggerActionByKeybinding(keyboard::Key, keyboard::Modifiers),
    ScrollableViewport(Viewport),
    SaveIntoDisk(Result<PathBuf, Arc<tokio::io::Error>>),
    Close,
}

impl Lucien {
    pub fn init(preferences: Preferences) -> (Self, Task<Message>) {
        let auto_focus_prompt_task = text_input::focus(TEXT_INPUT_ID.clone());
        let default_provider = ProviderKind::App(AppProvider);
        let scan_task = Task::perform(
            async move {
                default_provider
                    .handler()
                    .scan(Path::new("This parameter should be optional"))
            },
            Message::PopulateEntries,
        );
        let initial_tasks = Task::batch([auto_focus_prompt_task, scan_task]);

        let baked_icons = BakedIcons {
            enter: image::Handle::from_bytes(ENTER),
            magnifier: image::Handle::from_bytes(MAGNIFIER),
            star_active: image::Handle::from_bytes(STAR_ACTIVE),
            star_inactive: image::Handle::from_bytes(STAR_INACTIVE),
        };

        let initial_values = Self {
            provider: default_provider,
            prompt: String::new(),
            matcher: SkimMatcherV2::default(),
            cached_entries: Vec::new(),
            ranked_entries: Vec::new(),
            preferences,
            selected_entry: 0,
            last_viewport: None,
            search_handle: None,
            icons: baked_icons,
        };

        (initial_values, initial_tasks)
    }

    pub fn theme(&self) -> CustomTheme {
        self.preferences.theme.clone()
    }

    fn update_ranked_apps(&mut self) {
        let mut ranked: Vec<(i64, usize)> = self
            .cached_entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let score = self.matcher.fuzzy_match(&entry.main, &self.prompt)?;
                Some((score, index))
            })
            .collect();

        ranked.sort_by(|(score_a, index_a), (score_b, index_b)| {
            let entry_a = &self.cached_entries[*index_a];
            let entry_b = &self.cached_entries[*index_b];
            let a_is_fav = self.preferences.favorite_apps.contains(&entry_a.id);
            let b_is_fav = self.preferences.favorite_apps.contains(&entry_b.id);

            match (a_is_fav, b_is_fav) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => score_b.cmp(score_a),
            }
        });

        self.ranked_entries = ranked
            .into_iter()
            .map(|(_score, app_index)| app_index)
            .collect();
    }

    fn mark_favorite(&mut self, index: usize) -> Task<Message> {
        let Some(app) = self.ranked_entries.get(index) else {
            return Task::none();
        };

        let Some(ref path) = self.preferences.path else {
            tracing::warn!("In-memory defaults. Settings will not be saved");
            return Task::none();
        };

        let id = &self.cached_entries[*app].id;
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
        let total = self.ranked_entries.len();
        if total == 0 {
            return Task::none();
        }

        let old_pos = self.selected_entry;
        self.selected_entry = wrapped_index(self.selected_entry, total, step);

        if old_pos != self.selected_entry {
            let layout = AppLayout::new(&self.preferences, &self.prompt);
            return self.snap_if_needed(&layout);
        }

        Task::none()
    }

    fn launch_entry(&self, index: usize) -> Task<Message> {
        let Some(entry_index) = self.ranked_entries.get(index) else {
            return Task::none();
        };

        let entry = &self.cached_entries[*entry_index];

        self.provider.handler().launch(&entry.id)
    }

    fn handle_action(&mut self, action: Action) -> Task<Message> {
        match action {
            Action::Close => iced::exit(),
            Action::NextEntry => self.go_to_entry(1),
            Action::PreviousEntry => self.go_to_entry(-1),
            Action::ToggleFavorite => self.mark_favorite(self.selected_entry),
            Action::LaunchEntry(index) => self.launch_entry(index),
        }
    }

    pub fn snap_if_needed(&self, layout: &AppLayout) -> Task<Message> {
        let Some(viewport) = &self.last_viewport else {
            return Task::none();
        };

        // 1. Get coordinates from injected layout
        let entry_index = self.ranked_entries[self.selected_entry];
        let is_fav = self
            .preferences
            .favorite_apps
            .contains(&self.cached_entries[entry_index].id);
        let selection_top = layout.y_for_index(self.selected_entry, is_fav);
        let selection_bottom = selection_top + layout.item_height;

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
            if !layout.is_filtered
                && (self.selected_entry == 0 || self.selected_entry == layout.fav_count)
            {
                top -= SECTION_HEIGHT;
            }
            target_y = Some(top);
        } else if selection_bottom > scroll_bottom - PADDING {
            // Scroll Down
            target_y = Some(selection_bottom + PADDING - view_height);
        }

        let Some(y) = target_y else {
            return Task::none();
        };

        scrollable::snap_to(
            SCROLLABLE_ID.clone(),
            RelativeOffset {
                x: 0.0,
                y: (y.clamp(0.0, max_scroll)) / max_scroll,
            },
        )
    }

    fn swtich_provider(&mut self) -> Task<Message> {
        match self.prompt.as_str() {
            "@" => {
                self.prompt = "".to_string();
                self.provider = ProviderKind::App(AppProvider);
                let provider_clone = self.provider.clone();
                Task::perform(
                    async move {
                        provider_clone
                            .handler()
                            .scan(Path::new("This parameter should be optional"))
                    },
                    Message::PopulateEntries,
                )
            }
            "/" => {
                self.prompt = "".to_string();
                self.provider = ProviderKind::File(FileProvider);
                let provider_clone = self.provider.clone();
                let h = env!("HOME");
                Task::perform(
                    async move { provider_clone.handler().scan(Path::new(h)) },
                    Message::PopulateEntries,
                )
            }
            _ => Task::none(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PopulateEntries(entries) => {
                self.prompt = "".to_string();
                self.selected_entry = 0;
                self.cached_entries = entries;
                self.ranked_entries = (0..self.cached_entries.len()).collect();

                if !self.preferences.favorite_apps.is_empty() {
                    self.ranked_entries.sort_by_key(|index| {
                        let app = &self.cached_entries[*index];
                        !self.preferences.favorite_apps.contains(&app.id)
                    });
                }

                scrollable::snap_to(SCROLLABLE_ID.clone(), RelativeOffset { x: 0.0, y: 0.0 })
            }
            Message::SaveIntoDisk(result) => {
                match result {
                    Ok(path) => tracing::debug!("Preference saved into disk: {:?}", path),
                    Err(e) => tracing::error!("Failed to save preferences to disk: {}", e),
                }

                Task::none()
            }
            Message::TriggerAction(action) => {
                tracing::debug!(?action, "Action triggered");
                self.handle_action(action)
            }
            Message::TriggerActionByKeybinding(keys, modifiers) => {
                let keystroke = KeyStroke::from_iced_keyboard(keys, modifiers);
                if let Some(action) = self.preferences.keybindings.get(&keystroke) {
                    tracing::debug!(%keystroke, "Keystroke triggered");
                    return self.handle_action(*action);
                }

                Task::none()
            }
            Message::PromptChange(prompt) => {
                self.prompt = prompt;

                if let Some(handle) = self.search_handle.take() {
                    handle.abort();
                }

                if self.prompt == "/" || self.prompt == "@" {
                    return self.swtich_provider();
                }

                if self.prompt.is_empty() {
                    return Task::done(Message::DebouncedFilter);
                }

                let (task, handle) = Task::future(async {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    Message::DebouncedFilter
                })
                .abortable();

                self.search_handle = Some(handle);
                task
            }
            Message::DebouncedFilter => {
                self.selected_entry = 0;
                self.update_ranked_apps();

                scrollable::snap_to(SCROLLABLE_ID.clone(), RelativeOffset { x: 0.0, y: 0.0 })
            }
            Message::ScrollableViewport(viewport) => {
                self.last_viewport = Some(viewport);
                Task::none()
            }
            Message::Close => iced::exit(),

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
        Subscription::batch([event::listen_with(move |event, status, _| {
            match (status, event) {
                (_, Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. })) => {
                    Some(Message::TriggerActionByKeybinding(key, modifiers))
                }
                (event::Status::Ignored, Event::Mouse(mouse::Event::ButtonPressed(_))) => {
                    Some(Message::Close)
                }
                _ => None,
            }
        })])
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

        for (rank_pos, app_index) in self.ranked_entries.iter().enumerate() {
            let entry = &self.cached_entries[*app_index];
            let is_favorite = self.preferences.favorite_apps.contains(&entry.id);
            let is_selected = self.selected_entry == rank_pos;

            let item_height = theme.launchpad.entry.height;
            let style = &self.preferences.theme.launchpad.entry;

            let element = container(entry::display_entry(
                entry,
                &self.icons,
                style,
                rank_pos,
                is_selected,
                is_favorite,
            ))
            .height(item_height)
            .width(Length::Fill);

            // let element: Element<Message, CustomTheme> = container(iced::widget::lazy(
            //     (*app_index, is_selected, is_favorite),
            //     move |_| {
            //         display_entry(
            //             entry,
            //             &self.icons,
            //             style,
            //             rank_pos,
            //             is_selected,
            //             is_favorite,
            //         )
            //     },
            // ))
            // .height(item_height)
            // .width(Length::Fill)
            // .into();

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
            .push_maybe(self.ranked_entries.is_empty().then_some(results_not_found))
            .padding(theme.launchpad.padding)
            .width(Length::Fill);
        let results = iced::widget::scrollable(content)
            .on_scroll(Message::ScrollableViewport)
            .id(SCROLLABLE_ID.clone());

        let prompt = Prompt::new(&self.prompt, &self.preferences.theme)
            .indicator(self.provider_indicator())
            .magnifier(&self.icons.magnifier)
            .id(TEXT_INPUT_ID.clone())
            .on_input(Message::PromptChange)
            .on_submit(Message::TriggerAction(Action::LaunchEntry(
                self.selected_entry,
            )))
            .view();

        container(iced::widget::column![
            prompt,
            iced::widget::horizontal_rule(1),
            container(results),
        ])
        .class(ContainerClass::MainContainer)
    }

    fn provider_indicator<'a>(&'a self) -> Container<'a, Message, CustomTheme> {
        use iced::widget::image;

        let launcher_icon = match self.provider {
            ProviderKind::App(_) => CUBE_ACTIVE,
            _ => CUBE_INACTIVE,
        };

        let terminal_icon = match self.provider {
            ProviderKind::File(_) => FOLDER_ACTIVE,
            _ => FOLDER_INACTIVE,
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
