use crate::ui::entry::Entry;
use std::{io, os::unix::process::CommandExt, path::PathBuf, process};

use iced::futures::channel::mpsc::{Receiver as FuturesReceiver, Sender as FuturesSender};
use iced::futures::{SinkExt, StreamExt};
use iced::{Subscription, Task};

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

#[derive(Debug, Default, Clone)]
pub struct Context {
    pub path: PathBuf,
    pub scan_batch_size: usize,
    pub pattern: String,
    pub icon_size: u32,
}

impl Context {
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            ..Default::default()
        }
    }
}

pub trait Provider {
    // TODO: Maybe I should just return the stream, and make the subscription
    // logic in the subscripiton function
    fn scan(&self) -> Subscription<Message>;
    // Maybe, launch could consume self? But I have to get rid of dynamic dispatch first.
    // I could avoid couple clones doing this.
    fn launch(&self, id: &str) -> Task<Message>;
}

pub type Id = String;

#[derive(Debug, Clone)]
pub enum ScannerState {
    ContextChange(Context),
    Started(FuturesSender<Context>),
    Found(Vec<Entry>),
    Finished,
    Errored(Id, String),
}

struct Scanner {
    sender: FuturesSender<Message>,
    // receiver: FuturesReceiver<Context>,
    batch: Vec<Entry>,
    capacity: usize,
}

impl Scanner {
    pub fn new(mut sender: FuturesSender<Message>, capacity: usize) -> Self {
        // let (tx, receiver) = iced::futures::channel::mpsc::channel::<Context>(100);
        // let _ = sender.try_send(Message::ScanEvent(ScannerState::Started(tx)));
        Self {
            sender,
            // receiver,
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
    receiver: FuturesReceiver<Context>,
    batch: Vec<Entry>,
    capacity: Option<usize>,
}

impl AsyncScanner {
    async fn new(mut sender: FuturesSender<Message>) -> Self {
        println!("New async scanner");
        let (tx, receiver) = iced::futures::channel::mpsc::channel::<Context>(100);
        let _ = sender
            .send(Message::ScanEvent(ScannerState::Started(tx)))
            .await;
        Self {
            sender,
            receiver,
            capacity: None,
            batch: Vec::new(),
        }
    }

    async fn set_capcity(&mut self, capacity: usize) {
        self.capacity = Some(capacity);
    }

    async fn load(&mut self, entry: Entry) {
        self.batch.push(entry);

        if self.batch.len() >= self.capacity.unwrap() {
            self.flush().await;
        }
    }

    async fn flush(&mut self) {
        if !self.batch.is_empty() {
            let ready_batch =
                std::mem::replace(&mut self.batch, Vec::with_capacity(self.capacity.unwrap()));
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
        F: AsyncFn((&Context, &mut AsyncScanner)),
    {
        let mut scanner = AsyncScanner::new(sender).await;
        while let Some(ref context) = scanner.receiver.next().await {
            scanner.set_capcity(context.scan_batch_size).await;
            f((context, &mut scanner)).await;
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
