use std::{
    fmt::Write,
    path::{Path, PathBuf},
    process,
};

use gio::prelude::{AppInfoExt, IconExt};

use iced::{Subscription, Task, futures::SinkExt, widget::image};

use crate::{
    launcher::Message,
    providers::load_raster_icon,
    ui::icon::{APPLICATION_DEFAULT, ICON_EXTENSIONS, ICON_SIZES},
};

use super::{Entry, Provider, spawn_with_new_session};

#[derive(Debug, Clone, Copy)]
pub struct AppProvider;

impl Provider for AppProvider {
    fn scan(&self, _dir: PathBuf) -> Subscription<Message> {
        let stream = iced::stream::channel(100, |mut output| async move {
            let (sync_sender, mut sync_receiver) = tokio::sync::mpsc::channel::<Message>(100);
            tokio::task::spawn_blocking(move || {
                let xdg_dirs = xdg::BaseDirectories::new();
                let apps = gio::AppInfo::all();
                let _ = sync_sender.blocking_send(Message::ScanStarted);

                for app in apps {
                    if !app.should_show() {
                        continue;
                    }

                    let cmd = app
                        .commandline()
                        .map(|c| c.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let name = app.name().to_string();
                    let description = app.description().map(|d| d.to_string());
                    let icon = app
                        .icon()
                        .and_then(|i| i.to_string())
                        .and_then(|name| get_icon_path_from_xdgicon(&name, &xdg_dirs))
                        .and_then(|path| load_raster_icon(&path, 64))
                        .unwrap_or_else(|| image::Handle::from_bytes(APPLICATION_DEFAULT));

                    let entry = Entry::new(cmd, name, description, icon);

                    if sync_sender.blocking_send(Message::Scan(entry)).is_err() {
                        break;
                    };
                }
            });

            while let Some(entry) = sync_receiver.recv().await {
                let _ = output.send(entry).await;
            }

            let _ = output.send(Message::ScanCompleted).await;
        });

        iced::Subscription::run_with_id("app-provider-scan", stream)
    }

    fn launch(&self, id: &str) -> Task<Message> {
        let raw_command_without_placeholders = id
            .split_whitespace()
            .filter(|arg| !arg.starts_with('%'))
            .collect::<Vec<_>>();

        if let [binary, args @ ..] = raw_command_without_placeholders.as_slice() {
            let mut command = process::Command::new(binary);
            command.args(args);
            tracing::info!(binary = %binary, args = ?args, "Attempting to launch detached process.");

            if let Err(e) = spawn_with_new_session(&mut command) {
                tracing::error!(error = %e, binary = %binary, "Failed to spawn process.");
            } else {
                tracing::info!(binary = %binary, "Process launched successfully.");
            }
        } else {
            tracing::warn!("Launch failed: provided ID resulted in an empty command.");
        }

        iced::exit()
    }
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
