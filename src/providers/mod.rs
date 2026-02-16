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

pub async fn request_context(mut sender: FuturesSender<Message>) -> FuturesReceiver<Context> {
    let (tx, rx) = iced::futures::channel::mpsc::channel::<Context>(100);
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

    fn run<F>(sender: FuturesSender<Message>, f: F)
    where
        F: Fn(&Context, &mut Scanner),
    {
        // FIXME: maybe block_on can cause a deadlock
        let context = iced::futures::executor::block_on(async {
            request_context(sender.clone())
                .await
                .select_next_some()
                .await
        });
        let mut scanner = Scanner::new(sender.clone(), context.scan_batch_size);
        // FIXME: maybe block_on can cause a deadlock
        let mut context_rx = iced::futures::executor::block_on(request_context(sender.clone()));

        // FIXME: This loop is blocking the Close on out of focus action
        while let Some(context) = iced::futures::executor::block_on(context_rx.next()) {
            scanner.start();
            f(&context, &mut scanner);
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
        F: AsyncFn(&Context, &mut AsyncScanner),
    {
        // I'm using two channels
        // It's a waste of resources knowing that I'm receiving the
        // same datatype.
        // One here
        let mut context_receiver = request_context(sender.clone()).await;
        let ctx = context_receiver.select_next_some().await;
        let mut scanner = AsyncScanner::new(sender.clone(), ctx.scan_batch_size);
        // Second here
        let mut context_receiver = request_context(sender).await;
        while let Some(ref context) = context_receiver.next().await {
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
