use iced::{
    Alignment, Element, Font, Length, font,
    widget::{Container, button, container, image, row, text},
};

use crate::{
    launcher::{Message, SECTION_HEIGHT},
    preferences::{
        keybindings::Action,
        theme::{ButtonClass, CustomTheme, Entry as EntryStyle, TextClass},
    },
    providers::Entry,
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
    index: usize,
    is_selected: bool,
    is_favorite: bool,
) -> Element<'a, Message, CustomTheme> {
    let shortcut_label: Element<'a, Message, CustomTheme> = if is_selected {
        image(&baked_icons.enter).width(18).height(18).into()
    } else if index < CTRL_SHORTCUTS.len() {
        text(CTRL_SHORTCUTS[index])
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
        .push_maybe(mark_favorite)
        .push(shortcut_label)
        .align_y(Alignment::Center);
    let main = text(&entry.main)
        .size(style.font_size)
        .width(Length::Fill)
        .font(FONT_BOLD);
    let secondary = entry.secondary.as_ref().map(|desc| {
        text(desc)
            .size(style.secondary_font_size)
            .class(TextClass::SecondaryText)
    });

    let icon_view: Element<'a, Message, CustomTheme> = image(&entry.icon)
        .width(style.icon_size)
        .height(style.icon_size)
        .into();

    button(
        row![
            icon_view,
            iced::widget::column![main].push_maybe(secondary).spacing(2),
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
