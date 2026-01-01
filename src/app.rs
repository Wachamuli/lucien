use gio::{Icon, prelude::IconExt};
use iced::{
    Element, Length,
    widget::{
        button::{self},
        row, text,
    },
};
use std::path::PathBuf;

use gio::{AppInfo, AppLaunchContext, prelude::AppInfoExt};

use crate::launcher::Message;

#[derive(Debug)]
pub struct App {
    pub info: AppInfo,
    pub name: String,
    pub description: String,
    pub icon: Option<PathBuf>,
}

pub fn all_apps() -> Vec<App> {
    gio::AppInfo::all()
        .iter()
        .filter(|app| app.should_show())
        .map(|app| App {
            info: app.clone(),
            name: app.name().to_string(),
            description: app.description().unwrap_or_default().to_string(),
            icon: get_icon(app.icon()),
        })
        .collect()
}

fn get_icon_path_from_xdgicon(iconname: &str) -> Option<PathBuf> {
    if iconname.contains("/") || iconname.contains("\\") {
        return Some(PathBuf::from(iconname));
    }

    let scalable_icon_path = xdg::BaseDirectories::with_prefix("icons/hicolor/scalable/apps");

    if let Some(iconpath) = scalable_icon_path.find_data_file(format!("{iconname}.svg")) {
        return Some(iconpath);
    }

    let pixmappath = xdg::BaseDirectories::with_prefix("pixmaps");

    if let Some(iconpath) = pixmappath.find_data_file(format!("{iconname}.svg")) {
        return Some(iconpath);
    }

    if let Some(iconpath) = pixmappath.find_data_file(format!("{iconname}.png")) {
        return Some(iconpath);
    }

    for prefix in [
        "256x256", "128x128", "96x96", "64x64", "48x48", "32x32", "24x24", "16x16",
    ] {
        let iconpath = xdg::BaseDirectories::with_prefix(&format!("icons/hicolor/{prefix}/apps"));
        if let Some(iconpath) = iconpath.find_data_file(format!("{iconname}.png")) {
            return Some(iconpath);
        }
    }

    None
}

pub fn get_icon(icon: Option<Icon>) -> Option<PathBuf> {
    let path = icon?.to_string()?;
    if let Some(xdg_icon_path) = get_icon_path_from_xdgicon(&path) {
        return Some(xdg_icon_path);
    }

    None
}

impl App {
    pub fn launch(&self) {
        if let Err(e) = self.info.launch(&[], AppLaunchContext::NONE) {
            dbg!(e);
        }
    }

    // pub fn view(&self, index: usize) -> Element<Message> {
    //     let i = index;
    //     let file_ext = self
    //         .icon
    //         .as_ref()
    //         .and_then(|path| path.extension())
    //         .and_then(|ext| ext.to_str())
    //         .unwrap_or_default();

    //     let icon_view: Element<Message> = match file_ext {
    //         "svg" => iced::widget::svg(iced::widget::svg::Handle::from_path(
    //             self.icon.clone().unwrap_or_default(),
    //         ))
    //         .width(32)
    //         .height(32)
    //         .into(),
    //         _ => iced::widget::image(iced::widget::image::Handle::from_path(
    //             self.icon.clone().unwrap_or_default(),
    //         ))
    //         .width(32)
    //         .height(32)
    //         .into(),
    //     };

    //     iced::widget::button(
    //         row![
    //             icon_view,
    //             iced::widget::column![
    //                 iced::widget::text(&self.name),
    //                 iced::widget::text(&self.description)
    //                     .width(Length::Fill)
    //                     .size(12)
    //                     .wrapping(text::Wrapping::Glyph)
    //                     .line_height(1.0)
    //             ],
    //         ]
    //         .spacing(10),
    //     )
    //     .on_press(Message::OpenApp(index))
    //     .padding(10)
    //     .style(move |_, status| {
    //         if index == 0 {
    //             return button::Style {
    //                 background: Some(iced::Background::Color(iced::Color::from_rgb(
    //                     0.3, 0.3, 0.3,
    //                 ))),
    //                 text_color: iced::Color::WHITE,
    //                 border: iced::border::rounded(10),
    //                 shadow: Default::default(),
    //             };
    //         }

    //         match status {
    //             button::Status::Hovered => button::Style {
    //                 background: Some(iced::Background::Color(iced::Color::from_rgb(
    //                     0.3, 0.3, 0.3,
    //                 ))),
    //                 text_color: iced::Color::WHITE,
    //                 border: iced::border::rounded(10),
    //                 shadow: Default::default(),
    //             },
    //             _ => button::Style {
    //                 background: Some(iced::Background::Color(iced::color!(0, 0, 0))),
    //                 text_color: iced::Color::WHITE,
    //                 border: iced::border::rounded(20),
    //                 shadow: Default::default(),
    //             },
    //         }
    //     })
    //     .width(Length::Fill)
    //     .into()
    // }
}
