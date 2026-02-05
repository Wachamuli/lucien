use std::{io, os::unix::process::CommandExt, path::PathBuf, process};

use iced::Task;
use iced::widget::image;
use resvg::{tiny_skia, usvg};

use crate::{
    launcher::Message,
    providers::{app::AppProvider, file::FileProvider},
};

pub mod app;
pub mod file;

#[derive(Debug, Clone, Copy)]
pub enum ProviderKind {
    App(AppProvider),
    File(FileProvider),
}

impl ProviderKind {
    // TODO: Replace dynamic dispatch with monomorphization
    pub fn handler(&self) -> &dyn Provider {
        match self {
            ProviderKind::App(p) => p,
            ProviderKind::File(p) => p,
        }
    }
}

pub trait Provider {
    // Maybe this function should return a Task<Message::PopulateEntries>?
    fn scan(&self, dir: &PathBuf) -> Vec<Entry>;
    // Maybe, launch could consume self? But I have to get rid of dynamic dispatch first.
    // I could avoid couple clones doing this.
    fn launch(&self, id: &str) -> Task<Message>;
    fn get_icon(&self, entry: &Entry, size: u32) -> image::Handle;
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub main: String,
    pub secondary: Option<String>,
    pub icon: Option<PathBuf>,
}

impl Entry {
    fn new(
        id: impl Into<String>,
        main: impl Into<String>,
        secondary: Option<impl Into<String>>,
        icon: Option<PathBuf>,
    ) -> Self {
        Self {
            id: id.into(),
            main: main.into(),
            secondary: secondary.map(Into::into),
            icon,
        }
    }
}

fn spawn_with_new_session(command: &mut process::Command) -> io::Result<process::Child> {
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    // SAFETY: We are in the "fork-exec gap".
    // We avoid heap allocation and use only async-signal-safe calls.
    unsafe {
        command.pre_exec(|| {
            nix::unistd::setsid()
                .map(|_| ())
                .map_err(|e| io::Error::from_raw_os_error(e as i32))
        });
    }

    command.spawn()
}

fn rasterize_svg(path: &PathBuf, size: u32) -> Option<tiny_skia::Pixmap> {
    let svg_data = std::fs::read(path).ok()?;
    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default()).ok()?;

    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;
    let transform = tiny_skia::Transform::from_scale(
        size as f32 / tree.size().width(),
        size as f32 / tree.size().height(),
    );

    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(pixmap)
}

fn load_raster_icon(path: &PathBuf, size: u32) -> Option<image::Handle> {
    let extension = path.extension()?.to_str()?;

    match extension {
        "svg" => {
            let pixmap = rasterize_svg(path, size)?;
            Some(image::Handle::from_rgba(size, size, pixmap.data().to_vec()))
        }
        "png" => Some(image::Handle::from_path(path)),
        _ => None,
    }
}

pub fn load_icon_with_cache(path: &PathBuf, size: u32) -> Option<image::Handle> {
    use std::collections::HashMap;
    use std::sync::OnceLock;

    static CACHE: OnceLock<std::sync::Mutex<HashMap<PathBuf, Option<image::Handle>>>> =
        OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()));

    let mut cache = cache.lock().unwrap();

    if let Some(cached) = cache.get(path) {
        return cached.clone();
    }

    let handle = load_raster_icon(path, size);
    cache.insert(path.clone(), handle.clone());
    handle
}
