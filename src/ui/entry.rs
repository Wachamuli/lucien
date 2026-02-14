use std::collections::HashMap;

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use iced::{
    Alignment, Element, Font, Length, font,
    widget::{Container, button, container, image, row, text},
};

use crate::{
    launcher::{Message, SECTION_HEIGHT},
    preferences::{
        Preferences,
        keybindings::Action,
        theme::{ButtonClass, CustomTheme, Entry as EntryStyle, TextClass},
    },
    providers::{Entry, EntryIcon, Id},
    ui::icon::BakedIcons,
};

const CTRL_SHORTCUTS: [&str; 5] = ["Ctrl+1", "Ctrl+2", "Ctrl+3", "Ctrl+4", "Ctrl+5"];

// Maybe unnecesarry constant, this is stack based. (but nice to have ergonomic?)
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

pub fn display_entry<'a>(
    entry: &'a Entry,
    baked_icons: &'a BakedIcons,
    style: &'a EntryStyle,
    visual_index: usize,
    is_selected: bool,
    is_favorite: bool,
) -> Element<'a, Message, CustomTheme> {
    let shortcut_label: Element<'a, Message, CustomTheme> = if is_selected {
        image(&baked_icons.enter).width(18).height(18).into()
    } else if visual_index < CTRL_SHORTCUTS.len() {
        text(CTRL_SHORTCUTS[visual_index])
            .size(12)
            .class(TextClass::TextDim)
            .into()
    } else {
        text("").into()
    };

    let star_handle = if is_favorite {
        &baked_icons.star_active
    } else {
        &baked_icons.star_inactive
    };

    let mark_favorite: Option<Element<'a, Message, CustomTheme>> = is_selected.then(|| {
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
        .size(style.font_size as u32)
        .width(Length::Fill)
        .font(FONT_BOLD);
    let secondary = entry.secondary.as_ref().map(|desc| {
        text(desc)
            .size(style.secondary_font_size as u32)
            .class(TextClass::SecondaryText)
            .into()
    });
    let icon_view: Element<'a, Message, CustomTheme> = match &entry.icon {
        EntryIcon::Handle(handle) => image(handle)
            .width(style.icon_size as u32)
            .height(style.icon_size as u32)
            .into(),
        EntryIcon::Lazy(_) => image(&baked_icons.icon_placeholder)
            .width(style.icon_size as u32)
            .height(style.icon_size as u32)
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
    .on_press(Message::TriggerAction(Action::LaunchEntry(visual_index)))
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
    entry_indices: Vec<usize>,
    entry_index_map: HashMap<Id, usize>,
}

impl EntryRegistry {
    pub fn clear(&mut self) {
        self.entries.clear();
        self.entry_indices.clear();
        self.entry_index_map.clear();
    }

    #[allow(dead_code)]
    pub fn push(&mut self, entry: Entry) {
        let id = entry.id.clone();
        self.entries.push(entry);
        let index = self.entries.len();
        self.entry_indices.push(index);
        self.entry_index_map.insert(id, index);
    }

    pub fn extend<I>(&mut self, entries: I)
    where
        I: IntoIterator<Item = Entry>,
    {
        let mut current_index = self.entries.len();

        for entry in entries {
            let id = entry.id.clone();

            self.entry_index_map.insert(id, current_index);
            self.entries.push(entry);
            self.entry_indices.push(current_index);

            current_index += 1;
        }
    }

    pub fn get_by_index(&self, index: usize) -> Option<&Entry> {
        self.entries.get(index)
    }

    pub fn get_mut_by_index(&mut self, index: usize) -> Option<&mut Entry> {
        self.entries.get_mut(index)
    }

    #[allow(dead_code)]
    pub fn get_by_id(&mut self, id: &Id) -> Option<&Entry> {
        if let Some(index) = self.entry_index_map.get(id) {
            return self.get_by_index(*index);
        }

        None
    }

    pub fn get_mut_by_id(&mut self, id: &Id) -> Option<&mut Entry> {
        if let Some(index) = self.entry_index_map.get(id) {
            return self.get_mut_by_index(*index);
        }

        None
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn visible_len(&self) -> usize {
        self.entry_indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn is_visibles_empty(&self) -> bool {
        self.entry_indices.is_empty()
    }

    pub fn iter_visible(&self) -> impl Iterator<Item = &Entry> {
        self.entry_indices
            .iter()
            .filter_map(|&original_idx| self.entries.get(original_idx))
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
            let a_is_fav = preferences.favorite_apps.contains(&entry_a.id);
            let b_is_fav = preferences.favorite_apps.contains(&entry_b.id);

            match (a_is_fav, b_is_fav) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => score_b.cmp(score_a),
            }
        });

        self.entry_indices = ranked
            .into_iter()
            .map(|(_score, app_index)| app_index)
            .collect();
    }
}
