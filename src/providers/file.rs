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
            // FIXME: Same problem here.
            Entry::new(
                p.to_str().unwrap(),
                "..",
                Some(p.to_string_lossy()),
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

    fn get_icon(&self, entry: &Entry, size: u32) -> image::Handle {
        let default_icon =
            || image::Handle::from_path("assets/mimetypes/application-x-generic.png");

        let Some(icon_path) = &entry.icon else {
            return default_icon();
        };

        if icon_path.is_dir() {
            let dir_icon_path = "assets/mimetypes/inode-directory.svg";
            return load_icon_with_cache(&PathBuf::from(dir_icon_path), size)
                .unwrap_or_else(default_icon);
        }

        let file_extension = entry
            .main
            .rsplit_once('.')
            .map(|(_, ext)| ext.to_lowercase())
            .unwrap_or_default();

        let mimetype = MimeType::get_type_from_extension(&file_extension);
        let mimetype_icon_path = &mimetype.get_icon_from_type();
        load_icon_with_cache(mimetype_icon_path, size).unwrap_or_else(default_icon)
    }
}

#[derive(Debug)]
pub enum MimeType {
    Text,
    Application,
    Image,
    Audio,
    Video,
    Font,
    Multipart,
    Model,
    Unknown,
}

impl MimeType {
    fn get_type_from_extension(ext: &str) -> MimeType {
        match ext {
            "txt" | "md" | "html" | "css" | "csv" => MimeType::Text,
            "json" | "pdf" | "zip" | "wasm" | "xml" => MimeType::Application,
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" => MimeType::Image,
            "mp3" | "wav" | "ogg" | "m4a" => MimeType::Audio,
            "mp4" | "webm" | "avi" | "mov" => MimeType::Video,
            "ttf" | "otf" | "woff" | "woff2" => MimeType::Font,
            "mime" | "mhtml" => MimeType::Multipart,
            "obj" | "stl" | "glb" | "gltf" | "3ds" => MimeType::Model,
            _ => MimeType::Unknown,
        }
    }

    fn get_icon_from_type(&self) -> PathBuf {
        let icon_name = match self {
            MimeType::Text => "text-x-generic.svg",
            MimeType::Application => "application-x-executable.svg",
            MimeType::Image => "image-x-generic.svg",
            MimeType::Audio => "audio-x-generic.svg",
            MimeType::Video => "video-x-generic.svg",
            MimeType::Font => "font-x-generic.svg",
            MimeType::Multipart => "package-x-generic.svg",
            MimeType::Model => "model.svg",
            MimeType::Unknown => "application-x-generic.svg",
        };

        PathBuf::from(format!("assets/mimetypes/{}", icon_name))
    }
}
