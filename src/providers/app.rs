use std::fmt::Write;
use std::{
    path::{Path, PathBuf},
    process,
    sync::Arc,
};

use gio::prelude::{AppInfoExt, IconExt};

use iced::futures::{self, StreamExt};
use iced::{Subscription, Task, futures::SinkExt, widget::image};
use resvg::{tiny_skia, usvg};

use crate::providers::{Context, Scanner, ScannerState};
use crate::ui::entry::EntryIcon;
use crate::{
    launcher::Message,
    providers::Id,
    ui::icon::{APPLICATION_DEFAULT, ICON_EXTENSIONS, ICON_SIZES},
};

use super::{Entry, Provider, SCAN_BATCH_SIZE, spawn_with_new_session};

#[derive(Debug, Clone, Copy)]
pub struct AppProvider;

impl Provider for AppProvider {
    fn scan(&self) -> Subscription<Message> {
        Subscription::run(|| {
            iced::stream::channel(100, async move |output| {
                let spawn_handler = tokio::runtime::Handle::current();
                let (tx, mut rx) = iced::futures::channel::mpsc::channel::<Context>(100);

                // Notify UI that we are ready and give it the sender
                let _ = output
                    .clone()
                    .send(Message::ScanEvent(ScannerState::Started(tx)))
                    .await;

                tokio::task::spawn_blocking(move || {
                    let xdg_dirs = Arc::new(xdg::BaseDirectories::new());
                    let mut scanner = Scanner::new(output.clone(), SCAN_BATCH_SIZE);

                    let apps = gio::AppInfo::all()
                        .into_iter()
                        .filter(|app| app.should_show());

                    for app in apps {
                        let meta = AppMetadata::from(app);
                        let entry = Entry::new(
                            meta.id.clone(),
                            meta.name,
                            meta.description,
                            meta.icon.clone(),
                        );
                        if let EntryIcon::Lazy(icon_name) = meta.icon {
                            let output_clone = output.clone();
                            let xdg_dirs_clone = xdg_dirs.clone();
                            spawn_handler.spawn(async move {
                                resolve_icon(meta.id, icon_name, 64, xdg_dirs_clone, output_clone)
                                    .await
                            });
                        }
                        scanner.load(entry);
                    }
                });

                std::future::pending::<()>().await;
            })
        })
    }

    fn launch(&self, id: &str) -> Task<Message> {
        let raw_command_without_placeholders = id
            .split_whitespace()
            .filter(|arg| !arg.starts_with('%'))
            .collect::<Vec<_>>();

        let [binary, args @ ..] = raw_command_without_placeholders.as_slice() else {
            tracing::warn!("Launch failed: provided ID resulted in an empty command.");
            return Task::none();
        };

        let mut command = process::Command::new(binary);
        command.args(args);
        tracing::info!(binary = %binary, args = ?args, "Attempting to launch detached process.");

        if let Err(e) = spawn_with_new_session(&mut command) {
            tracing::error!(error = %e, binary = %binary, "Failed to spawn process.");
            return Task::none();
        }

        tracing::info!(binary = %binary, "Process launched successfully.");
        iced::exit()
    }
}

async fn resolve_icon(
    id: Id,
    name: String,
    size: u32,
    xdg_dirs: Arc<xdg::BaseDirectories>,
    mut output: futures::channel::mpsc::Sender<Message>,
) {
    if let Some(xdg_path) = get_icon_path_from_xdgicon(name, xdg_dirs.clone()).await {
        if let Some(handle) = load_raster_icon(xdg_path, size).await {
            let _ = output.send(Message::IconResolved { id, handle }).await;
            return;
        }
    }

    let _ = output
        .send(Message::IconResolved {
            id,
            handle: APPLICATION_DEFAULT.clone(),
        })
        .await;
}

pub async fn get_icon_path_from_xdgicon(
    icon_name: String,
    xdg_dirs: Arc<xdg::BaseDirectories>,
) -> Option<PathBuf> {
    tokio::task::spawn_blocking(move || {
        let path_iconname = Path::new(&icon_name);
        if path_iconname.is_absolute() && path_iconname.exists() {
            return Some(path_iconname.to_path_buf());
        }

        let mut path_str = String::with_capacity(128);

        write!(path_str, "icons/hicolor/scalable/apps/{}.svg", icon_name).ok()?;
        if let Some(found_path) = xdg_dirs.find_data_file(&path_str) {
            return Some(found_path);
        }

        for size in ICON_SIZES {
            path_str.clear();
            write!(path_str, "icons/hicolor/{}/apps/{}.png", size, icon_name).ok()?;
            if let Some(path) = xdg_dirs.find_data_file(&path_str) {
                return Some(path);
            }
        }

        for ext in ICON_EXTENSIONS {
            path_str.clear();
            write!(path_str, "pixmaps/{}.{}", icon_name, ext).ok()?;
            if let Some(path) = xdg_dirs.find_data_file(&path_str) {
                return Some(path);
            }
        }

        None
    })
    .await
    .ok()
    .flatten()
}
pub struct AppMetadata {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: EntryIcon,
}

impl From<gio::AppInfo> for AppMetadata {
    fn from(app: gio::AppInfo) -> Self {
        let id = app
            .commandline()
            .map(|c| c.to_string_lossy().into_owned())
            .unwrap_or_default();
        let name = app.name().to_string();
        let description = app.description().map(|d| d.to_string());
        let icon_type = app
            .icon()
            .and_then(|i| i.to_string())
            .map(|s| EntryIcon::Lazy(s.to_string()))
            .unwrap_or_else(|| EntryIcon::Handle(APPLICATION_DEFAULT.clone()));

        Self {
            id,
            name,
            description,
            icon: icon_type,
        }
    }
}

async fn rasterize_svg(path: PathBuf, size: u32) -> Option<tiny_skia::Pixmap> {
    tokio::task::spawn_blocking(move || {
        let svg_data = std::fs::read(path).ok()?;
        let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default()).ok()?;

        let mut pixmap = tiny_skia::Pixmap::new(size, size)?;
        let transform = tiny_skia::Transform::from_scale(
            size as f32 / tree.size().width(),
            size as f32 / tree.size().height(),
        );

        resvg::render(&tree, transform, &mut pixmap.as_mut());
        Some(pixmap)
    })
    .await
    .ok()
    .flatten()
}

async fn load_raster_icon(path: PathBuf, size: u32) -> Option<image::Handle> {
    let extension = path.extension()?.to_str()?;

    match extension {
        "svg" => {
            let pixmap = rasterize_svg(path, size).await?;
            Some(image::Handle::from_rgba(size, size, pixmap.data().to_vec()))
        }
        "png" => Some(image::Handle::from_path(path)),
        _ => None,
    }
}
