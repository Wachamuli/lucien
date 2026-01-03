use gio::{Icon, prelude::IconExt};
use iced::{
    Alignment, Element, Length,
    widget::{Button, button, image, row, svg, text},
};
use std::path::PathBuf;

use gio::{AppInfo, AppLaunchContext, prelude::AppInfoExt};

use crate::launcher::Message;

#[derive(Debug, Clone)]
pub enum IconHandle {
    Svg(svg::Handle),
    Raster(image::Handle),
}

#[derive(Debug, Clone)]
pub struct App {
    pub info: AppInfo,
    pub name: String,
    pub description: String,
    pub icon: Option<IconHandle>,
}

pub fn all_apps() -> Vec<App> {
    gio::AppInfo::all()
        .iter()
        .filter(|app| app.should_show())
        .map(|app| App {
            info: app.clone(),
            name: app.name().to_string(),
            description: app.description().unwrap_or_default().to_string(),
            icon: load_icon_handle(app.icon()),
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

fn load_icon_handle(icon: Option<Icon>) -> Option<IconHandle> {
    let path = icon?.to_string()?;
    let path = get_icon_path_from_xdgicon(&path)?;
    let extension = path.extension()?.to_str()?.to_lowercase();

    match extension.as_str() {
        "svg" => Some(IconHandle::Svg(svg::Handle::from_path(path))),
        "png" | "jpg" | "jpeg" => Some(IconHandle::Raster(image::Handle::from_path(path))),
        _ => None,
    }
}

impl App {
    pub fn launch(&self) {
        if let Err(e) = self.info.launch(&[], AppLaunchContext::NONE) {
            dbg!(e);
        }
    }

    pub fn itemlist<'a>(&'a self, current_index: usize, index: usize) -> Button<'a, Message> {
        let icon_view: Element<Message> = match &self.icon {
            Some(IconHandle::Svg(handle)) => svg(handle.clone()).width(32).height(32).into(),
            Some(IconHandle::Raster(handle)) => image(handle.clone()).width(32).height(32).into(),
            None => iced::widget::horizontal_space().width(32).into(),
        };

        let shortcut_label = match index {
            n if n == current_index => "Enter".to_string(),
            n @ 0..7 => format!("Alt-{}", n + 1),
            _ => "".to_string(),
        };

        button(
            row![
                icon_view,
                iced::widget::column![
                    text(&self.name).size(14),
                    text(&self.description)
                        .size(11)
                        .color(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.5))
                        .width(Length::Fill),
                ]
                .spacing(2),
                text(shortcut_label)
                    .size(11)
                    .color(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.5))
                    .align_x(Alignment::End),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::OpenApp(index))
        .padding(10)
        .width(Length::Fill)
    }
}
