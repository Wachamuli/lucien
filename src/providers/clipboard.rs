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

            migrate(&mut conn).await;

            let mut entries =
                sqlx::query_as::<_, Entry>("SELECT * FROM entries ORDER BY created_at DESC")
                    .fetch(&mut conn);

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
async fn migrate(conn: &mut sqlx::SqliteConnection) -> Result<(), sqlx::Error> {
    let mut tx = conn.begin().await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS entries (
            id TEXT PRIMARY KEY,
            main TEXT NOT NULL,
            secondary TEXT,
            icon BLOB,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS limit_entries_to_20
        AFTER INSERT ON entries
        BEGIN
            DELETE FROM entries
            WHERE id NOT IN (
                SELECT id FROM entries
                ORDER BY created_at DESC
                LIMIT 20
            );
        END;
        "#,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn handle_clipboard_insertion(content: &str, package_name: &str) -> anyhow::Result<()> {
    let clipboard_dir = xdg::BaseDirectories::with_prefix(package_name)
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

    migrate(&mut conn).await;

    sqlx::query("INSERT OR IGNORE INTO entries (id, main, secondary, icon) VALUES (?, ?, ?, NULL)")
        .bind(content)
        .bind(content)
        .bind("Text")
        .execute(&mut conn)
        .await?;

    Ok(())
}
