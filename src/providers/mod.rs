use iced::{
    Alignment, Element, Length,
    widget::{button, image, row, text},
};

use crate::{
    launcher::{BakedIcons, Message},
    preferences::theme::{ButtonClass, CustomTheme, Entry as EntryStyle, TextClass},
    providers::{app::App, file::File},
};

pub mod app;
pub mod file;

pub trait Provider {
    fn scan() -> Vec<AnyEntry>;
}

#[derive(Debug, Clone)]
pub enum AnyEntry {
    AppEntry(App),
    FileEntry(File),
}

impl Entry for AnyEntry {
    fn id(&self) -> &str {
        match self {
            AnyEntry::AppEntry(app) => &app.id,
            AnyEntry::FileEntry(file) => file.path.to_str().unwrap_or_default(),
        }
    }

    fn main(&self) -> &str {
        match self {
            AnyEntry::AppEntry(app) => &app.name,
            AnyEntry::FileEntry(file) => file
                .path
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default(),
        }
    }

    fn secondary(&self) -> Option<&str> {
        match self {
            AnyEntry::AppEntry(app) => app.description.as_deref(),
            AnyEntry::FileEntry(file) => file.path.to_str(),
        }
    }

    fn launch(&self) -> anyhow::Result<()> {
        match self {
            AnyEntry::AppEntry(app) => {
                let _ = app.launch();
                Ok(())
            }
            AnyEntry::FileEntry(file) => {
                let _ = file.launch();
                Ok(())
            }
        }
    }
}

pub enum ProviderKind {
    Apps,
}

pub trait Entry {
    fn id(&self) -> &str;
    fn main(&self) -> &str;
    fn secondary(&self) -> Option<&str>;
    fn launch(&self) -> anyhow::Result<()>;
}

pub fn display_entry<'a>(
    entry: &'a impl Entry,
    icons: &'a BakedIcons,
    style: &'a EntryStyle,
    index: usize,
    is_selected: bool,
    is_favorite: bool,
) -> Element<'a, Message, CustomTheme> {
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

    let shortcut_widget: Element<'a, Message, CustomTheme> = match &icons.enter {
        Some(handle) => image(handle).width(18).height(18).into(),
        None => iced::widget::horizontal_space().width(18).height(18).into(),
    };

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
        &icons.star_active
    } else {
        &icons.star_inactive
    };

    let mark_favorite: Element<'a, Message, CustomTheme> = match star_handle {
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

    let description = entry.secondary().map(|desc| {
        text(desc)
            .size(style.secondary_font_size)
            .class(TextClass::SecondaryText)
    });

    button(
        row![
            // icon_view,
            iced::widget::column![
                text(entry.main())
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
