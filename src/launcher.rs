use iced::{
    Element,
    widget::{self, mouse_area, operation::AbsoluteOffset},
};
use std::{
    env,
    path::PathBuf,
    sync::{Arc, LazyLock},
    usize,
};

use fuzzy_matcher::skim::SkimMatcherV2;
use iced::{
    Alignment, Length, Subscription, Task,
    widget::{
        Column, Container, container, image, row,
        scrollable::{RelativeOffset, Viewport},
        text,
    },
};
use iced_layershell::to_layer_message;

use crate::{
    preferences::{
        self, Preferences,
        keybindings::{Action, Keystrokes},
        theme::{ContainerClass, CustomTheme, TextClass},
    },
    providers::{Context, Id, ProviderKind, ScannerState, app::AppProvider, file::FileProvider},
    ui::{
        self,
        entry::{EntryIcon, EntryRegistry, FONT_ITALIC, section},
        icon::{CUBE_ACTIVE, CUBE_INACTIVE, FOLDER_ACTIVE, FOLDER_INACTIVE, MAGNIFIER},
        prompt::Prompt,
    },
};

// TODO: Remove this constant
pub const SECTION_HEIGHT: f32 = 36.0;

static TEXT_INPUT_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);
static SCROLLABLE_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);

pub struct Lucien {
    entry_registry: EntryRegistry,
    context: Context,
    is_scan_completed: bool,
    provider: ProviderKind,
    prompt: String,
    matcher: SkimMatcherV2,
    preferences: Preferences,
    selected_entry: usize,
    hovered_entry: usize,
    last_viewport: Option<Viewport>,
    search_handle: Option<iced::task::Handle>,
    scanner_tx: Option<iced::futures::channel::mpsc::Sender<Context>>,
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    RequestContext(iced::futures::channel::mpsc::Sender<Context>),
    ContextChange(Context),
    ScanEvent(ScannerState),
    PromptChange(String),
    DebouncedFilter,
    TriggerAction(Action),
    TriggerActionByKeybinding(Keystrokes),
    ScrollableViewport(Viewport),
    SaveIntoDisk(Result<PathBuf, Arc<tokio::io::Error>>),
    IconResolved { id: Id, handle: image::Handle },
    HoveredEntry(usize),
    HoveredExit(usize),
}

impl Lucien {
    pub fn new(preferences: Preferences) -> (Self, Task<Message>) {
        let default_provider = ProviderKind::App(AppProvider);
        let context = Context {
            path: PathBuf::from(env!("HOME")),
            pattern: String::new(),
            scan_batch_size: preferences.scan_batch_size,
            icon_size: preferences.theme.launchpad.entry.icon_size,
        };

        let initial_values = Self {
            selected_entry: 0,
            hovered_entry: 0,
            entry_registry: EntryRegistry::default(),
            is_scan_completed: false,
            context,
            provider: default_provider,
            prompt: String::new(),
            matcher: SkimMatcherV2::default(),
            preferences,
            last_viewport: None,
            search_handle: None,
            scanner_tx: None,
        };

        (initial_values, Task::none())
    }

    pub fn theme(&self) -> CustomTheme {
        self.preferences.theme.clone()
    }

    fn toggle_favorite(&mut self, index: usize) -> Task<Message> {
        let Some(app) = self.entry_registry.get_visible_by_index(index) else {
            return Task::none();
        };

        let Some(ref path) = self.preferences.path else {
            tracing::warn!("In-memory defaults. Settings will not be saved");
            return Task::none();
        };

        let id = &app.id;
        let path = path.clone();
        // Toggle_favorite is a very opaque function. It actually
        // modifies the in-memory favorite_apps variable.
        // Maybe I should expose this assignnment operation at this level.
        let favorite_apps = self.preferences.toggle_favorite(id);
        self.entry_registry
            .sort_by_rank(&self.preferences, &self.matcher, &self.prompt);

        Task::perform(
            preferences::save_into_disk(path, "favorite_apps", favorite_apps),
            Message::SaveIntoDisk,
        )
    }

