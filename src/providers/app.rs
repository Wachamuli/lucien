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
    fn scan(&self, _dir: &Path) -> Subscription<Message> {
        let stream = iced::stream::channel(100, |mut tx| async move {
            let (sync_sender, sync_receiver) = std::sync::mpsc::channel::<Entry>();
            let _join_handle = tokio::task::spawn_blocking(move || {
                let xdg_dirs = xdg::BaseDirectories::new();
                let apps = gio::AppInfo::all();

                for app in apps {
                    if !app.should_show() {
                        continue;
                    }

                    let icon = app
                        .icon()
                        .and_then(|i| i.to_string())
                        .and_then(|name| get_icon_path_from_xdgicon(&name, &xdg_dirs))
                        .and_then(|path| load_raster_icon(&path, 64))
                        .unwrap_or_else(|| image::Handle::from_bytes(APPLICATION_DEFAULT));

                    let cmd = app
                        .commandline()
                        .map(|c| c.to_string_lossy().into_owned())
                        .unwrap_or_default();

                    let entry = Entry::new(
                        cmd,
                        app.name().to_string(),
                        app.description().map(|d| d.to_string()),
                        icon,
                    );

                    // Send the entry into the bridge
                    if sync_sender.send(entry).is_err() {
                        break; // Receiver hung up (UI closed)
                    }
                }
            });

            // 3. The "Pumping" Loop
            // We convert the synchronous receiver into an async-friendly loop
            loop {
                // Try to fetch an item from the bridge without blocking the async thread
                match sync_receiver.try_recv() {
                    Ok(entry) => {
                        let _ = tx.send(Message::PopulateEntries(entry)).await;
                        // Optional throttle for visual effect
                        // tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // No items yet, yield back to the executor so the UI stays smooth
                        tokio::task::yield_now().await;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        // Scanning thread finished and dropped the sender
                        break;
                    }
                }
            }

            iced::futures::pending!()
        });

        iced::Subscription::run_with_id("app-provider-scan", stream)
    }
    // fn scan(&self, _dir: &Path) -> Subscription<Message> {
    //     let stream = iced::stream::channel(100, |mut tx| async move {
    //         let xdg_dirs = xdg::BaseDirectories::new();

    //         let entries = tokio::task::spawn_blocking(move || {
    //             gio::AppInfo::all()
    //                 .iter()
    //                 .filter_map(|app| {
    //                     if !app.should_show() {
    //                         return None;
    //                     }

    //                     let icon = app
    //                         .icon()
    //                         .and_then(|icon| icon.to_string())
    //                         .and_then(|icon_name| get_icon_path_from_xdgicon(&icon_name, &xdg_dirs))
    //                         .and_then(|path| load_raster_icon(&path, 64))
    //                         .unwrap_or_else(|| image::Handle::from_bytes(APPLICATION_DEFAULT));

    //                     Some(Entry::new(
    //                         app.commandline()?.to_str()?,
    //                         app.name().to_string(),
    //                         app.description(),
    //                         icon,
    //                     ))
    //                 })
    //                 .collect::<Vec<Entry>>()
    //         });

    //         for entry in entries {
    //             let _ = tx.send(Message::PopulateEntries(entry)).await;
    //         }

    //         iced::futures::pending!()
    //     });

    //     iced::Subscription::run_with_id("app-provider-scan", stream)
    // }

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
    // TODO: Maybe change it for Path instead, and then convert it to
    // PathBuf if necessary.
    let path_iconname = PathBuf::from(icon_name);
    if path_iconname.is_absolute() && path_iconname.exists() {
        return Some(path_iconname);
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
