use std::{io, os::unix::process::CommandExt, path::PathBuf, process::Command};

use iced::widget::image;

use crate::providers::app::load_icon_with_cache;

use super::{Entry, Provider};

#[derive(Debug, Clone, Copy)]
pub struct FileProvider;

impl Provider for FileProvider {
    fn scan(&self, dir: &PathBuf) -> Vec<Entry> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                let path = entry.path();

                Entry::new(
                    path.to_str().unwrap_or_default().to_string(),
                    path.file_name()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default()
                        .to_string(),
                    Some(path.to_str().unwrap_or_default().to_string()),
                    Some(path),
                )
            })
            .collect::<Vec<_>>()
    }

    fn launch(&self, id: &str) -> anyhow::Result<()> {
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
        Ok(())
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
