use iced::{
    Task,
    futures::{Stream, StreamExt},
};
use sqlx::Connection;

use crate::{
    launcher::Message,
    providers::{Provider, ScanRequest, Scanner},
    ui::entry::Entry,
};

#[allow(unused)]
struct ClipboardProvider;

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

    fn launch(_entry: &Entry) -> Task<Message> {
        todo!("Copied into clipboard")
    }
}
