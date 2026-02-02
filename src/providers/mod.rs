use iced::{
    Alignment, Element, Length,
    widget::{button, image, row, text},
};

use crate::{
    launcher::{BakedIcons, Message},
    preferences::theme::{ButtonClass, CustomTheme, Entry as EntryStyle, TextClass},
};

pub mod app;
pub mod file;

pub trait Provider {
    type Entry: Entry;

    fn scan() -> Vec<Self::Entry>;
}

pub trait Entry {
    fn id(&self) -> String;
    fn main(&self) -> String;
    fn secondary(&self) -> Option<String>;
    fn launch(&self) -> anyhow::Result<()>;
}

pub fn display_entry(
    entry: &impl Entry,
    icons: &BakedIcons,
    style: &EntryStyle,
    index: usize,
    current_index: usize,
    is_favorite: bool,
) -> Element<'static, Message, CustomTheme> {
    let is_selected = current_index == index;

    // let icon_view: Element<'_, Message, CustomTheme> = if let Some(icon_path) = &self.icon {
    //     match app::load_icon_sync(icon_path) {
    //         Some(handle) => image(handle)
    //             .width(style.icon_size)
    //             .height(style.icon_size)
    //             .into(),
    //         None => iced::widget::horizontal_space().width(0).into(),
    //     }
    // } else {
    //     iced::widget::horizontal_space().width(0).into()
    // };

    let shortcut_widget: Element<'static, Message, CustomTheme> = match &icons.enter {
        Some(handle) => image(handle).width(18).height(18).into(),
        None => iced::widget::horizontal_space().width(18).height(18).into(),
    };

    let shortcut_label: Element<'static, Message, CustomTheme> = if is_selected {
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
        &icons.star_active
    } else {
        &icons.star_inactive
    };

    let mark_favorite: Element<'static, Message, CustomTheme> = match star_handle {
        Some(handle) => button(image(handle).width(18).height(18))
            .on_press(Message::MarkFavorite(index))
            .class(ButtonClass::Transparent)
            .into(),
        None => iced::widget::horizontal_space().width(18).height(18).into(),
    };

    let actions = row![]
        .push_maybe(is_selected.then_some(mark_favorite))
        .push(shortcut_label)
        .align_y(Alignment::Center);

    let description = entry.secondary().as_ref().map(|desc| {
        text(desc.clone())
            .size(style.secondary_font_size)
            .class(TextClass::SecondaryText)
    });

    button(
        row![
            // icon_view,
            iced::widget::column![
                text(entry.main().clone())
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
    .on_press(Message::LaunchApp(index))
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
