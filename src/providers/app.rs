use std::{
    fmt::Write,
    path::{Path, PathBuf},
    process,
};

use gio::prelude::{AppInfoExt, IconExt};

use iced::{Task, widget::image};

use crate::{
    launcher::Message,
    providers::load_raster_icon,
    ui::icon::{APPLICATION_EXECUTABLE, ICON_EXTENSIONS, ICON_SIZES},
};

use super::{Entry, Provider, spawn_with_new_session};

#[derive(Debug, Clone, Copy)]
pub struct AppProvider;

impl Provider for AppProvider {
    fn scan(&self, _dir: &Path) -> Vec<Entry> {
        let xdg_dirs = xdg::BaseDirectories::new();

        gio::AppInfo::all()
            .iter()
            .filter_map(|app| {
                if !app.should_show() {
                    return None;
                }

                let icon = app
                    .icon()
                    .and_then(|icon| icon.to_string())
                    .and_then(|icon_name| get_icon_path_from_xdgicon(&icon_name, &xdg_dirs))
                    .and_then(|path| load_raster_icon(&path, 64))
                    .unwrap_or_else(|| image::Handle::from_bytes(APPLICATION_EXECUTABLE));

                Some(Entry::new(
                    app.commandline()?.to_str()?,
                    app.name().to_string(),
                    app.description(),
                    icon,
                ))
            })
            .collect()
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
    iconname: &str,
    xdg_dirs: &xdg::BaseDirectories,
) -> Option<PathBuf> {
    let path_iconname = PathBuf::from(iconname);
    if path_iconname.is_absolute() && path_iconname.exists() {
        return Some(path_iconname);
    }

    let mut path_str = String::with_capacity(128);

    write!(path_str, "icons/hicolor/scalable/apps/{}.svg", iconname).ok()?;
    if let Some(found_path) = xdg_dirs.find_data_file(&path_str) {
        return Some(found_path);
    }

    for size in ICON_SIZES {
        path_str.clear();
        write!(path_str, "icons/hicolor/{}/apps/{}.png", size, iconname).ok()?;
        if let Some(path) = xdg_dirs.find_data_file(&path_str) {
            return Some(path);
        }
    }

    for ext in ICON_EXTENSIONS {
        path_str.clear();
        write!(path_str, "pixmaps/{}.{}", iconname, ext).ok()?;
        if let Some(path) = xdg_dirs.find_data_file(&path_str) {
            return Some(path);
        }
    }

    None
}
