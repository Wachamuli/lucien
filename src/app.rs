use gio::prelude::AppInfoExt;
use gio::prelude::IconExt;
use iced::Alignment;
use iced::{
    Element, Length,
    widget::{Button, button, image, row, text},
};
use resvg::{tiny_skia, usvg};
use std::{io, os::unix::process::CommandExt, path::PathBuf, process};

use crate::launcher::ITEM_HEIGHT;
use crate::launcher::Message;

static STAR_ACTIVE: &[u8] = include_bytes!("../assets/star-fill.png");
static STAR_INACTIVE: &[u8] = include_bytes!("../assets/star-line.png");

#[derive(Debug, Clone)]
pub enum IconState {
    Ready(iced::widget::image::Handle),
    Loading,
    Empty,
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

static ENTER: &[u8] = include_bytes!("../assets/enter.png");

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

pub async fn process_icon(app_id: String, icon_name: Option<String>) -> (String, IconState) {
    let Some(name) = icon_name else {
        return (app_id, IconState::Empty);
    };

    match load_raster_icon(&name) {
        Some(handle) => (app_id, IconState::Ready(handle)),
        None => (app_id, IconState::Empty),
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

    pub fn itemlist<'a>(
        &'a self,
        current_index: usize,
        index: usize,
        is_favorite: bool,
    ) -> Button<'a, Message> {
        let icon_view: Element<Message> = match &self.icon_state {
            IconState::Ready(handle) => image(handle).width(32).height(32).into(),
            // Maybe add a placeholder?
            IconState::Loading => iced::widget::horizontal_space().width(0).into(),
            IconState::Empty => iced::widget::horizontal_space().width(0).into(),
        };

        use iced::widget::image;
        let shortcut_label: Element<_> = match index {
            n if n == current_index => image(image::Handle::from_bytes(ENTER))
                .width(18)
                .height(18)
                .into(),
            n @ 0..5 => text(format!("Alt+{}", n + 1))
                .size(12)
                .color([1.0, 1.0, 1.0, 0.5])
                .into(),
            _ => text("").into(),
        };

        let is_selected = current_index == index;

        let star = if is_favorite {
            image::Handle::from_bytes(STAR_ACTIVE)
        } else {
            image::Handle::from_bytes(STAR_INACTIVE)
        };

        let mark_favorite = button(image(star).width(18).height(18))
            .on_press(Message::MarkFavorite(index))
            .style(|_, _| iced::widget::button::Style {
                background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
                ..Default::default()
            });

        let actions = row![]
            .push_maybe(is_selected.then(|| mark_favorite))
            .push(shortcut_label)
            .align_y(Alignment::Center);

        let description = self
            .description
            .as_ref()
            .map(|desc| text(desc).size(12).color([1.0, 1.0, 1.0, 0.5]));

        button(
            row![
                icon_view,
                iced::widget::column![
                    text(&self.name)
                        .size(14)
                        .color([0.95, 0.95, 0.95, 1.0])
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
        .padding(10)
        .height(ITEM_HEIGHT)
        .width(Length::Fill)
    }
}
