use std::{
    io,
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{self, Command},
};

use super::Provider;

#[derive(Debug, Clone)]
pub struct File {
    pub path: PathBuf,
    pub is_dir: bool,
}

impl File {
    pub fn launch(&self) -> io::Result<process::Child> {
        let mut shell = Command::new("sh");

        unsafe {
            shell
                .arg("-c")
                .arg(format!("xdg-open {path} &", path = self.path.display()))
                .pre_exec(|| {
                    nix::unistd::setsid()
                        .map(|_| ())
                        .map_err(|e| io::Error::new(io::ErrorKind::PermissionDenied, e))
                });
        }

        shell.spawn()
    }
}

pub struct FileProvider;

impl Provider for FileProvider {
    fn scan() -> Vec<super::AnyEntry> {
        std::fs::read_dir("/home/wachamuli")
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                let path = entry.path();
                let is_dir = path.is_dir();

                super::AnyEntry::FileEntry(File { path, is_dir })
            })
            .collect::<Vec<_>>()
    }
}
