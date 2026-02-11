use std::{
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
};

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use iced::{
    Alignment, Length, Subscription, Task,
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
    providers::{Entry, ProviderKind, ScanState, app::AppProvider, file::FileProvider},
    ui::{
        entry::{self, FONT_ITALIC, section},
        icon::{
            BakedIcons, CUBE_ACTIVE, CUBE_INACTIVE, ENTER, FOLDER_ACTIVE, FOLDER_INACTIVE,
            MAGNIFIER, STAR_ACTIVE, STAR_INACTIVE,
        },
        prompt::Prompt,
    },
};

// TODO: Remove this constant
pub const SECTION_HEIGHT: f32 = 36.0;

static TEXT_INPUT_ID: LazyLock<text_input::Id> = LazyLock::new(text_input::Id::unique);
static SCROLLABLE_ID: LazyLock<scrollable::Id> = LazyLock::new(scrollable::Id::unique);

pub struct Lucien {
    is_scan_completed: bool,
    provider: ProviderKind,
    prompt: String,
    matcher: SkimMatcherV2,
    cached_entries: Vec<Entry>,
    entries: Vec<usize>,
    preferences: Preferences,
    selected_entry: usize,
    last_viewport: Option<Viewport>,
    search_handle: Option<iced::task::Handle>,
    icons: BakedIcons,
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    ScanEvent(ScanState),
    PromptChange(String),
    DebouncedFilter,
    TriggerAction(Action),
    TriggerActionByKeybinding(iced::keyboard::Key, iced::keyboard::Modifiers),
    ScrollableViewport(Viewport),
    SaveIntoDisk(Result<PathBuf, Arc<tokio::io::Error>>),
}

impl Lucien {
    pub fn new(preferences: Preferences) -> (Self, Task<Message>) {
        let auto_focus_prompt_task = text_input::focus(TEXT_INPUT_ID.clone());
        let default_provider = ProviderKind::App(AppProvider);

        let baked_icons = BakedIcons {
            enter: image::Handle::from_bytes(ENTER),
            magnifier: image::Handle::from_bytes(MAGNIFIER),
            star_active: image::Handle::from_bytes(STAR_ACTIVE),
            star_inactive: image::Handle::from_bytes(STAR_INACTIVE),
        };

        let initial_values = Self {
            is_scan_completed: false,
            provider: default_provider,
            prompt: String::new(),
            matcher: SkimMatcherV2::default(),
            cached_entries: Vec::new(),
            entries: Vec::new(),
            preferences,
            selected_entry: 0,
            last_viewport: None,
            search_handle: None,
            icons: baked_icons,
        };

        (initial_values, auto_focus_prompt_task)
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

        self.entries = ranked
            .into_iter()
            .map(|(_score, app_index)| app_index)
            .collect();
    }

