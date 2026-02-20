use crate::preferences::Preferences;
use crate::ui::entry::Entry;
use std::{io, os::unix::process::CommandExt, path::PathBuf, process};

use iced::futures::channel::mpsc::{Receiver as FuturesReceiver, Sender as FuturesSender};
use iced::futures::{SinkExt, StreamExt};
use iced::{Subscription, Task};

use crate::{launcher::Message, providers::app::AppProvider, providers::file::FileProvider};

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
pub struct Context {
    pub path: PathBuf,
    pub scan_batch_size: usize,
    pub pattern: String,
    pub icon_size: u32,
}

#[derive(Debug, Clone)]
pub struct ContextSealed {
    path: PathBuf,
    scan_batch_size: usize,
    pattern: String,
    icon_size: u32,
}

impl Context {
    pub fn create(preferences: &Preferences) -> ContextSealed {
        ContextSealed {
            path: PathBuf::from(env!("HOME")),
            pattern: String::new(),
            scan_batch_size: preferences.scan_batch_size,
            icon_size: preferences.theme.launchpad.entry.icon_size,
        }
    }
}

impl ContextSealed {
    pub fn with_path(&self, path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            ..self.clone()
        }
    }
}

pub trait Provider {
    // TODO: Maybe I should just return the stream, and make the subscription
    // logic in the subscripiton function
    fn scan(&self) -> Subscription<Message>;
    // Maybe, launch could consume self? But I have to get rid of dynamic dispatch first.
    // I could avoid couple clones doing this.
    fn launch(&self, id: &str, context: &ContextSealed) -> Task<Message>;
}

pub type Id = String;

pub async fn request_context(mut sender: FuturesSender<Message>) -> FuturesReceiver<ContextSealed> {
    let (tx, rx) = iced::futures::channel::mpsc::channel::<ContextSealed>(100);
    let _ = sender.send(Message::RequestContext(tx)).await;
    rx
}

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

    async fn run<F>(sender: FuturesSender<Message>, f: F)
    where
        F: Fn(&ContextSealed, &mut Scanner),
    {
        let mut context_rx = request_context(sender.clone()).await;
        let mut scanner_opt: Option<Scanner> = None;
        while let Some(context) = context_rx.next().await {
            let scanner = scanner_opt
                .get_or_insert_with(|| Scanner::new(sender.clone(), context.scan_batch_size));
            scanner.start();
            f(&context, scanner);
            scanner.finish();
        }
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

    pub async fn run<F>(sender: FuturesSender<Message>, f: F)
    where
        F: AsyncFn(&ContextSealed, &mut AsyncScanner),
    {
        let mut context_receiver = request_context(sender.clone()).await;
        let mut scanner_opt: Option<AsyncScanner> = None;
        while let Some(ref context) = context_receiver.next().await {
            let mut scanner = scanner_opt
                .get_or_insert_with(|| AsyncScanner::new(sender.clone(), context.scan_batch_size));
            scanner.start().await;
            f(context, &mut scanner).await;
            scanner.finish().await;
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
