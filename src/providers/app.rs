use std::{
    fmt::Write,
    path::{Path, PathBuf},
    process,
    sync::Arc,
};

use gio::prelude::{AppInfoExt, IconExt};

use iced::{Subscription, Task, futures::SinkExt, widget::image};

use crate::{
    launcher::Message,
    providers::{EntryIcon, load_raster_icon},
    ui::icon::{APPLICATION_DEFAULT, ICON_EXTENSIONS, ICON_SIZES},
};

use super::{Entry, Provider, SCAN_BATCH_SIZE, Scanner, spawn_with_new_session};

#[derive(Debug, Clone, Copy)]
pub struct AppProvider;

impl Provider for AppProvider {
    fn scan(&self, _dir: PathBuf) -> Subscription<Message> {
        let stream = iced::stream::channel(100, |output| async move {
            tokio::task::spawn_blocking(move || {
                let xdg_dirs = Arc::new(xdg::BaseDirectories::new());
                let apps = gio::AppInfo::all()
                    .into_iter()
                    .filter(|app| app.should_show());
                let mut scanner = Scanner::new(output.clone(), SCAN_BATCH_SIZE);

                for app in apps {
                    let id = app
                        .commandline()
                        .map(|c| c.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let icon_name = app.icon().and_then(|i| i.to_string());

                    let icon_type = icon_name
                        .clone()
                        .map(|s| EntryIcon::Lazy(s.to_string()))
                        .unwrap_or_else(|| {
                            EntryIcon::Handle(image::Handle::from_bytes(APPLICATION_DEFAULT))
                        });
                    let entry = Entry::new(
                        id.clone(),
                        app.name().to_string(),
                        app.description().map(|d| d.to_string()),
                        icon_type,
                    );
                    scanner.load(entry);

                    if let Some(icon) = icon_name {
                        let name_str = icon.to_string();
                        let xdg_clone = xdg_dirs.clone();
                        let mut icon_output = output.clone();

                        tokio::spawn(async move {
                            let handle = load_icon(name_str.clone(), xdg_clone).await;
                            let _ = icon_output.send(Message::IconLoaded { id, handle }).await;
                        });
                    };
                }
            });
        });

        iced::Subscription::run_with_id("app-provider-scan", stream)
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

async fn load_icon(name: String, xdg: Arc<xdg::BaseDirectories>) -> image::Handle {
    let handle = tokio::task::spawn_blocking(move || {
        let dirs = xdg;
        let xdg_path = get_icon_path_from_xdgicon(&name, &dirs)?;
        load_raster_icon(&xdg_path, 64)
    })
    .await
    .ok()
    .flatten();

    let final_handle = handle.unwrap_or_else(|| image::Handle::from_bytes(APPLICATION_DEFAULT));
    final_handle
}

pub fn get_icon_path_from_xdgicon(
    icon_name: &str,
    xdg_dirs: &xdg::BaseDirectories,
) -> Option<PathBuf> {
    let path_iconname = Path::new(icon_name);
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
