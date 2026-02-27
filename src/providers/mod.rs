use crate::preferences::Preferences;
use crate::providers::app::AppProvider;
use crate::providers::file::FileProvider;
use crate::ui::entry::Entry;
use std::ffi::OsString;
use std::hash::{Hash, Hasher};
use std::{io, os::unix::process::CommandExt, path::PathBuf, process};

use iced::futures::channel::mpsc::Sender as FuturesSender;
use iced::futures::{SinkExt, Stream};
use iced::{Subscription, Task};

use crate::launcher::Message;

pub mod app;
pub mod file;

pub trait Provider {
    fn scan(request: ScanRequest) -> impl Stream<Item = Message>;
    fn launch(entry: &Entry) -> Task<Message>;
}

#[derive(Debug, Clone, Copy, Hash)]
pub enum ProviderKind {
    App,
    File,
}

impl ProviderKind {
    pub fn launch(&self, entry: &Entry) -> Task<Message> {
        match self {
            ProviderKind::App => AppProvider::launch(entry),
            ProviderKind::File => FileProvider::launch(entry),
        }
    }
}

#[derive(Clone)]
pub struct ScanRequest {
    pub path: PathBuf,
    pub provider: ProviderKind,
    pub preferences: Preferences,
}

impl Hash for ScanRequest {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
        self.provider.hash(state);
    }
}

impl ScanRequest {
    pub fn subscribe(self) -> Subscription<Message> {
        match self.provider {
            ProviderKind::App => Subscription::run_with(self, |ctx| AppProvider::scan(ctx.clone())),
            ProviderKind::File => {
                Subscription::run_with(self, |ctx| FileProvider::scan(ctx.clone()))
            }
        }
    }
}

pub type Id = OsString;

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
    pub fn new(sender: FuturesSender<Message>, capacity: usize) -> Self {
        Self {
            sender,
            // receiver,
            batch: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn start(&mut self) {
        let _ = self
            .sender
            .try_send(Message::ScanEvent(ScannerState::Started));
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

    fn finish(&mut self) {
        self.flush();
        let _ = self
            .sender
            .try_send(Message::ScanEvent(ScannerState::Finished));
    }

    async fn run<F>(request: ScanRequest, sender: FuturesSender<Message>, f: F)
    where
        F: Fn(&ScanRequest, &mut Scanner),
    {
        let mut scanner = Scanner::new(sender, request.preferences.scan_batch_size);
        scanner.start();
        f(&request, &mut scanner);
        scanner.finish();
    }
}

impl Drop for Scanner {
    fn drop(&mut self) {
        self.finish();
    }
}

pub struct AsyncScanner {
    sender: FuturesSender<Message>,
    batch: Vec<Entry>,
    capacity: usize,
}

impl AsyncScanner {
    fn new(sender: FuturesSender<Message>, capacity: usize) -> Self {
        Self {
            sender,
            capacity,
            batch: Vec::with_capacity(capacity),
        }
    }

    async fn start(&mut self) {
        let _ = self
            .sender
            .send(Message::ScanEvent(ScannerState::Started))
            .await;
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

    async fn finish(&mut self) {
        self.flush().await;
        let _ = self
            .sender
            .send(Message::ScanEvent(ScannerState::Finished))
            .await;
    }

    pub async fn run<F>(request: ScanRequest, sender: FuturesSender<Message>, f: F)
    where
        F: AsyncFn(&ScanRequest, &mut AsyncScanner),
    {
        let mut scanner = AsyncScanner::new(sender, request.preferences.scan_batch_size);
        scanner.start().await;
        f(&request, &mut scanner).await;
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
