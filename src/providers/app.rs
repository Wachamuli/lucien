use gio::prelude::{AppInfoExt, IconExt};

use iced::widget::image;
use resvg::{tiny_skia, usvg};
use std::{io, os::unix::process::CommandExt, path::PathBuf, process};

use crate::providers::{Entry, Provider};

#[derive(Debug, Clone)]
pub enum IconState {
    Ready(iced::widget::image::Handle),
    Pending(PathBuf),
    Loading,
    NotFound,
}

impl IconState {
    pub fn hashable(&self) -> IconStatus {
        match self {
            IconState::Ready(_) => IconStatus::Ready,
            IconState::Loading => IconStatus::Loading,
            IconState::Pending(_) => IconStatus::Empty,
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
    pub icon: IconState,
}

fn get_icon_path_from_xdgicon(iconname: &PathBuf) -> Option<PathBuf> {
    if iconname.starts_with("/") || iconname.starts_with("\\") {
        return Some(PathBuf::from(iconname));
    }

    let xdg_dirs = xdg::BaseDirectories::new();

    let sizes = [
        "scalable", "512x512", "256x256", "128x128", "96x96", "64x64", "48x48", "32x32",
    ];

    for size in sizes {
        let extension = if size == "scalable" { "svg" } else { "png" };
        let sub_path = format!(
            "icons/hicolor/{size}/apps/{iconname}.{extension}",
            iconname = iconname.display()
        );

        if let Some(path) = xdg_dirs.find_data_file(&sub_path) {
            return Some(path);
        }
    }

    for ext in ["svg", "png", "ico"] {
        let pixmap_path = format!("pixmaps/{iconname}.{ext}", iconname = iconname.display());
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

fn load_raster_icon(icon: &PathBuf) -> Option<image::Handle> {
    let path = get_icon_path_from_xdgicon(icon)?;
    let extension = path.extension()?.to_str()?;

    match extension {
        "svg" => rasterize_svg(path, 64),
        "png" | "jpg" | "jpeg" => Some(image::Handle::from_path(path)),
        _ => None,
    }
}

pub async fn process_icon(app_index: usize, icon_name: PathBuf) -> (usize, IconState) {
    match load_raster_icon(&icon_name) {
        Some(handle) => (app_index, IconState::Ready(handle)),
        None => (app_index, IconState::NotFound),
    }
}

pub fn launch(app: &App) -> io::Result<process::Child> {
    let raw_cmd = app.commandline.as_ref().ok_or_else(|| {
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

impl Entry for &App {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn main(&self) -> String {
        self.name.clone()
    }

    fn secondary(&self) -> Option<String> {
        self.description.clone()
    }

    fn launch(&self) -> anyhow::Result<()> {
        launch(self);
        Ok(())
    }
}

impl Entry for App {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn main(&self) -> String {
        self.name.clone()
    }

    fn secondary(&self) -> Option<String> {
        self.description.clone()
    }

    fn launch(&self) -> anyhow::Result<()> {
        launch(self);
        Ok(())
    }
}

pub struct AppProvider;

impl Provider for AppProvider {
    type Entry = App;

    fn scan() -> Vec<App> {
        gio::AppInfo::all()
            .iter()
            .filter_map(|app| {
                if !app.should_show() {
                    return None;
                }

                let icon = app.icon().map_or(IconState::NotFound, |i| {
                    let Some(cion) = i.to_string() else {
                        return IconState::NotFound;
                    };

                    IconState::Pending(PathBuf::from(cion))
                });

                Some(App {
                    id: app.id().unwrap_or_default().to_string(),
                    commandline: app.commandline(),
                    name: app.name().to_string(),
                    description: app.description().map(String::from),
                    icon,
                })
            })
            .collect()
    }
}
