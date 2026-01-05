use gio::prelude::AppInfoExt;
use gio::{Icon, prelude::IconExt};
use iced::{
    Element, Length,
    widget::{Button, button, image, row, text},
};
use resvg::{tiny_skia, usvg};
use std::{io, os::unix::process::CommandExt, path::PathBuf, process};

use crate::launcher::Message;

#[derive(Debug, Clone)]
pub struct App {
    commandline: Option<PathBuf>,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<iced::widget::image::Handle>,
}

static ENTER: &[u8] = include_bytes!("../assets/enter.png");

pub fn all_apps() -> Vec<App> {
    gio::AppInfo::all()
        .iter()
        .filter(|app| app.should_show())
        .map(|app| App {
            commandline: app.commandline(),
            name: app.name().to_string(),
            description: app.description().map(String::from),
            icon: load_raster_icon(app.icon()),
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

fn load_raster_icon(icon: Option<Icon>) -> Option<image::Handle> {
    let path_str = icon?.to_string()?;
    let path = get_icon_path_from_xdgicon(&path_str)?;
    let extension = path.extension()?.to_str()?;

    match extension {
        "svg" => rasterize_svg(path, 64),
        "png" | "jpg" | "jpeg" => Some(image::Handle::from_path(path)),
        _ => None,
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
                    libc::setsid();
                    Ok(())
                });
        }

        shell.spawn()
    }

    pub fn itemlist<'a>(&'a self, current_index: usize, index: usize) -> Button<'a, Message> {
        let icon_view: Element<Message> = match &self.icon {
            Some(handle) => image(handle.clone()).width(32).height(32).into(),
            None => iced::widget::horizontal_space().width(0).into(),
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
                shortcut_label
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::LaunchApp(index))
        .padding(10)
        .height(58)
        .width(Length::Fill)
    }
}
