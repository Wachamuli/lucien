use iced::futures::SinkExt;
use std::{
    path::{Path, PathBuf},
    process,
};

use iced::{Subscription, Task, widget::image};

use crate::{
    launcher::Message,
    providers::load_raster_icon,
    ui::icon::{
        APPLICATION_DEFAULT, AUDIO_GENERIC, FOLDER_DEFAULT, FONT_GENERIC, IMAGE_GENERIC,
        MODEL_GENERIC, MULTIPART_GENERIC, TEXT_GENERIC, VIDEO_GENERIC,
    },
};

use super::{Entry, Provider, spawn_with_new_session};

#[derive(Debug, Clone, Copy)]
pub struct FileProvider;

impl Provider for FileProvider {
    fn scan(&self, dir: &Path) -> Subscription<Message> {
        let owned_dir = dir.to_path_buf();

        let stream = iced::stream::channel(100, |mut tx| async move {
            let child_entries = std::fs::read_dir(&owned_dir)
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
                            get_icon_from_mimetype(&path, 28),
                        ))
                    })
                })
                .into_iter()
                .flatten();

            let parent_dir = &owned_dir.parent();

            let parent_dir_entry = parent_dir.map(|p| {
                // FIXME: Same problem here.
                Entry::new(
                    p.to_str().unwrap(),
                    "..",
                    Some(p.to_string_lossy()),
                    get_icon_from_mimetype(&p, 28),
                )
            });

            let dirs = parent_dir_entry
                .into_iter()
                .chain(child_entries)
                .collect::<Vec<_>>();

            for dir in dirs {
                let _ = tx.send(Message::Scan(dir)).await;
            }

            iced::futures::pending!()
        });

        iced::Subscription::run_with_id("file-provider-scan", stream)
    }

    fn launch(&self, id: &str) -> Task<Message> {
        let provider_clone = self.clone();
        let path = PathBuf::from(id);

        // if path.is_dir() {
        //     return Task::perform(
        //         async move { provider_clone.scan(&path) },
        //         Message::PopulateEntries,
        //     );
        // }

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
}

fn get_icon_from_mimetype(path: &Path, size: u32) -> image::Handle {
    if path.is_dir() {
        return image::Handle::from_bytes(FOLDER_DEFAULT);
    }

    let file_extension = path
        .to_string_lossy()
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_lowercase())
        .unwrap_or_default();

    let mimetype = MimeType::get_type_from_extension(&file_extension);

    // TODO: Feature to override or add new mimetype icons.
    // load_raster_icon(&mimetype.get_icon_from_type(), size).unwrap_or_else(default_icon)
    mimetype.get_icon_from_type()
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

    fn get_icon_from_type(&self) -> image::Handle {
        let icon_bytes = match self {
            MimeType::Text => TEXT_GENERIC,
            MimeType::Application => APPLICATION_DEFAULT,
            MimeType::Image => IMAGE_GENERIC,
            MimeType::Audio => AUDIO_GENERIC,
            MimeType::Video => VIDEO_GENERIC,
            MimeType::Font => FONT_GENERIC,
            MimeType::Multipart => MULTIPART_GENERIC,
            MimeType::Model => MODEL_GENERIC,
            MimeType::Unknown => TEXT_GENERIC,
        };

        image::Handle::from_bytes(icon_bytes)
    }
}
