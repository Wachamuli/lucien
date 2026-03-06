use std::{
    io::Write,
    process::{Command, Stdio},
};

use iced::{
    Task,
    futures::{Stream, StreamExt},
    window,
};
use sqlx::{Connection, sqlite::SqliteConnectOptions};

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
            let clipboard_dir = xdg::BaseDirectories::with_prefix(env!("CARGO_PKG_NAME"))
                .place_data_file("clipboard.db")
                .unwrap();

            let conn_options = SqliteConnectOptions::new()
                .filename(&clipboard_dir)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
                .create_if_missing(true);
            let mut conn = sqlx::SqliteConnection::connect_with(&conn_options)
                .await
                .unwrap();

            sqlx::query(
                r#"

                CREATE TABLE IF NOT EXISTS entries (
                    id TEXT PRIMARY KEY,
                    main TEXT NOT NULL,
                    secondary TEXT,
                    provider TEXT NOT NULL,
                    icon BLOB
                )
                "#,
            )
            .execute(&mut conn)
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
        let command = Command::new("wl-copy")
            .arg("--trim-newline")
            .stdin(Stdio::piped())
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .spawn();

        let Ok(mut child) = command else {
            tracing::error!(
                "Failed to spawn 'wl-copy'. Make sure 'wl-clipboard' is installed and in your PATH."
            );
            return Task::none();
        };

        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(entry.id.as_bytes()) {
                tracing::error!("Failed to write to wl-copy stdin: {}", e);
            }
        }

        window::latest().and_then(window::close)
    }
}
