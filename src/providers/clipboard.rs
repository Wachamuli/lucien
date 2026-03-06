use std::{
    io::Write,
    process::{Command, Stdio},
};

use iced::{
    Task,
    futures::{Stream, StreamExt},
    window,
};
use sqlx::Connection;

use crate::{
    launcher::Message,
    providers::{Provider, ScanRequest, Scanner},
    ui::entry::Entry,
};

pub struct ClipboardProvider;

impl Provider for ClipboardProvider {
    fn scan(request: ScanRequest) -> impl Stream<Item = Message> {
        iced::stream::channel(100, async move |output| {
            let scan_batch_size = request.preferences.scan_batch_size;
            let mut scanner = Scanner::new(output, scan_batch_size);
            scanner.start().await;
            let mut conn = sqlx::SqliteConnection::connect("/home/wachamuli/clipboard.db")
                .await
                .unwrap();
            let mut entries = sqlx::query_as::<_, Entry>("SELECT * FROM entries").fetch(&mut conn);

            while let Some(Ok(entry)) = entries.next().await {
                scanner.load(entry).await;
            }

            scanner.finish().await;
        })
    }

    fn launch(entry: &Entry) -> Task<Message> {
        let content = entry.id.as_str();
        let command = Command::new("wl-copy")
            .arg("--trim-newline")
            .stdin(Stdio::piped())
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .spawn();

        let Ok(mut child) = command else {
            tracing::error!("");
            return Task::none();
        };

        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(content.as_bytes()) {
                tracing::error!("Failed to write to wl-copy stdin: {}", e);
            }
        }

        window::latest().and_then(window::close)
    }
}
