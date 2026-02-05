use iced::{
    Alignment, Element, Length,
    widget::{button, image, row, text},
};

use crate::{
    launcher::Message,
    preferences::theme::{ButtonClass, CustomTheme, Entry as EntryStyle, TextClass},
    providers::Entry,
    ui::icon::BakedIcons,
};

pub fn display_entry<'a>(
    entry: &'a Entry,
    icon: Option<image::Handle>,
    baked_icons: &'a BakedIcons,
    style: &'a EntryStyle,
    index: usize,
    is_selected: bool,
    is_favorite: bool,
) -> Element<'a, Message, CustomTheme> {
    let shortcut_widget: Element<'a, Message, CustomTheme> =
        image(&baked_icons.enter).width(18).height(18).into();

    let shortcut_label: Element<'a, Message, CustomTheme> = if is_selected {
        shortcut_widget
    } else if index < 5 {
        text(format!("Alt+{}", index + 1))
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

    let mark_favorite: Element<'a, Message, CustomTheme> =
        button(image(star_handle).width(18).height(18))
            .on_press(Message::MarkFavorite(index))
            .class(ButtonClass::Transparent)
            .into();

    let actions = row![]
        .push_maybe(is_selected.then_some(mark_favorite))
        .push(shortcut_label)
        .align_y(Alignment::Center);

    let description = entry.secondary.as_ref().map(|desc| {
        text(desc)
            .size(style.secondary_font_size)
            .class(TextClass::SecondaryText)
    });

    let icon_view: Element<'a, Message, CustomTheme> = match icon {
        Some(handle) => image(handle)
            .width(style.icon_size)
            .height(style.icon_size)
            .into(),
        None => iced::widget::horizontal_space().width(0).into(),
    };

    button(
        row![
            icon_view,
            iced::widget::column![
                text(&entry.main)
                    .size(style.font_size)
                    .width(Length::Fill)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    }),
            ]
            .push_maybe(description)
            .spacing(2),
            actions
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
    )
    .on_press(Message::LaunchEntry(index))
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
