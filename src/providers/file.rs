use std::{
    path::{Path, PathBuf},
    process,
};

use iced::{Subscription, Task, widget::image};

use crate::{
    launcher::Message,
    providers::{AsyncScanner, Context, SCAN_BATCH_SIZE},
    ui::{
        entry::{Entry, EntryIcon},
        icon::{
            APPLICATION_DEFAULT, AUDIO_GENERIC, FOLDER_DEFAULT, FONT_GENERIC, IMAGE_GENERIC,
            MODEL_GENERIC, MULTIPART_GENERIC, TEXT_GENERIC, VIDEO_GENERIC,
        },
    },
};

use super::{Provider, ScannerState, spawn_with_new_session};

#[derive(Debug, Clone, Copy)]
pub struct FileProvider;

impl Provider for FileProvider {
    // FIXME: Unix-like systems accept non-UTF-8 valid sequences
    // as valid file names. Right now, these entries are being skip.
    // In order to fix this, id should be a PathBuf or similar.
    // This funcion call is the culprit: Path::to_str() -> Option<&str>
    fn scan(&self) -> Subscription<Message> {
        iced::Subscription::run(|| {
            iced::stream::channel(100, async |output| {
                AsyncScanner::run(output, SCAN_BATCH_SIZE, async |(ctx, scanner)| {
                    if let Some(parent_directory) = ctx.path.parent() {
                        let parent_entry = Entry::new(
                            parent_directory.to_str().unwrap(),
                            "..",
                            Some(parent_directory.to_string_lossy()),
                            EntryIcon::Handle(get_icon_from_mimetype(
                                &parent_directory,
                                ctx.icon_size,
                            )),
                        );
                        scanner.load(parent_entry).await;
                    }

                    let mut child_directories = tokio::fs::read_dir(ctx.path).await.unwrap();
                    while let Some(child_dir) = child_directories.next_entry().await.unwrap() {
                        let path = child_dir.path();
                        let id_str = path.to_string_lossy();
                        let main_display = path.file_name().unwrap().to_string_lossy();
                        let child_entry = Entry::new(
                            id_str.clone(),
                            main_display,
                            Some(id_str),
                            EntryIcon::Handle(get_icon_from_mimetype(&path, ctx.icon_size)),
                        );
                        scanner.load(child_entry).await;
                    }
                })
                .await
            })
        })
    }

    fn launch(&self, id: &str) -> Task<Message> {
        let path = PathBuf::from(id);

        if path.is_dir() {
            let new_context = Context::with_path(path);
            return Task::done(Message::ScanEvent(ScannerState::ContextChange(new_context)));
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
}

fn get_icon_from_mimetype(path: &Path, size: u32) -> image::Handle {
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
