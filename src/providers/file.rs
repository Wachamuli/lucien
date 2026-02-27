use std::{
    path::{Path, PathBuf},
    process,
};

use iced::{Task, futures::Stream, widget::image, window};

use crate::{
    launcher::Message,
    providers::{AsyncScanner, ScanRequest},
    ui::{
        entry::{Entry, EntryIcon},
        icon::{
            APPLICATION_DEFAULT, AUDIO_GENERIC, FOLDER_DEFAULT, FONT_GENERIC, IMAGE_GENERIC,
            MODEL_GENERIC, MULTIPART_GENERIC, TEXT_GENERIC, VIDEO_GENERIC,
        },
    },
};

use super::{Provider, spawn_with_new_session};

#[derive(Debug, Clone, Copy)]
pub struct FileProvider;

impl Provider for FileProvider {
    fn scan(request: ScanRequest) -> impl Stream<Item = Message> {
        iced::stream::channel(100, async move |output| {
            AsyncScanner::run(request, output, async move |req, scanner| {
                let icon_size = req.preferences.theme.launchpad.entry.icon_size;
                if let Some(parent_directory) = req.path.parent() {
                    let parent_entry = Entry::new(
                        parent_directory.as_os_str().to_os_string(),
                        "..",
                        Some(parent_directory.to_string_lossy()),
                        EntryIcon::Handle(get_icon_from_mimetype(parent_directory, icon_size)),
                    );
                    scanner.load(parent_entry).await;
                }

                let mut child_directories = tokio::fs::read_dir(&req.path).await.unwrap();
                while let Some(child_dir) = child_directories.next_entry().await.unwrap() {
                    let path = child_dir.path();
                    let main_display = path.file_name().unwrap().to_string_lossy();
                    let child_entry = Entry::new(
                        path.as_os_str().to_os_string(),
                        main_display,
                        Some(path.to_string_lossy()),
                        EntryIcon::Handle(get_icon_from_mimetype(&path, icon_size)),
                    );
                    scanner.load(child_entry).await;
                }
            })
            .await
        })
    }

    fn launch(entry: &Entry) -> Task<Message> {
        let path = PathBuf::from(&entry.id);

        if path.is_dir() {
            return Task::done(Message::ChangePath(path));
        }
        let mut command = process::Command::new("xdg-open");
        command.arg(&path);
        tracing::info!(binary = ?command.get_program(), arg = ?path, "Attempting to launch detached process.");

        if let Err(e) = spawn_with_new_session(&mut command) {
            tracing::error!(error = %e, binary = ?command.get_program(), "Failed to spawn process.");
        } else {
            tracing::info!(binary = ?command.get_program(), "Process launched successfully.");
        }

        window::latest().and_then(window::close)
    }
}

fn get_icon_from_mimetype(path: &Path, _size: u32) -> image::Handle {
    if path.is_dir() {
        return FOLDER_DEFAULT.clone();
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
        match self {
            MimeType::Text => TEXT_GENERIC.clone(),
            MimeType::Application => APPLICATION_DEFAULT.clone(),
            MimeType::Image => IMAGE_GENERIC.clone(),
            MimeType::Audio => AUDIO_GENERIC.clone(),
            MimeType::Video => VIDEO_GENERIC.clone(),
            MimeType::Font => FONT_GENERIC.clone(),
            MimeType::Multipart => MULTIPART_GENERIC.clone(),
            MimeType::Model => MODEL_GENERIC.clone(),
            MimeType::Unknown => TEXT_GENERIC.clone(),
        }
    }
}
