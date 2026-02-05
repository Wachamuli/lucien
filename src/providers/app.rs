use std::{path::PathBuf, process};

use gio::prelude::{AppInfoExt, IconExt};

use iced::{Task, widget::image};

use crate::{
    launcher::Message,
    ui::icon::{ICON_EXTENSION, ICON_SIZES},
};

use super::{Entry, Provider, load_icon_with_cache, spawn_with_new_session};

#[derive(Debug, Clone, Copy)]
pub struct AppProvider;

impl Provider for AppProvider {
    fn scan(&self, _dir: &PathBuf) -> Vec<Entry> {
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

    fn get_icon(&self, path: &PathBuf, size: u32) -> Option<image::Handle> {
        let icon_path = get_icon_path_from_xdgicon(path)?;
        load_icon_with_cache(&icon_path, size)
    }
}

pub fn get_icon_path_from_xdgicon(iconname: &PathBuf) -> Option<PathBuf> {
    if iconname.is_absolute() && iconname.exists() {
        return Some(PathBuf::from(iconname));
    }

    let xdg_dirs = xdg::BaseDirectories::new();

    for size in ICON_SIZES {
        let extension = if size == "scalable" { "svg" } else { "png" };
        let sub_path = format!(
            "icons/hicolor/{size}/apps/{iconname}.{extension}",
            iconname = iconname.display()
        );

        if let Some(path) = xdg_dirs.find_data_file(&sub_path) {
            return Some(path);
        }
    }

    for ext in ICON_EXTENSION {
        let pixmap_path = format!("pixmaps/{iconname}.{ext}", iconname = iconname.display());
        if let Some(path) = xdg_dirs.find_data_file(&pixmap_path) {
            return Some(path);
        }
    }

    None
}
