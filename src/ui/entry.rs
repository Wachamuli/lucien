use std::{borrow::Cow, collections::HashMap};

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use iced::{
    Alignment, Element, Font, Length, font,
    widget::{Container, button, container, image, row, space, text},
};

use crate::{
    launcher::{Message, SECTION_HEIGHT},
    preferences::{
        Preferences,
        keybindings::Action,
        theme::{ButtonClass, CustomTheme, Entry as EntryStyle, TextClass},
    },
    providers::Id,
    ui::icon::{ENTER, ICON_PLACEHOLDER, STAR_ACTIVE, STAR_INACTIVE},
};

const CTRL_SHORTCUTS: [&str; 5] = ["Ctrl+1", "Ctrl+2", "Ctrl+3", "Ctrl+4", "Ctrl+5"];

const FONT_BOLD: Font = Font {
    weight: font::Weight::Bold,
    family: font::Family::SansSerif,
    style: font::Style::Normal,
    stretch: font::Stretch::Normal,
};

pub const FONT_ITALIC: Font = Font {
    weight: font::Weight::Normal,
    family: font::Family::SansSerif,
    style: font::Style::Italic,
    stretch: font::Stretch::Normal,
};

#[derive(Debug, Clone)]
pub enum EntryIcon {
    Lazy(String),
    Handle(iced::widget::image::Handle),
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: Id,
    pub main: String,
    pub secondary: Option<String>,
    pub icon: EntryIcon,
}

impl Entry {
    pub fn new(
        id: impl Into<Id>,
        main: impl Into<String>,
        secondary: Option<impl Into<String>>,
        icon: EntryIcon,
    ) -> Self {
        Self {
            id: id.into(),
            main: main.into(),
            secondary: secondary.map(Into::into),
            icon,
        }
    }
}

pub fn display_entry<'a>(
    entry: &'a Entry,
    style: &'a EntryStyle,
    index: usize,
    is_selected: bool,
    is_hovered: bool,
    is_favorite: bool,
) -> Element<'a, Message, CustomTheme> {
    let shortcut_label: Element<'a, Message, CustomTheme> = if is_selected {
        image(ENTER.clone()).width(18).height(18).into()
    } else if index < CTRL_SHORTCUTS.len() {
        text(CTRL_SHORTCUTS[index])
            .size(12)
            .class(TextClass::TextDim)
            .into()
    } else {
        space::horizontal().width(0).into()
    };

    let star_handle = if is_favorite {
        STAR_ACTIVE.clone()
    } else {
        STAR_INACTIVE.clone()
    };

    let mark_favorite: Option<Element<'a, Message, CustomTheme>> = (is_selected || is_hovered)
        .then(|| {
            button(image(star_handle).width(18).height(18))
                .on_press(Message::TriggerAction(Action::ToggleFavorite))
                .class(ButtonClass::Transparent)
                .into()
        });
    let actions = row![]
        .extend(mark_favorite)
        .push(shortcut_label)
        .align_y(Alignment::Center);
    let main = text(&entry.main)
        .size(style.font_size)
        .width(Length::Fill)
        .font(FONT_BOLD);
    let secondary = entry.secondary.as_deref().map(|desc| {
        text(truncate_with_elipsis(desc, 95))
            .size(style.secondary_font_size)
            .class(TextClass::SecondaryText)
            .into()
    });
    let icon_view: Element<'a, Message, CustomTheme> = match &entry.icon {
        EntryIcon::Handle(handle) => image(handle)
            .width(style.icon_size)
            .height(style.icon_size)
            .into(),
        EntryIcon::Lazy(_) => image(ICON_PLACEHOLDER.clone())
            .width(style.icon_size)
            .height(style.icon_size)
            .into(),
    };

    button(
        row![
            icon_view,
            iced::widget::column![main].extend(secondary).spacing(2),
            actions
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
    )
    .on_press(Message::TriggerAction(Action::LaunchEntry(index)))
    .padding(iced::Padding::from(&style.padding))
    .height(style.height)
    .width(Length::Fill)
    .class(if is_selected {
        ButtonClass::ItemlistSelected
    } else {
        ButtonClass::Itemlist
    })
    .into()
}

fn truncate_with_elipsis(text: &str, limit: usize) -> Cow<'_, str> {
    if text.len() <= limit {
        return Cow::Borrowed(text);
    }
    Cow::Owned(format!("{}...", &text[..text.floor_char_boundary(limit)]))
}

pub fn section(name: &str) -> Container<'_, Message, CustomTheme> {
    container(
        text(name)
            .size(14)
            .class(TextClass::TextDim)
            .width(Length::Fill)
            .font(Font {
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

#[derive(Default)]
pub struct EntryRegistry {
    entries: Vec<Entry>,
    projection: Vec<usize>,
    registry: HashMap<Id, usize>,
}

impl EntryRegistry {
    pub fn clear(&mut self) {
        self.entries.clear();
        self.projection.clear();
        self.registry.clear();
    }

    #[allow(dead_code)]
    pub fn push(&mut self, entry: Entry) {
        let id = entry.id.clone();
        let index = self.entries.len();
        self.entries.push(entry);
        self.projection.push(index);
        self.registry.insert(id, index);
    }

    pub fn extend<I>(&mut self, entries: I)
    where
        I: IntoIterator<Item = Entry>,
    {
        for entry in entries {
            let id = entry.id.clone();
            let current_index = self.entries.len();

            self.registry.insert(id, current_index);
            self.entries.push(entry);
            self.projection.push(current_index);
        }
    }

    pub fn get_visible_by_index(&self, visual_index: usize) -> Option<&Entry> {
        let &original_index = self.projection.get(visual_index)?;
        self.entries.get(original_index)
    }

    pub fn get_by_index(&self, index: usize) -> Option<&Entry> {
        self.entries.get(index)
    }

    pub fn get_mut_by_index(&mut self, index: usize) -> Option<&mut Entry> {
        self.entries.get_mut(index)
    }

    #[allow(dead_code)]
    pub fn get_by_id(&self, id: &Id) -> Option<&Entry> {
        if let Some(index) = self.registry.get(id) {
            return self.get_by_index(*index);
        }

        None
    }

    pub fn get_mut_by_id(&mut self, id: &Id) -> Option<&mut Entry> {
        if let Some(index) = self.registry.get(id) {
            return self.get_mut_by_index(*index);
        }

        None
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn visible_len(&self) -> usize {
        self.projection.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn is_visibles_empty(&self) -> bool {
        self.projection.is_empty()
    }

    pub fn iter_visible(&self) -> impl Iterator<Item = &Entry> {
        self.projection.iter().map(|&index| &self.entries[index])
    }

    pub fn sort_by_rank(
        &mut self,
        preferences: &Preferences,
        matcher: &SkimMatcherV2,
        pattern: &str,
    ) {
        let mut ranked: Vec<(i64, usize)> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let score = matcher.fuzzy_match(&entry.main, pattern)?;
                Some((score, index))
            })
            .collect();

        ranked.sort_by(|(score_a, index_a), (score_b, index_b)| {
            let entry_a = &self.entries[*index_a];
            let entry_b = &self.entries[*index_b];
            let a_is_fav = preferences
                .favorite_apps
                .contains(&entry_a.id.to_string_lossy().into_owned());
            let b_is_fav = preferences
                .favorite_apps
                .contains(&entry_b.id.to_string_lossy().into_owned());

            match (a_is_fav, b_is_fav) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => score_b.cmp(score_a),
            }
        });

        self.projection = ranked
            .into_iter()
            .map(|(_score, app_index)| app_index)
            .collect();
    }
}