    fn toggle_favorite(&mut self, index: usize) -> Task<Message> {
        let Some(app) = self.entries.get(index) else {
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
        let total = self.entries.len();
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
        let Some(entry_index) = self.entries.get(index) else {
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
            Action::ToggleFavorite => self.toggle_favorite(self.selected_entry),
            Action::LaunchEntry(index) => self.launch_entry(index),
        }
    }

    pub fn snap_if_needed(&self, layout: &AppLayout) -> Task<Message> {
        let Some(viewport) = &self.last_viewport else {
            return Task::none();
        };

        // 1. Get coordinates from injected layout
        let entry_index = self.entries[self.selected_entry];
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

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ScanEvent(scan_event) => {
                match scan_event {
                    ScanState::Started => {
                        self.prompt.clear();
                        self.cached_entries.clear();
                        self.entries.clear();
                        self.is_scan_completed = false;
                    }
                    ScanState::Found(entry_batch) => {
                        let batch_len = entry_batch.len();
                        let start_index = self.cached_entries.len();

                        self.cached_entries.extend(entry_batch);
                        self.entries.extend(start_index..start_index + batch_len);

                        if !self.preferences.favorite_apps.is_empty() {
                            self.entries.sort_by_key(|index| {
                                let app = &self.cached_entries[*index];
                                !self.preferences.favorite_apps.contains(&app.id)
                            });
                        }
                    }
                    ScanState::Finished => {
                        self.is_scan_completed = true;
                    }
                    ScanState::Errored(id, error) => {
                        tracing::error!(error = error, "An error ocurred while scanning {id}");
                    }
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

                if self.prompt.is_empty() {
                    return Task::done(Message::DebouncedFilter);
                }

                match self.prompt.as_str() {
                    "@" => {
                        self.prompt.clear();
                        self.provider = ProviderKind::App(AppProvider)
                    }
                    "/" => {
                        self.prompt.clear();
                        self.provider = ProviderKind::File(FileProvider)
                    }
                    _ => {}
                };

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
            Message::SetInputRegion(_action_callback) => todo!(),
            Message::AnchorChange(_anchor) => todo!(),
            Message::AnchorSizeChange(_anchor, _) => todo!(),
            Message::LayerChange(_layer) => todo!(),
            Message::MarginChange(_) => todo!(),
            Message::SizeChange(_) => todo!(),
            Message::VirtualKeyboardPressed { .. } => todo!(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        use iced::Event::{Keyboard as IcedKeyboardEvent, Window as IcedWindowEvent};
        use iced::window;
        use iced::{event, keyboard};

        Subscription::batch([
            self.provider.handler().scan(PathBuf::from(env!("HOME"))),
            event::listen_with(move |event, _, _| match event {
                IcedKeyboardEvent(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                    Some(Message::TriggerActionByKeybinding(key, modifiers))
                }
                IcedWindowEvent(window::Event::Unfocused) => {
                    Some(Message::TriggerAction(Action::Close))
                }
                _ => None,
            }),
        ])
    }

    pub fn view(&self) -> Container<'_, Message, CustomTheme> {
        let theme = &self.preferences.theme;
        let show_sections = self.prompt.is_empty() && !self.preferences.favorite_apps.is_empty();

        let mut starred_column =
            Column::new().push_maybe(show_sections.then(|| section("Starred")));
        let mut general_column =
            Column::new().push_maybe(show_sections.then(|| section("General")));

        for (rank_pos, app_index) in self.entries.iter().enumerate() {
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

        let results_not_found: Option<Container<Message, CustomTheme>> =
            (self.entries.is_empty() && self.is_scan_completed).then(|| {
                container(
                    text("No Results Found")
                        .size(14)
                        .class(TextClass::TextDim)
                        .width(Length::Fill)
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                        .font(FONT_ITALIC),
                )
                .padding(19.8)
            });

        let show_results = !self.entries.is_empty() || self.is_scan_completed;

        let content = Column::new()
            .push(starred_column)
            .push(general_column)
            .push_maybe(results_not_found)
            .padding(theme.launchpad.padding)
            .width(Length::Fill);

        let results = show_results.then(|| {
            iced::widget::scrollable(content)
                .on_scroll(Message::ScrollableViewport)
                .id(SCROLLABLE_ID.clone())
        });

        let horizontal_rule = show_results.then(|| iced::widget::horizontal_rule(1));

        let prompt = Prompt::new(&self.prompt, &self.preferences.theme)
            .indicator(self.provider_indicator())
            .magnifier(&self.icons.magnifier)
            .id(TEXT_INPUT_ID.clone())
            .on_input(Message::PromptChange)
            .on_submit(Message::TriggerAction(Action::LaunchEntry(
                self.selected_entry,
            )))
            .view();

        container(
            iced::widget::column![prompt]
                .push_maybe(horizontal_rule)
                .push_maybe(results),
        )
        .class(ContainerClass::MainContainer)
    }

    fn provider_indicator<'a>(&'a self) -> Container<'a, Message, CustomTheme> {
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
