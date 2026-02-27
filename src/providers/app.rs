use std::ffi::OsStr;
use std::fmt::Write;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::{
    path::{Path, PathBuf},
    process,
    sync::Arc,
};

use iced::futures::{Stream, StreamExt};
use iced::{Task, futures::SinkExt, widget::image};
use iced::{futures, window};
use resvg::{tiny_skia, usvg};

use crate::providers::{AsyncScanner, ScanRequest};
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
    fn scan(request: ScanRequest) -> impl Stream<Item = Message> {
        iced::stream::channel(100, async move |output| {
            AsyncScanner::run(request, output.clone(), async move |req, scanner| {
                let svg_options = Arc::new(usvg::Options::default());
                let xdg_dirs = Arc::new(xdg::BaseDirectories::new());
                let icon_size = req.preferences.theme.launchpad.entry.icon_size;
                let mut app_stream = discover_apps().await;
                while let Some(app) = app_stream.next().await {
                    let icon = app
                        .icon
                        .map(EntryIcon::Lazy)
                        .unwrap_or_else(|| EntryIcon::Handle(APPLICATION_DEFAULT.clone()));

                    if let EntryIcon::Lazy(icon_name) = icon.clone() {
                        tokio::spawn(resolve_icon(
                            app.exec.clone().into(),
                            icon_name,
                            icon_size,
                            xdg_dirs.clone(),
                            svg_options.clone(),
                            output.clone(),
                        ));
                    }

                    let entry = Entry::new(app.exec, app.name, app.comment, icon);
                    scanner.load(entry).await;
                }

                Ok(())
            })
            .await;
        })
    }

    fn launch(entry: &Entry) -> Task<Message> {
        let bytes = entry.id.clone().into_vec();
        let raw_command_without_placeholders: Vec<&OsStr> = bytes
            .split(|&b| b == b' ')
            .filter(|chunk| !chunk.is_empty() && !chunk.starts_with(b"%"))
            .map(OsStr::from_bytes)
            .collect();

        let [binary, args @ ..] = raw_command_without_placeholders.as_slice() else {
            tracing::warn!("Launch failed: provided ID resulted in an empty command.");
            return Task::none();
        };

        let mut command = process::Command::new(&binary);
        command.args(args);
        tracing::info!(binary = ?binary, args = ?args, "Attempting to launch detached process.");

        if let Err(e) = spawn_with_new_session(&mut command) {
            tracing::error!(error = %e, binary = ?binary, "Failed to spawn process.");
            return Task::none();
        }

        tracing::info!(binary = ?binary, "Process launched successfully.");
        window::latest().and_then(window::close)
    }
}

async fn resolve_icon(
    id: Id,
    name: String,
    size: u32,
    xdg_dirs: Arc<xdg::BaseDirectories>,
    opts: Arc<usvg::Options<'_>>,
    mut output: futures::channel::mpsc::Sender<Message>,
) {
    let handle = get_icon_path_from_xdgicon(name, xdg_dirs)
        .and_then(|path| load_raster_icon(&path, size, opts))
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

fn rasterize_svg(path: &Path, size: u32, opts: &usvg::Options) -> Option<tiny_skia::Pixmap> {
    let svg_data = std::fs::read(path).ok()?;
    let tree = usvg::Tree::from_data(&svg_data, &opts).ok()?;

    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;
    let transform = tiny_skia::Transform::from_scale(
        size as f32 / tree.size().width(),
        size as f32 / tree.size().height(),
    );

    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(pixmap)
}

fn load_raster_icon(path: &Path, size: u32, opts: Arc<usvg::Options>) -> Option<image::Handle> {
    let extension = path.extension()?.to_str()?;

    match extension {
        "svg" => {
            let pixmap = rasterize_svg(path, size, &opts)?;
            Some(image::Handle::from_rgba(size, size, pixmap.data().to_vec()))
        }
        "png" => Some(image::Handle::from_path(path)),
        _ => None,
    }
}

#[derive(Default)]
pub struct App {
    pub name: String,
    pub exec: String,
    pub comment: Option<String>,
    pub icon: Option<String>,
}

async fn discover_apps() -> futures::channel::mpsc::Receiver<App> {
    let (tx, rx) = futures::channel::mpsc::channel(100);
    let xdg_dirs = Arc::new(xdg::BaseDirectories::new());
    let mut search_paths = xdg_dirs.get_data_dirs();
    search_paths.insert(0, xdg_dirs.get_data_home().unwrap_or_default());

    for path in search_paths {
        let app_dir = path.join("applications");
        if let Ok(mut entries) = tokio::fs::read_dir(app_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let file_path = entry.path();
                let file_name = entry.file_name().to_string_lossy().into_owned();

                if !file_name.ends_with(".desktop") {
                    continue;
                }

                let mut tx_clone = tx.clone();
                tokio::spawn(async move {
                    if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
                        if let Some(app) = parse_desktop_entry(&content) {
                            let _ = tx_clone.send(app).await;
                        }
                    }
                });
            }
        }
    }

    rx
}

fn parse_desktop_entry(content: &str) -> Option<App> {
    let mut app = App::default();
    let mut in_main_section = false;

    let mut has_name = false;
    let mut has_exec = false;
    let mut has_type = false;
    let mut has_icon = false;
    let mut has_comment = false;

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') {
            if in_main_section {
                break;
            }
            in_main_section = line == "[Desktop Entry]";
            continue;
        }

        if in_main_section {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "Type" => {
                        if value != "Application" {
                            return None;
                        }
                        has_type = true;
                    }
                    "Hidden" | "NoDisplay" => {
                        if value == "true" {
                            return None;
                        }
                    }
                    "Name" => {
                        app.name = value.to_string();
                        has_name = true;
                    }
                    "Exec" => {
                        app.exec = value.to_string();
                        has_exec = true;
                    }
                    "Icon" => {
                        app.icon = Some(value.to_string());
                        has_icon = true;
                    }
                    "Comment" => {
                        app.comment = Some(value.to_string());
                        has_comment = true;
                    }
                    _ => {}
                }

                if has_name && has_exec && has_type && has_icon && has_comment {
                    return Some(app);
                }
            }
        }
    }

    if has_name && has_exec && has_type {
        Some(app)
    } else {
        None
    }
}