    fn go_to_entry(&mut self, step: isize) -> Task<Message> {
        let total = self.entry_registry.visible_len();
        if total == 0 {
            return Task::none();
        }

        let old_pos = self.selected_entry;
        self.selected_entry = wrapped_index(self.selected_entry, total, step);

        if old_pos != self.selected_entry {
            let layout = AppLayout::new(&self.preferences, &self.prompt);
            return self.snap_to_entry(&layout);
        }

        Task::none()
    }

    fn launch_entry(&self, index: usize) -> Task<Message> {
        if let Some(entry) = &self.entry_registry.get_visible_by_index(index) {
            return self.provider.handler().launch(&entry.id);
        };

        Task::none()
    }

    fn handle_action(&mut self, action: Action) -> Task<Message> {
        tracing::debug!(?action, "Action triggered");
        match action {
            Action::Close => iced::exit(),
            Action::NextEntry => self.go_to_entry(1),
            Action::PreviousEntry => self.go_to_entry(-1),
            Action::ToggleFavorite => self.toggle_favorite(self.selected_entry),
            Action::LaunchEntry(index) => self.launch_entry(index),
        }
    }

    pub fn snap_to_entry(&self, layout: &AppLayout) -> Task<Message> {
        let Some(viewport) = &self.last_viewport else {
            return Task::none();
        };

        let Some(entry) = self.entry_registry.get_by_index(self.selected_entry) else {
            return Task::none();
        };

        // 1. Get coordinates from injected layout
        let is_fav = self.preferences.favorite_apps.contains(&entry.id);
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

        widget::operation::snap_to(
            SCROLLABLE_ID.clone(),
            RelativeOffset {
                x: 0.0,
                y: (y.clamp(0.0, max_scroll)) / max_scroll,
            },
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RequestContext(mut sender) => {
                // This should be async
                let _ = sender.try_send(self.context.clone());
                self.scanner_tx = Some(sender);
                Task::none()
            }
            Message::ContextChange(context) => {
                let Some(tx) = &mut self.scanner_tx else {
                    return Task::none();
                };

                self.context = context;

                self.entry_registry.clear();
                self.selected_entry = 0;
                self.prompt = String::new();

                let _ = tx.try_send(self.context.clone());

                widget::operation::scroll_to(
                    SCROLLABLE_ID.clone(),
                    AbsoluteOffset { x: 0.0, y: 0.0 },
                )
            }
            Message::ScanEvent(scan_event) => {
                match scan_event {
                    ScannerState::Started => {
                        self.prompt.clear();
                        self.selected_entry = 0;
                        self.entry_registry.clear();

                        self.is_scan_completed = false;
                        // let _ = tx.try_send(self.context.clone());
                        // self.scanner_tx = Some(tx);

                        // TODO: Move to an Init message, because the prompt should be ready
                        // instantly.
                        widget::operation::focus(TEXT_INPUT_ID.clone())
                    }
                    ScannerState::Found(batch) => {
                        self.entry_registry.extend(batch);
                        Task::none()
                    }
                    ScannerState::Finished => {
                        self.is_scan_completed = true;
                        Task::none()
                    }
                    ScannerState::Errored(id, error) => {
                        tracing::error!(error = error, "An error ocurred while scanning {id}");
                        Task::none()
                    }
                }
            }
            Message::IconResolved { id, handle } => {
                if let Some(entry) = self.entry_registry.get_mut_by_id(&id) {
                    entry.icon = EntryIcon::Handle(handle);
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
            Message::TriggerAction(action) => self.handle_action(action),
            Message::TriggerActionByKeybinding(keystrokes) => {
                tracing::debug!(%keystrokes, "Keystrokes triggered");
                if let Some(action) = self.preferences.keybindings.get(&keystrokes) {
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
                self.entry_registry
                    .sort_by_rank(&self.preferences, &self.matcher, &self.prompt);

                widget::operation::scroll_to(
                    SCROLLABLE_ID.clone(),
                    AbsoluteOffset { x: 0.0, y: 0.0 },
                )
            }
            Message::ScrollableViewport(viewport) => {
                self.last_viewport = Some(viewport);
                Task::none()
            }
            Message::HoveredEntry(index) => {
                self.hovered_entry = index;
                Task::none()
            }
            Message::HoveredExit(index) => {
                if self.hovered_entry == index {
                    self.hovered_entry = self.selected_entry;
                }
                Task::none()
            }
            Message::SetInputRegion(_action_callback) => todo!(),
            Message::AnchorChange(_anchor) => todo!(),
            Message::AnchorSizeChange(_anchor, _) => todo!(),
            Message::LayerChange(_layer) => todo!(),
            Message::MarginChange(_) => todo!(),
            Message::SizeChange(_) => todo!(),
            Message::VirtualKeyboardPressed { .. } => todo!(),
            Message::ExclusiveZoneChange(_) => todo!(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        use iced::Event::{Keyboard as IcedKeyboardEvent, Window as IcedWindowEvent};
        use iced::window;
        use iced::{event, keyboard};

        Subscription::batch([
            self.provider.handler().scan(),
            event::listen_with(move |event, _, _| match event {
                IcedKeyboardEvent(keyboard::Event::KeyPressed { modifiers, key, .. }) => {
                    let keystrokes = Keystrokes::from_iced_keystrokes(modifiers, key);
                    Some(Message::TriggerActionByKeybinding(keystrokes))
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
        let item_height = theme.launchpad.entry.height;
        let style = &self.preferences.theme.launchpad.entry;
        let show_sections = self.prompt.is_empty() && !self.preferences.favorite_apps.is_empty();

        let mut starred_column =
            Column::new().extend(show_sections.then(|| section("Starred").into()));
        let mut general_column =
            Column::new().extend(show_sections.then(|| section("General").into()));

        for (index, entry) in self.entry_registry.iter_visible().enumerate() {
            let is_favorite = self.preferences.favorite_apps.contains(&entry.id);
            let is_selected = self.selected_entry == index;
            let is_hovered = self.hovered_entry == index;

            let entry_view = mouse_area(
                container(ui::entry::display_entry(
                    &entry,
                    style,
                    index,
                    is_selected,
                    is_hovered,
                    is_favorite,
                ))
                .height(item_height)
                .width(Length::Fill),
            )
            .on_enter(Message::HoveredEntry(index))
            .on_exit(Message::HoveredExit(index));

            if is_favorite && self.prompt.is_empty() {
                starred_column = starred_column.push(entry_view);
            } else {
                general_column = general_column.push(entry_view);
            }
        }

        let results_not_found = (self.entry_registry.is_visibles_empty() && self.is_scan_completed)
            .then(|| {
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
            })
            .map(Element::from);

        let show_results = !self.entry_registry.is_empty() || self.is_scan_completed;

        let content = Column::new()
            .push(starred_column)
            .push(general_column)
            .extend(results_not_found)
            .padding(theme.launchpad.padding)
            .width(Length::Fill);

        let results = show_results
            .then(|| {
                iced::widget::scrollable(content)
                    .on_scroll(Message::ScrollableViewport)
                    .id(SCROLLABLE_ID.clone())
            })
            .map(Element::from);

        let horizontal_rule = show_results
            .then(|| widget::rule::horizontal(1))
            .map(Element::from);

        let prompt = Prompt::new(&self.prompt, &self.preferences.theme)
            .indicator(self.provider_indicator())
            .magnifier(MAGNIFIER.clone())
            .id(TEXT_INPUT_ID.clone())
            .on_input(Message::PromptChange)
            .on_submit(Message::TriggerAction(Action::LaunchEntry(
                self.selected_entry,
            )))
            .view();

        container(
            iced::widget::column![prompt]
                .extend(horizontal_rule)
                .extend(results),
        )
        .class(ContainerClass::MainContainer)
    }

    fn provider_indicator<'a>(&'a self) -> Container<'a, Message, CustomTheme> {
        // let apps_icon = match self.provider {
        //     ProviderKind::App(_) => CUBE_ACTIVE.clone(),
        //     _ => CUBE_INACTIVE.clone(),
        // };
        let folder_icon = match self.provider {
            ProviderKind::File(_) => FOLDER_ACTIVE.clone(),
            _ => FOLDER_INACTIVE.clone(),
        };

        container(
            row![
                // image(apps_icon).width(18).height(18),
                image(folder_icon).width(18).height(18),
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
