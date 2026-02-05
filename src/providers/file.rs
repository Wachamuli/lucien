use std::{io, os::unix::process::CommandExt, path::PathBuf, process::Command};

use iced::{Task, widget::image};

use crate::{launcher::Message, providers::app::load_icon_with_cache};

use super::{Entry, Provider};

#[derive(Debug, Clone, Copy)]
pub struct FileProvider;

impl Provider for FileProvider {
    fn scan(&self, dir: &PathBuf) -> Vec<Entry> {
        std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();

                Some(Entry::new(
                    path.to_str()?.to_string(),
                    path.file_name()?.to_str()?.to_string(),
                    Some(path.to_str()?.to_string()),
                    Some(path),
                ))
            })
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

        let mut shell = Command::new("sh");

        unsafe {
            shell
                .arg("-c")
                .arg(format!("xdg-open {path} &", path = id))
                .pre_exec(|| {
                    nix::unistd::setsid()
                        .map(|_| ())
                        .map_err(|e| io::Error::new(io::ErrorKind::PermissionDenied, e))
                });
        }

        shell.spawn();

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
