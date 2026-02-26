use std::fmt::Write;
use std::{
    path::{Path, PathBuf},
    process,
    sync::Arc,
};

use gio::prelude::{AppInfoExt, IconExt};

use iced::futures::Stream;
use iced::{Task, futures::SinkExt, widget::image};
use iced::{futures, window};
use resvg::{tiny_skia, usvg};

use crate::providers::{ScanRequest, Scanner};
use crate::ui::entry::EntryIcon;
use crate::{
    launcher::Message,
    providers::Id,
    ui::icon::{APPLICATION_DEFAULT, ICON_EXTENSIONS, ICON_SIZES},
};

use super::{Entry, Provider, spawn_with_new_session};

#[derive(Debug, Clone, Copy)]
pub struct AppProvider;

impl Provider for AppProvider {
    fn scan(ctx: ScanRequest) -> impl Stream<Item = Message> {
        iced::stream::channel(100, async move |output| {
            let xdg_dirs = Arc::new(xdg::BaseDirectories::new());
            Scanner::run(ctx, output.clone(), |ctx, scanner| {
                let icon_size = ctx.preferences.theme.launchpad.entry.icon_size;
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
                        tokio::spawn(resolve_icon(
                            meta.id,
                            icon_name,
                            icon_size,
                            xdg_dirs.clone(),
                            output.clone(),
                        ));
                    }
                    scanner.load(entry);
                }
            })
            .await;
        })
    }

    fn launch(entry: &Entry) -> Task<Message> {
        let raw_command_without_placeholders = entry
            .id
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
        window::latest().and_then(window::close)
    }
}

async fn resolve_icon(
    id: Id,
    name: String,
    size: u32,
    xdg_dirs: Arc<xdg::BaseDirectories>,
    mut output: futures::channel::mpsc::Sender<Message>,
) {
    let handle = get_icon_path_from_xdgicon(name, xdg_dirs)
        .and_then(|path| load_raster_icon(path, size))
        .unwrap_or_else(|| APPLICATION_DEFAULT.clone());

    let _ = output.send(Message::IconResolved { id, handle }).await;
}

pub fn get_icon_path_from_xdgicon(
    icon_name: String,
    xdg_dirs: Arc<xdg::BaseDirectories>,
) -> Option<PathBuf> {
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
}

fn rasterize_svg(path: PathBuf, size: u32) -> Option<tiny_skia::Pixmap> {
    let svg_data = std::fs::read(path).ok()?;
    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default()).ok()?;

    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;
    let transform = tiny_skia::Transform::from_scale(
        size as f32 / tree.size().width(),
        size as f32 / tree.size().height(),
    );

    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(pixmap)
}

fn load_raster_icon(path: PathBuf, size: u32) -> Option<image::Handle> {
    let extension = path.extension()?.to_str()?;

    match extension {
        "svg" => {
            let pixmap = rasterize_svg(path, size)?;
            Some(image::Handle::from_rgba(size, size, pixmap.data().to_vec()))
        }
        "png" => Some(image::Handle::from_path(path)),
        _ => None,
    }
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
        let icon = app
            .icon()
            .and_then(|i| i.to_string())
            .map(|s| EntryIcon::Lazy(s.to_string()))
            .unwrap_or_else(|| EntryIcon::Handle(APPLICATION_DEFAULT.clone()));

        Self {
            id,
            name,
            description,
            icon,
        }
    }
}
