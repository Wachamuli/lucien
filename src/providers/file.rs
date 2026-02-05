use std::{path::PathBuf, process};

use iced::{Task, widget::image};

use crate::launcher::Message;

use super::{Entry, Provider, load_icon_with_cache, spawn_with_new_session};

#[derive(Debug, Clone, Copy)]
pub struct FileProvider;

impl Provider for FileProvider {
    fn scan(&self, dir: &PathBuf) -> Vec<Entry> {
        let child_entries = std::fs::read_dir(dir)
            .map(|entries| {
                entries.filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();

                    // FIXME: Unix-like systems accept non-UTF-8 valid sequences
                    // as valid file names. Right now, these entries are being skip.
                    // In order to fix this, id should be a PathBuf or similar.
                    let id_str = path.to_str()?.to_owned();
                    let main_display = path.file_name()?.to_string_lossy().into_owned();

                    Some(Entry::new(
                        id_str.clone(),
                        main_display,
                        Some(id_str),
                        Some(path),
                    ))
                })
            })
            .into_iter()
            .flatten();

        let parent_dir = dir.parent();

        let parent_dir_entry = parent_dir.map(|p| {
            Entry::new(
                p.to_str().unwrap().to_string(),
                "..".to_string(),
                Some(p.to_str().unwrap().to_string()),
                Some(p.to_path_buf()),
            )
        });

        parent_dir_entry
            .into_iter()
            .chain(child_entries)
            .collect::<Vec<_>>()
    }

    fn launch(&self, id: &str) -> Task<Message> {
        let provider_clone = self.clone();
        let path = PathBuf::from(id);

        if path.is_dir() {
            return Task::perform(
                async move { provider_clone.scan(&path) },
                Message::PopulateEntries,
            );
        }

        let mut command = process::Command::new("xdg-open");
        command.arg(&path);
        tracing::info!(binary = ?command.get_program(), arg = ?path, "Attempting to launch detached process.");

        if let Err(e) = spawn_with_new_session(&mut command) {
            tracing::error!(error = %e, binary = ?command.get_program(), "Failed to spawn process.");
        } else {
            tracing::info!(binary = ?command.get_program(), "Process launched successfully.");
        }

        iced::exit()
    }

    fn get_icon(&self, path: &PathBuf, size: u32) -> Option<image::Handle> {
        let dir_icon_path = "/usr/share/icons/Adwaita/scalable/mimetypes/inode-directory.svg";
        let file_icon_path =
            "/usr/share/icons/Adwaita/scalable/mimetypes/application-x-generic.svg";

        if path.is_dir() {
            return load_icon_with_cache(&PathBuf::from(dir_icon_path), size);
        }

        load_icon_with_cache(&PathBuf::from(file_icon_path), size)
    }
}
