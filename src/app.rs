use gio::prelude::AppInfoExt;
use gio::prelude::IconExt;
use iced::Alignment;
use iced::{
    Element, Length,
    widget::{button, image, row, text},
};
use resvg::{tiny_skia, usvg};
use std::{io, os::unix::process::CommandExt, path::PathBuf, process};

use crate::launcher::BakedIcons;
use crate::launcher::Message;
use crate::theme::ButtonClass;
use crate::theme::CustomTheme;
use crate::theme::Entry as EntryStyle;
use crate::theme::TextClass;

#[derive(Debug, Clone)]
pub enum IconState {
    Ready(iced::widget::image::Handle),
    Loading,
    Empty,
    NotFound,
}

impl IconState {
    pub fn status(&self) -> IconStatus {
        match self {
            IconState::Ready(_) => IconStatus::Ready,
            IconState::Loading => IconStatus::Loading,
            IconState::Empty => IconStatus::Empty,
            IconState::NotFound => IconStatus::NotFound,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconStatus {
    Empty,
    Loading,
    Ready,
    NotFound,
}

#[derive(Debug, Clone)]
pub struct App {
    commandline: Option<PathBuf>,
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon_state: IconState,
    pub icon_name: Option<String>, // Change for PathBuf?
}

pub fn all_apps() -> Vec<App> {
    gio::AppInfo::all()
        .iter()
        .filter(|app| app.should_show())
        .map(|app| App {
            id: app.id().unwrap_or_default().to_string(),
            commandline: app.commandline(),
            name: app.name().to_string(),
            description: app.description().map(String::from),
            icon_state: IconState::Empty,
            icon_name: app.icon().and_then(|s| s.to_string()).map(String::from),
        })
        .collect()
}

fn get_icon_path_from_xdgicon(iconname: &str) -> Option<PathBuf> {
    if iconname.contains("/") || iconname.contains("\\") {
        return Some(PathBuf::from(iconname));
    }

    let xdg_dirs = xdg::BaseDirectories::new();

    let sizes = [
        "scalable", "512x512", "256x256", "128x128", "96x96", "64x64", "48x48", "32x32",
    ];

    for size in sizes {
        let extension = if size == "scalable" { "svg" } else { "png" };
        let sub_path = format!("icons/hicolor/{size}/apps/{iconname}.{extension}");

        if let Some(path) = xdg_dirs.find_data_file(&sub_path) {
            return Some(path);
        }
    }

    for ext in ["svg", "png", "ico"] {
        let pixmap_path = format!("pixmaps/{}.{}", iconname, ext);
        if let Some(path) = xdg_dirs.find_data_file(&pixmap_path) {
            return Some(path);
        }
    }

    None
}

fn rasterize_svg(path: PathBuf, size: u32) -> Option<image::Handle> {
    let svg_data = std::fs::read(path).ok()?;
    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default()).ok()?;

    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;
    let transform = tiny_skia::Transform::from_scale(
        size as f32 / tree.size().width(),
        size as f32 / tree.size().height(),
    );

    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(image::Handle::from_rgba(size, size, pixmap.data().to_vec()))
}

fn load_raster_icon(icon: &str) -> Option<image::Handle> {
    let path = get_icon_path_from_xdgicon(&icon)?;
    let extension = path.extension()?.to_str()?;

    match extension {
        "svg" => rasterize_svg(path, 64),
        "png" | "jpg" | "jpeg" => Some(image::Handle::from_path(path)),
        _ => None,
    }
}

pub async fn process_icon(app_index: usize, icon_name: Option<String>) -> (usize, IconState) {
    let Some(name) = icon_name else {
        return (app_index, IconState::Empty);
    };

    match load_raster_icon(&name) {
        Some(handle) => (app_index, IconState::Ready(handle)),
        None => (app_index, IconState::Empty),
    }
}

impl App {
    pub fn launch(&self) -> io::Result<process::Child> {
        let raw_cmd = self.commandline.as_ref().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "No command line found")
        })?;
        let clean_cmd = raw_cmd
            .to_str()
            .unwrap_or("")
            .split_whitespace()
            .filter(|arg| !arg.starts_with('%'))
            .collect::<Vec<_>>()
            .join(" ");
        let mut shell = process::Command::new("sh");

        unsafe {
            shell
                .arg("-c")
                .arg(format!("{} &", clean_cmd))
                .pre_exec(|| {
                    nix::unistd::setsid()
                        .map(|_| ())
                        .map_err(|e| io::Error::new(io::ErrorKind::PermissionDenied, e.desc()))
                });
        }

        shell.spawn()
    }

    pub fn entry(
        &self,
        icons: &BakedIcons,
        style: &EntryStyle,
        index: usize,
        current_index: usize,
        is_favorite: bool,
    ) -> Element<'static, Message, CustomTheme> {
        let is_selected = current_index == index;

        let icon_view: Element<'static, Message, CustomTheme> = match &self.icon_state {
            IconState::Ready(handle) => image(handle.clone())
                .width(style.icon_size)
                .height(style.icon_size)
                .into(),
            IconState::Loading => iced::widget::horizontal_space()
                .width(style.icon_size)
                .height(style.icon_size)
                .into(),
            _ => iced::widget::horizontal_space().width(0).into(),
        };

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
            .push_maybe(is_selected.then(|| mark_favorite))
            .push(shortcut_label)
            .align_y(Alignment::Center);

        let description = self.description.as_ref().map(|desc| {
            text(desc.clone())
                .size(style.secondary_font_size)
                .class(TextClass::SecondaryText)
        });

        button(
            row![
                icon_view,
                iced::widget::column![
                    text(self.name.clone())
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
}
