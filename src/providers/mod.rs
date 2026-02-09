use std::path::PathBuf;
use std::{io, os::unix::process::CommandExt, path::Path, process};

use iced::widget::image;
use iced::{Subscription, Task};
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

#[derive(Debug, Clone)]
pub enum ScanState {
    Start,
    Load(Entry),
    Finish,
}

pub trait Provider {
    // TODO: Maybe I should just return the stream, and make the subscription
    // logic in the subscripiton function
    // Also, Scan can have many states. Scanning, Completed, and Error
    // I can create an enum to handle each case
    fn scan(&self, dir: PathBuf) -> Subscription<Message>;
    // Maybe, launch could consume self? But I have to get rid of dynamic dispatch first.
    // I could avoid couple clones doing this.
    fn launch(&self, id: &str) -> Task<Message>;
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub main: String,
    pub secondary: Option<String>,
    pub icon: iced::widget::image::Handle,
}

impl Entry {
    fn new(
        id: impl Into<String>,
        main: impl Into<String>,
        secondary: Option<impl Into<String>>,
        icon: iced::widget::image::Handle,
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

fn rasterize_svg(path: &Path, size: u32) -> Option<tiny_skia::Pixmap> {
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

// TODO: Maybe I should create my own IconType to distinguish
// between  default and custom icons. I don't want to perform
// any of this logic if the Icon Is a default one.
fn load_raster_icon(path: &Path, size: u32) -> Option<image::Handle> {
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
