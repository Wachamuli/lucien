use std::{
    fmt::Write,
    path::{Path, PathBuf},
    process,
};

use gio::prelude::{AppInfoExt, IconExt};

use iced::{Task, widget::image};

use crate::{
    launcher::Message,
    ui::icon::{ICON_EXTENSIONS, ICON_SIZES},
};

use super::{Entry, Provider, load_icon_with_cache, spawn_with_new_session};

#[derive(Debug, Clone, Copy)]
pub struct AppProvider;

impl Provider for AppProvider {
    fn scan(&self, _dir: &Path) -> Vec<Entry> {
        gio::AppInfo::all()
            .iter()
            .filter_map(|app| {
                if !app.should_show() {
                    return None;
                }

                Some(Entry {
                    id: app.commandline()?.to_str()?.to_string(),
                    main: app.name().to_string(),
                    secondary: app.description().map(String::from),
                    icon: app.icon().and_then(|p| p.to_string()).map(PathBuf::from),
                })
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

    fn get_icon(&self, entry: &Entry, size: u32) -> image::Handle {
        let default_icon =
            || image::Handle::from_path("assets/mimetypes/application-x-executable.png");

        let Some(icon_path) = &entry.icon else {
            return default_icon();
        };

        if let Some(xdg_icon_path) = get_icon_path_from_xdgicon(icon_path) {
            load_icon_with_cache(&xdg_icon_path, size).unwrap_or_else(default_icon)
        } else {
            default_icon()
        }
    }
}

pub fn get_icon_path_from_xdgicon(iconname: &Path) -> Option<PathBuf> {
    if iconname.is_absolute() && iconname.exists() {
        return Some(iconname.to_owned());
    }

    let xdg_dirs = xdg::BaseDirectories::new();
    let iconname_str = iconname.to_str()?;
    let mut path_str = String::with_capacity(128);

    write!(path_str, "icons/hicolor/scalable/apps/{}.svg", iconname_str).ok()?;
    if let Some(path) = xdg_dirs.find_data_file(&path_str) {
        return Some(path);
    }

    for size in ICON_SIZES {
        path_str.clear();
        write!(path_str, "icons/hicolor/{}/apps/{}.png", size, iconname_str).ok()?;
        if let Some(path) = xdg_dirs.find_data_file(&path_str) {
            return Some(path);
        }
    }

    for ext in ICON_EXTENSIONS {
        path_str.clear();
        write!(path_str, "pixmaps/{}.{}", iconname_str, ext).ok()?;
        if let Some(path) = xdg_dirs.find_data_file(&path_str) {
            return Some(path);
        }
    }

    None
}
