use std::path::PathBuf;
use std::{io, os::unix::process::CommandExt, path::Path, process};

use iced::futures::SinkExt;
use iced::futures::channel::mpsc::Sender as FuturesSender;
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

pub trait Provider {
    // TODO: Maybe I should just return the stream, and make the subscription
    // logic in the subscripiton function
    fn scan(&self, dir: PathBuf) -> Subscription<Message>;
    // Maybe, launch could consume self? But I have to get rid of dynamic dispatch first.
    // I could avoid couple clones doing this.
    fn launch(&self, id: &str) -> Task<Message>;
}

pub type Id = String;

#[derive(Debug, Clone)]
pub enum EntryIcon {
    Lazy(String),
    Handle(iced::widget::image::Handle),
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: Id,
    pub main: String,
    pub secondary: Option<String>,
    pub icon: EntryIcon,
}

impl Entry {
    fn new(
        id: impl Into<String>,
        main: impl Into<String>,
        secondary: Option<impl Into<String>>,
        icon: EntryIcon,
    ) -> Self {
        Self {
            id: id.into(),
            main: main.into(),
            secondary: secondary.map(Into::into),
            icon,
        }
    }
}

// TODO: Move to configuration file
pub const SCAN_BATCH_SIZE: usize = 10;

#[derive(Debug, Clone)]
pub enum ScannerState {
    Started,
    Found(Vec<Entry>),
    Finished,
    Errored(Id, String),
}

struct Scanner {
    sender: FuturesSender<Message>,
    batch: Vec<Entry>,
    capacity: usize,
}

impl Scanner {
    pub fn new(mut sender: FuturesSender<Message>, capacity: usize) -> Self {
        let _ = sender.try_send(Message::ScanEvent(ScannerState::Started));
        Self {
            sender,
            batch: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn load(&mut self, entry: Entry) {
        self.batch.push(entry);

        if self.batch.len() >= self.capacity {
            self.flush()
        }
    }

    fn flush(&mut self) {
        if !self.batch.is_empty() {
            let ready_batch = std::mem::replace(&mut self.batch, Vec::with_capacity(self.capacity));
            let _ = self
                .sender
                .try_send(Message::ScanEvent(ScannerState::Found(ready_batch)));
        }
    }
}

impl Drop for Scanner {
    fn drop(&mut self) {
        self.flush();
        let _ = self
            .sender
            .try_send(Message::ScanEvent(ScannerState::Finished));
    }
}

pub struct AsyncScanner {
    sender: FuturesSender<Message>,
    batch: Vec<Entry>,
    capacity: usize,
}

impl AsyncScanner {
    async fn new(mut sender: FuturesSender<Message>, capacity: usize) -> Self {
        let _ = sender.send(Message::ScanEvent(ScannerState::Started)).await;
        Self {
            sender,
            capacity,
            batch: Vec::with_capacity(capacity),
        }
    }

    async fn load(&mut self, entry: Entry) {
        self.batch.push(entry);

        if self.batch.len() >= self.capacity {
            self.flush().await;
        }
    }

    async fn flush(&mut self) {
        if !self.batch.is_empty() {
            let ready_batch = std::mem::replace(&mut self.batch, Vec::with_capacity(self.capacity));
            let _ = self
                .sender
                .send(Message::ScanEvent(ScannerState::Found(ready_batch)))
                .await;
        }
    }

    async fn finish(mut self) {
        self.flush().await;
        let _ = self
            .sender
            .send(Message::ScanEvent(ScannerState::Finished))
            .await;
    }

    pub async fn run<F>(sender: FuturesSender<Message>, capacity: usize, f: F)
    where
        F: AsyncFnOnce(&mut AsyncScanner),
    {
        let mut scanner = Self::new(sender, capacity).await;
        // TODO: Return a Result and handle errors with the ScanState::Errored variant
        f(&mut scanner).await;
        scanner.finish().await;
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
